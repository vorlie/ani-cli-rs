use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use futures_util::future::join_all;
use regex::Regex;
use reqwest::{Client, Method, Response, header};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::{Mutex, RwLock};
use tracing::warn;
use url::Url;

use crate::{
    AniError, CipherMapInfo, CryptoDebugInfo, RequestHeaders, Result, SearchOptions, SearchResult,
    StreamLink, TranslationType,
    cipher::{builtin_cipher_map, decode_url, load_cached, parse_upstream_cipher_map, save_cached},
    crypto::{
        BUILD_ID, CryptoMaterial, LEGACY_BUILD_ID, QUERY_HASH, aa_req, decode_episode_response,
        episode_sources, fallback_material, now_ms, query_hash, xor_key,
    },
    models::{sort_episodes, sort_streams},
};

const SEARCH_GQL: &str = "query( $search: SearchInput $limit: Int $page: Int $translationType: VaildTranslationTypeEnumType $countryOrigin: VaildCountryOriginEnumType ) { shows( search: $search limit: $limit page: $page translationType: $translationType countryOrigin: $countryOrigin ) { edges { _id name availableEpisodes __typename } }}";
const EPISODES_GQL: &str =
    "query ($showId: String!) { show( _id: $showId ) { _id availableEpisodesDetail }}";
const EPISODE_GQL: &str = "query ($showId: String!, $translationType: VaildTranslationTypeEnumType!, $episodeString: String!) { episode( showId: $showId translationType: $translationType episodeString: $episodeString ) { episodeString sourceUrls }}";
const DEFAULT_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0";

#[derive(Clone, Debug)]
pub struct AllAnimeClientBuilder {
    api_url: String,
    base_url: String,
    bootstrap_url: String,
    referer: String,
    user_agent: String,
    timeout: Duration,
    state_dir: Option<PathBuf>,
}

impl Default for AllAnimeClientBuilder {
    fn default() -> Self {
        let state_dir =
            directories::ProjectDirs::from("org", "ani-cli", "ani-cli-rs").map(|dirs| {
                dirs.state_dir()
                    .unwrap_or_else(|| dirs.data_local_dir())
                    .to_path_buf()
            });
        Self {
            api_url: "https://api.mkissa.net/api".into(),
            base_url: "https://allanime.day".into(),
            bootstrap_url: "https://mkissa.to".into(),
            referer: "https://mkissa.to/".into(),
            user_agent: DEFAULT_AGENT.into(),
            timeout: Duration::from_secs(10),
            state_dir,
        }
    }
}

impl AllAnimeClientBuilder {
    pub fn api_url(mut self, value: impl Into<String>) -> Self {
        self.api_url = value.into();
        self
    }
    pub fn base_url(mut self, value: impl Into<String>) -> Self {
        self.base_url = value.into();
        self
    }
    pub fn bootstrap_url(mut self, value: impl Into<String>) -> Self {
        self.bootstrap_url = value.into();
        self
    }
    pub fn referer(mut self, value: impl Into<String>) -> Self {
        self.referer = value.into();
        self
    }
    pub fn timeout(mut self, value: Duration) -> Self {
        self.timeout = value;
        self
    }
    pub fn state_dir(mut self, value: impl Into<PathBuf>) -> Self {
        self.state_dir = Some(value.into());
        self
    }

    pub fn build(self) -> Result<AllAnimeClient> {
        let http = Client::builder()
            .timeout(self.timeout)
            .user_agent(&self.user_agent)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;
        Ok(AllAnimeClient {
            inner: Arc::new(Inner {
                http,
                api_url: self.api_url,
                base_url: self.base_url,
                bootstrap_url: self.bootstrap_url,
                referer: self.referer,
                user_agent: self.user_agent,
                state_dir: self.state_dir,
                crypto: Mutex::new(None),
                cipher_map: RwLock::new(builtin_cipher_map()),
                cipher_loaded: Mutex::new(false),
            }),
        })
    }
}

struct Inner {
    http: Client,
    api_url: String,
    base_url: String,
    bootstrap_url: String,
    referer: String,
    user_agent: String,
    state_dir: Option<PathBuf>,
    crypto: Mutex<Option<CryptoMaterial>>,
    cipher_map: RwLock<HashMap<String, String>>,
    cipher_loaded: Mutex<bool>,
}

