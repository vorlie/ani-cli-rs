use std::error::Error as StdError;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, AniError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Network / Provider errors (ACL-1xxx)
    ProviderUnavailable,
    ProviderRequestFailed,
    ProviderInvalidResponse,
    ProviderRateLimited,
    ProviderCatalogError,
    ProviderUrlValidationFailed,
    ProviderResponseSizeExceeded,

    // Search / Anime / Episode errors (ACL-2xxx)
    NoSearchResults,
    EmptySearchQuery,
    NoEpisodesAvailable,
    InvalidEpisodeSelection,
    SelectionOutOfRange,
    CommandRequiresQuery,

    // Streaming / Source errors (ACL-3xxx)
    NoPlayableSources,
    SourceResolutionFailed,
    StreamUnavailable,
    UnsupportedEmbedHost,
    NoNativeStreams,

    // Download errors (ACL-4xxx)
    DownloadFailed,
    NoDownloadTool,
    HlsDownloadFailed,
    SubtitleDownloadFailed,
    SubtitleSizeExceeded,
    DownloadOutputError,

    // Playback errors (ACL-5xxx)
    PlayerNotFound,
    PlayerLaunchFailed,
    PlayerExitedWithError,
    AndroidTerminalRequired,
    GeneralPlayerError,

    // Filesystem / Configuration errors (ACL-6xxx)
    IoError,
    HistoryStateDirectoryError,
    HistoryOperationFailed,
    InvalidHistoryEntry,

    // Authentication / External services errors (ACL-7xxx)
    UpdateCheckFailed,
    InvalidReleaseTag,
    InstallerExecutionFailed,
    PlatformNotSupported,

    // Internal / Unexpected errors (ACL-9xxx)
    InternalError,
    JsonParsingError,
    UrlParsingError,
    InvalidInput,
}

