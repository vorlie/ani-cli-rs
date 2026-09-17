use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use cbc::cipher::{BlockModeDecrypt, KeyIvInit};
use regex::Regex;
use reqwest::{Client, Response, StatusCode, header};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::{
    AniError, CatalogProvider, RequestHeaders, Result, SearchOptions, SearchResult, SearchSort,
    StreamLink, SubtitleTrack, TranslationType,
    models::{sort_episodes, sort_streams},
};

const DEFAULT_BASE: &str = "https://anikoto.cz";
const DEFAULT_MAPPER_BASE: &str = "https://mapper.nekostream.site/api/mal";
const DEFAULT_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36";
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const CACHE_LIMIT: usize = 100;
const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const MAX_PLAYLIST_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct AnikotoCzId {
    slug: String,
    title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    episodes: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Episode {
    number: String,
    slug: String,
    token: String,
    mal_id: String,
    timestamp: String,
    sub: bool,
    dub: bool,
}

#[derive(Clone, Debug)]
struct Series {
    canonical: String,
    episodes: Vec<Episode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Server {
    label: String,
    token: String,
}

#[derive(Clone, Debug)]
struct Cached<T> {
    expires_at: Instant,
    value: T,
}

#[derive(Clone, Debug)]
pub struct AnikotoCzClientBuilder {
    base: String,
    mapper_base: String,
    user_agent: String,
    timeout: Duration,
}

impl Default for AnikotoCzClientBuilder {
    fn default() -> Self {
        Self {
            base: DEFAULT_BASE.into(),
            mapper_base: DEFAULT_MAPPER_BASE.into(),
            user_agent: DEFAULT_AGENT.into(),
            timeout: Duration::from_secs(15),
        }
    }
}

impl AnikotoCzClientBuilder {
    pub fn base_url(mut self, value: impl Into<String>) -> Self {
        self.base = value.into();
        self
    }

    pub fn mapper_base(mut self, value: impl Into<String>) -> Self {
        self.mapper_base = value.into();
        self
    }

    pub fn timeout(mut self, value: Duration) -> Self {
        self.timeout = value;
        self
    }

    pub fn build(self) -> Result<AnikotoCzClient> {
        let http = Client::builder()
            .timeout(self.timeout)
            .user_agent(&self.user_agent)
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;
        Ok(AnikotoCzClient {
            inner: Arc::new(Inner {
                http,
                base: self.base.trim_end_matches('/').into(),
                mapper_base: self.mapper_base.trim_end_matches('/').into(),
                user_agent: self.user_agent,
                series: Mutex::new(HashMap::new()),
            }),
        })
    }
}

struct Inner {
    http: Client,
    base: String,
    mapper_base: String,
    user_agent: String,
    series: Mutex<HashMap<String, Cached<Series>>>,
}

#[derive(Clone)]
pub struct AnikotoCzClient {
    inner: Arc<Inner>,
}

impl AnikotoCzClient {
    pub fn builder() -> AnikotoCzClientBuilder {
        AnikotoCzClientBuilder::default()
    }

    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    pub async fn search(&self, query: &str, mode: TranslationType) -> Result<Vec<SearchResult>> {
        self.search_with_options(query, mode, SearchOptions::default())
            .await
    }

    pub async fn search_with_options(
        &self,
        query: &str,
        _mode: TranslationType,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let query = query.trim();
        if query.is_empty() {
            return Err(AniError::EmptySearchQuery);
        }

        let mut request_urls = filter_search_urls_for_base(&self.inner.base, query, options.sort);
        let mut ajax_url = Url::parse(&format!("{}/ajax/anime/search", self.inner.base))?;
        ajax_url.query_pairs_mut().append_pair("keyword", query);
        request_urls.push(ajax_url.to_string());

        for request_url in request_urls {
            let html = if request_url.contains("/filter") {
                match self
                    .get_text(
                        &request_url,
                        &self.inner.base,
                        false,
                        None,
                        MAX_RESPONSE_BYTES,
                    )
                    .await
                {
                    Ok(html) => html,
                    Err(_) => continue,
                }
            } else {
                let payload = match self
                    .get_json(&request_url, &self.inner.base, true, None)
                    .await
                {
                    Ok(payload) => payload,
                    Err(_) => continue,
                };
                let result = match provider_result(&payload, "search") {
                    Ok(result) => result,
                    Err(_) => continue,
                };
                match result
                    .as_str()
                    .or_else(|| result.get("html").and_then(Value::as_str))
                {
                    Some(html) => html.into(),
                    None => continue,
                }
            };

            let results = parse_search(&self.inner.base, &html)?;
            if !results.is_empty() {
                return Ok(results);
            }
        }

        Err(AniError::NoSearchResults)
    }

    pub async fn episodes(&self, show_id: &str, mode: TranslationType) -> Result<Vec<String>> {
        let id = decode_id(show_id)?;
        let series = self.load_series(&id.slug).await?;
        let mut episodes = series
            .episodes
            .iter()
            .filter(|episode| match mode {
                TranslationType::Sub => episode.sub,
                TranslationType::Dub => episode.dub,
            })
            .map(|episode| episode.number.clone())
            .collect::<Vec<_>>();
        sort_episodes(&mut episodes);
        if episodes.is_empty() {
            eprintln!("Anikoto.cz has no {mode} episodes for {}", id.title);
            return Err(AniError::NoEpisodesAvailable {
                anime: Some(id.title.clone()),
            });
        }
        Ok(episodes)
    }

    pub async fn streams(
        &self,
        show_id: &str,
        episode: &str,
        mode: TranslationType,
    ) -> Result<Vec<StreamLink>> {
        let id = decode_id(show_id)?;
        let series = self.load_series(&id.slug).await?;
        let episode_number = normalize_episode(episode)?;
        let selected = series
            .episodes
            .iter()
            .find(|value| value.number == episode_number)
            .ok_or_else(|| AniError::InvalidEpisodeSelection {
                episode: episode.to_string(),
                reason: "episode is not available for this anime".to_string(),
            })?;
        let available = match mode {
            TranslationType::Sub => selected.sub,
            TranslationType::Dub => selected.dub,
        };
        if !available {
            eprintln!("Episode {episode} has no {mode} servers");
            return Err(AniError::NoEpisodesAvailable {
                anime: Some(id.title.clone()),
            });
        }

        let episode_url = format!(
            "{}/ep-{}",
            series.canonical,
            url::form_urlencoded::byte_serialize(selected.slug.as_bytes()).collect::<String>()
        );
        self.get_text(
            &episode_url,
            &series.canonical,
            false,
            None,
            MAX_RESPONSE_BYTES,
        )
        .await?;
        let mut server_url = Url::parse(&format!("{}/ajax/server/list", self.inner.base))?;
        server_url
            .query_pairs_mut()
            .append_pair("servers", &selected.token);
        let payload = self
            .get_json(server_url.as_str(), &episode_url, true, None)
            .await?;
        let html = provider_result(&payload, "server list")?
            .as_str()
            .ok_or_else(|| provider_invalid_response("server list contained no markup"))?;
        let mut servers = parse_servers(html, mode)?;
        servers.extend(self.mapper_servers(selected, mode).await);
        let mut seen = HashSet::new();
        servers.retain(|server| seen.insert(server.token.clone()));

        let mut streams = Vec::new();
        let mut failures = Vec::new();
        for server in servers {
            match self.resolve_server(&server, &episode_url).await {
                Ok(embed) => match self
                    .extract_native(&embed, &server.label, &episode_url, mode)
                    .await
                {
                    Ok(mut resolved) => streams.append(&mut resolved),
                    Err(error) => failures.push(format!("{}: {error}", server.label)),
                },
                Err(error) => failures.push(format!("{}: {error}", server.label)),
            }
        }
        let mut seen = HashSet::new();
        streams.retain(|stream| seen.insert(stream.url.clone()));
        sort_streams(&mut streams);
        if streams.is_empty() {
            let detail = if failures.is_empty() {
                "no supported native servers were returned".into()
            } else {
                failures.join("; ")
            };
            eprintln!("Anikoto.cz native source resolution failed: {detail}");
            return Err(AniError::NoPlayableSources {
                anime: Some(id.title.clone()),
                episode: Some(episode.to_string()),
                mode: Some(mode.to_string()),
            });
        }
        Ok(streams)
    }

    async fn load_series(&self, slug: &str) -> Result<Series> {
        validate_slug(slug)?;
        if let Some(value) = cache_get(&self.inner.series, slug) {
            return Ok(value);
        }
        let requested = format!("{}/watch/{slug}", self.inner.base);
        let show_html = self
            .get_text(
                &requested,
                &self.inner.base,
                false,
                None,
                MAX_RESPONSE_BYTES,
            )
            .await?;
        let (show_id, canonical) = parse_show(&self.inner.base, &show_html)?;
        let mut list_url = Url::parse(&format!("{}/ajax/episode/list/{show_id}", self.inner.base))?;
        list_url
            .query_pairs_mut()
            .append_pair("style", "grid")
            .append_pair("vrf", "");
        let payload = self
            .get_json(list_url.as_str(), &canonical, true, None)
            .await?;
        let html = provider_result(&payload, "episode list")?
            .as_str()
            .ok_or_else(|| provider_invalid_response("episode list contained no markup"))?;
        let episodes = parse_episodes(html)?;
        if episodes.is_empty() {
            return Err(AniError::NoEpisodesAvailable { anime: None });
        }
        let series = Series {
            canonical,
            episodes,
        };
        cache_put(&self.inner.series, slug.into(), series.clone());
        Ok(series)
    }

    async fn mapper_servers(&self, episode: &Episode, mode: TranslationType) -> Vec<Server> {
        if episode.mal_id.is_empty() || episode.timestamp.is_empty() {
            return Vec::new();
        }
        let mode = mode.to_string();
        let url = format!(
            "{}/{}/{}/{}",
            self.inner.mapper_base,
            encode_path(&episode.mal_id),
            encode_path(&episode.slug),
            encode_path(&episode.timestamp)
        );
        let Ok(payload) = self.get_json(&url, &self.inner.base, false, None).await else {
            return Vec::new();
        };
        parse_mapper(&payload, &mode)
    }

    async fn resolve_server(&self, server: &Server, episode_url: &str) -> Result<String> {
        let mut url = Url::parse(&format!("{}/ajax/server", self.inner.base))?;
        url.query_pairs_mut().append_pair("get", &server.token);
        let payload = self.get_json(url.as_str(), episode_url, true, None).await?;
        let result = provider_result(&payload, "server")?;
        let raw = result
            .get("url")
            .and_then(Value::as_str)
            .or_else(|| result.as_str())
            .ok_or_else(|| provider_invalid_response("server returned no embed URL"))?;
        validate_remote_url(raw).map(|url| url.to_string())
    }

    async fn extract_native(
        &self,
        embed_url: &str,
        provider: &str,
        episode_url: &str,
        mode: TranslationType,
    ) -> Result<Vec<StreamLink>> {
        let embed = validate_remote_url(embed_url)?;
        let host = embed.host_str().unwrap_or_default();
        if ![
            "megaplay.buzz",
            "vidtube.site",
            "megap.shiora.top",
            "shiora.top",
            "megap.kotocdn.site",
            "megap.akirax.buzz",
            "akirax.buzz",
        ]
        .iter()
        .any(|domain| host_matches(host, domain))
        {
            return Err(AniError::UnsupportedEmbedHost {
                host: host.to_string(),
            });
        }
        let origin = embed.origin().ascii_serialization();
        let html = self
            .get_text(embed.as_str(), episode_url, false, None, MAX_RESPONSE_BYTES)
            .await?;
        let data_id = parse_data_id(&html).ok_or_else(|| {
            provider_invalid_response("embed did not expose a playable source id")
        })?;
        let payload = self
            .get_native_sources(&origin, embed.as_str(), &data_id, mode, &html)
            .await?;
        let (sources, subtitles) = parse_sources(&payload);
        if sources.is_empty() {
            return Err(AniError::NoNativeStreams {
                provider: host.to_string(),
            });
        }

        let headers = RequestHeaders {
            referer: Some(format!("{origin}/")),
            origin: Some(origin.clone()),
            extra: [("User-Agent".into(), self.inner.user_agent.clone())].into(),
        };
        let mut streams = Vec::new();
        for (url, label) in sources {
            let parsed = validate_remote_url(&url)?;
            let hls = parsed.path().to_ascii_lowercase().contains(".m3u8")
                || parsed.query().is_some_and(|query| query.contains(".m3u8"));
            let variants = if hls {
                self.expand_hls(&url, &headers).await?
            } else {
                vec![(label.clone(), url)]
            };
            for (resolution, url) in variants {
                streams.push(StreamLink {
                    url,
                    resolution: if resolution.eq_ignore_ascii_case("auto") {
                        label.clone()
                    } else {
                        resolution
                    },
                    hls,
                    provider: provider.into(),
                    downloadable: true,
                    headers: headers.clone(),
                    subtitles: subtitles.clone(),
                });
            }
        }
        Ok(streams)
    }

    async fn get_native_sources(
        &self,
        origin: &str,
        embed_url: &str,
        data_id: &str,
        mode: TranslationType,
        html: &str,
    ) -> Result<Value> {
        let mut source_url = Url::parse(&format!("{origin}/stream/getSourcesNew"))?;
        // VidTube shares episode IDs across languages and defaults to sub.
        source_url
            .query_pairs_mut()
            .append_pair("id", data_id)
            .append_pair("type", &mode.to_string());
        let raw_text = self
            .get_text(source_url.as_str(), embed_url, true, Some(origin), MAX_RESPONSE_BYTES)
            .await?;
        eprintln!("[DEBUG] getSourcesNew raw: {}", &raw_text[..raw_text.len().min(500)]);
        let mut payload: Value = serde_json::from_str(&raw_text).map_err(|error| AniError::ProviderInvalidResponse {
            provider: "Anikoto.cz".to_string(),
            message: "provider returned invalid JSON".to_string(),
            source: Some(Box::new(error)),
        })?;

        if let Some(enc) = payload.get("enc").and_then(Value::as_str) {
            eprintln!("[DEBUG] enc field found, attempting decryption");
            let e1_script_url = Html::parse_fragment(html)
                .select(
                    &Selector::parse("script[src]")
                        .map_err(|_| provider_invalid_response("invalid selector"))?,
                )
                .find_map(|elem| {
                    let src = elem.value().attr("src")?;
                    if src.contains("e1-player") {
                        Some(src.to_string())
                    } else {
                        None
                    }
                })
                .ok_or_else(|| provider_invalid_response("Missing e1-player script"))?;

            let e1_script_url = if e1_script_url.starts_with("//") {
                format!("https:{}", e1_script_url)
            } else if e1_script_url.starts_with('/') {
                format!("{}{}", origin, e1_script_url)
            } else {
                e1_script_url
            };

            eprintln!("[DEBUG] fetching e1-player script: {}", e1_script_url);
            let js = self
                .get_text(&e1_script_url, embed_url, false, None, MAX_RESPONSE_BYTES)
                .await?;
            let (key, iv, secret, ttl) = player_parameters(&js)?;
            eprintln!("[DEBUG] player params: key={:?} iv={:?} ttl={}", &key[..key.len().min(8)], &iv[..iv.len().min(8)], ttl);

            let enc_owned = enc.to_owned();
            let mut decrypted = decrypt_source(&enc_owned, &key, &iv)?;
            eprintln!("[DEBUG] decrypted: {}", serde_json::to_string(&decrypted).unwrap_or_default().chars().take(300).collect::<String>());

            fn recursively_sign(v: &mut Value, secret: &str, ttl: u64) {
                match v {
                    Value::String(s) => {
                        if let Ok(signed) = signed_url(s, secret, ttl) {
                            *s = signed;
                        }
                    }
                    Value::Array(arr) => {
                        for item in arr.iter_mut() {
                            recursively_sign(item, secret, ttl);
                        }
                    }
                    Value::Object(obj) => {
                        for (k, val) in obj.iter_mut() {
                            if k == "file" || k == "url" || k == "src" {
                                if let Value::String(s) = val {
                                    if let Ok(signed) = signed_url(s, secret, ttl) {
                                        *s = signed;
                                    }
                                }
                            } else {
                                recursively_sign(val, secret, ttl);
                            }
                        }
                    }
                    _ => {}
                }
            }
            recursively_sign(&mut decrypted, &secret, ttl);
            eprintln!("[DEBUG] signed decrypted: {}", serde_json::to_string(&decrypted).unwrap_or_default().chars().take(300).collect::<String>());
            payload
                .as_object_mut()
                .unwrap()
                .insert("sources".to_string(), decrypted);
        } else {
            eprintln!("[DEBUG] no enc field found in payload, keys: {:?}", payload.as_object().map(|o| o.keys().collect::<Vec<_>>()));
        }
        Ok(payload)
    }

    async fn expand_hls(
        &self,
        url: &str,
        headers: &RequestHeaders,
    ) -> Result<Vec<(String, String)>> {
        let mut request = self.inner.http.get(url);
        request = apply_headers(request, headers);
        let response = request.send().await?;
        if !response.status().is_success() {
            return Ok(vec![("Auto".into(), url.into())]);
        }
        let bytes = response.bytes().await?;
        if bytes.len() > MAX_PLAYLIST_BYTES {
            return Err(AniError::ProviderResponseSizeExceeded {
                provider: "Anikoto.cz".to_string(),
                size: bytes.len(),
                limit: MAX_PLAYLIST_BYTES,
            });
        }
        let text = String::from_utf8_lossy(&bytes);
        if !text.contains("#EXTM3U") || !text.contains("#EXT-X-STREAM-INF") {
            return Ok(vec![("Auto".into(), url.into())]);
        }
        let base = Url::parse(url)?;
        let resolution = Regex::new(r"(?i)RESOLUTION=\d+x(\d+)").expect("static regex");
        let lines = text.lines().collect::<Vec<_>>();
        let mut variants = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            if !line.trim_start().starts_with("#EXT-X-STREAM-INF") {
                continue;
            }
            let label = resolution
                .captures(line)
                .map(|captures| format!("{}p", &captures[1]))
                .unwrap_or_else(|| "Auto".into());
            if let Some(path) = lines[index + 1..]
                .iter()
                .map(|line| line.trim())
                .find(|line| !line.is_empty() && !line.starts_with('#'))
            {
                variants.push((label, base.join(path)?.to_string()));
            }
        }
        if variants.is_empty() {
            variants.push(("Auto".into(), url.into()));
        }
        variants.sort_by_key(|(label, _)| std::cmp::Reverse(quality_number(label)));
        Ok(variants)
    }

    async fn get_json(
        &self,
        url: &str,
        referer: &str,
        ajax: bool,
        origin: Option<&str>,
    ) -> Result<Value> {
        let text = self
            .get_text(url, referer, ajax, origin, MAX_RESPONSE_BYTES)
            .await?;
        serde_json::from_str(&text).map_err(|error| AniError::ProviderInvalidResponse {
            provider: "Anikoto.cz".to_string(),
            message: "provider returned invalid JSON".to_string(),
            source: Some(Box::new(error)),
        })
    }

    async fn get_text(
        &self,
        url: &str,
        referer: &str,
        ajax: bool,
        origin: Option<&str>,
        max_bytes: usize,
    ) -> Result<String> {
        let mut request = self
            .inner
            .http
            .get(url)
            .header(header::REFERER, referer)
            .header(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9");
        if ajax {
            request = request.header("X-Requested-With", "XMLHttpRequest").header(
                header::ACCEPT,
                "application/json, text/javascript, */*; q=0.01",
            );
        } else {
            request = request.header(header::ACCEPT, "text/html,application/xhtml+xml,*/*;q=0.8");
        }
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        checked_text(request.send().await?, max_bytes).await
    }
}

fn encode_id(value: &AnikotoCzId) -> Result<String> {
    Ok(format!(
        "anikoto2:{}",
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(value)?)
    ))
}

fn decode_id(value: &str) -> Result<AnikotoCzId> {
    if validate_slug(value).is_ok() && !value.starts_with("anikoto2:") {
        return Ok(AnikotoCzId {
            slug: value.into(),
            title: value.replace('-', " "),
            episodes: None,
        });
    }
    let payload = value
        .strip_prefix("anikoto2:")
        .ok_or_else(|| AniError::InvalidInput {
            input: value.to_string(),
            reason: "invalid Anikoto.cz show ID".to_string(),
        })?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| AniError::InvalidInput {
            input: value.to_string(),
            reason: "invalid Anikoto.cz show ID encoding".to_string(),
        })?;
    let decoded: AnikotoCzId =
        serde_json::from_slice(&bytes).map_err(|_| AniError::InvalidInput {
            input: value.to_string(),
            reason: "invalid Anikoto.cz show metadata".to_string(),
        })?;
    validate_slug(&decoded.slug)?;
    Ok(decoded)
}

