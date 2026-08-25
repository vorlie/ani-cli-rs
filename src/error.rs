use std::error::Error as StdError;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, AniError>;

#[derive(Debug, Error)]
pub enum AniError {
    #[error("network request failed")]
    Network(String),
    #[error("malformed provider data")]
    Provider(String),
    #[error("catalog error")]
    Catalog { provider: String, message: String },
    #[error("provider rate limited")]
    ProviderRateLimited {
        provider: String,
        retry_after_seconds: u64,
    },
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
    DownloadFailed,
    #[error("could not determine state directory")]
    HistoryStateDirectory,
    #[error("player executable not found")]
    PlayerNotFound,
    #[error("player launch failed")]
    PlayerLaunchFailed,
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
    pub fn key(&self) -> &'static str {
        match self {
            Self::Network(_) => "errors.network",
            Self::Provider(_) => "errors.provider",
            Self::Catalog { .. } => "errors.catalog",
            Self::ProviderRateLimited { .. } => "errors.provider.rate_limited",
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
            Self::DownloadFailed => "errors.download.failed",
            Self::HistoryStateDirectory => "errors.history.state_directory",
            Self::PlayerNotFound => "errors.player.not_found",
            Self::PlayerLaunchFailed => "errors.player.launch_failed",
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