impl ErrorCode {
    pub const fn id(self) -> &'static str {
        match self {
            // Network / Provider errors (ACL-1xxx)
            Self::ProviderUnavailable => "ACL-1001",
            Self::ProviderRequestFailed => "ACL-1002",
            Self::ProviderInvalidResponse => "ACL-1003",
            Self::ProviderRateLimited => "ACL-1004",
            Self::ProviderCatalogError => "ACL-1005",
            Self::ProviderUrlValidationFailed => "ACL-1006",
            Self::ProviderResponseSizeExceeded => "ACL-1007",

            // Search / Anime / Episode errors (ACL-2xxx)
            Self::NoSearchResults => "ACL-2001",
            Self::EmptySearchQuery => "ACL-2002",
            Self::NoEpisodesAvailable => "ACL-2003",
            Self::InvalidEpisodeSelection => "ACL-2004",
            Self::SelectionOutOfRange => "ACL-2005",
            Self::CommandRequiresQuery => "ACL-2006",

            // Streaming / Source errors (ACL-3xxx)
            Self::NoPlayableSources => "ACL-3001",
            Self::SourceResolutionFailed => "ACL-3002",
            Self::StreamUnavailable => "ACL-3003",
            Self::UnsupportedEmbedHost => "ACL-3004",
            Self::NoNativeStreams => "ACL-3005",

            // Download errors (ACL-4xxx)
            Self::DownloadFailed => "ACL-4001",
            Self::NoDownloadTool => "ACL-4002",
            Self::HlsDownloadFailed => "ACL-4003",
            Self::SubtitleDownloadFailed => "ACL-4004",
            Self::SubtitleSizeExceeded => "ACL-4005",
            Self::DownloadOutputError => "ACL-4006",

            // Playback errors (ACL-5xxx)
            Self::PlayerNotFound => "ACL-5001",
            Self::PlayerLaunchFailed => "ACL-5002",
            Self::PlayerExitedWithError => "ACL-5003",
            Self::AndroidTerminalRequired => "ACL-5004",
            Self::GeneralPlayerError => "ACL-5005",

            // Filesystem / Configuration errors (ACL-6xxx)
            Self::IoError => "ACL-6001",
            Self::HistoryStateDirectoryError => "ACL-6002",
            Self::HistoryOperationFailed => "ACL-6003",
            Self::InvalidHistoryEntry => "ACL-6004",

            // Authentication / External services errors (ACL-7xxx)
            Self::UpdateCheckFailed => "ACL-7001",
            Self::InvalidReleaseTag => "ACL-7002",
            Self::InstallerExecutionFailed => "ACL-7003",
            Self::PlatformNotSupported => "ACL-7004",

            // Internal / Unexpected errors (ACL-9xxx)
            Self::InternalError => "ACL-9001",
            Self::JsonParsingError => "ACL-9002",
            Self::UrlParsingError => "ACL-9003",
            Self::InvalidInput => "ACL-9004",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            // Network / Provider errors (ACL-1xxx)
            Self::ProviderUnavailable => "provider-unavailable",
            Self::ProviderRequestFailed => "provider-request-failed",
            Self::ProviderInvalidResponse => "provider-invalid-response",
            Self::ProviderRateLimited => "provider-rate-limited",
            Self::ProviderCatalogError => "provider-catalog-error",
            Self::ProviderUrlValidationFailed => "provider-url-validation-failed",
            Self::ProviderResponseSizeExceeded => "provider-response-size-exceeded",

            // Search / Anime / Episode errors (ACL-2xxx)
            Self::NoSearchResults => "no-search-results",
            Self::EmptySearchQuery => "empty-search-query",
            Self::NoEpisodesAvailable => "no-episodes-available",
            Self::InvalidEpisodeSelection => "invalid-episode-selection",
            Self::SelectionOutOfRange => "selection-out-of-range",
            Self::CommandRequiresQuery => "command-requires-query",

            // Streaming / Source errors (ACL-3xxx)
            Self::NoPlayableSources => "no-playable-sources",
            Self::SourceResolutionFailed => "source-resolution-failed",
            Self::StreamUnavailable => "stream-unavailable",
            Self::UnsupportedEmbedHost => "unsupported-embed-host",
            Self::NoNativeStreams => "no-native-streams",

            // Download errors (ACL-4xxx)
            Self::DownloadFailed => "download-failed",
            Self::NoDownloadTool => "no-download-tool",
            Self::HlsDownloadFailed => "hls-download-failed",
            Self::SubtitleDownloadFailed => "subtitle-download-failed",
            Self::SubtitleSizeExceeded => "subtitle-size-exceeded",
            Self::DownloadOutputError => "download-output-error",

            // Playback errors (ACL-5xxx)
            Self::PlayerNotFound => "player-not-found",
            Self::PlayerLaunchFailed => "player-launch-failed",
            Self::PlayerExitedWithError => "player-exited-with-error",
            Self::AndroidTerminalRequired => "android-terminal-required",
            Self::GeneralPlayerError => "general-player-error",

            // Filesystem / Configuration errors (ACL-6xxx)
            Self::IoError => "io-error",
            Self::HistoryStateDirectoryError => "history-state-directory-error",
            Self::HistoryOperationFailed => "history-operation-failed",
            Self::InvalidHistoryEntry => "invalid-history-entry",

            // Authentication / External services errors (ACL-7xxx)
            Self::UpdateCheckFailed => "update-check-failed",
            Self::InvalidReleaseTag => "invalid-release-tag",
            Self::InstallerExecutionFailed => "installer-execution-failed",
            Self::PlatformNotSupported => "platform-not-supported",

            // Internal / Unexpected errors (ACL-9xxx)
            Self::InternalError => "internal-error",
            Self::JsonParsingError => "json-parsing-error",
            Self::UrlParsingError => "url-parsing-error",
            Self::InvalidInput => "invalid-input",
        }
    }
}

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

// Additional conversions for structured errors
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ErrorVerbosity {
    Normal,
    Verbose,
    Debug,
}

pub struct ErrorReport<'a> {
    pub code: ErrorCode,
    pub title: &'static str,
    pub message: String,
    pub help: Option<String>,
    pub docs: Option<&'static str>,
    pub source: Option<&'a (dyn StdError + 'static)>,
    pub context: Option<ErrorContext>,
}

#[derive(Clone)]
pub struct ErrorContext {
    pub provider: Option<String>,
    pub anime: Option<String>,
    pub episode: Option<String>,
    pub mode: Option<String>,
    pub additional_fields: Vec<(String, String)>,
}

