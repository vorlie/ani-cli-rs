use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use regex::Regex;
use reqwest::{Response, StatusCode, header};
use url::Url;

use crate::{AniError, Result};

pub(crate) const DEFAULT_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36";
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const CACHE_LIMIT: usize = 100;

#[derive(Clone, Debug)]
struct Cached<T> {
    expires_at: Instant,
    value: T,
}

pub(crate) struct TtlCache<T> {
    entries: Mutex<HashMap<String, Cached<T>>>,
}

impl<T: Clone> TtlCache<T> {
    pub(crate) fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn get(&self, key: &str) -> Option<T> {
        let mut cache = self.entries.lock().ok()?;
        cache.retain(|_, value| value.expires_at > Instant::now());
        cache.get(key).map(|value| value.value.clone())
    }

    pub(crate) fn put(&self, key: String, value: T) {
        if let Ok(mut cache) = self.entries.lock() {
            if cache.len() >= CACHE_LIMIT
                && let Some(evict_key) = cache.keys().next().cloned()
            {
                cache.remove(&evict_key);
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
}

pub(crate) async fn checked_text(
    response: Response,
    provider: &str,
    max_bytes: Option<usize>,
) -> Result<String> {
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
        return Err(AniError::ProviderCatalogError {
            provider: provider.into(),
            message: format!("HTTP {}", status),
            source: None,
        });
    }
    if let Some(limit) = max_bytes {
        if let Some(length) = response
            .content_length()
            .filter(|&length| length > limit as u64)
        {
            return Err(AniError::ProviderResponseSizeExceeded {
                provider: provider.to_string(),
                size: length as usize,
                limit,
            });
        }
        let bytes = response.bytes().await?;
        if bytes.len() > limit {
            return Err(AniError::ProviderResponseSizeExceeded {
                provider: provider.to_string(),
                size: bytes.len(),
                limit,
            });
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    } else {
        response.text().await.map_err(Into::into)
    }
}

pub(crate) fn parse_data_id(html: &str) -> Option<String> {
    Regex::new(r#"(?i)\bdata-id=["'](\d+)["']"#)
        .expect("static regex")
        .captures(html)
        .map(|captures| captures[1].to_owned())
}

pub(crate) fn has_separate_audio_rendition(playlist: &str) -> bool {
    playlist.lines().any(|line| {
        let line = line.trim_start().to_ascii_uppercase();
        line.starts_with("#EXT-X-MEDIA:")
            && (line.contains("TYPE=AUDIO") || line.contains("TYPE=\"AUDIO\""))
    })
}

pub(crate) fn expand_hls_playlist(
    url: &str,
    text: &str,
    fallback_resolution: &str,
) -> Result<Vec<(String, String)>> {
    if !text.trim_start().starts_with("#EXTM3U") && !text.contains("#EXTM3U") {
        return Ok(vec![(fallback_resolution.into(), url.into())]);
    }
    // A media playlist contains only its video segments when audio is a
    // separate rendition. Keep the master so the player can follow its
    // EXT-X-MEDIA audio URI through the relay.
    if has_separate_audio_rendition(text) {
        return Ok(vec![(fallback_resolution.into(), url.into())]);
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
            .unwrap_or_else(|| fallback_resolution.into());
        if let Some(path) = lines[index + 1..]
            .iter()
            .map(|line| line.trim())
            .find(|line| !line.is_empty() && !line.starts_with('#'))
        {
            variants.push((label, base.join(path)?.to_string()));
        }
    }
    if variants.is_empty() {
        variants.push((fallback_resolution.into(), url.into()));
    }
    Ok(variants)
}

pub(crate) fn host_matches(host: &str, domain: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    let domain = domain.trim_end_matches('.').to_ascii_lowercase();
    host == domain || host.ends_with(&format!(".{domain}"))
}

pub(crate) fn is_megaplay_media_host(host: &str) -> bool {
    [
        "megaplay.buzz",
        "mewstream.buzz",
        "lostproject.club",
        "voltara.click",
        "kotocdn.site",
        "megap.shiora.top",
        "shiora.top",
        "megap.kotocdn.site",
        "megap.akirax.buzz",
        "akirax.buzz",
    ]
    .iter()
    .any(|domain| host_matches(host, domain))
}
