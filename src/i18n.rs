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
        match error {
            AniError::Network(msg) => format!("Network request failed: {msg}"),
            AniError::Provider(msg) => format!("Provider returned invalid data: {msg}"),
            AniError::Catalog { provider, message } => {
                format!("{provider} catalog error: {message}")
            }
            AniError::ProviderRateLimited {
                provider,
                retry_after_seconds,
            } => {
                format!(
                    "{provider} is rate limiting requests. Try again in {retry_after_seconds} seconds."
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

            AniError::DownloadNoDownloader => {
                "HLS downloads require yt-dlp or FFmpeg to be installed and available in PATH."
                    .to_string()
            }
            AniError::DownloadFailed => "HLS download failed.".to_string(),

            AniError::HistoryStateDirectory => {
                "Could not determine where to store history data.".to_string()
            }

            AniError::PlayerNotFound => {
                "Player executable not found. Make sure your configured player is installed."
                    .to_string()
            }

            AniError::PlayerLaunchFailed => "Could not launch the player.".to_string(),

            AniError::PlayerExitFailed => "Player exited with an error.".to_string(),

            AniError::PlayerAndroidTerminalRequired => {
                "Android HLS playback requires an interactive Termux terminal.".to_string()
            }

            AniError::InputSelectionOutOfRange => "Selection is out of range.".to_string(),

            AniError::InputEmptyQuery => "Search query cannot be empty.".to_string(),

            AniError::InputInvalidEpisode => "Invalid episode selection.".to_string(),

            AniError::InputRequiresQuery => "This command requires an anime query.".to_string(),

            AniError::UnavailableNoResults => "No results found.".to_string(),

            AniError::UnavailableNoStreams => {
                "No streams are available for this episode.".to_string()
            }

            AniError::UnavailableNoEpisodes => "No episodes are available.".to_string(),
        }
    }
}
