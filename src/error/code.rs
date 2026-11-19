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