fn filter_search_urls_for_base(
    base: &str,
    query: &str,
    preferred_sort: Option<SearchSort>,
) -> Vec<String> {
    let base = Url::parse(base).unwrap_or_else(|_| Url::parse("https://anikoto.cz").unwrap());
    let defaults = [
        SearchSort::LatestUpdated,
        SearchSort::LatestAdded,
        SearchSort::Score,
        SearchSort::NameAz,
        SearchSort::ReleaseDate,
        SearchSort::MostViewed,
        SearchSort::NumberOfEpisodes,
    ];

    let mut urls = Vec::new();
    let mut seen = HashSet::new();
    let candidates = std::iter::once(None)
        .chain(std::iter::once(preferred_sort))
        .chain(defaults.into_iter().map(Some));

    for sort in candidates {
        let sort_value = sort.map(|value| value.to_string());
        if !seen.insert(sort_value.clone()) {
            continue;
        }

        let mut url = base.join("/filter").unwrap();
        {
            let mut query_pairs = url.query_pairs_mut();
            query_pairs.clear();
            query_pairs.append_pair("keyword", query);
            query_pairs.append_pair("type", "");
            query_pairs.append_pair("ep_min", "");
            query_pairs.append_pair("ep_max", "");
            if let Some(sort) = sort_value.as_deref() {
                query_pairs.append_pair("sort", sort);
            }
        }
        urls.push(url.to_string());
    }
    urls
}