impl<'a> ErrorReport<'a> {
    pub fn from_error(error: &'a AniError) -> Self {
        let code = error.code();
        let title = match code {
            ErrorCode::ProviderUnavailable => "Provider unavailable",
            ErrorCode::ProviderRequestFailed => "Provider request failed",
            ErrorCode::ProviderInvalidResponse => "Provider returned invalid response",
            ErrorCode::ProviderRateLimited => "Provider rate limited",
            ErrorCode::ProviderCatalogError => "Provider catalog error",
            ErrorCode::ProviderUrlValidationFailed => "Provider URL validation failed",
            ErrorCode::ProviderResponseSizeExceeded => "Provider response size exceeded",

            ErrorCode::NoSearchResults => "No search results found",
            ErrorCode::EmptySearchQuery => "Empty search query",
            ErrorCode::NoEpisodesAvailable => "No episodes available",
            ErrorCode::InvalidEpisodeSelection => "Invalid episode selection",
            ErrorCode::SelectionOutOfRange => "Selection out of range",
            ErrorCode::CommandRequiresQuery => "Command requires query",

            ErrorCode::NoPlayableSources => "No playable sources found",
            ErrorCode::SourceResolutionFailed => "Source resolution failed",
            ErrorCode::StreamUnavailable => "Stream unavailable",
            ErrorCode::UnsupportedEmbedHost => "Unsupported embed host",
            ErrorCode::NoNativeStreams => "No native streams available",

            ErrorCode::DownloadFailed => "Download failed",
            ErrorCode::NoDownloadTool => "No download tool available",
            ErrorCode::HlsDownloadFailed => "HLS download failed",
            ErrorCode::SubtitleDownloadFailed => "Subtitle download failed",
            ErrorCode::SubtitleSizeExceeded => "Subtitle size exceeded",
            ErrorCode::DownloadOutputError => "Download output error",

            ErrorCode::PlayerNotFound => "Player not found",
            ErrorCode::PlayerLaunchFailed => "Player launch failed",
            ErrorCode::PlayerExitedWithError => "Player exited with error",
            ErrorCode::AndroidTerminalRequired => "Android terminal required",
            ErrorCode::GeneralPlayerError => "General player error",

            ErrorCode::IoError => "I/O error",
            ErrorCode::HistoryStateDirectoryError => "History state directory error",
            ErrorCode::HistoryOperationFailed => "History operation failed",
            ErrorCode::InvalidHistoryEntry => "Invalid history entry",

            ErrorCode::UpdateCheckFailed => "Update check failed",
            ErrorCode::InvalidReleaseTag => "Invalid release tag",
            ErrorCode::InstallerExecutionFailed => "Installer execution failed",
            ErrorCode::PlatformNotSupported => "Platform not supported",

            ErrorCode::InternalError => "Internal error",
            ErrorCode::JsonParsingError => "JSON parsing error",
            ErrorCode::UrlParsingError => "URL parsing error",
            ErrorCode::InvalidInput => "Invalid input",
        };

        let message = Self::message(error);
        let help = match code {
            ErrorCode::ProviderUnavailable => Some("Check your internet connection, try again later, or try another provider.".to_string()),
            ErrorCode::ProviderRateLimited => Some("Wait a few minutes and try again, or try another provider.".to_string()),
            ErrorCode::NoSearchResults => Some("Check your search query spelling, try a different search term, or try another provider.".to_string()),
            ErrorCode::EmptySearchQuery => Some("Provide a search query or use interactive mode if available.".to_string()),
            ErrorCode::NoEpisodesAvailable => Some("Try another provider, check if the anime is released, or try a different anime.".to_string()),
            ErrorCode::NoPlayableSources => Some("Try another provider, try subtitle/dub mode toggle, or try a different episode.".to_string()),
            ErrorCode::NoDownloadTool => Some("Install yt-dlp or FFmpeg and ensure they are in your PATH.".to_string()),
            ErrorCode::PlayerNotFound => Some("Install the player, check player installation, or update player path in configuration.".to_string()),
            ErrorCode::AndroidTerminalRequired => Some("Use interactive terminal on Android or use download mode instead.".to_string()),
            ErrorCode::InternalError => Some("Run again with --verbose flag and report the issue with full error details.".to_string()),
            _ => None,
        };

        let docs = Some("https://vorlie.github.io/ani-cli-rs/errors");

        // Extract context from error
        let context = Self::extract_context(error);

        Self {
            code,
            title,
            message,
            help,
            docs,
            source: error.source(),
            context,
        }
    }

