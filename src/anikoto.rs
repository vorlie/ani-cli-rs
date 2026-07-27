use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use regex::Regex;
use reqwest::{Client, Response, StatusCode, header};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;

use crate::{
    AniError, CatalogProvider, RequestHeaders, Result, SearchOptions, SearchResult, StreamLink,
    SubtitleTrack, TranslationType,
    models::{sort_episodes, sort_streams},
};

const DEFAULT_ANIKOTO_API: &str = "https://anikotoapi.site";
const DEFAULT_ANILIST_API: &str = "https://graphql.anilist.co";
const DEFAULT_MEGAPLAY_BASE: &str = "https://megaplay.buzz";
const DEFAULT_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36";
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const CACHE_LIMIT: usize = 100;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct AnikotoId {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    anilist_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    anikoto_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    episodes: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AnikotoEpisode {
    number: String,
    embed_id: Option<String>,
    sub_url: Option<String>,
    dub_url: Option<String>,
}

#[derive(Clone, Debug)]
struct Cached<T> {
    expires_at: Instant,
    value: T,
}

#[derive(Clone, Debug)]
pub struct AnikotoClientBuilder {
    anikoto_api: String,
    anilist_api: String,
    megaplay_base: String,
    user_agent: String,
    timeout: Duration,
}

impl Default for AnikotoClientBuilder {
    fn default() -> Self {
        Self {
            anikoto_api: DEFAULT_ANIKOTO_API.into(),
            anilist_api: DEFAULT_ANILIST_API.into(),
            megaplay_base: DEFAULT_MEGAPLAY_BASE.into(),
            user_agent: DEFAULT_AGENT.into(),
            timeout: Duration::from_secs(12),
        }
    }
}

impl AnikotoClientBuilder {
    pub fn anikoto_api(mut self, value: impl Into<String>) -> Self {
        self.anikoto_api = value.into();
        self
    }

    pub fn anilist_api(mut self, value: impl Into<String>) -> Self {
        self.anilist_api = value.into();
        self
    }

    pub fn megaplay_base(mut self, value: impl Into<String>) -> Self {
        self.megaplay_base = value.into();
        self
    }

    pub fn timeout(mut self, value: Duration) -> Self {
        self.timeout = value;
        self
    }

    pub fn build(self) -> Result<AnikotoClient> {
        let http = Client::builder()
            .timeout(self.timeout)
            .user_agent(&self.user_agent)
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;
        Ok(AnikotoClient {
            inner: Arc::new(Inner {
                http,
                anikoto_api: self.anikoto_api.trim_end_matches('/').into(),
                anilist_api: self.anilist_api,
                megaplay_base: self.megaplay_base.trim_end_matches('/').into(),
                user_agent: self.user_agent,
                searches: Mutex::new(HashMap::new()),
                series: Mutex::new(HashMap::new()),
            }),
        })
    }
}

struct Inner {
    http: Client,
    anikoto_api: String,
    anilist_api: String,
    megaplay_base: String,
    user_agent: String,
    searches: Mutex<HashMap<String, Cached<Vec<SearchResult>>>>,
    series: Mutex<HashMap<String, Cached<Vec<AnikotoEpisode>>>>,
}

#[derive(Clone)]
pub struct AnikotoClient {
    inner: Arc<Inner>,
}

impl AnikotoClient {
    pub fn builder() -> AnikotoClientBuilder {
        AnikotoClientBuilder::default()
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
            return Err(AniError::Input("search query cannot be empty".into()));
        }
        let cache_key = format!("{}:{}", options.allow_adult, query.to_ascii_lowercase());
        if let Some(value) = cache_get(&self.inner.searches, &cache_key) {
            return Ok(value);
        }

        let recent = self.search_recent(query, options.allow_adult);
        let anilist = self.search_anilist(query, options.allow_adult);
        let (recent, anilist) = tokio::join!(recent, anilist);
        let values = match (recent, anilist) {
            (Ok(recent), Ok(anilist)) => merge_search_results(recent, anilist),
            (Ok(recent), Err(_)) if !recent.is_empty() => recent,
            (Err(_), Ok(anilist)) => anilist,
            (Ok(recent), Err(error)) => {
                if recent.is_empty() {
                    return Err(error);
                }
                recent
            }
            (Err(error @ AniError::ProviderRateLimited { .. }), Err(_))
            | (Err(_), Err(error @ AniError::ProviderRateLimited { .. })) => return Err(error),
            (Err(first), Err(second)) => {
                return Err(AniError::Catalog {
                    provider: "Anikoto".into(),
                    message: format!(
                        "recent catalog failed ({first}); AniList search failed ({second})"
                    ),
                });
            }
        };
        cache_put(&self.inner.searches, cache_key, values.clone());
        Ok(values)
    }

    pub async fn episodes(&self, show_id: &str, _mode: TranslationType) -> Result<Vec<String>> {
        let id = decode_id(show_id)?;
        let series = match self.load_series(&id).await {
            Ok(series) => series,
            Err(AniError::Network(_)) if id.episodes.is_some() => Vec::new(),
            Err(error) => return Err(error),
        };
        if !series.is_empty() {
            return Ok(series.into_iter().map(|episode| episode.number).collect());
        }
        let count = id.episodes.unwrap_or(0);
        if count == 0 {
            return Err(AniError::Unavailable(
                "Anikoto could not determine the episode list for this title".into(),
            ));
        }
        Ok((1..=count).map(|episode| episode.to_string()).collect())
    }

    pub async fn streams(
        &self,
        show_id: &str,
        episode: &str,
        mode: TranslationType,
    ) -> Result<Vec<StreamLink>> {
        let id = decode_id(show_id)?;
        let series = match self.load_series(&id).await {
            Ok(series) => series,
            Err(AniError::Network(_)) if id.anilist_id.is_some() || id.mal_id.is_some() => {
                Vec::new()
            }
            Err(error) => return Err(error),
        };
        let selected = series.iter().find(|value| value.number == episode);
        let candidates = embed_candidates(&self.inner.megaplay_base, &id, episode, mode, selected);
        if candidates.is_empty() {
            return Err(AniError::Unavailable(
                "Anikoto has no mapping for the selected episode".into(),
            ));
        }

        let mut failures = Vec::new();
        for candidate in candidates {
            match self.resolve_megaplay(&candidate).await {
                Ok(mut streams) if !streams.is_empty() => {
                    sort_streams(&mut streams);
                    return Ok(streams);
                }
                Ok(_) => failures.push("MegaPlay returned no native streams".to_owned()),
                Err(error @ AniError::ProviderRateLimited { .. }) => return Err(error),
                Err(error) => failures.push(error.to_string()),
            }
        }
        Err(AniError::Unavailable(format!(
            "Anikoto native source resolution failed: {}",
            failures.join("; ")
        )))
    }

    async fn search_recent(&self, query: &str, allow_adult: bool) -> Result<Vec<SearchResult>> {
        let url = format!("{}/recent-anime?page=1&per_page=40", self.inner.anikoto_api);
        let value = self.get_json(&url, None, None).await?;
        let needle = query.to_ascii_lowercase();
        Ok(parse_search_payload(&value, allow_adult)
            .into_iter()
            .filter(|value| value.name.to_ascii_lowercase().contains(&needle))
            .collect())
    }

    async fn search_anilist(&self, query: &str, allow_adult: bool) -> Result<Vec<SearchResult>> {
        let graphql = r#"query ($search: String!) { Page(page: 1, perPage: 40) { media(type: ANIME, search: $search, sort: SEARCH_MATCH) { id idMal title { romaji english native } episodes isAdult } } }"#;
        let response = self
            .inner
            .http
            .post(&self.inner.anilist_api)
            .header(header::REFERER, "https://anilist.co/")
            .json(&json!({"query":graphql,"variables":{"search":query}}))
            .send()
            .await?;
        let value = checked_json(response, "AniList").await?;
        Ok(parse_search_payload(&value, allow_adult))
    }

    async fn load_series(&self, id: &AnikotoId) -> Result<Vec<AnikotoEpisode>> {
        let Some(series_id) = id.anikoto_id.as_deref() else {
            return Ok(vec![]);
        };
        if let Some(value) = cache_get(&self.inner.series, series_id) {
            return Ok(value);
        }
        let url = format!("{}/series/{}", self.inner.anikoto_api, series_id);
        let value = parse_episode_payload(&self.get_json(&url, None, None).await?);
        cache_put(&self.inner.series, series_id.to_owned(), value.clone());
        Ok(value)
    }

    async fn resolve_megaplay(&self, embed_url: &str) -> Result<Vec<StreamLink>> {
        validate_remote_url(embed_url)?;
        let html = self
            .inner
            .http
            .get(embed_url)
            .header(header::REFERER, format!("{}/", self.inner.megaplay_base))
            .header(header::ACCEPT, "text/html,application/json,text/plain,*/*")
            .send()
            .await?;
        let html = checked_text(html, "MegaPlay").await?;
        let data_id = parse_data_id(&html).ok_or_else(|| {
            AniError::Provider("MegaPlay did not expose a playable source id".into())
        })?;
        let source_url = format!(
            "{}/stream/getSources?id={data_id}",
            self.inner.megaplay_base
        );
        let payload = self
            .get_json(
                &source_url,
                Some(embed_url),
                Some(&self.inner.megaplay_base),
            )
            .await?;
        let (sources, subtitles) = parse_megaplay_sources(&payload);
        if sources.is_empty() {
            return Err(AniError::Provider(
                "MegaPlay did not return any supported native streams".into(),
            ));
        }

        let mut streams = Vec::new();
        for (url, resolution) in sources {
            let parsed = validate_remote_url(&url)?;
            let hls = parsed.path().to_ascii_lowercase().contains(".m3u8")
                || parsed.query().is_some_and(|query| query.contains(".m3u8"));
            let headers = media_headers(
                parsed.host_str().unwrap_or_default(),
                &self.inner.user_agent,
            );
            let mut expanded = if hls {
                self.expand_hls(&url, &resolution, &headers).await?
            } else {
                vec![stream_link(url, resolution, false, headers)]
            };
            for stream in &mut expanded {
                stream.subtitles = subtitles.clone();
            }
            streams.extend(expanded);
        }
        let mut seen = HashSet::new();
        streams.retain(|stream| seen.insert(stream.url.clone()));
        Ok(streams)
    }

    async fn expand_hls(
        &self,
        url: &str,
        fallback_resolution: &str,
        headers: &RequestHeaders,
    ) -> Result<Vec<StreamLink>> {
        let mut request = self.inner.http.get(url);
        request = apply_headers(request, headers);
        let response = request.send().await?;
        if !response.status().is_success() {
            return Ok(vec![stream_link(
                url.into(),
                fallback_resolution.into(),
                true,
                headers.clone(),
            )]);
        }
        let text = response.text().await?;
        if !text.trim_start().starts_with("#EXTM3U") {
            return Ok(vec![stream_link(
                url.into(),
                fallback_resolution.into(),
                true,
                headers.clone(),
            )]);
        }
        let base = Url::parse(url)?;
        let resolution = Regex::new(r"RESOLUTION=\d+x(\d+)").expect("static regex");
        let mut streams = Vec::new();
        let mut lines = text.lines();
        while let Some(line) = lines.next() {
            if !line.starts_with("#EXT-X-STREAM-INF:") {
                continue;
            }
            let label = resolution
                .captures(line)
                .map(|captures| format!("{}p", &captures[1]))
                .unwrap_or_else(|| fallback_resolution.into());
            if let Some(path) = lines.by_ref().find(|line| {
                let line = line.trim();
                !line.is_empty() && !line.starts_with('#')
            }) {
                streams.push(stream_link(
                    base.join(path.trim())?.to_string(),
                    label,
                    true,
                    headers.clone(),
                ));
            }
        }
        if streams.is_empty() {
            streams.push(stream_link(
                url.into(),
                fallback_resolution.into(),
                true,
                headers.clone(),
            ));
        }
        Ok(streams)
    }

    async fn get_json(
        &self,
        url: &str,
        referer: Option<&str>,
        origin: Option<&str>,
    ) -> Result<Value> {
        let mut request = self
            .inner
            .http
            .get(url)
            .header(header::ACCEPT, "application/json,text/plain,*/*");
        if let Some(referer) = referer {
            request = request.header(header::REFERER, referer);
        }
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        let provider = if url.starts_with(&self.inner.megaplay_base) {
            "MegaPlay"
        } else {
            "Anikoto"
        };
        checked_json(request.send().await?, provider).await
    }
}

