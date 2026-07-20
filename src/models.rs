use std::{cmp::Ordering, collections::BTreeMap, fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{AniError, Result};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TranslationType {
    #[default]
    Sub,
    Dub,
}

impl fmt::Display for TranslationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Sub => "sub",
            Self::Dub => "dub",
        })
    }
}

impl FromStr for TranslationType {
    type Err = AniError;
    fn from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "sub" => Ok(Self::Sub),
            "dub" => Ok(Self::Dub),
            _ => Err(AniError::Input(format!(
                "translation type must be sub or dub, got {value}"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct SearchOptions {
    /// Include titles marked as adult by AllAnime.
    pub allow_adult: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SearchResult {
    pub id: String,
    pub name: String,
    pub episodes: f64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct RequestHeaders {
    pub referer: Option<String>,
    pub origin: Option<String>,
    #[serde(default)]
    pub extra: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct StreamLink {
    pub url: String,
    pub resolution: String,
    pub hls: bool,
    pub provider: String,
    pub downloadable: bool,
    #[serde(default)]
    pub headers: RequestHeaders,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CryptoDebugInfo {
    pub source: String,
    pub epoch: u64,
    pub build_id: String,
    pub part_a: String,
    pub part_b: String,
    pub derived_key_hex: String,
    pub query_hash: String,
    pub api_url: String,
    pub referer: String,
    pub app_js_url: Option<String>,
    pub fetched_at_unix_ms: u64,
    pub cache_expires_at_unix_ms: u64,
    pub legacy_ctr: bool,
    pub error: Option<String>,
}

fn resolution_weight(value: &str) -> i32 {
    let digits: String = value.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().unwrap_or_else(|_| {
        if value.eq_ignore_ascii_case("auto") {
            -1
        } else {
            0
        }
    })
}

fn provider_weight(value: &str) -> i32 {
    let value = value.to_ascii_lowercase();
    if value.contains("s-mp4") {
        3_000
    } else if value.contains("mp4") {
        2_000
    } else if value.contains("default") {
        1_000
    } else {
        0
    }
}

pub(crate) fn sort_streams(streams: &mut [StreamLink]) {
    streams.sort_by(|a, b| {
        provider_weight(&b.provider)
            .cmp(&provider_weight(&a.provider))
            .then_with(|| resolution_weight(&b.resolution).cmp(&resolution_weight(&a.resolution)))
            .then_with(|| b.hls.cmp(&a.hls))
            .then_with(|| a.provider.cmp(&b.provider))
    });
}

pub fn choose_quality<'a>(streams: &'a [StreamLink], quality: &str) -> Option<&'a StreamLink> {
    if streams.is_empty() {
        return None;
    }
    match quality.to_ascii_lowercase().as_str() {
        "best" => streams.first(),
        "worst" => streams
            .iter()
            .filter(|s| resolution_weight(&s.resolution) > 0)
            .min_by_key(|s| resolution_weight(&s.resolution))
            .or_else(|| streams.last()),
        requested => streams
            .iter()
            .find(|s| s.resolution.to_ascii_lowercase().contains(requested))
            .or_else(|| streams.first()),
    }
}

fn episode_number(value: &str) -> Option<f64> {
    value.parse::<f64>().ok().filter(|n| n.is_finite())
}

pub fn sort_episodes(episodes: &mut [String]) {
    episodes.sort_by(|a, b| match (episode_number(a), episode_number(b)) {
        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
        _ => a.cmp(b),
    });
}

pub fn expand_episode_selection(selection: &str, available: &[String]) -> Result<Vec<String>> {
    let trimmed = selection.trim();
    if trimmed == "-1" {
        return available
            .last()
            .cloned()
            .map(|v| vec![v])
            .ok_or_else(|| AniError::Unavailable("no episodes".into()));
    }
    if trimmed.contains(char::is_whitespace) {
        let requested: Vec<_> = trimmed.split_whitespace().map(str::to_owned).collect();
        if requested.iter().all(|v| available.contains(v)) {
            return Ok(requested);
        }
        return Err(AniError::Input(
            "one or more selected episodes do not exist".into(),
        ));
    }
    if let Some((start, end)) = trimmed.split_once('-') {
        let end = if end == "-1" || end.is_empty() {
            available.last().map(String::as_str).unwrap_or("")
        } else {
            end
        };
        let start_index = available
            .iter()
            .position(|v| v == start)
            .ok_or_else(|| AniError::Input(format!("episode {start} does not exist")))?;
        let end_index = available
            .iter()
            .position(|v| v == end)
            .ok_or_else(|| AniError::Input(format!("episode {end} does not exist")))?;
        if start_index > end_index {
            return Err(AniError::Input("episode range is reversed".into()));
        }
        return Ok(available[start_index..=end_index].to_vec());
    }
    if available.iter().any(|v| v == trimmed) {
        Ok(vec![trimmed.to_owned()])
    } else {
        Err(AniError::Input(format!("episode {trimmed} does not exist")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn stream(resolution: &str) -> StreamLink {
        StreamLink {
            url: resolution.into(),
            resolution: resolution.into(),
            hls: false,
            provider: "Default".into(),
            downloadable: true,
            headers: RequestHeaders::default(),
        }
    }

    #[test]
    fn quality_selection_falls_back_to_best() {
        let streams = vec![stream("1080p"), stream("720p"), stream("480p")];
        assert_eq!(
            choose_quality(&streams, "worst").unwrap().resolution,
            "480p"
        );
        assert_eq!(choose_quality(&streams, "720").unwrap().resolution, "720p");
        assert_eq!(
            choose_quality(&streams, "1440p").unwrap().resolution,
            "1080p"
        );
    }

    #[test]
    fn expands_ranges_and_latest() {
        let eps = vec!["1".into(), "2".into(), "2.5".into(), "3".into()];
        assert_eq!(
            expand_episode_selection("2-3", &eps).unwrap(),
            vec!["2", "2.5", "3"]
        );
        assert_eq!(expand_episode_selection("-1", &eps).unwrap(), vec!["3"]);
    }
}