    fn extract_context(error: &AniError) -> Option<ErrorContext> {
        let mut ctx = ErrorContext {
            provider: None,
            anime: None,
            episode: None,
            mode: None,
            additional_fields: Vec::new(),
        };

        match error {
            AniError::ProviderUnavailable { provider, .. } => {
                ctx.provider = Some(provider.clone());
            }
            AniError::ProviderRequestFailed { provider, .. } => {
                ctx.provider = Some(provider.clone());
            }
            AniError::ProviderInvalidResponse { provider, .. } => {
                ctx.provider = Some(provider.clone());
            }
            AniError::ProviderRateLimited {
                provider,
                retry_after_seconds,
            } => {
                ctx.provider = Some(provider.clone());
                ctx.additional_fields
                    .push(("retry_after".to_string(), format!("{retry_after_seconds}s")));
            }
            AniError::ProviderCatalogError { provider, .. } => {
                ctx.provider = Some(provider.clone());
            }
            AniError::NoPlayableSources {
                anime,
                episode,
                mode,
            } => {
                ctx.anime = anime.clone();
                ctx.episode = episode.clone();
                ctx.mode = mode.clone();
            }
            AniError::PlayerNotFound { executable } => {
                ctx.additional_fields
                    .push(("executable".to_string(), executable.clone()));
            }
            AniError::PlayerLaunchFailed { executable, .. } => {
                ctx.additional_fields
                    .push(("executable".to_string(), executable.clone()));
            }
            AniError::PlayerExitedWithError {
                executable,
                exit_code,
            } => {
                ctx.additional_fields
                    .push(("executable".to_string(), executable.clone()));
                if let Some(code) = exit_code {
                    ctx.additional_fields
                        .push(("exit_code".to_string(), code.to_string()));
                }
            }
            AniError::SubtitleSizeExceeded { size, limit } => {
                ctx.additional_fields
                    .push(("size".to_string(), format!("{} bytes", size)));
                ctx.additional_fields
                    .push(("limit".to_string(), format!("{} bytes", limit)));
            }
            AniError::ProviderResponseSizeExceeded {
                provider,
                size,
                limit,
            } => {
                ctx.provider = Some(provider.clone());
                ctx.additional_fields
                    .push(("size".to_string(), format!("{} bytes", size)));
                ctx.additional_fields
                    .push(("limit".to_string(), format!("{} bytes", limit)));
            }
            _ => {}
        }

        if ctx.provider.is_some() || ctx.anime.is_some() || !ctx.additional_fields.is_empty() {
            Some(ctx)
        } else {
            None
        }
    }