#[cfg(test)]
fn filter_search_urls(query: &str) -> Vec<String> {
    filter_search_urls_for_base("https://anikoto.cz", query, None)
}

fn parse_search(base: &str, html: &str) -> Result<Vec<SearchResult>> {
    let document = Html::parse_fragment(html);
    let list = Selector::parse("#list-items .item")
        .map_err(|_| provider_invalid_response("invalid result list selector"))?;
    let title_selector = Selector::parse(".name, .d-title")
        .map_err(|_| provider_invalid_response("invalid title selector"))?;
    let base = Url::parse(base)?;
    let expected_host = base.host_str().unwrap_or_default();
    let mut seen = HashSet::new();
    let mut results = Vec::new();
    let anchor = Selector::parse("a[href]")
        .map_err(|_| provider_invalid_response("invalid anchor selector"))?;
    for element in document.select(&list) {
        let Some(href) = element
            .select(&anchor)
            .next()
            .and_then(|anchor| anchor.value().attr("href"))
        else {
            continue;
        };
        let Ok(url) = base.join(href) else {
            continue;
        };
        if url.scheme() != "https" || url.host_str() != Some(expected_host) {
            continue;
        }
        let Some(slug) = url
            .path()
            .trim_end_matches('/')
            .strip_prefix("/watch/")
            .and_then(|slug| slug.split('/').next())
            .filter(|slug| validate_slug(slug).is_ok())
        else {
            continue;
        };
        if !seen.insert(slug.to_owned()) {
            continue;
        }
        let title = element
            .select(&title_selector)
            .next()
            .map(|value| clean_text(&value.text().collect::<String>()))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| slug.replace('-', " "));
        results.push(SearchResult {
            id: encode_id(&AnikotoCzId {
                slug: slug.into(),
                title: title.clone(),
                episodes: None,
            })?,
            name: title,
            episodes: 0.0,
            provider: CatalogProvider::Anikoto2,
        });
    }
    Ok(results)
}