pub fn provider_from_show_id(show_id: &str) -> CatalogProvider {
    if show_id.starts_with("anikoto2:") {
        CatalogProvider::Anikoto2
    } else {
        CatalogProvider::Anikoto
    }
}

fn encode_id(id: &AnikotoId) -> Result<String> {
    Ok(format!(
        "anikoto:{}",
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(id)?)
    ))
}

fn decode_id(value: &str) -> Result<AnikotoId> {
    if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(AnikotoId {
            anikoto_id: Some(value.into()),
            ..Default::default()
        });
    }
    let payload = value
        .strip_prefix("anikoto:")
        .ok_or_else(|| AniError::Input("invalid Anikoto show ID".into()))?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| AniError::Input("invalid Anikoto show ID encoding".into()))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| AniError::Input("invalid Anikoto show metadata".into()))
}

fn parse_search_payload(value: &Value, allow_adult: bool) -> Vec<SearchResult> {
    let values = value
        .pointer("/data/Page/media")
        .or_else(|| value.get("data"))
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
        .cloned()
        .unwrap_or_default();
    values
        .iter()
        .filter_map(|item| {
            if !allow_adult
                && item
                    .get("isAdult")
                    .or_else(|| item.get("is_adult"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            {
                return None;
            }
            let anilist_id = string_value(item.get("ani_id").or_else(|| item.get("id")))?;
            let title = item
                .get("title")
                .and_then(|title| {
                    if let Some(title) = title.as_str() {
                        return Some(title.to_owned());
                    }
                    ["english", "romaji", "native"]
                        .iter()
                        .find_map(|key| string_value(title.get(*key)))
                })
                .or_else(|| string_value(item.get("name")))
                .unwrap_or_else(|| format!("AniList {anilist_id}"));
            let episodes = number_value(item.get("episodes")).unwrap_or(0.0);
            let anikoto_id = item.get("ani_id").and_then(|ani_id| {
                let item_id = string_value(item.get("id"));
                let ani_id = string_value(Some(ani_id));
                (item_id != ani_id).then_some(item_id).flatten()
            });
            let id = encode_id(&AnikotoId {
                anilist_id: Some(anilist_id),
                mal_id: string_value(item.get("idMal").or_else(|| item.get("mal_id"))),
                anikoto_id,
                title: Some(title.clone()),
                episodes: (episodes.is_finite() && episodes > 0.0).then_some(episodes as u32),
            })
            .ok()?;
            Some(SearchResult {
                id,
                name: title,
                episodes,
                provider: CatalogProvider::Anikoto,
            })
        })
        .collect()
}

fn merge_search_results(
    recent: Vec<SearchResult>,
    anilist: Vec<SearchResult>,
) -> Vec<SearchResult> {
    let mut seen = HashSet::new();
    recent
        .into_iter()
        .chain(anilist)
        .filter(|item| {
            let id = decode_id(&item.id).ok();
            let key = id
                .and_then(|id| id.anilist_id)
                .map(|id| format!("ani:{id}"))
                .unwrap_or_else(|| format!("title:{}", normalize_title(&item.name)));
            seen.insert(key)
        })
        .collect()
}

fn parse_episode_payload(value: &Value) -> Vec<AnikotoEpisode> {
    let data = value.get("data").unwrap_or(value);
    let mut episodes = data
        .get("episodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let number = string_value(
                item.get("number")
                    .or_else(|| item.get("episode"))
                    .or_else(|| item.get("episode_number")),
            )?;
            Some(AnikotoEpisode {
                number,
                embed_id: string_value(item.get("episode_embed_id")),
                sub_url: item.pointer("/embed_url/sub").and_then(value_string),
                dub_url: item.pointer("/embed_url/dub").and_then(value_string),
            })
        })
        .collect::<Vec<_>>();
    let mut numbers = episodes
        .iter()
        .map(|value| value.number.clone())
        .collect::<Vec<_>>();
    sort_episodes(&mut numbers);
    let positions = numbers
        .into_iter()
        .enumerate()
        .map(|(index, number)| (number, index))
        .collect::<HashMap<_, _>>();
    episodes.sort_by_key(|episode| {
        positions
            .get(&episode.number)
            .copied()
            .unwrap_or(usize::MAX)
    });
    episodes
}