    fn message(error: &AniError) -> String {
        match error {
            AniError::ProviderUnavailable { provider, .. } => {
                format!("{provider} could not be reached.")
            }
            AniError::ProviderRequestFailed { provider, .. } => {
                format!("A request to {provider} failed.")
            }
            AniError::ProviderInvalidResponse {
                provider, message, ..
            } => format!("{provider} returned data ani-cli-rs could not understand: {message}"),
            AniError::ProviderRateLimited {
                provider,
                retry_after_seconds,
            } => format!("{provider} asked us to retry after {retry_after_seconds} seconds."),
            AniError::ProviderCatalogError {
                provider, message, ..
            } => format!("{provider}'s catalog request failed: {message}"),
            AniError::ProviderUrlValidationFailed { reason } => {
                format!("A provider URL failed validation: {reason}")
            }
            AniError::ProviderResponseSizeExceeded {
                provider,
                size,
                limit,
            } => format!("{provider} returned {size} bytes, exceeding the {limit}-byte limit."),
            AniError::NoEpisodesAvailable { anime: Some(anime) } => {
                format!("No episodes are available for {anime}.")
            }
            AniError::InvalidEpisodeSelection { episode, reason } => {
                format!("Episode selection {episode:?} is invalid: {reason}")
            }
            AniError::SelectionOutOfRange { max, selected } => match selected {
                Some(selected) => {
                    format!("Selection {selected} is outside the available range (1–{max}).")
                }
                None => format!("The selection is outside the available range (1–{max})."),
            },
            AniError::SourceResolutionFailed { provider, reason } => {
                format!("Could not resolve a playable source from {provider}: {reason}")
            }
            AniError::StreamUnavailable { reason } => {
                format!("The selected stream is unavailable: {reason}")
            }
            AniError::UnsupportedEmbedHost { host } => {
                format!("The provider uses unsupported embed host {host}.")
            }
            AniError::NoNativeStreams { provider } => {
                format!("{provider} did not provide a supported native stream.")
            }
            AniError::NoPlayableSources { anime, episode, mode } => {
                let context = match (anime, episode, mode) {
                    (Some(anim), Some(epi), Some(m)) => {
                        format!(" for {anim} — Episode {epi} ({m})")
                    }
                    (Some(anim), Some(epi), None) => {
                        format!(" for {anim} — Episode {epi}")
                    }
                    (Some(anim), None, Some(m)) => {
                        format!(" for {anim} ({m})")
                    }
                    (None, Some(epi), Some(m)) => {
                        format!(" for episode {epi} ({m})")
                    }
                    (Some(anim), None, None) => {
                        format!(" for {anim}")
                    }
                    (None, Some(epi), None) => {
                        format!(" for episode {epi}")
                    }
                    (None, None, Some(m)) => {
                        format!(" ({m})")
                    }
                    (None, None, None) => String::new(),
                };
                format!("No playable sources were found{context}.")
            }
            AniError::DownloadFailed { reason, .. } | AniError::HlsDownloadFailed { reason } => {
                format!("Download failed: {reason}")
            }
            AniError::SubtitleDownloadFailed { track, reason } => {
                format!("Could not download subtitle track {track:?}: {reason}")
            }
            AniError::SubtitleSizeExceeded { size, limit } => {
                format!("The subtitle track is {size} bytes, exceeding the {limit}-byte limit.")
            }
            AniError::DownloadOutputError { path, reason } => {
                format!("Could not write download output to {path}: {reason}")
            }
            AniError::PlayerNotFound { executable } => {
                format!("Player executable {executable:?} was not found.")
            }
            AniError::PlayerLaunchFailed { executable, reason } => {
                format!("Could not launch {executable:?}: {reason}")
            }
            AniError::PlayerExitedWithError {
                executable,
                exit_code,
            } => match exit_code {
                Some(code) => format!("{executable:?} exited with status {code}."),
                None => format!("{executable:?} exited unsuccessfully."),
            },
            AniError::GeneralPlayerError { reason } => format!("Player failed: {reason}"),
            AniError::IoError { source, context } => format!("Could not {context}: {source}"),
            AniError::HistoryOperationFailed { operation, reason } => {
                format!("Could not {operation} history: {reason}")
            }
            AniError::InvalidHistoryEntry { reason } => format!("Invalid history entry: {reason}"),
            AniError::UpdateCheckFailed { reason } => {
                format!("Could not check for updates: {reason}")
            }
            AniError::InvalidReleaseTag { tag } => {
                format!("GitHub returned an invalid release tag: {tag}")
            }
            AniError::InstallerExecutionFailed { exit_code } => match exit_code {
                Some(code) => format!("The installer exited with status {code}."),
                None => "Could not start the installer.".to_string(),
            },
            AniError::PlatformNotSupported { platform } => {
                format!("Automatic updates are not supported on {platform}.")
            }
            AniError::InternalError { message } => message.clone(),
            AniError::JsonParsingError { context, .. } => {
                format!("Could not parse JSON while {context}.")
            }
            AniError::UrlParsingError { context, .. } => {
                format!("Could not parse a URL while {context}.")
            }
            AniError::InvalidInput { input, reason } => {
                format!("Invalid input {input:?}: {reason}")
            }
            _ => error.to_string(),
        }
    }

    pub fn render(&self, verbosity: ErrorVerbosity) -> String {
        match verbosity {
            ErrorVerbosity::Normal => self.render_normal(),
            ErrorVerbosity::Verbose => self.render_verbose(),
            ErrorVerbosity::Debug => self.render_debug(),
        }
    }

    fn render_normal(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("error[{}]: {}\n", self.code.id(), self.title));
        output.push('\n');
        output.push_str(&self.message);
        output.push('\n');

        if let Some(help) = &self.help {
            output.push('\n');
            output.push_str(&format!("help: {}", help));
        }

        if let Some(docs) = self.docs {
            output.push_str(&format!("\ndocs: {docs}#{}", self.code.id()));
        }