fn parse_show(base: &str, html: &str) -> Result<(String, String)> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("#watch-main")
        .map_err(|_| provider_invalid_response("invalid show selector"))?;
    let element = document
        .select(&selector)
        .next()
        .ok_or_else(|| provider_invalid_response("show page did not expose #watch-main"))?;
    let show_id = element.value().attr("data-id").unwrap_or_default();
    let canonical = element.value().attr("data-url").unwrap_or_default();
    if !show_id.bytes().all(|byte| byte.is_ascii_digit()) || show_id.is_empty() {
        return Err(AniError::ProviderInvalidResponse {
            provider: "Anikoto.cz".to_string(),
            message: "show page exposed an invalid catalog ID".to_string(),
            source: None,
        });
    }
    let base = Url::parse(base)?;
    let canonical = base.join(canonical)?;
    if canonical.scheme() != "https" || canonical.host_str() != base.host_str() {
        return Err(AniError::ProviderInvalidResponse {
            provider: "Anikoto.cz".to_string(),
            message: "show page exposed an invalid canonical URL".to_string(),
            source: None,
        });
    }
    Ok((
        show_id.into(),
        canonical.to_string().trim_end_matches('/').into(),
    ))
}

fn parse_episodes(html: &str) -> Result<Vec<Episode>> {
    let document = Html::parse_fragment(html);
    let selector = Selector::parse("a[data-num][data-ids]")
        .map_err(|_| provider_invalid_response("invalid episode selector"))?;
    let mut seen = HashSet::new();
    let mut episodes = Vec::new();
    for element in document.select(&selector) {
        let Some(number) = element
            .value()
            .attr("data-num")
            .and_then(|value| normalize_episode(value).ok())
        else {
            continue;
        };
        if !seen.insert(number.clone()) {
            continue;
        }
        let attrs = element.value();
        episodes.push(Episode {
            slug: attrs.attr("data-slug").unwrap_or(&number).into(),
            token: attrs.attr("data-ids").unwrap_or_default().into(),
            mal_id: attrs.attr("data-mal").unwrap_or_default().into(),
            timestamp: attrs.attr("data-timestamp").unwrap_or_default().into(),
            sub: attrs.attr("data-sub") == Some("1"),
            dub: attrs.attr("data-dub") == Some("1"),
            number,
        });
    }
    episodes.sort_by(|a, b| {
        a.number
            .parse::<f64>()
            .unwrap_or_default()
            .total_cmp(&b.number.parse::<f64>().unwrap_or_default())
    });
    Ok(episodes)
}