fn embed_candidates(
    base: &str,
    id: &AnikotoId,
    episode: &str,
    mode: TranslationType,
    selected: Option<&AnikotoEpisode>,
) -> Vec<String> {
    let language = mode.to_string();
    let mut candidates = Vec::new();
    if let Some(selected) = selected {
        let explicit = match mode {
            TranslationType::Sub => selected.sub_url.as_ref(),
            TranslationType::Dub => selected.dub_url.as_ref(),
        };
        if let Some(url) = explicit {
            candidates.push(url.clone());
        }
        if let Some(embed_id) = &selected.embed_id {
            candidates.push(format!("{base}/stream/s-2/{embed_id}/{language}"));
        }
    }
    if let Some(anilist_id) = &id.anilist_id {
        candidates.push(format!(
            "{base}/stream/ani/{anilist_id}/{episode}/{language}"
        ));
    }
    if let Some(mal_id) = &id.mal_id {
        candidates.push(format!("{base}/stream/mal/{mal_id}/{episode}/{language}"));
    }
    let mut seen = HashSet::new();
    candidates.retain(|candidate| seen.insert(candidate.clone()));
    candidates
}

fn parse_data_id(html: &str) -> Option<String> {
    Regex::new(r#"(?i)\bdata-id=["'](\d+)["']"#)
        .expect("static regex")
        .captures(html)
        .map(|captures| captures[1].to_owned())
}

fn parse_megaplay_sources(value: &Value) -> (Vec<(String, String)>, Vec<SubtitleTrack>) {
    fn collect_sources(value: &Value, values: &mut Vec<(String, String)>) {
        match value {
            Value::String(url) => values.push((url.clone(), "Auto".into())),
            Value::Array(items) => items.iter().for_each(|item| collect_sources(item, values)),
            Value::Object(object) => {
                if let Some(url) = object
                    .get("file")
                    .or_else(|| object.get("url"))
                    .or_else(|| object.get("src"))
                    .and_then(Value::as_str)
                {
                    let label = object
                        .get("label")
                        .or_else(|| object.get("quality"))
                        .and_then(Value::as_str)
                        .unwrap_or("Auto");
                    values.push((url.into(), label.into()));
                }
                for key in ["sources", "source", "links"] {
                    if let Some(child) = object.get(key) {
                        collect_sources(child, values);
                    }
                }
            }
            _ => {}
        }
    }

    fn collect_tracks(value: &Value, tracks: &mut Vec<SubtitleTrack>) {
        match value {
            Value::Array(items) => items.iter().for_each(|item| collect_tracks(item, tracks)),
            Value::Object(object) => {
                let kind = object
                    .get("kind")
                    .or_else(|| object.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if !kind.is_empty()
                    && !kind.contains("caption")
                    && !kind.contains("subtitle")
                    && !kind.contains("sub")
                {
                    return;
                }
                if let Some(url) = object
                    .get("file")
                    .or_else(|| object.get("url"))
                    .or_else(|| object.get("src"))
                    .and_then(Value::as_str)
                {
                    tracks.push(SubtitleTrack {
                        label: object
                            .get("label")
                            .or_else(|| object.get("title"))
                            .and_then(Value::as_str)
                            .unwrap_or("Subtitle")
                            .into(),
                        url: url.into(),
                        default: object
                            .get("default")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    });
                }
            }
            _ => {}
        }
    }

    let mut sources = Vec::new();
    let mut tracks = Vec::new();
    if let Some(value) = value.get("sources") {
        collect_sources(value, &mut sources);
    }
    if let Some(value) = value.get("source") {
        collect_sources(value, &mut sources);
    }
    for key in ["tracks", "captions", "subtitles"] {
        if let Some(value) = value.get(key) {
            collect_tracks(value, &mut tracks);
        }
    }
    let mut seen = HashSet::new();
    sources.retain(|(url, _)| seen.insert(url.clone()));
    let mut seen = HashSet::new();
    tracks.retain(|track| seen.insert(track.url.clone()));
    (sources, tracks)
}

fn media_headers(host: &str, user_agent: &str) -> RequestHeaders {
    if is_megaplay_media_host(host) {
        RequestHeaders {
            referer: Some("https://megaplay.buzz/".into()),
            origin: Some("https://megaplay.buzz".into()),
            extra: [("User-Agent".into(), user_agent.into())].into(),
        }
    } else {
        RequestHeaders::default()
    }
}

pub(crate) fn is_megaplay_media_host(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    [
        "megaplay.buzz",
        "mewstream.buzz",
        "lostproject.club",
        "voltara.click",
        "kotocdn.site",
    ]
    .iter()
    .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
}

pub fn requires_hls_relay(stream: &StreamLink) -> bool {
    stream.hls
        && Url::parse(&stream.url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .is_some_and(|host| is_megaplay_media_host(&host))
}

fn stream_link(url: String, resolution: String, hls: bool, headers: RequestHeaders) -> StreamLink {
    StreamLink {
        url,
        resolution,
        hls,
        provider: "MegaPlay".into(),
        downloadable: true,
        headers,
        subtitles: vec![],
    }
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

async fn checked_json(response: Response, provider: &str) -> Result<Value> {
    let status = response.status();
    if status == StatusCode::TOO_MANY_REQUESTS {
        let retry_after_seconds = response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok())
            .unwrap_or(120);
        return Err(AniError::ProviderRateLimited {
            provider: provider.into(),
            retry_after_seconds,
        });
    }
    if !status.is_success() {
        return Err(AniError::Catalog {
            provider: provider.into(),
            message: format!("HTTP {status}"),
        });
    }
    response.json().await.map_err(Into::into)
}

async fn checked_text(response: Response, provider: &str) -> Result<String> {
    let status = response.status();
    if status == StatusCode::TOO_MANY_REQUESTS {
        let retry_after_seconds = response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok())
            .unwrap_or(120);
        return Err(AniError::ProviderRateLimited {
            provider: provider.into(),
            retry_after_seconds,
        });
    }
    if !status.is_success() {
        return Err(AniError::Catalog {
            provider: provider.into(),
            message: format!("HTTP {status}"),
        });
    }
    response.text().await.map_err(Into::into)
}

fn validate_remote_url(value: &str) -> Result<Url> {
    let url = Url::parse(value)?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AniError::Provider("media URL contains credentials".into()));
    }
    let loopback = url.host_str().is_some_and(|host| {
        host == "localhost"
            || host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    });
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        return Err(AniError::Provider("media URL must use HTTPS".into()));
    }
    Ok(url)
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

