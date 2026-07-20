//! Reusable AllAnime client and desktop ani-cli support modules.

mod cipher;
mod client;
mod crypto;
mod download;
mod error;
mod history;
mod models;
mod player;

pub use cipher::{CipherMapInfo, parse_upstream_cipher_map};
pub use client::{AllAnimeClient, AllAnimeClientBuilder};
pub use download::{DownloadOptions, download_stream};
pub use error::{AniError, Result};
pub use history::{HistoryEntry, HistoryStore};
pub use models::{
    CryptoDebugInfo, RequestHeaders, SearchOptions, SearchResult, StreamLink, TranslationType,
    choose_quality, expand_episode_selection,
};
pub use player::{Player, PlayerKind, PlayerOptions};