fn parse_servers(html: &str, mode: TranslationType) -> Result<Vec<Server>> {
    let document = Html::parse_fragment(html);
    let groups = Selector::parse("div.type")
        .map_err(|_| provider_invalid_response("invalid server selector"))?;
    let labels = Selector::parse("label")
        .map_err(|_| provider_invalid_response("invalid server label selector"))?;
    let items = Selector::parse("li[data-link-id]")
        .map_err(|_| provider_invalid_response("invalid server item selector"))?;
    let mut servers = Vec::new();
    let mut seen = HashSet::new();
    for group in document.select(&groups) {
        let type_name = group.value().attr("data-type").unwrap_or_default();
        let label = group
            .select(&labels)
            .next()
            .map(|label| clean_text(&label.text().collect::<String>()))
            .unwrap_or_default();
        let kind = server_kind(type_name, &label);
        if (mode == TranslationType::Dub) != (kind == "dub") {
            continue;
        }
        for item in group.select(&items) {
            let Some(token) = item.value().attr("data-link-id") else {
                continue;
            };
            if !seen.insert(token.to_owned()) {
                continue;
            }
            let prefix = if kind == "hsub" { "H-SUB" } else { kind };
            let name = clean_text(&item.text().collect::<String>());
            servers.push(Server {
                label: format!(
                    "{} · {}",
                    prefix.to_ascii_uppercase(),
                    if name.is_empty() { "Server" } else { &name }
                ),
                token: token.into(),
            });
        }
    }
    Ok(servers)
}

fn server_kind(type_name: &str, label: &str) -> &'static str {
    let combined = format!("{type_name} {label}").to_ascii_lowercase();
    if type_name.eq_ignore_ascii_case("dub")
        || Regex::new(r"\b(?:a-?dub|dub|h-?dub)\b")
            .expect("static regex")
            .is_match(&combined)
    {
        "dub"
    } else if type_name.eq_ignore_ascii_case("hsub")
        || Regex::new(r"\bh[\s-]*sub\b")
            .expect("static regex")
            .is_match(&combined)
    {
        "hsub"
    } else {
        "sub"
    }
}

fn parse_mapper(payload: &Value, mode: &str) -> Vec<Server> {
    let Some(object) = payload.as_object() else {
        return Vec::new();
    };
    let labels = [
        ("gogoanime", "Vidstream"),
        ("animepahe", "Kiwi-Stream"),
        ("anivibe", "Vibe-Stream"),
    ]
    .into_iter()
    .collect::<HashMap<_, _>>();
    object
        .iter()
        .filter(|(provider, _)| provider.as_str() != "status")
        .filter_map(|(provider, entry)| {
            let token = entry.get(mode)?.get("url")?.as_str()?;
            Some(Server {
                label: format!(
                    "{} · {}",
                    if mode == "dub" { "A-DUB" } else { "H-SUB" },
                    labels.get(provider.as_str()).copied().unwrap_or(provider)
                ),
                token: token.into(),
            })
        })
        .collect()
}

