//! Deterministic debug-only fixtures backing the showcase/dev recordings
//! (`--demo-mode`). This module is compiled only in debug builds.

use ani_lib::{
    CatalogProvider, RequestHeaders, SearchOptions, SearchResult, StreamLink, SubtitleTrack,
};

pub(crate) fn showcase_search(
    provider: CatalogProvider,
    query: &str,
    options: SearchOptions,
) -> Vec<SearchResult> {
    let titles = [
        ("starfall-atelier", "Starfall Atelier", 12.0, false),
        ("signal-bloom", "Signal Bloom", 10.0, false),
        ("midnight-orbit", "Midnight Orbit", 24.0, false),
        ("velvet-nebula", "Velvet Nebula", 8.0, true),
    ];
    let query = query.trim().to_ascii_lowercase();
    titles
        .into_iter()
        .filter(|(_, title, _, adult)| {
            (query.is_empty() || title.to_ascii_lowercase().contains(&query))
                && (options.allow_adult || !adult)
        })
        .map(|(id, name, episodes, _)| SearchResult {
            id: match provider {
                CatalogProvider::Anikoto => format!("anikoto:showcase-{id}"),
                CatalogProvider::Anikoto2 => format!("anikoto2:showcase-{id}"),
            },
            name: name.into(),
            episodes,
            provider,
        })
        .collect()
}

pub(crate) fn showcase_streams(provider: CatalogProvider, episode: &str) -> Vec<StreamLink> {
    let provider_name = match provider {
        CatalogProvider::Anikoto => "MegaPlay Showcase",
        CatalogProvider::Anikoto2 => "Anikoto.cz Showcase",
    };
    ["1080p", "720p", "480p"]
        .into_iter()
        .map(|resolution| StreamLink {
            url: format!(
                "https://media.showcase.invalid/starfall-atelier/episode-{episode}/{resolution}.m3u8"
            ),
            resolution: resolution.into(),
            hls: true,
            provider: provider_name.into(),
            downloadable: true,
            headers: RequestHeaders {
                referer: Some("https://showcase.invalid/".into()),
                origin: Some("https://showcase.invalid".into()),
                ..RequestHeaders::default()
            },
            subtitles: vec![SubtitleTrack {
                label: "English".into(),
                url: format!(
                    "https://media.showcase.invalid/starfall-atelier/episode-{episode}/en.vtt"
                ),
                default: true,
            }],
        })
        .collect()
}
