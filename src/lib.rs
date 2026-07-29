//! Reusable Anikoto clients and cross-platform ani-cli support modules.

mod anikoto;
mod anikoto_cz;
mod download;
mod error;
mod history;
mod hls_relay;
mod models;
mod player;

pub use anikoto::{AnikotoClient, AnikotoClientBuilder, provider_from_show_id, requires_hls_relay};
pub use anikoto_cz::{AnikotoCzClient, AnikotoCzClientBuilder};
pub use download::{DownloadOptions, download_stream};
pub use error::{AniError, Result};
pub use history::{HistoryEntry, HistoryStore};
pub use hls_relay::{HlsRelay, relay_stream, relay_stream_without_hls_subtitles};
pub use models::{
    CatalogProvider, RequestHeaders, SearchOptions, SearchResult, StreamLink, SubtitleTrack,
    TranslationType, choose_quality, expand_episode_selection,
};
pub use player::{Player, PlayerKind, PlayerOptions};