fn parse_sources(value: &Value) -> (Vec<(String, String)>, Vec<SubtitleTrack>) {
    fn collect(value: &Value, output: &mut Vec<(String, String)>) {
        match value {
            Value::String(url) => output.push((url.clone(), "Auto".into())),
            Value::Array(values) => values.iter().for_each(|value| collect(value, output)),
            Value::Object(object) => {
                if let Some(url) = ["file", "url", "src"]
                    .iter()
                    .find_map(|key| object.get(*key).and_then(Value::as_str))
                {
                    let label = ["label", "quality"]
                        .iter()
                        .find_map(|key| object.get(*key).and_then(Value::as_str))
                        .unwrap_or("Auto");
                    output.push((url.into(), clean_text(label)));
                }
                let mut traversed = false;
                for key in ["sources", "source", "links"] {
                    if let Some(child) = object.get(key) {
                        collect(child, output);
                        traversed = true;
                    }
                }
                if !traversed
                    && !["file", "url", "src"]
                        .iter()
                        .any(|key| object.contains_key(*key))
                {
                    object.values().for_each(|value| collect(value, output));
                }
            }
            _ => {}
        }
    }

    let mut sources = Vec::new();
    if let Some(root) = value.get("sources").or_else(|| value.get("source")) {
        collect(root, &mut sources);
    }
    let mut subtitles = Vec::new();
    for key in ["tracks", "captions", "subtitles"] {
        let Some(entries) = value.get(key).and_then(Value::as_array) else {
            continue;
        };
        for entry in entries {
            let kind = entry
                .get("kind")
                .or_else(|| entry.get("type"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !kind.is_empty()
                && !kind.contains("caption")
                && !kind.contains("subtitle")
                && !kind.contains("sub")
            {
                continue;
            }
            let Some(url) = ["file", "src", "url"]
                .iter()
                .find_map(|key| entry.get(*key).and_then(Value::as_str))
                .and_then(|url| validate_remote_url(url).ok())
            else {
                continue;
            };
            subtitles.push(SubtitleTrack {
                label: entry
                    .get("label")
                    .or_else(|| entry.get("title"))
                    .and_then(Value::as_str)
                    .map(clean_text)
                    .filter(|label| !label.is_empty())
                    .unwrap_or_else(|| "Unknown".into()),
                url: url.to_string(),
                default: entry
                    .get("default")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            });
        }
    }
    let mut seen = HashSet::new();
    sources.retain(|(url, _)| validate_remote_url(url).is_ok() && seen.insert(url.clone()));
    let mut seen = HashSet::new();
    subtitles.retain(|track| seen.insert(track.url.clone()));
    (sources, subtitles)
}

fn parse_data_id(html: &str) -> Option<String> {
    Regex::new(r#"(?i)\bdata-id=["'](\d+)["']"#)
        .expect("static regex")
        .captures(html)
        .map(|captures| captures[1].into())
}

fn provider_invalid_response(message: impl Into<String>) -> AniError {
    AniError::ProviderInvalidResponse {
        provider: "Anikoto.cz".to_string(),
        message: message.into(),
        source: None,
    }
}

fn provider_result<'a>(value: &'a Value, context: &str) -> Result<&'a Value> {
    if value.get("status").and_then(Value::as_u64) != Some(200) {
        let message = value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(context);
        return Err(AniError::ProviderInvalidResponse {
            provider: "Anikoto.cz".to_string(),
            message: format!("invalid Anikoto.cz {context} response: {message}"),
            source: None,
        });
    }
    value
        .get("result")
        .ok_or_else(|| provider_invalid_response(format!("{context} response has no result")))
}

async fn checked_text(response: Response, max_bytes: usize) -> Result<String> {
    let status = response.status();
    if status == StatusCode::TOO_MANY_REQUESTS {
        let retry_after_seconds = response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok())
            .unwrap_or(120);
        return Err(AniError::ProviderRateLimited {
            provider: "Anikoto.cz".into(),
            retry_after_seconds,
        });
    }
    if !status.is_success() {
        return Err(AniError::ProviderCatalogError {
            provider: "Anikoto.cz".into(),
            message: format!("HTTP {}", status),
            source: None,
        });
    }
    if let Some(length) = response
        .content_length()
        .filter(|&length| length > max_bytes as u64)
    {
        return Err(AniError::ProviderResponseSizeExceeded {
            provider: "Anikoto.cz".to_string(),
            size: length as usize,
            limit: max_bytes,
        });
    }
    let bytes = response.bytes().await?;
    if bytes.len() > max_bytes {
        return Err(AniError::ProviderResponseSizeExceeded {
            provider: "Anikoto.cz".to_string(),
            size: bytes.len(),
            limit: max_bytes,
        });
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn apply_headers(
    mut request: reqwest::RequestBuilder,
    headers: &RequestHeaders,
) -> reqwest::RequestBuilder {
    if let Some(referer) = &headers.referer {
        request = request.header(header::REFERER, referer);
    }
    if let Some(origin) = &headers.origin {
        request = request.header(header::ORIGIN, origin);
    }
    for (name, value) in &headers.extra {
        request = request.header(name, value);
    }
    request
}

fn normalize_episode(value: &str) -> Result<String> {
    let number = value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite() && *number >= 0.0)
        .ok_or_else(|| AniError::InvalidInput {
            input: value.to_string(),
            reason: "invalid episode number".to_string(),
        })?;
    if number.fract() == 0.0 {
        Ok(format!("{number:.0}"))
    } else {
        Ok(number.to_string())
    }
}

fn validate_slug(value: &str) -> Result<()> {
    if Regex::new(r"(?i)^[a-z0-9][a-z0-9-]{0,199}$")
        .expect("static regex")
        .is_match(value)
    {
        Ok(())
    } else {
        Err(AniError::InvalidInput {
            input: value.to_string(),
            reason: "invalid Anikoto.cz show slug".to_string(),
        })
    }
}

fn validate_remote_url(value: &str) -> Result<Url> {
    let url = Url::parse(value)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(AniError::ProviderUrlValidationFailed {
            reason: "unsafe provider URL".to_string(),
        });
    }
    if url
        .host_str()
        .is_some_and(|host| host.parse::<std::net::IpAddr>().is_ok())
    {
        return Err(AniError::ProviderUrlValidationFailed {
            reason: "literal-IP provider URLs are not allowed".to_string(),
        });
    }
    Ok(url)
}