#[derive(Clone)]
pub struct AllAnimeClient {
    inner: Arc<Inner>,
}

impl AllAnimeClient {
    pub fn builder() -> AllAnimeClientBuilder {
        AllAnimeClientBuilder::default()
    }
    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    async fn checked(&self, request: reqwest::RequestBuilder) -> Result<Response> {
        let request = request.build()?;
        for attempt in 0..=2 {
            let request = request
                .try_clone()
                .ok_or_else(|| AniError::Input("HTTP request body could not be retried".into()))?;
            let response = self.inner.http.execute(request).await?;
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let retry_after_seconds = response
                    .headers()
                    .get(header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(5)
                    .clamp(1, 30);
                if attempt < 2 {
                    warn!(
                        retry_after_seconds,
                        attempt = attempt + 1,
                        "AllAnime rate limited an HTTP request; waiting before retry"
                    );
                    tokio::time::sleep(Duration::from_secs(retry_after_seconds)).await;
                    continue;
                }
                return Err(AniError::RateLimited {
                    retry_after_seconds,
                });
            }
            if !response.status().is_success() {
                return Err(AniError::GraphQl(format!(
                    "HTTP {} from {}",
                    response.status(),
                    response.url().origin().ascii_serialization()
                )));
            }
            return Ok(response);
        }
        unreachable!("rate-limit retry loop always returns")
    }

    fn common_headers(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .header(header::REFERER, &self.inner.referer)
            .header(header::USER_AGENT, &self.inner.user_agent)
    }

    pub async fn search(&self, query: &str, mode: TranslationType) -> Result<Vec<SearchResult>> {
        self.search_with_options(query, mode, SearchOptions::default())
            .await
    }

    pub async fn search_with_options(
        &self,
        query: &str,
        mode: TranslationType,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let body = json!({
            "variables": {"search":{"allowAdult":options.allow_adult,"allowUnknown":false,"query":query},"limit":40,"page":1,"translationType":mode.to_string(),"countryOrigin":"ALL"},
            "query": SEARCH_GQL,
        });
        let value: Value = self
            .checked(
                self.common_headers(self.inner.http.post(&self.inner.api_url))
                    .json(&body),
            )
            .await?
            .json()
            .await?;
        reject_graphql_errors(&value)?;
        let edges = value
            .pointer("/data/shows/edges")
            .and_then(Value::as_array)
            .ok_or_else(|| AniError::GraphQl("search response had no shows list".into()))?;
        Ok(edges
            .iter()
            .filter_map(|edge| {
                let id = edge.get("_id")?.as_str()?.to_owned();
                let name = edge.get("name")?.as_str()?.to_owned();
                let episodes = edge
                    .pointer(&format!("/availableEpisodes/{mode}"))
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                Some(SearchResult { id, name, episodes })
            })
            .collect())
    }

    pub async fn episodes(&self, show_id: &str, mode: TranslationType) -> Result<Vec<String>> {
        let body = json!({"variables":{"showId":show_id},"query":EPISODES_GQL});
        let value: Value = self
            .checked(
                self.common_headers(self.inner.http.post(&self.inner.api_url))
                    .json(&body),
            )
            .await?
            .json()
            .await?;
        reject_graphql_errors(&value)?;
        let values = value
            .pointer(&format!("/data/show/availableEpisodesDetail/{mode}"))
            .and_then(Value::as_array)
            .ok_or_else(|| AniError::GraphQl("episode response had no episode list".into()))?;
        let mut episodes: Vec<String> = values
            .iter()
            .filter_map(|value| match value {
                Value::String(value) => Some(value.clone()),
                Value::Number(value) => Some(value.to_string()),
                _ => None,
            })
            .collect();
        sort_episodes(&mut episodes);
        Ok(episodes)
    }

    async fn crypto_material(&self, refresh: bool) -> CryptoMaterial {
        let mut cache = self.inner.crypto.lock().await;
        if refresh {
            *cache = None;
        }
        if let Some(material) = cache.as_ref().filter(|m| m.expires_at_ms > now_ms()) {
            return material.clone();
        }
        let material = match self.fetch_dynamic_crypto().await {
            Ok(material) => material,
            Err(error) => {
                warn!(%error, "dynamic AllAnime crypto unavailable; using bundled fallback");
                fallback_material(Some(error.to_string()))
            }
        };
        *cache = Some(material.clone());
        material
    }

