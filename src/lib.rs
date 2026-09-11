// src/lib.rs
//! Reusable Anikoto clients and cross-platform ani-cli support modules.

mod anikoto;
mod anikoto_cz;
mod download;
mod error;
mod history;
mod hls_relay;
mod i18n;
mod models;
mod player;

#[cfg(feature = "gui")]
pub mod gui;

pub use anikoto::{
    AnikotoClient, AnikotoClientBuilder, provider_from_show_id,
    requires_hls_relay as anikoto_requires_hls_relay,
};
pub use anikoto_cz::{
    AnikotoCzClient, AnikotoCzClientBuilder, requires_hls_relay as anikoto_cz_requires_hls_relay,
};

/// Unified function to check if a stream requires HLS relay
/// This checks both anikoto and anikoto_cz domains
pub fn requires_hls_relay(stream: &StreamLink) -> bool {
    anikoto_requires_hls_relay(stream) || anikoto_cz_requires_hls_relay(stream)
}

/// Force all streams through HLS relay regardless of host allowlist
pub fn force_hls_relay(stream: &StreamLink) -> bool {
    stream.hls
}
pub use download::{DownloadOptions, download_stream};
pub use error::{AniError, Result};
pub use history::{HistoryEntry, HistoryStore};
pub use hls_relay::{HlsRelay, relay_stream, relay_stream_without_hls_subtitles};
pub use i18n::{I18n, Locale};
pub use models::{
    CatalogProvider, RequestHeaders, SearchOptions, SearchResult, SearchSort, StreamLink,
    SubtitleTrack, TranslationType, choose_quality, expand_episode_selection,
};
pub use player::{Player, PlayerKind, PlayerOptions};
