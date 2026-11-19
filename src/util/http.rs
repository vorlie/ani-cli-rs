use std::path::Path;

use reqwest::{RequestBuilder, header};
use url::Url;

use crate::{AniError, RequestHeaders, Result, SubtitleTrack};

pub(crate) fn apply_headers(
    mut request: RequestBuilder,
    headers: &RequestHeaders,
) -> RequestBuilder {
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

pub(crate) fn subtitle_extension(url: &str) -> &'static str {
    let extension = Url::parse(url).ok().and_then(|url| {
        Path::new(url.path())
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
    });
    match extension.as_deref() {
        Some("srt") => "srt",
        Some("ass") => "ass",
        Some("ssa") => "ssa",
        _ => "vtt",
    }
}

pub(crate) async fn fetch_subtitle_bytes(
    track: &SubtitleTrack,
    headers: &RequestHeaders,
    max_bytes: usize,
) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder().build()?;
    let request = apply_headers(client.get(&track.url), headers);
    let response = request.send().await?;
    if !response.status().is_success() {
        return Err(AniError::SubtitleDownloadFailed {
            track: track.label.clone(),
            reason: format!("subtitle server returned {}", response.status()),
        });
    }
    if let Some(length) = response
        .content_length()
        .filter(|&length| length > max_bytes as u64)
    {
        return Err(AniError::SubtitleSizeExceeded {
            size: length as usize,
            limit: max_bytes,
        });
    }
    let bytes = response.bytes().await?;
    if bytes.len() > max_bytes {
        return Err(AniError::SubtitleSizeExceeded {
            size: bytes.len(),
            limit: max_bytes,
        });
    }
    Ok(bytes.to_vec())
}

pub(crate) fn validate_remote_url(
    value: &str,
    allow_loopback_http: bool,
    reject_literal_ip: bool,
) -> Result<Url> {
    let url = Url::parse(value)?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AniError::ProviderUrlValidationFailed {
            reason: "media URL contains credentials".to_string(),
        });
    }
    if url.host_str().is_none() {
        return Err(AniError::ProviderUrlValidationFailed {
            reason: "unsafe provider URL".to_string(),
        });
    }
    let loopback = url.host_str().is_some_and(|host| {
        host == "localhost"
            || host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    });
    if url.scheme() != "https" && !(allow_loopback_http && url.scheme() == "http" && loopback) {
        return Err(AniError::ProviderUrlValidationFailed {
            reason: "media URL must use HTTPS".to_string(),
        });
    }
    if reject_literal_ip
        && url
            .host_str()
            .is_some_and(|host| host.parse::<std::net::IpAddr>().is_ok())
    {
        return Err(AniError::ProviderUrlValidationFailed {
            reason: "literal-IP provider URLs are not allowed".to_string(),
        });
    }
    Ok(url)
}