fn host_matches(host: &str, domain: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    let domain = domain.trim_end_matches('.').to_ascii_lowercase();
    host == domain || host.ends_with(&format!(".{domain}"))
}

fn clean_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn quality_number(value: &str) -> u32 {
    Regex::new(r"(\d{3,4})")
        .expect("static regex")
        .captures(value)
        .and_then(|captures| captures[1].parse().ok())
        .unwrap_or(0)
}

fn encode_path(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn cache_get<T: Clone>(cache: &Mutex<HashMap<String, Cached<T>>>, key: &str) -> Option<T> {
    let mut cache = cache.lock().ok()?;
    cache.retain(|_, value| value.expires_at > Instant::now());
    cache.get(key).map(|value| value.value.clone())
}

fn cache_put<T>(cache: &Mutex<HashMap<String, Cached<T>>>, key: String, value: T) {
    if let Ok(mut cache) = cache.lock() {
        if cache.len() >= CACHE_LIMIT
            && let Some(key) = cache.keys().next().cloned()
        {
            cache.remove(&key);
        }
        cache.insert(
            key,
            Cached {
                expires_at: Instant::now() + CACHE_TTL,
                value,
            },
        );
    }
}

pub fn requires_hls_relay(stream: &StreamLink) -> bool {
    stream.hls
        && Url::parse(&stream.url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .is_some_and(|host| {
                [
                    "megaplay.buzz",
                    "megap.shiora.top",
                    "shiora.top",
                    "megap.kotocdn.site",
                    "kotocdn.site",
                    "megap.akirax.buzz",
                    "akirax.buzz",
                ]
                .iter()
                .any(|domain| host_matches(&host, domain))
            })
}

fn player_parameters(script: &str) -> Result<(String, String, String, u64)> {
    let re = Regex::new(r#"\bconst\s+[\w$]+="((?:\\.|[^"\\])*)",[\w$]+="((?:\\.|[^"\\])*)",[\w$]+="((?:\\.|[^"\\])*)",[\w$]+=(\d+);function\s+[\w$]+\("#).unwrap();
    for captures in re.captures_iter(script) {
        let m = captures.get(0).unwrap();
        let end = m.end();
        let limit = (end + 2500).min(script.len());
        if script[end..limit].contains("AES-CBC") {
            let parse = |s: &str| -> String {
                serde_json::from_str(&format!("\"{}\"", s)).unwrap_or_else(|_| s.to_string())
            };
            let key = parse(&captures[1]);
            let iv = parse(&captures[2]);
            let secret = parse(&captures[3]);
            let ttl = captures[4].parse::<u64>().unwrap_or(0);
            return Ok((key, iv, secret, ttl));
        }
    }
    Err(provider_invalid_response(
        "Player crypto parameters changed; update player_parameters().",
    ))
}

fn decrypt_source(encoded: &str, key: &str, iv: &str) -> Result<Value> {
    let mut key_bytes = [0u8; 32];
    let key_src = key.as_bytes();
    let key_len = key_src.len().min(32);
    key_bytes[..key_len].copy_from_slice(&key_src[..key_len]);

    let mut iv_bytes = [0u8; 16];
    let iv_src = iv.as_bytes();
    let iv_len = iv_src.len().min(16);
    iv_bytes[..iv_len].copy_from_slice(&iv_src[..iv_len]);

    let mut enc = encoded.to_string();
    if !enc.len().is_multiple_of(4) {
        enc.push_str(&"=".repeat(4 - (enc.len() % 4)));
    }
    let mut encrypted = base64::engine::general_purpose::URL_SAFE
        .decode(enc)
        .map_err(|_| provider_invalid_response("Invalid base64 in enc"))?;

    let pt = cbc::Decryptor::<aes::Aes256>::new(&key_bytes.into(), &iv_bytes.into())
        .decrypt_padded::<aes::cipher::block_padding::Pkcs7>(&mut encrypted)
        .map_err(|_| provider_invalid_response("Decryption failed"))?;

    let json_str = std::str::from_utf8(pt)
        .map_err(|_| provider_invalid_response("Invalid UTF-8 in decrypted data"))?;
    serde_json::from_str(json_str)
        .map_err(|_| provider_invalid_response("Invalid JSON in decrypted data"))
}

fn signed_url(url: &str, secret: &str, ttl: u64) -> Result<String> {
    let mut parsed =
        Url::parse(url).map_err(|_| provider_invalid_response("Invalid url for signing"))?;
    if parsed.query_pairs().any(|(k, _)| k == "token") {
        return Ok(url.to_string());
    }

    let path = parsed.path();
    let re = Regex::new(r"(?i)/([a-f0-9]{32})/([a-f0-9]{32})/").unwrap();
    let captures = re.captures(path).ok_or_else(|| {
        provider_invalid_response("Unexpected CDN path; cannot reproduce the player's token.")
    })?;

    let expires = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + ttl;
    let message = format!(
        "{}|{}/{}",
        expires,
        captures[1].to_ascii_lowercase(),
        captures[2].to_ascii_lowercase()
    );

    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| provider_invalid_response("Invalid secret size for HMAC"))?;
    mac.update(message.as_bytes());
    let signature = mac.finalize().into_bytes();

    let encode =
        |value: &[u8]| -> String { base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(value) };
    let token = format!("{}.{}", encode(message.as_bytes()), encode(&signature));

    parsed.query_pairs_mut().append_pair("token", &token);
    Ok(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path, query_param},
    };

    #[tokio::test]
    async fn native_source_requests_preserve_language_for_shared_episode_ids() {
        let server = MockServer::start().await;
        let client = AnikotoCzClient::new().unwrap();

        for (mode, language) in [(TranslationType::Sub, "sub"), (TranslationType::Dub, "dub")] {
            let embed_url = format!("https://vidtube.site/stream/example/{language}");
            let media_url = format!("https://media.example/{language}.m3u8");
            Mock::given(method("GET"))
                .and(path("/stream/getSourcesNew"))
                .and(query_param("id", "42"))
                .and(query_param("type", language))
                .and(header("Referer", embed_url.as_str()))
                .and(header("Origin", server.uri().as_str()))
                .and(header("X-Requested-With", "XMLHttpRequest"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "sources": {"file": media_url}
                })))
                .expect(1)
                .mount(&server)
                .await;

            let payload = client
                .get_native_sources(&server.uri(), &embed_url, "42", mode, "<html></html>")
                .await
                .unwrap();
            let (sources, _) = parse_sources(&payload);
            assert_eq!(sources[0].0, media_url);
        }
    }

    #[test]
    fn ids_round_trip_and_raw_slugs_are_supported() {
        let id = AnikotoCzId {
            slug: "black-torch-1d364".into(),
            title: "Black Torch".into(),
            episodes: None,
        };
        let encoded = encode_id(&id).unwrap();
        assert_eq!(decode_id(&encoded).unwrap(), id);
        assert_eq!(
            decode_id("black-torch-1d364").unwrap().slug,
            "black-torch-1d364"
        );
    }

    #[test]
    fn parses_search_and_deduplicates_slugs() {
        let html = r#"
            <div id="list-items" class="ani items">
              <div class="item"><a href="/watch/black-torch-1d364"><div class="name">Black Torch</div></a></div>
              <div class="item"><a href="/watch/black-torch-1d364"><span class="d-title">Duplicate</span></a></div>
            </div>
        "#;
        let values = parse_search("https://anikoto.cz", html).unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].name, "Black Torch");
        assert_eq!(values[0].provider, CatalogProvider::Anikoto2);
    }

    #[test]
    fn ignores_unrelated_items_outside_the_filter_result_list() {
        let html = r#"
            <div class="top-rated">
              <a class="item" href="/watch/dorohedoro-season-2-bqfe6"><span class="name">Dorohedoro Season 2</span></a>
            </div>
            <div id="list-items" class="ani items">
              <div class="item"><a href="/watch/one-piece-episode-of-luffy-hand-island-adventure-br7lf/ep-1"><span class="d-title">One Piece: Episode of Luffy - Hand Island Adventure</span></a></div>
            </div>
        "#;
        let values = parse_search("https://anikoto.cz", html).unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(
            values[0].name,
            "One Piece: Episode of Luffy - Hand Island Adventure"
        );
    }

    #[test]
    fn builds_supported_filter_sort_urls() {
        let urls = filter_search_urls("one piece");
        assert_eq!(urls.len(), 8);
        assert!(urls[0].contains("/filter?keyword=one+piece"));
        assert!(urls.iter().any(|url| url.contains("sort=latest-updated")));
        assert!(urls.iter().any(|url| url.contains("sort=name-az")));
        assert!(
            urls.iter()
                .any(|url| url.contains("sort=number_of_episodes"))
        );
    }

    #[test]
    fn parses_fractional_episode_availability() {
        let html = r#"
            <a data-num="2.5" data-slug="2-5" data-ids="two" data-sub="1" data-dub="0"></a>
            <a data-num="1" data-slug="1" data-ids="one" data-sub="1" data-dub="1"></a>
        "#;
        let episodes = parse_episodes(html).unwrap();
        assert_eq!(episodes[0].number, "1");
        assert!(episodes[0].dub);
        assert_eq!(episodes[1].number, "2.5");
    }

    #[test]
    fn keeps_soft_hard_and_dub_servers_separate() {
        let html = r#"
            <div class="type" data-type="sub"><label>SUB</label>
              <ul><li data-link-id="soft">VidPlay-1</li></ul>
            </div>
            <div class="type" data-type="hsub"><label>HSUB</label>
              <ul><li data-link-id="hard">HD-1</li></ul>
            </div>
            <div class="type" data-type="dub"><label>DUB</label>
              <ul><li data-link-id="dub">Vidstream-2</li></ul>
            </div>
        "#;
        let sub = parse_servers(html, TranslationType::Sub).unwrap();
        assert_eq!(sub.len(), 2);
        assert!(sub[1].label.starts_with("H-SUB"));
        let dub = parse_servers(html, TranslationType::Dub).unwrap();
        assert_eq!(dub.len(), 1);
        assert!(dub[0].label.starts_with("DUB"));
    }

    #[test]
    fn nested_sources_and_subtitles_are_normalized() {
        let (sources, subtitles) = parse_sources(&serde_json::json!({
            "sources":{"links":[{"file":"https://megap.kotocdn.site/master.m3u8","label":"1080p"}]},
            "tracks":[{"file":"https://megap.kotocdn.site/en.vtt","label":"English","kind":"captions","default":true}]
        }));
        assert_eq!(sources[0].1, "1080p");
        assert_eq!(subtitles[0].label, "English");
        assert!(subtitles[0].default);
    }

    #[tokio::test]
    async fn live_anikoto_cz_smoke_test_is_opt_in() {
        if std::env::var("ANI_CLI_LIVE_ANIKOTO2").as_deref() != Ok("1") {
            return;
        }
        let client = AnikotoCzClient::new().unwrap();
        let results = client
            .search("black torch", TranslationType::Sub)
            .await
            .unwrap();
        let show = results
            .iter()
            .find(|result| result.name.eq_ignore_ascii_case("Black Torch"))
            .unwrap();
        let episodes = client
            .episodes(&show.id, TranslationType::Sub)
            .await
            .unwrap();
        assert!(episodes.iter().any(|episode| episode == "1"));
        let streams = client
            .streams(&show.id, "1", TranslationType::Sub)
            .await
            .unwrap();
        assert!(!streams.is_empty());
        assert!(streams.iter().any(|stream| stream.hls));
    }
}