fn value_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn string_value(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(value) => Some(value.trim().to_owned()).filter(|value| !value.is_empty()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn number_value(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(value) => value.as_f64(),
        Value::String(value) => value.parse().ok(),
        _ => None,
    }
}

fn normalize_title(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    #[test]
    fn ids_round_trip_and_detect_provider() {
        let value = AnikotoId {
            anilist_id: Some("123".into()),
            mal_id: Some("456".into()),
            anikoto_id: Some("789".into()),
            title: Some("Example".into()),
            episodes: Some(12),
        };
        let encoded = encode_id(&value).unwrap();
        assert_eq!(decode_id(&encoded).unwrap(), value);
        assert_eq!(provider_from_show_id(&encoded), CatalogProvider::Anikoto);
        assert_eq!(
            provider_from_show_id("anikoto2:metadata"),
            CatalogProvider::Anikoto2
        );
        assert_eq!(provider_from_show_id("legacy"), CatalogProvider::Anikoto);
    }

    #[test]
    fn parses_series_and_candidate_order() {
        let episodes = parse_episode_payload(&json!({"data":{"episodes":[
            {"number":"2","episode_embed_id":"22"},
            {"number":1,"episode_embed_id":"11","embed_url":{"sub":"https://megaplay.buzz/explicit"}}
        ]}}));
        assert_eq!(episodes[0].number, "1");
        let id = AnikotoId {
            anilist_id: Some("1".into()),
            mal_id: Some("2".into()),
            ..Default::default()
        };
        let candidates = embed_candidates(
            "https://megaplay.buzz",
            &id,
            "1",
            TranslationType::Sub,
            Some(&episodes[0]),
        );
        assert_eq!(candidates[0], "https://megaplay.buzz/explicit");
        assert!(candidates[1].contains("/stream/s-2/11/sub"));
        assert!(candidates[2].contains("/stream/ani/1/1/sub"));
        assert!(candidates[3].contains("/stream/mal/2/1/sub"));
    }

    #[test]
    fn parses_nested_sources_and_subtitles() {
        let (sources, subtitles) = parse_megaplay_sources(&json!({
            "sources":{"links":[{"file":"https://megap.kotocdn.site/master.m3u8","label":"1080p"}]},
            "tracks":[{"file":"https://megap.kotocdn.site/en.vtt","label":"English","kind":"captions","default":true}]
        }));
        assert_eq!(sources[0].1, "1080p");
        assert_eq!(subtitles[0].label, "English");
        assert!(subtitles[0].default);
    }

    #[test]
    fn host_allowlist_rejects_lookalikes() {
        assert!(is_megaplay_media_host("megap.kotocdn.site"));
        assert!(!is_megaplay_media_host("kotocdn.site.example.com"));
        assert!(!is_megaplay_media_host("evilmegaplay.buzz"));
    }

    #[test]
    fn adult_search_results_are_filtered() {
        let payload = json!({"data":{"Page":{"media":[
            {"id":1,"title":{"english":"Safe"},"episodes":12,"isAdult":false},
            {"id":2,"title":{"english":"Adult"},"episodes":1,"isAdult":true}
        ]}}});
        assert_eq!(parse_search_payload(&payload, false).len(), 1);
        assert_eq!(parse_search_payload(&payload, true).len(), 2);
    }

    #[tokio::test]
    async fn search_tolerates_anilist_failure_when_catalog_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/recent-anime"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"data":[{"id":9,"ani_id":1,"title":"Example","episodes":2}]}),
            ))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let client = AnikotoClient::builder()
            .anikoto_api(server.uri())
            .anilist_api(format!("{}/graphql", server.uri()))
            .build()
            .unwrap();
        let values = client
            .search("example", TranslationType::Sub)
            .await
            .unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].provider, CatalogProvider::Anikoto);
        assert_eq!(
            decode_id(&values[0].id).unwrap().anikoto_id.as_deref(),
            Some("9")
        );
    }

    #[tokio::test]
    async fn megaplay_candidates_fall_back_to_the_mal_route() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/stream/ani/1/1/sub"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/stream/mal/2/1/sub"))
            .respond_with(ResponseTemplate::new(200).set_body_string("<div data-id=\"99\"></div>"))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/stream/getSources"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"sources":{"file":"https://voltara.click/video.mp4"}})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = AnikotoClient::builder()
            .megaplay_base(server.uri())
            .build()
            .unwrap();
        let id = encode_id(&AnikotoId {
            anilist_id: Some("1".into()),
            mal_id: Some("2".into()),
            episodes: Some(1),
            ..Default::default()
        })
        .unwrap();
        let streams = client
            .streams(&id, "1", TranslationType::Sub)
            .await
            .unwrap();
        assert_eq!(streams[0].url, "https://voltara.click/video.mp4");
    }

    #[tokio::test]
    async fn live_anikoto_smoke_test_is_opt_in() {
        if std::env::var("ANI_CLI_LIVE_ANIKOTO").as_deref() != Ok("1") {
            return;
        }
        let results = AnikotoClient::new()
            .unwrap()
            .search("Frieren", TranslationType::Sub)
            .await
            .unwrap();
        assert!(!results.is_empty());
    }
}