    async fn fetch_dynamic_crypto(&self) -> Result<CryptoMaterial> {
        let page = self
            .checked(
                self.inner
                    .http
                    .get(&self.inner.bootstrap_url)
                    .header(header::ACCEPT, "text/html,application/xhtml+xml"),
            )
            .await?
            .text()
            .await?;
        let app_regex = Regex::new(r#"https://cdn\.(?:mkissa\.net|allanime\.day)/all/mk/_app/immutable/entry/app\.[^"']+\.js"#).expect("valid regex");
        let app_js_url = app_regex
            .find(&page)
            .map(|m| m.as_str().to_owned())
            .ok_or_else(|| AniError::Bootstrap("page exposed no app bundle".into()))?;
        let epoch_regex = Regex::new(r#""epoch"\s*:\s*(\d+)"#).expect("valid regex");
        let part_b_regex = Regex::new(r#""partB"\s*:\s*"([^"]+)""#).expect("valid regex");
        let epoch = epoch_regex
            .captures(&page)
            .and_then(|c| c[1].parse().ok())
            .ok_or_else(|| AniError::Bootstrap("page exposed no epoch".into()))?;
        let part_b = part_b_regex
            .captures(&page)
            .map(|c| c[1].to_owned())
            .ok_or_else(|| AniError::Bootstrap("page exposed no Part B".into()))?;
        let app_js = self
            .checked(
                self.inner
                    .http
                    .get(&app_js_url)
                    .header(header::ACCEPT, "text/javascript,*/*;q=0.8"),
            )
            .await?
            .text()
            .await?;
        let api_regex = Regex::new(r#"https://[a-zA-Z0-9.-]+/(?:allanimeapi|api)(?:[\"'])"#)
            .expect("valid regex");
        let discovered_api_url = api_regex
            .find(&app_js)
            .map(|matched| matched.as_str().trim_end_matches(['\"', '\'']).to_owned());
        let chunk_regex = Regex::new(r#"\.\./chunks/[^"',\]]+\.js"#).expect("valid regex");
        let key_regex = Regex::new(r"(?i)([0-9a-f]{64})").expect("valid regex");
        let build_regex = Regex::new(r#"(?i)[0-9a-f]{64}.[^;]*"(\d+)""#).expect("valid regex");
        let mut seen = HashSet::new();
        let mut found = None;
        for capture in chunk_regex.find_iter(&app_js) {
            if !seen.insert(capture.as_str().to_owned()) {
                continue;
            }
            let url = Url::parse(&app_js_url)?.join(capture.as_str())?;
            let Ok(response) = self
                .checked(
                    self.inner
                        .http
                        .get(url)
                        .header(header::ACCEPT, "text/javascript,*/*;q=0.8"),
                )
                .await
            else {
                continue;
            };
            let Ok(chunk) = response.text().await else {
                continue;
            };
            if let (Some(mask), Some(build)) =
                (key_regex.captures(&chunk), build_regex.captures(&chunk))
            {
                found = Some((mask[1].to_owned(), build[1].to_owned()));
                break;
            }
        }
        let (part_a, build_id) = found
            .ok_or_else(|| AniError::Bootstrap("crypto chunk exposed no key material".into()))?;
        let fetched_at_ms = now_ms();
        Ok(CryptoMaterial {
            epoch,
            build_id,
            key: xor_key(&part_a, &part_b)?,
            legacy_ctr: false,
            source: "dynamic".into(),
            part_a,
            part_b,
            app_js_url: Some(app_js_url),
            api_url: discovered_api_url,
            fetched_at_ms,
            expires_at_ms: fetched_at_ms + 30 * 60_000,
            error: None,
        })
    }

    pub async fn crypto_debug(&self, refresh: bool) -> Result<CryptoDebugInfo> {
        let material = self.crypto_material(refresh).await;
        Ok(CryptoDebugInfo {
            source: material.source,
            epoch: material.epoch,
            build_id: material.build_id,
            part_a: material.part_a,
            part_b: material.part_b,
            derived_key_hex: hex::encode(material.key),
            query_hash: query_hash(EPISODE_GQL),
            api_url: material
                .api_url
                .unwrap_or_else(|| self.inner.api_url.clone()),
            referer: self.inner.referer.clone(),
            app_js_url: material.app_js_url,
            fetched_at_unix_ms: material.fetched_at_ms,
            cache_expires_at_unix_ms: material.expires_at_ms,
            legacy_ctr: material.legacy_ctr,
            error: material.error,
        })
    }

    fn material_candidates(dynamic: CryptoMaterial) -> Vec<CryptoMaterial> {
        let mut candidates = vec![dynamic.clone()];
        // During a crypto rollout the bootstrap CDN and GraphQL edge can briefly disagree on
        // which epoch is active. The key does not change when only the epoch changes, so try the
        // adjacent epochs before falling back to bundled, genuinely old build material.
        if dynamic.source == "dynamic" {
            if dynamic.epoch > 0 {
                let mut previous = dynamic.clone();
                previous.epoch -= 1;
                previous.source = "dynamic-previous-epoch".into();
                candidates.push(previous);
            }
            let mut next = dynamic.clone();
            next.epoch = next.epoch.saturating_add(1);
            next.source = "dynamic-next-epoch".into();
            candidates.push(next);
        }
        for build in [BUILD_ID, LEGACY_BUILD_ID] {
            let mut fallback = fallback_material(None);
            fallback.build_id = build.into();
            if !candidates.iter().any(|m| {
                m.epoch == fallback.epoch
                    && m.build_id == fallback.build_id
                    && m.key == fallback.key
            }) {
                candidates.push(fallback);
            }
        }
        candidates
    }

    async fn episode_payload(
        &self,
        show_id: &str,
        episode: &str,
        mode: TranslationType,
    ) -> Result<Value> {
        let variables =
            json!({"showId":show_id,"translationType":mode.to_string(),"episodeString":episode});
        let materials = Self::material_candidates(self.crypto_material(false).await);
        let full_query_hash = query_hash(EPISODE_GQL);
        let mut last_error = "no request attempted".to_owned();
        for material in &materials {
            let api_url = material.api_url.as_deref().unwrap_or(&self.inner.api_url);
            let extensions = json!({"persistedQuery":{"version":1,"sha256Hash":QUERY_HASH},"aaReq":aa_req(material, QUERY_HASH, now_ms())?});
            let response = self
                .common_headers(self.inner.http.get(api_url))
                .header(header::ORIGIN, &self.inner.referer)
                .header("x-build-id", &material.build_id)
                .query(&[
                    ("variables", variables.to_string()),
                    ("extensions", extensions.to_string()),
                ]);
            match self.episode_request(response, material).await {
                Ok(value) => return Ok(value),
                Err(error) => {
                    if let Some(retry_after_seconds) = graphql_rate_limit_seconds(&error) {
                        return Err(AniError::RateLimited {
                            retry_after_seconds,
                        });
                    }
                    last_error = error;
                }
            }
        }
        for material in &materials {
            let api_url = material.api_url.as_deref().unwrap_or(&self.inner.api_url);
            let body = json!({"variables":variables,"query":EPISODE_GQL,"extensions":{"aaReq":aa_req(material, &full_query_hash, now_ms())?}});
            let response = self
                .common_headers(self.inner.http.post(api_url))
                .header("x-build-id", &material.build_id)
                .json(&body);
            match self.episode_request(response, material).await {
                Ok(value) => return Ok(value),
                Err(error) => {
                    if let Some(retry_after_seconds) = graphql_rate_limit_seconds(&error) {
                        return Err(AniError::RateLimited {
                            retry_after_seconds,
                        });
                    }
                    last_error = error;
                }
            }
        }
        Err(AniError::Unavailable(format!(
            "episode {episode} sources could not be decoded: {last_error}"
        )))
    }

    async fn episode_request(
        &self,
        request: reqwest::RequestBuilder,
        material: &CryptoMaterial,
    ) -> std::result::Result<Value, String> {
        for attempt in 0..=2 {
            let Some(request) = request.try_clone() else {
                return Err("episode request body could not be retried".into());
            };
            let response = self
                .checked(request)
                .await
                .map_err(|error| error.to_string())?;
            let raw = response.text().await.map_err(|error| error.to_string())?;
            let value =
                decode_episode_response(&raw, material).map_err(|error| error.to_string())?;
            if episode_sources(&value).is_some() {
                return Ok(value);
            }

            let error =
                graphql_error_text(&value).unwrap_or_else(|| "response had no sources".into());
            if let Some(retry_after_seconds) = graphql_rate_limit_seconds(&error)
                && attempt < 2
            {
                warn!(
                    retry_after_seconds,
                    attempt = attempt + 1,
                    "AllAnime returned a GraphQL rate limit; waiting before retry"
                );
                tokio::time::sleep(Duration::from_secs(retry_after_seconds)).await;
                continue;
            }
            return Err(error);
        }
        unreachable!("GraphQL rate-limit retry loop always returns")
    }

    async fn ensure_cipher_map(&self) {
        let mut loaded = self.inner.cipher_loaded.lock().await;
        if *loaded {
            return;
        }
        if let Some(path) = self.cipher_path()
            && let Some(info) = load_cached(&path).await
        {
            *self.inner.cipher_map.write().await = info.cipher_map;
        }
        *loaded = true;
    }

    fn cipher_path(&self) -> Option<PathBuf> {
        self.inner
            .state_dir
            .as_ref()
            .map(|dir| dir.join("ciphermap.json"))
    }

    pub async fn refresh_cipher_map(&self) -> Result<CipherMapInfo> {
        #[derive(Deserialize)]
        struct Release {
            tag_name: String,
        }
        let release: Release = self
            .checked(
                self.inner
                    .http
                    .get("https://api.github.com/repos/pystardust/ani-cli/releases/latest"),
            )
            .await?
            .json()
            .await?;
        let raw_url = format!(
            "https://raw.githubusercontent.com/pystardust/ani-cli/{}/ani-cli",
            release.tag_name
        );
        let content = self
            .checked(self.inner.http.get(raw_url))
            .await?
            .text()
            .await?;
        let cipher_map = parse_upstream_cipher_map(&content)?;
        let info = CipherMapInfo {
            source: format!("github:pystardust/ani-cli@{}", release.tag_name),
            tag: Some(release.tag_name),
            generated_at_unix_ms: now_ms(),
            entries: cipher_map.len(),
            cipher_map,
        };
        if let Some(path) = self.cipher_path() {
            save_cached(&path, &info).await?;
        }
        *self.inner.cipher_map.write().await = info.cipher_map.clone();
        *self.inner.cipher_loaded.lock().await = true;
        Ok(info)
    }

    pub async fn streams(
        &self,
        show_id: &str,
        episode: &str,
        mode: TranslationType,
    ) -> Result<Vec<StreamLink>> {
        self.ensure_cipher_map().await;
        let payload = self.episode_payload(show_id, episode, mode).await?;
        let mut source_values = episode_sources(&payload)
            .cloned()
            .ok_or_else(|| AniError::Provider("episode response had no source list".into()))?;
        if let Value::String(encoded) = &source_values {
            source_values = serde_json::from_str(encoded)?;
        }
        let sources = source_values
            .as_array()
            .ok_or_else(|| AniError::Provider("episode source list was not an array".into()))?;
        let map = self.inner.cipher_map.read().await.clone();
        let tasks = sources.iter().filter_map(|source| {
            let source_url = source.get("sourceUrl")?.as_str()?.to_owned();
            let source_name = source
                .get("sourceName")
                .and_then(Value::as_str)
                .unwrap_or("Default")
                .to_owned();
            let client = self.clone();
            let map = map.clone();
            Some(async move {
                client
                    .resolve_source(&decode_url(&source_url, &map), &source_name)
                    .await
            })
        });
        let mut streams = Vec::new();
        for result in join_all(tasks).await {
            match result {
                Ok(mut links) => streams.append(&mut links),
                Err(error) => warn!(%error, "skipped AllAnime source"),
            }
        }
        let mut seen = HashSet::new();
        streams.retain(|stream| {
            seen.insert((
                stream.url.clone(),
                stream.provider.clone(),
                stream.resolution.clone(),
            ))
        });
        sort_streams(&mut streams);
        if streams.is_empty() {
            return Err(AniError::Unavailable(format!(
                "episode {episode} is released but no supported sources resolved"
            )));
        }
        Ok(streams)
    }

    fn stream(
        &self,
        url: String,
        resolution: String,
        hls: bool,
        provider: &str,
        referer: Option<String>,
    ) -> StreamLink {
        StreamLink {
            url,
            resolution,
            hls,
            provider: provider.into(),
            downloadable: true,
            headers: RequestHeaders {
                referer,
                origin: Some(self.inner.referer.clone()),
                extra: Default::default(),
            },
        }
    }

    async fn resolve_source(&self, source_url: &str, provider: &str) -> Result<Vec<StreamLink>> {
        let source_url = source_url.trim();
        if source_url.is_empty() {
            return Ok(vec![]);
        }
        if Regex::new(r"(?i)^https?://(?:www\.)?mp4upload\.com/")
            .unwrap()
            .is_match(source_url)
        {
            let html = self
                .checked(
                    self.common_headers(self.inner.http.get(source_url))
                        .header(header::ACCEPT, "text/html,application/xhtml+xml"),
                )
                .await?
                .text()
                .await?;
            let regex = Regex::new(r#"(?i)\bsrc\s*:\s*["']([^"']+)["']"#).unwrap();
            let Some(url) = regex
                .captures(&html)
                .map(|c| c[1].replace(r"\/", "/").replace(r"\u0026", "&"))
            else {
                return Ok(vec![]);
            };
            return Ok(vec![self.stream(
                url.clone(),
                "Auto".into(),
                url.contains(".m3u8"),
                "Mp4Upload",
                Some("https://www.mp4upload.com".into()),
            )]);
        }
        if source_url.starts_with("http://") || source_url.starts_with("https://") {
            if !is_direct_media(source_url) {
                return Ok(vec![]);
            }
            if source_url.contains("tools.fast4speed.rsvp") && !self.is_reachable(source_url).await
            {
                return Ok(vec![]);
            }
            if source_url.contains(".m3u8") {
                return self
                    .expand_hls(source_url, provider, Some(self.inner.referer.clone()))
                    .await;
            }
            return Ok(vec![self.stream(
                source_url.into(),
                "Auto".into(),
                false,
                provider,
                Some(self.inner.referer.clone()),
            )]);
        }
        if !(source_url.starts_with("/apivtwo/") || source_url.starts_with("/apiv2/")) {
            return Ok(vec![]);
        }
        let clock = source_url.replace("/clock", "/clock.json");
        let endpoint = format!("{}{}", self.inner.base_url.trim_end_matches('/'), clock);
        let value: Value = self
            .checked(
                self.common_headers(self.inner.http.get(endpoint))
                    .header(header::ORIGIN, &self.inner.referer)
                    .header(header::ACCEPT, "application/json, text/plain, */*"),
            )
            .await?
            .json()
            .await?;
        let links = value
            .get("links")
            .and_then(Value::as_array)
            .ok_or_else(|| AniError::Provider("clock response had no links".into()))?;
        let mut result = Vec::new();
        for item in links {
            let Some(url) = item.get("link").and_then(Value::as_str) else {
                continue;
            };
            let resolution = item
                .get("resolutionStr")
                .and_then(Value::as_str)
                .unwrap_or("Auto");
            let hls =
                item.get("hls").and_then(Value::as_bool).unwrap_or(false) || url.contains(".m3u8");
            if url.contains("repackager.wixmp.com") {
                result.extend(self.expand_wix(url, provider));
            } else if hls {
                result.extend(
                    self.expand_hls(url, provider, Some(self.inner.referer.clone()))
                        .await
                        .unwrap_or_else(|_| {
                            vec![self.stream(
                                url.into(),
                                resolution.into(),
                                true,
                                provider,
                                Some(self.inner.referer.clone()),
                            )]
                        }),
                );
            } else {
                result.push(self.stream(
                    url.into(),
                    resolution.into(),
                    false,
                    provider,
                    Some(self.inner.referer.clone()),
                ));
            }
        }
        Ok(result)
    }

    async fn is_reachable(&self, url: &str) -> bool {
        self.inner
            .http
            .request(Method::GET, url)
            .header(header::RANGE, "bytes=0-0")
            .header(header::REFERER, &self.inner.referer)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    fn expand_wix(&self, url: &str, provider: &str) -> Vec<StreamLink> {
        let base = Regex::new(r"repackager\.wixmp\.com/")
            .unwrap()
            .replace(url, "");
        let base = Regex::new(r"\.urlset.*")
            .unwrap()
            .replace(&base, "")
            .into_owned();
        let regex = Regex::new(r",([^/]*),/mp4").unwrap();
        let Some(qualities) = regex.captures(url).map(|c| c[1].to_owned()) else {
            return vec![self.stream(
                url.into(),
                "Auto".into(),
                false,
                provider,
                Some(self.inner.referer.clone()),
            )];
        };
        let replace = Regex::new(r",[^/]*").unwrap();
        qualities
            .split(',')
            .map(|quality| {
                self.stream(
                    replace.replace(&base, quality).into_owned(),
                    quality.into(),
                    false,
                    provider,
                    Some(self.inner.referer.clone()),
                )
            })
            .collect()
    }

    async fn expand_hls(
        &self,
        url: &str,
        provider: &str,
        referer: Option<String>,
    ) -> Result<Vec<StreamLink>> {
        let mut request = self.inner.http.get(url);
        if let Some(value) = &referer {
            request = request.header(header::REFERER, value);
        }
        let text = self.checked(request).await?.text().await?;
        if !text.starts_with("#EXTM3U") {
            return Ok(vec![self.stream(
                url.into(),
                "Auto".into(),
                true,
                provider,
                referer,
            )]);
        }
        let base = Url::parse(url)?;
        let mut streams = Vec::new();
        let mut lines = text.lines();
        let resolution_regex = Regex::new(r"RESOLUTION=\d+x(\d+)").unwrap();
        while let Some(line) = lines.next() {
            if !line.starts_with("#EXT-X-STREAM-INF:") {
                continue;
            }
            let resolution = resolution_regex
                .captures(line)
                .map(|c| format!("{}p", &c[1]))
                .unwrap_or_else(|| "Auto".into());
            if let Some(path) = lines
                .by_ref()
                .find(|line| !line.starts_with('#') && !line.trim().is_empty())
            {
                streams.push(self.stream(
                    base.join(path.trim())?.to_string(),
                    resolution,
                    true,
                    provider,
                    referer.clone(),
                ));
            }
        }
        if streams.is_empty() {
            streams.push(self.stream(url.into(), "Auto".into(), true, provider, referer));
        }
        Ok(streams)
    }
}

fn is_direct_media(url: &str) -> bool {
    let url = url.to_ascii_lowercase();
    [
        "tools.fast4speed.rsvp",
        ".m3u8",
        ".mp4",
        "/videoplayback",
        "video.wixstatic.com/video/",
    ]
    .iter()
    .any(|needle| url.contains(needle))
}

fn graphql_error_text(value: &Value) -> Option<String> {
    value
        .get("errors")?
        .as_array()?
        .iter()
        .filter_map(|error| error.get("message")?.as_str())
        .next()
        .map(str::to_owned)
}

fn graphql_rate_limit_seconds(message: &str) -> Option<u64> {
    let lower = message.to_ascii_lowercase();
    if !lower.contains("too many requests") && !lower.contains("rate limit") {
        return None;
    }
    let seconds = Regex::new(r"(?i)(\d+)\s*seconds?")
        .expect("valid rate-limit regex")
        .captures(message)
        .and_then(|capture| capture[1].parse::<u64>().ok())
        .unwrap_or(5);
    Some(seconds.clamp(1, 30))
}

fn reject_graphql_errors(value: &Value) -> Result<()> {
    if let Some(error) = graphql_error_text(value) {
        Err(AniError::GraphQl(error))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{AllAnimeClient, graphql_rate_limit_seconds};

    #[test]
    fn extracts_graphql_rate_limit_delay() {
        assert_eq!(
            graphql_rate_limit_seconds("Too many requests, please try again in 5 seconds."),
            Some(5)
        );
        assert_eq!(graphql_rate_limit_seconds("ordinary GraphQL error"), None);
    }

    #[test]
    fn expands_wix_qualities_without_a_leading_comma() {
        let client = AllAnimeClient::new().unwrap();
        let links = client.expand_wix(
            "https://repackager.wixmp.com/video.wixstatic.com/video/id/,360p,720p,1080p,/mp4/file.mp4.urlset/master.m3u8",
            "Default",
        );
        assert_eq!(
            links[2].url,
            "https://video.wixstatic.com/video/id/1080p/mp4/file.mp4"
        );
    }
}