        output
    }

    fn render_verbose(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("error[{}]: {}\n", self.code.id(), self.title));
        output.push('\n');
        output.push_str(&self.message);
        output.push('\n');

        // Add context information
        if let Some(ctx) = &self.context {
            let mut context_lines = Vec::new();
            if let Some(provider) = &ctx.provider {
                context_lines.push(format!("Provider: {}", provider));
            }
            if let Some(anime) = &ctx.anime {
                context_lines.push(format!("Anime: {}", anime));
            }
            if let Some(episode) = &ctx.episode {
                context_lines.push(format!("Episode: {}", episode));
            }
            if let Some(mode) = &ctx.mode {
                context_lines.push(format!("Mode: {}", mode));
            }
            for (key, value) in &ctx.additional_fields {
                context_lines.push(format!("{}: {}", key, value));
            }

            if !context_lines.is_empty() {
                output.push('\n');
                for line in context_lines {
                    output.push_str(&format!("{}\n", line));
                }
            }
        }

        if let Some(help) = &self.help {
            output.push('\n');
            output.push_str(&format!("help: {}", help));
        }

        if let Some(docs) = self.docs {
            output.push_str(&format!("\ndocs: {docs}#{}", self.code.id()));
        }

        // Add error chain
        if let Some(source) = self.source {
            output.push_str("\n\nCaused by:\n");
            let mut current = Some(source);
            let mut indent = 0;
            while let Some(err) = current {
                output.push_str(&format!("  {}: {}\n", "  ".repeat(indent), err));
                current = err.source();
                indent += 1;
                if indent > 5 {
                    output.push_str("  ...\n");
                    break;
                }
            }
        }

        output
    }

    fn render_debug(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("error[{}]\n", self.code.id()));
        output.push('\n');
        output.push_str("Debug information:\n");

        // Add detailed context
        if let Some(ctx) = &self.context {
            if let Some(provider) = &ctx.provider {
                output.push_str(&format!("  provider = {}\n", provider));
            }
            if let Some(anime) = &ctx.anime {
                output.push_str(&format!("  anime = {}\n", anime));
            }
            if let Some(episode) = &ctx.episode {
                output.push_str(&format!("  episode = {}\n", episode));
            }
            if let Some(mode) = &ctx.mode {
                output.push_str(&format!("  mode = {}\n", mode));
            }
            for (key, value) in &ctx.additional_fields {
                output.push_str(&format!("  {} = {}\n", key, value));
            }
        }

        output.push_str(&format!("  error_code = {}\n", self.code.id()));
        output.push_str(&format!("  error_slug = {}\n", self.code.slug()));

        // Add full error chain
        if let Some(source) = self.source {
            output.push_str("\nError chain:\n");
            let mut current = Some(source);
            let mut indent = 0;
            while let Some(err) = current {
                output.push_str(&format!("  [{}]: {}\n", indent, err));
                current = err.source();
                indent += 1;
                if indent > 10 {
                    output.push_str("  [...]\n");
                    break;
                }
            }
        }

        output.push_str("\nUser message:\n");
        output.push_str(&format!("  {}\n", self.title));
        output.push_str(&format!("  {}\n", self.message));

        if let Some(help) = &self.help {
            output.push_str("\nHelp:\n");
            output.push_str(&format!("  {}\n", help));
        }

        if let Some(docs) = self.docs {
            output.push_str(&format!("\nDocumentation: {docs}#{}\n", self.code.id()));
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::{AniError, ErrorCode, ErrorReport, ErrorVerbosity};

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

    #[test]
    fn verbose_rate_limit_report_includes_provider_and_retry_after() {
        let error = AniError::ProviderRateLimited {
            provider: "Anikoto.cz".to_string(),
            retry_after_seconds: 120,
        };

        let output = ErrorReport::from_error(&error).render(ErrorVerbosity::Verbose);

        assert!(output.contains("error[ACL-1004]: Provider rate limited"));
        assert!(output.contains("Provider: Anikoto.cz"));
        assert!(output.contains("retry_after: 120s"));
        assert!(output.contains("docs: https://vorlie.github.io/ani-cli-rs/errors#ACL-1004"));
    }

    #[test]
    fn normal_report_is_compact_and_actionable() {
        let error = AniError::NoPlayableSources {
            anime: Some("One Piece".to_string()),
            episode: Some("1132".to_string()),
            mode: Some("sub".to_string()),
        };

        let output = ErrorReport::from_error(&error).render(ErrorVerbosity::Normal);

        assert!(output.contains("error[ACL-3001]: No playable sources found"));
        assert!(output.contains("help: Try another provider"));
        assert!(!output.contains("Provider:"));
    }
}
