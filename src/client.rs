use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use futures_util::future::join_all;
use p256::{
    ecdsa::{Signature, SigningKey, signature::Signer},
    elliptic_curve::rand_core::{OsRng, RngCore},
};
use regex::Regex;
use reqwest::{Client, Method, RequestBuilder, Response, header};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, warn};
use url::Url;

use crate::{
    AniError, CipherMapInfo, CryptoDebugInfo, RequestHeaders, Result, SearchOptions, SearchResult,
    StreamLink, TranslationType,
    cipher::{builtin_cipher_map, decode_url, load_cached, parse_upstream_cipher_map, save_cached},
    crypto::{
        CryptoMaterial, LEGACY_BUILD_IDS, QUERY_HASH, aa_req, decode_episode_response,
        episode_sources, fallback_material, legacy_fallback_material, now_ms, query_hash, xor_key,
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
    provider_referer: String,
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
            provider_referer: "https://youtu-chan.com/".into(),
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
    pub fn provider_referer(mut self, value: impl Into<String>) -> Self {
        self.provider_referer = value.into();
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
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;
        Ok(AllAnimeClient {
            inner: Arc::new(Inner {
                http,
                api_url: self.api_url,
                base_url: self.base_url,
                bootstrap_url: self.bootstrap_url,
                referer: self.referer,
                provider_referer: self.provider_referer,
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
    provider_referer: String,
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
                Some(SearchResult {
                    id,
                    name,
                    episodes,
                    provider: crate::CatalogProvider::AllAnime,
                })
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
        let fallback_candidates = std::iter::once(fallback_material(None))
            .chain(LEGACY_BUILD_IDS.into_iter().map(legacy_fallback_material));
        for fallback in fallback_candidates {
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
                }
            }

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
                let decoded_url = decode_url(&source_url, &map);
                debug!(
                    %source_name,
                    source_shape = %source_shape(&decoded_url),
                    "resolving AllAnime source"
                );
                let result = client.resolve_source(&decoded_url, &source_name).await;
                (source_name, result)
            })
        });
        let mut streams = Vec::new();
        let mut failures = Vec::new();
        for (provider, result) in join_all(tasks).await {
            match result {
                Ok(mut links) if !links.is_empty() => streams.append(&mut links),
                Ok(_) => failures.push(format!("{provider} (unsupported or empty response)")),
                Err(error) => {
                    let kind = provider_error_kind(&error);
                    warn!(%provider, %kind, "skipped AllAnime source");
                    failures.push(format!("{provider} ({kind})"));
                }
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
            let details = if failures.is_empty() {
                "no source entries contained a URL".to_owned()
            } else {
                failures.join(", ")
            };
            return Err(AniError::Unavailable(format!(
                "episode {episode} is released but no supported sources resolved; extracted {} source entries: {details}",
                sources.len()
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
        let origin = referer
            .as_deref()
            .and_then(request_origin)
            .or_else(|| request_origin(&self.inner.provider_referer));
        StreamLink {
            url,
            resolution,
            hls,
            provider: provider.into(),
            downloadable: true,
            headers: RequestHeaders {
                referer,
                origin,
                extra: Default::default(),
            },
            subtitles: vec![],
        }
    }

    async fn resolve_source(&self, source_url: &str, provider: &str) -> Result<Vec<StreamLink>> {
        let source_url = source_url.trim();
        if source_url.is_empty() {
            return Ok(vec![]);
        }
        let source_url = if source_url.starts_with("//") {
            Cow::Owned(format!("https:{source_url}"))
        } else {
            Cow::Borrowed(source_url)
        };
        let source_url = source_url.as_ref();
        if Regex::new(r"(?i)^https?://(?:www\.)?mp4upload\.com/")
            .unwrap()
            .is_match(source_url)
        {
            let html = self
                .checked(
                    self.common_headers(self.inner.http.get(source_url))
                        .header(header::REFERER, &self.inner.provider_referer)
                        .header(header::ACCEPT, "text/html,application/xhtml+xml"),
                )
                .await?
                .text()
                .await?;
            let regex = Regex::new(r#"(?i)\bsrc\s*:\s*["']([^"']+)["']"#).unwrap();
            let embedded_urls = embedded_media_urls(&html);
            debug!(
                provider = "Mp4Upload",
                response_bytes = html.len(),
                embedded_media_count = embedded_urls.len(),
                has_packed_script = html.contains("eval(function(p,a,c,k,e,d)"),
                "inspected provider page"
            );
            let url = regex
                .captures(&html)
                .map(|c| c[1].replace(r"\/", "/").replace(r"\u0026", "&"))
                .or_else(|| embedded_urls.into_iter().next());
            let Some(url) = url else {
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
        if let Some(video_id) = okru_video_id(source_url) {
            let embed_url = format!("https://ok.ru/videoembed/{video_id}");
            let html = self
                .checked(
                    self.inner
                        .http
                        .get(&embed_url)
                        .header(header::REFERER, &self.inner.provider_referer)
                        .header(header::ACCEPT, "text/html,application/xhtml+xml"),
                )
                .await?
                .text()
                .await?;
            let mut links = Vec::new();
            let options = okru_player_options(&html);
            debug!(
                provider = "OK.ru",
                response_bytes = html.len(),
                has_data_options = html.contains("data-options"),
                parsed_player_options = options.is_some(),
                "inspected provider page"
            );
            if html.contains("copyrightsRestricted") || html.contains("data-movie-id=\"null\"") {
                return Err(AniError::Unavailable(
                    "OK.ru reports that the video is blocked or unavailable".into(),
                ));
            }
            if let Some(options) = options {
                if let Some(metadata) = okru_embedded_metadata(&options) {
                    links.extend(okru_links(&metadata));
                } else if let Some((metadata_url, location)) = okru_metadata_request(&options) {
                    let mut body = url::form_urlencoded::Serializer::new(String::new());
                    if let Some(location) = location {
                        body.append_pair("st.location", &location);
                    }
                    let response = self
                        .checked(
                            self.inner
                                .http
                                .post(metadata_url)
                                .header(header::REFERER, &embed_url)
                                .header(header::ACCEPT, "application/json, text/plain, */*")
                                .header(
                                    header::CONTENT_TYPE,
                                    "application/x-www-form-urlencoded; charset=UTF-8",
                                )
                                .body(body.finish()),
                        )
                        .await?;
                    if let Ok(metadata) = response.json::<Value>().await {
                        links.extend(okru_links(&metadata));
                    }
                }
            }
            if links.is_empty() {
                links.extend(embedded_media_urls(&html).into_iter().map(|url| ClockLink {
                    hls: url.to_ascii_lowercase().contains(".m3u8"),
                    url,
                    resolution: "Auto".into(),
                }));
            }
            return Ok(links
                .into_iter()
                .map(|link| {
                    self.stream(
                        link.url,
                        link.resolution,
                        link.hls,
                        "OK.ru",
                        Some("https://ok.ru/".into()),
                    )
                })
                .collect());
        }
        if is_filemoon_provider(source_url) {
            return self.resolve_filemoon(source_url, provider).await;
        }
        if is_embedded_video_provider(source_url) {
            let html = self
                .checked(
                    self.inner
                        .http
                        .get(source_url)
                        .header(header::REFERER, &self.inner.provider_referer)
                        .header(header::ACCEPT, "text/html,application/xhtml+xml"),
                )
                .await?
                .text()
                .await?;
            let mut urls = embedded_media_urls(&html);
            let frames = embedded_frame_urls(&html, source_url);
            for frame_url in frames.iter().take(3) {
                if let Ok(response) = self
                    .checked(
                        self.inner
                            .http
                            .get(frame_url)
                            .header(header::REFERER, source_url)
                            .header(header::ACCEPT, "text/html,application/xhtml+xml"),
                    )
                    .await
                    && let Ok(frame_html) = response.text().await
                {
                    urls.extend(embedded_media_urls(&frame_html));
                }
            }
            debug!(
                provider,
                response_bytes = html.len(),
                embedded_media_count = urls.len(),
                embedded_frame_count = frames.len(),
                page_title = html_title(&html).unwrap_or_default(),
                has_packed_script = html.contains("eval(function(p,a,c,k,e,d)"),
                has_filemoon_api = html.contains("/api/videos/"),
                has_adblocker_payload = html.contains("window.ADBLOCKER"),
                has_cloudflare_challenge = html.contains("challenge-platform"),
                appears_missing = html.to_ascii_lowercase().contains("file not found")
                    || html.to_ascii_lowercase().contains("video not found")
                    || html.to_ascii_lowercase().contains("file was deleted"),
                "inspected embedded provider page"
            );
            return Ok(urls
                .into_iter()
                .map(|url| {
                    let hls = url.to_ascii_lowercase().contains(".m3u8");
                    self.stream(url, "Auto".into(), hls, provider, Some(source_url.into()))
                })
                .collect());
        }
        let internal_path = internal_clock_path(source_url, &self.inner.base_url);
        if internal_path.is_none()
            && (source_url.starts_with("http://") || source_url.starts_with("https://"))
        {
            if !is_direct_media(source_url) {
                return Ok(vec![]);
            }
            if source_url.contains("tools.fast4speed.rsvp") && !self.is_reachable(source_url).await
            {
                return Ok(vec![]);
            }
            if source_url.contains(".m3u8") {
                return self
                    .expand_hls(
                        source_url,
                        provider,
                        Some(self.inner.provider_referer.clone()),
                    )
                    .await;
            }
            return Ok(vec![self.stream(
                source_url.into(),
                "Auto".into(),
                false,
                provider,
                Some(self.inner.provider_referer.clone()),
            )]);
        }
        let Some(internal_path) = internal_path else {
            return Ok(vec![]);
        };
        let clock = clock_json_path(&internal_path);
        let endpoint = format!("{}{}", self.inner.base_url.trim_end_matches('/'), clock);
        let value: Value = self
            .checked(
                self.common_headers(self.inner.http.get(endpoint))
                    .header(header::REFERER, &self.inner.provider_referer)
                    .header(header::ORIGIN, &self.inner.provider_referer)
                    .header(header::ACCEPT, "application/json, text/plain, */*"),
            )
            .await?
            .json()
            .await?;
        let referer = find_string_key(&value, "referer")
            .map(str::to_owned)
            .unwrap_or_else(|| self.inner.provider_referer.clone());
        let links = clock_links(&value);
        if links.is_empty() {
            return Err(AniError::Provider(
                "clock response had no recognized media links".into(),
            ));
        }
        let mut result = Vec::new();
        for item in links {
            let url = item.url.as_str();
            let resolution = item.resolution.as_str();
            let hls = item.hls;
            if url.contains("repackager.wixmp.com") {
                result.extend(self.expand_wix(url, provider, Some(referer.clone())));
            } else if hls {
                result.extend(
                    self.expand_hls(url, provider, Some(referer.clone()))
                        .await
                        .unwrap_or_else(|_| {
                            vec![self.stream(
                                url.into(),
                                resolution.into(),
                                true,
                                provider,
                                Some(referer.clone()),
                            )]
                        }),
                );
            } else {
                result.push(self.stream(
                    url.into(),
                    resolution.into(),
                    false,
                    provider,
                    Some(referer.clone()),
                ));
            }
        }
        Ok(result)
    }

    async fn resolve_filemoon(&self, source_url: &str, provider: &str) -> Result<Vec<StreamLink>> {
        let source = Url::parse(source_url)?;
        let segments: Vec<_> = source
            .path_segments()
            .map(|segments| segments.collect())
            .unwrap_or_default();
        let (link_type, video_id) = segments
            .windows(2)
            .find(|pair| {
                matches!(pair[0], "e" | "d") && pair[1].chars().all(|c| c.is_ascii_alphanumeric())
            })
            .map(|pair| (pair[0], pair[1]))
            .ok_or_else(|| AniError::Provider("Filemoon embed had no video ID".into()))?;
        let source_origin = request_origin(source_url)
            .ok_or_else(|| AniError::Provider("Filemoon embed had no origin".into()))?;

        let details: Value = self
            .checked(filemoon_request(
                self.inner.http.get(format!(
                    "{source_origin}/api/videos/{video_id}/embed/details"
                )),
                source_url,
                &source_origin,
            ))
            .await
            .map_err(|_| AniError::Provider("Filemoon details request failed".into()))?
            .json()
            .await?;
        let embed_frame_url = details
            .get("embed_frame_url")
            .and_then(Value::as_str)
            .ok_or_else(|| AniError::Provider("Filemoon details omitted embed_frame_url".into()))?;
        let playback_origin = request_origin(embed_frame_url)
            .ok_or_else(|| AniError::Provider("Filemoon playback frame had no origin".into()))?;

        let challenge: Value = self
            .checked(filemoon_request(
                self.inner
                    .http
                    .post(format!("{playback_origin}/api/videos/access/challenge")),
                embed_frame_url,
                &playback_origin,
            ))
            .await
            .map_err(|_| AniError::Provider("Filemoon challenge request failed".into()))?
            .json()
            .await?;
        let challenge_id = challenge
            .get("challenge_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AniError::Provider("Filemoon challenge omitted challenge_id".into()))?;
        let nonce = challenge
            .get("nonce")
            .and_then(Value::as_str)
            .ok_or_else(|| AniError::Provider("Filemoon challenge omitted nonce".into()))?;
        let (signature, public_key) = filemoon_attestation(nonce);
        let viewer_id = random_hex_id();
        let device_id = random_hex_id();
        let attest_payload = json!({
            "viewer_id": viewer_id,
            "device_id": device_id,
            "challenge_id": challenge_id,
            "nonce": nonce,
            "signature": signature,
            "public_key": public_key,
            "client": {
                "user_agent": self.inner.user_agent,
                "architecture": "x86",
                "bitness": "64",
                "platform": "Windows",
                "platform_version": "10.0.0",
                "pixel_ratio": 1.0,
                "screen_width": 1920,
                "screen_height": 1080,
                "languages": ["en-US"]
            },
            "storage": {
                "cookie": viewer_id,
                "local_storage": viewer_id,
                "indexed_db": format!("{viewer_id}:{device_id}"),
                "cache_storage": format!("{viewer_id}:{device_id}")
            },
            "attributes": { "entropy": "high" }
        });
        let attest: Value = self
            .checked(
                filemoon_request(
                    self.inner
                        .http
                        .post(format!("{playback_origin}/api/videos/access/attest")),
                    embed_frame_url,
                    &playback_origin,
                )
                .json(&attest_payload),
            )
            .await
            .map_err(|_| AniError::Provider("Filemoon attestation request failed".into()))?
            .json()
            .await?;
        let token = attest
            .get("token")
            .and_then(Value::as_str)
            .ok_or_else(|| AniError::Provider("Filemoon attestation omitted token".into()))?;
        let viewer_id = attest
            .get("viewer_id")
            .and_then(Value::as_str)
            .unwrap_or(&viewer_id);
        let device_id = attest
            .get("device_id")
            .and_then(Value::as_str)
            .unwrap_or(&device_id);
        let confidence = attest
            .get("confidence")
            .and_then(Value::as_f64)
            .ok_or_else(|| AniError::Provider("Filemoon attestation omitted confidence".into()))?;
        let playback_payload = json!({
            "fingerprint": {
                "token": token,
                "viewer_id": viewer_id,
                "device_id": device_id,
                "confidence": confidence
            }
        });
        let mut request = filemoon_request(
            self.inner.http.post(format!(
                "{playback_origin}/api/videos/{video_id}/embed/playback"
            )),
            embed_frame_url,
            &playback_origin,
        )
        .json(&playback_payload);
        if link_type == "e" {
            request = request.header("X-Embed-Parent", source_url);
        }
        let playback: Value = self
            .checked(request)
            .await
            .map_err(|_| AniError::Provider("Filemoon playback request failed".into()))?
            .json()
            .await?;
        let decrypted = decrypt_filemoon_playback(
            playback
                .get("playback")
                .ok_or_else(|| AniError::Provider("Filemoon response omitted playback".into()))?,
        )?;
        let streams = decrypted
            .get("sources")
            .and_then(Value::as_array)
            .ok_or_else(|| AniError::Provider("Filemoon playback omitted sources".into()))?;
        Ok(streams
            .iter()
            .filter_map(|source| source.get("url").and_then(Value::as_str))
            .map(|url| {
                self.stream(
                    url.into(),
                    "Auto".into(),
                    url.to_ascii_lowercase().contains(".m3u8"),
                    provider,
                    Some(embed_frame_url.into()),
                )
            })
            .collect())
    }

    async fn is_reachable(&self, url: &str) -> bool {
        self.inner
            .http
            .request(Method::GET, url)
            .header(header::RANGE, "bytes=0-0")
            .header(header::REFERER, &self.inner.provider_referer)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    fn expand_wix(&self, url: &str, provider: &str, referer: Option<String>) -> Vec<StreamLink> {
        let base = Regex::new(r"repackager\.wixmp\.com/")
            .unwrap()
            .replace(url, "");
        let base = Regex::new(r"\.urlset.*")
            .unwrap()
            .replace(&base, "")
            .into_owned();
        let regex = Regex::new(r",([^/]*),/mp4").unwrap();
        let Some(qualities) = regex.captures(url).map(|c| c[1].to_owned()) else {
            return vec![self.stream(url.into(), "Auto".into(), false, provider, referer)];
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
                    referer.clone(),
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct ClockLink {
    url: String,
    resolution: String,
    hls: bool,
}

fn internal_clock_path(source_url: &str, base_url: &str) -> Option<String> {
    if source_url.starts_with("/apivtwo/") || source_url.starts_with("/apiv2/") {
        return Some(source_url.to_owned());
    }
    let source = Url::parse(source_url).ok()?;
    if !source.path().starts_with("/apivtwo/") && !source.path().starts_with("/apiv2/") {
        return None;
    }
    let base_host = Url::parse(base_url).ok()?.host_str()?.to_ascii_lowercase();
    let source_host = source.host_str()?.to_ascii_lowercase();
    if source_host != base_host && source_host != "allanime.day" {
        return None;
    }
    let mut path = source.path().to_owned();
    if let Some(query) = source.query() {
        path.push('?');
        path.push_str(query);
    }
    Some(path)
}

fn clock_json_path(path: &str) -> String {
    if path.contains("/clock.json") {
        path.to_owned()
    } else {
        path.replacen("/clock", "/clock.json", 1)
    }
}

fn find_string_key<'a>(value: &'a Value, wanted: &str) -> Option<&'a str> {
    match value {
        Value::Object(object) => object
            .iter()
            .find(|(key, value)| key.eq_ignore_ascii_case(wanted) && value.is_string())
            .and_then(|(_, value)| value.as_str())
            .or_else(|| {
                object
                    .values()
                    .find_map(|value| find_string_key(value, wanted))
            }),
        Value::Array(values) => values
            .iter()
            .find_map(|value| find_string_key(value, wanted)),
        _ => None,
    }
}

fn clock_links(value: &Value) -> Vec<ClockLink> {
    fn visit(value: &Value, links: &mut Vec<ClockLink>) {
        match value {
            Value::Object(object) => {
                let url = object
                    .get("link")
                    .or_else(|| object.get("url"))
                    .and_then(Value::as_str);
                if let Some(url) = url.filter(|url| is_direct_media(url)) {
                    let language = object.get("hardsub_lang").and_then(Value::as_str);
                    if language.is_none_or(|language| language.eq_ignore_ascii_case("en-US")) {
                        let resolution = object
                            .get("resolutionStr")
                            .or_else(|| object.get("resolution"))
                            .and_then(Value::as_str)
                            .unwrap_or("Auto");
                        let hls_marker = object.get("hls").is_some_and(|value| {
                            value.as_bool().unwrap_or(false)
                                || value
                                    .as_str()
                                    .is_some_and(|value| value.eq_ignore_ascii_case("hls"))
                        }) || object
                            .get("type")
                            .and_then(Value::as_str)
                            .is_some_and(|value| value.eq_ignore_ascii_case("hls"));
                        links.push(ClockLink {
                            url: url.to_owned(),
                            resolution: resolution.to_owned(),
                            hls: hls_marker || url.to_ascii_lowercase().contains(".m3u8"),
                        });
                    }
                }
                for child in object.values() {
                    visit(child, links);
                }
            }
            Value::Array(values) => {
                for child in values {
                    visit(child, links);
                }
            }
            _ => {}
        }
    }

    let mut links = Vec::new();
    visit(value, &mut links);
    let mut seen = HashSet::new();
    links.retain(|link| seen.insert(link.url.clone()));
    links
}

fn provider_error_kind(error: &AniError) -> String {
    match error {
        AniError::Network(_) => "network request failed".into(),
        AniError::GraphQl(_) => "provider returned an HTTP error".into(),
        AniError::RateLimited { .. } => "rate limited".into(),
        AniError::Unavailable(_) => "provider reports video unavailable".into(),
        AniError::Provider(message) if message.starts_with("Filemoon ") => message.clone(),
        AniError::Provider(_) | AniError::Json(_) | AniError::Url(_) => {
            "malformed provider data".into()
        }
        AniError::Decryption(_) => "decryption failed".into(),
        _ => "source resolution failed".into(),
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

fn embedded_media_urls(html: &str) -> Vec<String> {
    let decoded = html
        .replace(r"\/", "/")
        .replace(r"\u0026", "&")
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&amp;", "&");
    let regex = Regex::new(r#"https?://[^\s"'<>\\]+"#).expect("valid embedded URL regex");
    let mut seen = HashSet::new();
    regex
        .find_iter(&decoded)
        .map(|value| value.as_str().trim_end_matches([')', ']', '}', ',', ';']))
        .filter(|url| is_direct_media(url))
        .filter(|url| seen.insert((*url).to_owned()))
        .map(str::to_owned)
        .collect()
}

fn embedded_frame_urls(html: &str, page_url: &str) -> Vec<String> {
    let decoded = html_unescape(html).replace(r"\/", "/");
    let regex = Regex::new(
        r#"(?is)(?:<iframe[^>]+src\s*=\s*|(?:window\.)?location(?:\.href)?\s*=\s*)[\"']([^\"']+)[\"']"#,
    )
    .expect("valid embedded frame regex");
    let Ok(base) = Url::parse(page_url) else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    regex
        .captures_iter(&decoded)
        .filter_map(|capture| base.join(capture.get(1)?.as_str()).ok())
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .map(|url| url.to_string())
        .filter(|url| seen.insert(url.clone()))
        .collect()
}

fn html_title(html: &str) -> Option<String> {
    let regex = Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap();
    let title = regex.captures(html)?.get(1)?.as_str();
    Some(html_unescape(
        &Regex::new(r"<[^>]+>").unwrap().replace_all(title, ""),
    ))
}

fn okru_video_id(source_url: &str) -> Option<String> {
    let url = Url::parse(source_url).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    if host != "ok.ru" && !host.ends_with(".ok.ru") {
        return None;
    }
    let mut segments = url.path_segments()?;
    if segments.next()? != "videoembed" {
        return None;
    }
    segments
        .next()
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn is_embedded_video_provider(source_url: &str) -> bool {
    let Ok(url) = Url::parse(source_url) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    matches!(
        host.trim_start_matches("www.")
            .to_ascii_lowercase()
            .as_str(),
        "bysekoze.com" | "listeamed.net"
    )
}

fn is_filemoon_provider(source_url: &str) -> bool {
    let Ok(url) = Url::parse(source_url) else {
        return false;
    };
    let host = url
        .host_str()
        .unwrap_or_default()
        .trim_start_matches("www.")
        .to_ascii_lowercase();
    matches!(
        host.as_str(),
        "filemoon.site"
            | "filemoon.sx"
            | "bf0skv.org"
            | "bysejikuar.com"
            | "moflix-stream.link"
            | "bysezoxexe.com"
            | "bysebuho.com"
            | "bysekoze.com"
            | "bysesayeveum.com"
    )
}

fn filemoon_request(request: RequestBuilder, referer: &str, origin: &str) -> RequestBuilder {
    request
        .header(header::REFERER, referer)
        .header(header::ORIGIN, origin)
        .header(header::ACCEPT, "application/json, text/plain, */*")
}

fn random_hex_id() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn filemoon_attestation(nonce: &str) -> (String, Value) {
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let point = verifying_key.to_encoded_point(false);
    let signature: Signature = signing_key.sign(nonce.as_bytes());
    let x = point.x().expect("uncompressed P-256 point has x");
    let y = point.y().expect("uncompressed P-256 point has y");
    (
        URL_SAFE_NO_PAD.encode(signature.to_bytes()),
        json!({
            "crv": "P-256",
            "ext": true,
            "key_ops": ["verify"],
            "kty": "EC",
            "x": URL_SAFE_NO_PAD.encode(x),
            "y": URL_SAFE_NO_PAD.encode(y)
        }),
    )
}

fn decode_base64_url(value: &str) -> Result<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(value)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(value))
        .map_err(|error| AniError::Provider(format!("invalid Filemoon Base64: {error}")))
}

fn decrypt_filemoon_playback(playback: &Value) -> Result<Value> {
    let iv = decode_base64_url(
        playback
            .get("iv")
            .and_then(Value::as_str)
            .ok_or_else(|| AniError::Provider("Filemoon playback omitted iv".into()))?,
    )?;
    if iv.len() != 12 {
        return Err(AniError::Provider(
            "Filemoon playback used an invalid GCM nonce".into(),
        ));
    }
    let payload = decode_base64_url(
        playback
            .get("payload")
            .and_then(Value::as_str)
            .ok_or_else(|| AniError::Provider("Filemoon playback omitted payload".into()))?,
    )?;
    let key_parts = playback
        .get("key_parts")
        .and_then(Value::as_array)
        .ok_or_else(|| AniError::Provider("Filemoon playback omitted key_parts".into()))?;
    let mut key = Vec::new();
    for part in key_parts {
        key.extend(decode_base64_url(part.as_str().ok_or_else(|| {
            AniError::Provider("Filemoon key part was not a string".into())
        })?)?);
    }
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| AniError::Provider("Filemoon playback used an invalid AES key".into()))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&iv), payload.as_ref())
        .map_err(|_| AniError::Provider("Filemoon playback decryption failed".into()))?;
    serde_json::from_slice(&plaintext).map_err(AniError::from)
}

fn okru_links(value: &Value) -> Vec<ClockLink> {
    fn visit(value: &Value, links: &mut Vec<ClockLink>) {
        match value {
            Value::Object(object) => {
                if let Some(videos) = object.get("videos").and_then(Value::as_array) {
                    for video in videos {
                        let Some(url) = video.get("url").and_then(Value::as_str) else {
                            continue;
                        };
                        let name = video.get("name").and_then(Value::as_str).unwrap_or("Auto");
                        let resolution = match name.to_ascii_lowercase().as_str() {
                            "mobile" => "144p",
                            "lowest" => "240p",
                            "low" => "360p",
                            "sd" => "480p",
                            "hd" => "720p",
                            "full" => "1080p",
                            "quad" => "1440p",
                            "ultra" => "2160p",
                            _ => name,
                        };
                        links.push(ClockLink {
                            url: url.to_owned(),
                            resolution: resolution.to_owned(),
                            hls: url.to_ascii_lowercase().contains(".m3u8"),
                        });
                    }
                }
                for child in object.values() {
                    visit(child, links);
                }
            }
            Value::Array(values) => {
                for child in values {
                    visit(child, links);
                }
            }
            _ => {}
        }
    }

    let mut links = Vec::new();
    visit(value, &mut links);
    let mut seen = HashSet::new();
    links.retain(|link| seen.insert(link.url.clone()));
    links
}

fn html_unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn okru_player_options(html: &str) -> Option<Value> {
    let double_quoted = Regex::new(r#"(?is)data-options\s*=\s*\"([^\"]*)\""#).unwrap();
    let single_quoted = Regex::new(r#"(?is)data-options\s*=\s*'([^']*)'"#).unwrap();
    let encoded = double_quoted
        .captures(html)
        .or_else(|| single_quoted.captures(html))?
        .get(1)?
        .as_str();
    serde_json::from_str(&html_unescape(encoded)).ok()
}

fn okru_flashvars(options: &Value) -> Option<&serde_json::Map<String, Value>> {
    options
        .get("flashvars")
        .or_else(|| options.pointer("/player/flashvars"))?
        .as_object()
}

fn okru_embedded_metadata(options: &Value) -> Option<Value> {
    let metadata = okru_flashvars(options)?.get("metadata")?;
    match metadata {
        Value::String(value) => serde_json::from_str(value).ok(),
        Value::Object(_) => Some(metadata.clone()),
        _ => None,
    }
}

fn okru_metadata_request(options: &Value) -> Option<(String, Option<String>)> {
    let flashvars = okru_flashvars(options)?;
    let metadata_url = flashvars.get("metadataUrl")?.as_str()?;
    let metadata_url = percent_decode(metadata_url);
    let location = flashvars
        .get("location")
        .and_then(Value::as_str)
        .map(percent_decode);
    Some((metadata_url, location))
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_digit(bytes[index + 1]), hex_digit(bytes[index + 2]))
        {
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn source_shape(source_url: &str) -> String {
    let source_url = source_url.trim();
    if source_url.starts_with("/apivtwo/") {
        return "/apivtwo/…".into();
    }
    if source_url.starts_with("/apiv2/") {
        return "/apiv2/…".into();
    }
    let normalized = source_url
        .strip_prefix("//")
        .map(|value| format!("https://{value}"))
        .unwrap_or_else(|| source_url.to_owned());
    if let Ok(url) = Url::parse(&normalized) {
        let first_segment = url.path_segments().and_then(|mut values| values.next());
        return match first_segment.filter(|value| !value.is_empty()) {
            Some(segment) => format!(
                "{}://{}/{segment}/…",
                url.scheme(),
                url.host_str().unwrap_or("?"),
                segment = segment
                    .split(['-', '_'])
                    .next()
                    .filter(|value| value.len() <= 16)
                    .unwrap_or("path")
            ),
            None => format!("{}://{}/", url.scheme(), url.host_str().unwrap_or("?")),
        };
    }
    "unrecognized URL shape".into()
}

fn request_origin(referer: &str) -> Option<String> {
    let url = Url::parse(referer).ok()?;
    let host = url.host_str()?;
    let mut origin = format!("{}://{host}", url.scheme());
    if let Some(port) = url.port() {
        origin.push(':');
        origin.push_str(&port.to_string());
    }
    Some(origin)
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
    use serde_json::json;

    use aes_gcm::{
        Aes256Gcm, Nonce,
        aead::{Aead, KeyInit},
    };
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

    use super::{
        AllAnimeClient, clock_json_path, clock_links, decrypt_filemoon_playback,
        embedded_frame_urls, embedded_media_urls, graphql_rate_limit_seconds, internal_clock_path,
        okru_embedded_metadata, okru_links, okru_metadata_request, okru_player_options,
        okru_video_id, request_origin,
    };

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
            Some("https://allanime.example/".into()),
        );
        assert_eq!(
            links[2].url,
            "https://video.wixstatic.com/video/id/1080p/mp4/file.mp4"
        );
        assert_eq!(
            links[2].headers.referer.as_deref(),
            Some("https://allanime.example/")
        );
    }

    #[test]
    fn accepts_relative_and_same_host_absolute_clock_urls() {
        assert_eq!(
            internal_clock_path("/apivtwo/clock?id=token", "https://allanime.day"),
            Some("/apivtwo/clock?id=token".into())
        );
        assert_eq!(
            internal_clock_path(
                "https://allanime.day/apiv2/clock.json?id=token",
                "https://allanime.day"
            ),
            Some("/apiv2/clock.json?id=token".into())
        );
        assert_eq!(
            clock_json_path("/apivtwo/clock?id=token"),
            "/apivtwo/clock.json?id=token"
        );
        assert_eq!(
            clock_json_path("/apivtwo/clock.json?id=token"),
            "/apivtwo/clock.json?id=token"
        );
    }

    #[test]
    fn extracts_legacy_links_and_nested_english_hls() {
        let payload = json!({
            "links": [{"link":"https://media.example/video.mp4","resolutionStr":"720p"}],
            "alternatives": [
                {"type":"hls","url":"https://media.example/en/master.m3u8","hardsub_lang":"en-US"},
                {"type":"hls","url":"https://media.example/es/master.m3u8","hardsub_lang":"es-ES"}
            ]
        });
        let links = clock_links(&payload);
        assert_eq!(links.len(), 2);
        let direct = links
            .iter()
            .find(|link| link.url.ends_with("video.mp4"))
            .unwrap();
        assert_eq!(direct.resolution, "720p");
        assert!(!direct.hls);
        let hls = links.iter().find(|link| link.hls).unwrap();
        assert_eq!(hls.url, "https://media.example/en/master.m3u8");
    }

    #[test]
    fn derives_origin_from_provider_referer() {
        assert_eq!(
            request_origin("https://youtu-chan.com/watch/episode"),
            Some("https://youtu-chan.com".into())
        );
        assert_eq!(
            request_origin("http://127.0.0.1:8787/watch"),
            Some("http://127.0.0.1:8787".into())
        );
    }

    #[test]
    fn extracts_escaped_embedded_media_urls() {
        let html = r#"<script>player.setup({file:\"https:\/\/media.example\/video.mp4?x=1\u0026y=2\"});</script>"#;
        assert_eq!(
            embedded_media_urls(html),
            vec!["https://media.example/video.mp4?x=1&y=2"]
        );
    }

    #[test]
    fn resolves_relative_embedded_frames() {
        let html = r#"<iframe src="/player/video"></iframe>"#;
        assert_eq!(
            embedded_frame_urls(html, "https://provider.example/e/id"),
            vec!["https://provider.example/player/video"]
        );
    }

    #[test]
    fn decrypts_filemoon_playback_envelope() {
        let key = [7_u8; 32];
        let iv = [9_u8; 12];
        let plaintext = br#"{"sources":[{"url":"https://media.example/master.m3u8"}]}"#;
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let payload = cipher
            .encrypt(Nonce::from_slice(&iv), plaintext.as_ref())
            .unwrap();
        let envelope = json!({
            "iv": URL_SAFE_NO_PAD.encode(iv),
            "payload": URL_SAFE_NO_PAD.encode(payload),
            "key_parts": [
                URL_SAFE_NO_PAD.encode(&key[..16]),
                URL_SAFE_NO_PAD.encode(&key[16..])
            ]
        });
        let value = decrypt_filemoon_playback(&envelope).unwrap();
        assert_eq!(
            value
                .pointer("/sources/0/url")
                .and_then(serde_json::Value::as_str),
            Some("https://media.example/master.m3u8")
        );
    }

    #[test]
    fn parses_okru_video_metadata() {
        assert_eq!(
            okru_video_id("https://ok.ru/videoembed/123456"),
            Some("123456".into())
        );
        let links = okru_links(&json!({
            "movie": {"url":"https://ok.ru/video/123"},
            "videos": [
                {"name":"sd","url":"https://media.example/video?id=1"},
                {"name":"full","url":"https://media.example/video?id=2"}
            ]
        }));
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].resolution, "480p");
        assert_eq!(links[1].resolution, "1080p");
    }

    #[test]
    fn parses_okru_embedded_player_metadata() {
        let metadata = r#"{"videos":[{"name":"hd","url":"https://media.example/720"}]}"#;
        let options = serde_json::to_string(&json!({
            "flashvars": { "metadata": metadata }
        }))
        .unwrap()
        .replace('"', "&quot;");
        let html = format!(r#"<div data-options="{options}"></div>"#);
        let options = okru_player_options(&html).unwrap();
        let metadata = okru_embedded_metadata(&options).unwrap();
        let links = okru_links(&metadata);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].resolution, "720p");
    }

    #[test]
    fn parses_okru_remote_metadata_request() {
        let options = json!({
            "flashvars": {
                "metadataUrl": "https%3A%2F%2Fok.ru%2Fvideo%2Fmeta",
                "location": "https%3A%2F%2Fok.ru%2Fvideoembed%2F123"
            }
        });
        assert_eq!(
            okru_metadata_request(&options),
            Some((
                "https://ok.ru/video/meta".into(),
                Some("https://ok.ru/videoembed/123".into())
            ))
        );
    }
}
