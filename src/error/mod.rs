use std::error::Error as StdError;

use thiserror::Error;

mod code;
mod report;

pub use code::ErrorCode;
pub use report::{ErrorReport, ErrorVerbosity};

pub type Result<T> = std::result::Result<T, AniError>;

#[derive(Debug, Error)]
pub enum AniError {
    // Network / Provider errors
    #[error("provider unavailable: {provider}")]
    ProviderUnavailable {
        provider: String,
        #[source]
        source: Option<reqwest::Error>,
    },

    #[error("provider request failed: {provider}")]
    ProviderRequestFailed {
        provider: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("provider returned invalid response: {provider}")]
    ProviderInvalidResponse {
        provider: String,
        message: String,
        #[source]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    #[error("provider rate limited: {provider}")]
    ProviderRateLimited {
        provider: String,
        retry_after_seconds: u64,
    },

    #[error("provider catalog error: {provider}")]
    ProviderCatalogError {
        provider: String,
        message: String,
        #[source]
        source: Option<reqwest::Error>,
    },

    #[error("provider URL validation failed: {reason}")]
    ProviderUrlValidationFailed { reason: String },

    #[error("provider response size exceeded")]
    ProviderResponseSizeExceeded {
        provider: String,
        size: usize,
        limit: usize,
    },

    // Search / Anime / Episode errors
    #[error("no search results found")]
    NoSearchResults,

    #[error("empty search query")]
    EmptySearchQuery,

    #[error("no episodes available")]
    NoEpisodesAvailable { anime: Option<String> },

    #[error("invalid episode selection")]
    InvalidEpisodeSelection { episode: String, reason: String },

    #[error("selection out of range")]
    SelectionOutOfRange { max: usize, selected: Option<usize> },

    #[error("command requires query")]
    CommandRequiresQuery,

    // Streaming / Source errors
    #[error("no playable sources found")]
    NoPlayableSources {
        anime: Option<String>,
        episode: Option<String>,
        mode: Option<String>,
    },

    #[error("source resolution failed")]
    SourceResolutionFailed { provider: String, reason: String },

    #[error("stream unavailable")]
    StreamUnavailable { reason: String },

    #[error("unsupported embed host: {host}")]
    UnsupportedEmbedHost { host: String },

    #[error("no native streams available")]
    NoNativeStreams { provider: String },

    // Download errors
    #[error("download failed")]
    DownloadFailed {
        reason: String,
        #[source]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    #[error("no download tool available")]
    NoDownloadTool,

    #[error("HLS download failed")]
    HlsDownloadFailed { reason: String },

    #[error("subtitle download failed")]
    SubtitleDownloadFailed { track: String, reason: String },

    #[error("subtitle size exceeded")]
    SubtitleSizeExceeded { size: usize, limit: usize },

    #[error("download output error")]
    DownloadOutputError { path: String, reason: String },

    // Playback errors
    #[error("player not found")]
    PlayerNotFound { executable: String },

    #[error("player launch failed")]
    PlayerLaunchFailed { executable: String, reason: String },

    #[error("player exited with error")]
    PlayerExitedWithError {
        executable: String,
        exit_code: Option<i32>,
    },

    #[error("Android playback requires interactive terminal")]
    AndroidTerminalRequired,

    #[error("general player error")]
    GeneralPlayerError { reason: String },

    // Filesystem / Configuration errors
    #[error("I/O error")]
    IoError {
        #[source]
        source: std::io::Error,
        context: String,
    },

    #[error("history state directory error")]
    HistoryStateDirectoryError,

    #[error("history operation failed")]
    HistoryOperationFailed { operation: String, reason: String },

    #[error("invalid history entry")]
    InvalidHistoryEntry { reason: String },

    // Authentication / External services errors
    #[error("update check failed")]
    UpdateCheckFailed { reason: String },

    #[error("invalid release tag")]
    InvalidReleaseTag { tag: String },

    #[error("installer execution failed")]
    InstallerExecutionFailed { exit_code: Option<i32> },

    #[error("platform not supported")]
    PlatformNotSupported { platform: String },

    // Internal / Unexpected errors
    #[error("internal error")]
    InternalError { message: String },

    #[error("JSON parsing error")]
    JsonParsingError {
        #[source]
        source: serde_json::Error,
        context: String,
    },

    #[error("URL parsing error")]
    UrlParsingError {
        #[source]
        source: url::ParseError,
        context: String,
    },

    #[error("invalid input")]
    InvalidInput { input: String, reason: String },

    // Legacy variants for backward compatibility (will be deprecated)
    #[error("network request failed")]
    Network(String),
    #[error("malformed provider data")]
    Provider(String),
    #[error("catalog error")]
    Catalog { provider: String, message: String },
    #[error("episode unavailable")]
    Unavailable(String),
    #[error("player failed")]
    Player(String),
    #[error("download failed")]
    Download(String),
    #[error("history operation failed")]
    History(String),
    #[error("update failed")]
    Update(String),
    #[error("invalid input")]
    Input(String),
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("JSON error")]
    Json(#[from] serde_json::Error),
    #[error("URL error")]
    Url(#[from] url::ParseError),
    // Specific user-facing error variants
    #[error("HLS downloads require yt-dlp or FFmpeg")]
    DownloadNoDownloader,
    #[error("HLS download failed")]
    HlsDownloadFailedLegacy,
    #[error("could not determine state directory")]
    HistoryStateDirectory,
    #[error("player executable not found")]
    PlayerNotFoundLegacy,
    #[error("player launch failed")]
    PlayerLaunchFailedLegacy,
    #[error("player exited with error")]
    PlayerExitFailed,
    #[error("Android playback requires interactive terminal")]
    PlayerAndroidTerminalRequired,
    #[error("selection out of range")]
    InputSelectionOutOfRange,
    #[error("empty search query")]
    InputEmptyQuery,
    #[error("episode selection invalid")]
    InputInvalidEpisode,
    #[error("command requires query")]
    InputRequiresQuery,
    #[error("no results found")]
    UnavailableNoResults,
    #[error("no streams available")]
    UnavailableNoStreams,
    #[error("no episodes available")]
    UnavailableNoEpisodes,
}

impl AniError {
    pub fn code(&self) -> ErrorCode {
        match self {
            // New structured errors
            // Network / Provider errors (ACL-1xxx)
            Self::ProviderUnavailable { .. } => ErrorCode::ProviderUnavailable,
            Self::ProviderRequestFailed { .. } => ErrorCode::ProviderRequestFailed,
            Self::ProviderInvalidResponse { .. } => ErrorCode::ProviderInvalidResponse,
            Self::ProviderRateLimited { .. } => ErrorCode::ProviderRateLimited,
            Self::ProviderCatalogError { .. } => ErrorCode::ProviderCatalogError,
            Self::ProviderUrlValidationFailed { .. } => ErrorCode::ProviderUrlValidationFailed,
            Self::ProviderResponseSizeExceeded { .. } => ErrorCode::ProviderResponseSizeExceeded,

            // Search / Anime / Episode errors (ACL-2xxx)
            Self::NoSearchResults => ErrorCode::NoSearchResults,
            Self::EmptySearchQuery => ErrorCode::EmptySearchQuery,
            Self::NoEpisodesAvailable { .. } => ErrorCode::NoEpisodesAvailable,
            Self::InvalidEpisodeSelection { .. } => ErrorCode::InvalidEpisodeSelection,
            Self::SelectionOutOfRange { .. } => ErrorCode::SelectionOutOfRange,
            Self::CommandRequiresQuery => ErrorCode::CommandRequiresQuery,

            // Streaming / Source errors (ACL-3xxx)
            Self::NoPlayableSources { .. } => ErrorCode::NoPlayableSources,
            Self::SourceResolutionFailed { .. } => ErrorCode::SourceResolutionFailed,
            Self::StreamUnavailable { .. } => ErrorCode::StreamUnavailable,
            Self::UnsupportedEmbedHost { .. } => ErrorCode::UnsupportedEmbedHost,
            Self::NoNativeStreams { .. } => ErrorCode::NoNativeStreams,

            // Download errors (ACL-4xxx)
            Self::DownloadFailed { .. } => ErrorCode::DownloadFailed,
            Self::NoDownloadTool => ErrorCode::NoDownloadTool,
            Self::HlsDownloadFailed { .. } => ErrorCode::HlsDownloadFailed,
            Self::SubtitleDownloadFailed { .. } => ErrorCode::SubtitleDownloadFailed,
            Self::SubtitleSizeExceeded { .. } => ErrorCode::SubtitleSizeExceeded,
            Self::DownloadOutputError { .. } => ErrorCode::DownloadOutputError,

            // Playback errors (ACL-5xxx)
            Self::PlayerNotFound { .. } => ErrorCode::PlayerNotFound,
            Self::PlayerLaunchFailed { .. } => ErrorCode::PlayerLaunchFailed,
            Self::PlayerExitedWithError { .. } => ErrorCode::PlayerExitedWithError,
            Self::PlayerNotFoundLegacy => ErrorCode::PlayerNotFound,
            Self::PlayerLaunchFailedLegacy => ErrorCode::PlayerLaunchFailed,
            Self::AndroidTerminalRequired => ErrorCode::AndroidTerminalRequired,
            Self::GeneralPlayerError { .. } => ErrorCode::GeneralPlayerError,

            // Filesystem / Configuration errors (ACL-6xxx)
            Self::IoError { .. } => ErrorCode::IoError,
            Self::HistoryStateDirectoryError => ErrorCode::HistoryStateDirectoryError,
            Self::HistoryOperationFailed { .. } => ErrorCode::HistoryOperationFailed,
            Self::InvalidHistoryEntry { .. } => ErrorCode::InvalidHistoryEntry,

            // Authentication / External services errors (ACL-7xxx)
            Self::UpdateCheckFailed { .. } => ErrorCode::UpdateCheckFailed,
            Self::InvalidReleaseTag { .. } => ErrorCode::InvalidReleaseTag,
            Self::InstallerExecutionFailed { .. } => ErrorCode::InstallerExecutionFailed,
            Self::PlatformNotSupported { .. } => ErrorCode::PlatformNotSupported,

            // Internal / Unexpected errors (ACL-9xxx)
            Self::InternalError { .. } => ErrorCode::InternalError,
            Self::JsonParsingError { .. } => ErrorCode::JsonParsingError,
            Self::UrlParsingError { .. } => ErrorCode::UrlParsingError,
            Self::InvalidInput { .. } => ErrorCode::InvalidInput,

            // Legacy variants for backward compatibility
            Self::Network(_) => ErrorCode::ProviderUnavailable,
            Self::Provider(_) => ErrorCode::ProviderInvalidResponse,
            Self::Catalog { .. } => ErrorCode::ProviderCatalogError,
            Self::UnavailableNoResults => ErrorCode::NoSearchResults,
            Self::InputEmptyQuery => ErrorCode::EmptySearchQuery,
            Self::UnavailableNoEpisodes => ErrorCode::NoEpisodesAvailable,
            Self::InputInvalidEpisode => ErrorCode::InvalidEpisodeSelection,
            Self::InputSelectionOutOfRange => ErrorCode::SelectionOutOfRange,
            Self::InputRequiresQuery => ErrorCode::CommandRequiresQuery,
            Self::UnavailableNoStreams => ErrorCode::NoPlayableSources,
            Self::Unavailable(_) => ErrorCode::StreamUnavailable,
            Self::Download(_) => ErrorCode::DownloadFailed,
            Self::DownloadNoDownloader => ErrorCode::NoDownloadTool,
            Self::HlsDownloadFailedLegacy => ErrorCode::HlsDownloadFailed,
            Self::PlayerExitFailed => ErrorCode::PlayerExitedWithError,
            Self::PlayerAndroidTerminalRequired => ErrorCode::AndroidTerminalRequired,
            Self::Player(_) => ErrorCode::GeneralPlayerError,
            Self::Io(_) => ErrorCode::IoError,
            Self::HistoryStateDirectory => ErrorCode::HistoryStateDirectoryError,
            Self::History(_) => ErrorCode::HistoryOperationFailed,
            Self::Update(_) => ErrorCode::UpdateCheckFailed,
            Self::Input(_) => ErrorCode::InvalidInput,
            Self::Json(_) => ErrorCode::JsonParsingError,
            Self::Url(_) => ErrorCode::UrlParsingError,
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            // New structured errors
            Self::ProviderUnavailable { .. } => "errors.provider.unavailable",
            Self::ProviderRequestFailed { .. } => "errors.provider.request_failed",
            Self::ProviderInvalidResponse { .. } => "errors.provider.invalid_response",
            Self::ProviderRateLimited { .. } => "errors.provider.rate_limited",
            Self::ProviderCatalogError { .. } => "errors.provider.catalog_error",
            Self::ProviderUrlValidationFailed { .. } => "errors.provider.url_validation_failed",
            Self::ProviderResponseSizeExceeded { .. } => "errors.provider.response_size_exceeded",
            Self::NoSearchResults => "errors.search.no_results",
            Self::EmptySearchQuery => "errors.search.empty_query",
            Self::NoEpisodesAvailable { .. } => "errors.search.no_episodes",
            Self::InvalidEpisodeSelection { .. } => "errors.search.invalid_episode",
            Self::SelectionOutOfRange { .. } => "errors.search.selection_out_of_range",
            Self::CommandRequiresQuery => "errors.search.requires_query",
            Self::NoPlayableSources { .. } => "errors.streaming.no_sources",
            Self::SourceResolutionFailed { .. } => "errors.streaming.resolution_failed",
            Self::StreamUnavailable { .. } => "errors.streaming.unavailable",
            Self::UnsupportedEmbedHost { .. } => "errors.streaming.unsupported_host",
            Self::NoNativeStreams { .. } => "errors.streaming.no_native_streams",
            Self::DownloadFailed { .. } => "errors.download.failed",
            Self::NoDownloadTool => "errors.download.no_tool",
            Self::HlsDownloadFailed { .. } => "errors.download.hls_failed",
            Self::HlsDownloadFailedLegacy => "errors.download.failed",
            Self::SubtitleDownloadFailed { .. } => "errors.download.subtitle_failed",
            Self::SubtitleSizeExceeded { .. } => "errors.download.subtitle_size_exceeded",
            Self::DownloadOutputError { .. } => "errors.download.output_error",
            Self::PlayerNotFound { .. } => "errors.player.not_found",
            Self::PlayerLaunchFailed { .. } => "errors.player.launch_failed",
            Self::PlayerNotFoundLegacy => "errors.player.not_found",
            Self::PlayerLaunchFailedLegacy => "errors.player.launch_failed",
            Self::PlayerExitedWithError { .. } => "errors.player.exited_with_error",
            Self::AndroidTerminalRequired => "errors.player.android_terminal_required",
            Self::GeneralPlayerError { .. } => "errors.player.general_error",
            Self::IoError { .. } => "errors.filesystem.io_error",
            Self::HistoryStateDirectoryError => "errors.filesystem.history_state_directory",
            Self::HistoryOperationFailed { .. } => "errors.filesystem.history_operation_failed",
            Self::InvalidHistoryEntry { .. } => "errors.filesystem.invalid_history_entry",
            Self::UpdateCheckFailed { .. } => "errors.external.update_check_failed",
            Self::InvalidReleaseTag { .. } => "errors.external.invalid_release_tag",
            Self::InstallerExecutionFailed { .. } => "errors.external.installer_execution_failed",
            Self::PlatformNotSupported { .. } => "errors.external.platform_not_supported",
            Self::InternalError { .. } => "errors.internal.error",
            Self::JsonParsingError { .. } => "errors.internal.json_parsing_error",
            Self::UrlParsingError { .. } => "errors.internal.url_parsing_error",
            Self::InvalidInput { .. } => "errors.internal.invalid_input",

            // Legacy variants for backward compatibility
            Self::Network(_) => "errors.network",
            Self::Provider(_) => "errors.provider",
            Self::Catalog { .. } => "errors.catalog",
            Self::Unavailable(_) => "errors.unavailable",
            Self::Player(_) => "errors.player",
            Self::Download(_) => "errors.download",
            Self::History(_) => "errors.history",
            Self::Update(_) => "errors.update",
            Self::Input(_) => "errors.input",
            Self::Io(_) => "errors.io",
            Self::Json(_) => "errors.json",
            Self::Url(_) => "errors.url",
            Self::DownloadNoDownloader => "errors.download.no_downloader",
            Self::HistoryStateDirectory => "errors.history.state_directory",
            Self::PlayerExitFailed => "errors.player.exit_failed",
            Self::PlayerAndroidTerminalRequired => "errors.player.android_terminal_required",
            Self::InputSelectionOutOfRange => "errors.input.selection_out_of_range",
            Self::InputEmptyQuery => "errors.input.empty_query",
            Self::InputInvalidEpisode => "errors.input.invalid_episode",
            Self::InputRequiresQuery => "errors.input.requires_query",
            Self::UnavailableNoResults => "errors.unavailable.no_results",
            Self::UnavailableNoStreams => "errors.unavailable.no_streams",
            Self::UnavailableNoEpisodes => "errors.unavailable.no_episodes",
        }
    }
}

impl From<reqwest::Error> for AniError {
    fn from(error: reqwest::Error) -> Self {
        let mut message = error.to_string();
        let mut source = error.source();
        while let Some(cause) = source {
            let cause_message = cause.to_string();
            if !message.ends_with(&cause_message) {
                message.push_str(": ");
                message.push_str(&cause_message);
            }
            source = cause.source();
        }
        Self::Network(message)
    }
}

impl AniError {
    pub fn io_error(source: std::io::Error, context: impl Into<String>) -> Self {
        Self::IoError {
            source,
            context: context.into(),
        }
    }

    pub fn json_error(source: serde_json::Error, context: impl Into<String>) -> Self {
        Self::JsonParsingError {
            source,
            context: context.into(),
        }
    }

    pub fn url_error(source: url::ParseError, context: impl Into<String>) -> Self {
        Self::UrlParsingError {
            source,
            context: context.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorCode;

    #[test]
    fn public_error_code_ids_are_stable() {
        assert_eq!(ErrorCode::ProviderUnavailable.id(), "ACL-1001");
        assert_eq!(ErrorCode::NoSearchResults.id(), "ACL-2001");
        assert_eq!(ErrorCode::NoPlayableSources.id(), "ACL-3001");
        assert_eq!(ErrorCode::DownloadFailed.id(), "ACL-4001");
        assert_eq!(ErrorCode::PlayerNotFound.id(), "ACL-5001");
        assert_eq!(ErrorCode::IoError.id(), "ACL-6001");
        assert_eq!(ErrorCode::UpdateCheckFailed.id(), "ACL-7001");
        assert_eq!(ErrorCode::InternalError.id(), "ACL-9001");
    }
}
