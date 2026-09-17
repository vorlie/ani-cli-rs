use crate::AniError;

pub struct I18n {
    locale: Locale,
}

pub enum Locale {
    En,
    // Pl later
}

impl Default for I18n {
    fn default() -> Self {
        Self::new(Locale::En)
    }
}

impl I18n {
    pub fn new(locale: Locale) -> Self {
        Self { locale }
    }

    pub fn error(&self, error: &AniError) -> String {
        match self.locale {
            Locale::En => self.error_en(error),
        }
    }

    fn error_en(&self, error: &AniError) -> String {
        const DOCS_TROUBLESHOOTING: &str =
            "https://vorlie.github.io/ani-cli-rs/support/troubleshooting/";
        match error {
            // New structured errors - use default display for now
            AniError::ProviderUnavailable { .. } => error.to_string(),
            AniError::ProviderRequestFailed { .. } => error.to_string(),
            AniError::ProviderInvalidResponse { .. } => error.to_string(),
            AniError::ProviderCatalogError { .. } => error.to_string(),
            AniError::ProviderUrlValidationFailed { .. } => error.to_string(),
            AniError::ProviderResponseSizeExceeded { .. } => error.to_string(),
            AniError::NoSearchResults => error.to_string(),
            AniError::EmptySearchQuery => error.to_string(),
            AniError::NoEpisodesAvailable { .. } => error.to_string(),
            AniError::InvalidEpisodeSelection { .. } => error.to_string(),
            AniError::SelectionOutOfRange { .. } => error.to_string(),
            AniError::CommandRequiresQuery => error.to_string(),
            AniError::NoPlayableSources { .. } => error.to_string(),
            AniError::SourceResolutionFailed { .. } => error.to_string(),
            AniError::NoDownloadTool => {
                format!(
                    "HLS downloads require yt-dlp or FFmpeg to be installed and available in PATH.\n\
                    Help: {DOCS_TROUBLESHOOTING}#mpv-vlc-syncplay-aria2c-yt-dlp-or-ffmpeg-not-found"
                )
            }
            AniError::StreamUnavailable { .. } => error.to_string(),
            AniError::UnsupportedEmbedHost { .. } => error.to_string(),
            AniError::NoNativeStreams { .. } => error.to_string(),
            AniError::DownloadFailed { .. } => error.to_string(),
            AniError::HlsDownloadFailed { .. } => error.to_string(),
            AniError::SubtitleDownloadFailed { .. } => error.to_string(),
            AniError::SubtitleSizeExceeded { .. } => error.to_string(),
            AniError::DownloadOutputError { .. } => error.to_string(),
            AniError::PlayerNotFound { .. } => error.to_string(),
            AniError::PlayerLaunchFailed { .. } => error.to_string(),
            AniError::PlayerExitedWithError { .. } => error.to_string(),
            AniError::AndroidTerminalRequired => error.to_string(),
            AniError::GeneralPlayerError { .. } => error.to_string(),
            AniError::IoError { .. } => error.to_string(),
            AniError::HistoryStateDirectoryError => error.to_string(),
            AniError::HistoryOperationFailed { .. } => error.to_string(),
            AniError::InvalidHistoryEntry { .. } => error.to_string(),
            AniError::UpdateCheckFailed { .. } => error.to_string(),
            AniError::InvalidReleaseTag { .. } => error.to_string(),
            AniError::InstallerExecutionFailed { .. } => error.to_string(),
            AniError::PlatformNotSupported { .. } => error.to_string(),
            AniError::InternalError { .. } => error.to_string(),
            AniError::JsonParsingError { .. } => error.to_string(),
            AniError::UrlParsingError { .. } => error.to_string(),
            AniError::InvalidInput { .. } => error.to_string(),

            // Legacy error handling
            AniError::Network(msg) => format!("Network request failed: {msg}"),
            AniError::Provider(msg) => format!(
                "Provider returned invalid data: {msg}\n\
                 Help: {DOCS_TROUBLESHOOTING}#search-works-but-sources-fail"
            ),
            AniError::Catalog { provider, message } => {
                format!(
                    "{provider} catalog error: {message}\n\
                     Help: {DOCS_TROUBLESHOOTING}#search-works-but-sources-fail"
                )
            }
            AniError::ProviderRateLimited {
                provider,
                retry_after_seconds,
            } => {
                format!(
                    "{provider} is rate limiting requests. Try again in {retry_after_seconds} seconds.\n\
                     Help: {DOCS_TROUBLESHOOTING}#search-works-but-sources-fail"
                )
            }

            AniError::Unavailable(msg) => format!("Episode unavailable: {msg}"),
            AniError::Player(msg) => format!("Player failed: {msg}"),
            AniError::Download(msg) => format!("Download failed: {msg}"),
            AniError::History(msg) => format!("History operation failed: {msg}"),
            AniError::Update(msg) => format!("Update failed: {msg}"),
            AniError::Input(msg) => format!("Invalid input: {msg}"),

            AniError::Io(msg) => format!("Could not access local data: {msg}"),
            AniError::Json(msg) => format!("Could not process response data: {msg}"),
            AniError::Url(msg) => format!("Invalid URL: {msg}"),

            AniError::DownloadNoDownloader => format!(
                "HLS downloads require yt-dlp or FFmpeg to be installed and available in PATH.\n\
                Help: {DOCS_TROUBLESHOOTING}#mpv-vlc-syncplay-aria2c-yt-dlp-or-ffmpeg-not-found"
            ),
            AniError::HlsDownloadFailedLegacy => format!(
                "HLS download failed.\n\
                Help: {DOCS_TROUBLESHOOTING}#part-remains-after-a-download"
            ),

            AniError::HistoryStateDirectory => {
                "Could not determine where to store history data.".to_string()
            }

            AniError::PlayerNotFoundLegacy => format!(
                "Player executable not found. Make sure your configured player is installed.\n\
                Help: {DOCS_TROUBLESHOOTING}#mpv-vlc-syncplay-aria2c-yt-dlp-or-ffmpeg-not-found"
            ),

            AniError::PlayerLaunchFailedLegacy => format!(
                "Could not launch the player.\n\
                Help: {DOCS_TROUBLESHOOTING}#mpv-vlc-syncplay-aria2c-yt-dlp-or-ffmpeg-not-found"
            ),

            AniError::PlayerExitFailed => format!(
                "Player exited with an error.\n\
                Help: {DOCS_TROUBLESHOOTING}#mpv-vlc-syncplay-aria2c-yt-dlp-or-ffmpeg-not-found"
            ),

            AniError::PlayerAndroidTerminalRequired => format!(
                "Android HLS playback requires an interactive Termux terminal.\n\
                Help: {DOCS_TROUBLESHOOTING}#termux-playback-stops-after-returning-to-the-terminal"
            ),

            AniError::InputSelectionOutOfRange => "Selection is out of range.".to_string(),

            AniError::InputEmptyQuery => "Search query cannot be empty.".to_string(),

            AniError::InputInvalidEpisode => "Invalid episode selection.".to_string(),

            AniError::InputRequiresQuery => "This command requires an anime query.".to_string(),

            AniError::UnavailableNoResults => "No results found.".to_string(),

            AniError::UnavailableNoStreams => format!(
                "No streams are available for this episode.\n\
                Help: {DOCS_TROUBLESHOOTING}#search-works-but-sources-fail"
            ),

            AniError::UnavailableNoEpisodes => format!(
                "No episodes are available.\n\
                Help: {DOCS_TROUBLESHOOTING}#search-works-but-sources-fail"
            ),
        }
    }
}
