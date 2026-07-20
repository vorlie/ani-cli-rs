use thiserror::Error;

pub type Result<T> = std::result::Result<T, AniError>;

#[derive(Debug, Error)]
pub enum AniError {
    #[error("network request failed: {0}")]
    Network(#[from] reqwest::Error),
    #[error("AllAnime GraphQL error: {0}")]
    GraphQl(String),
    #[error(
        "AllAnime rate limit persisted after retries; try again in {retry_after_seconds} seconds"
    )]
    RateLimited { retry_after_seconds: u64 },
    #[error("AllAnime bootstrap failed: {0}")]
    Bootstrap(String),
    #[error("AllAnime payload decryption failed: {0}")]
    Decryption(String),
    #[error("malformed provider data: {0}")]
    Provider(String),
    #[error("episode is unavailable: {0}")]
    Unavailable(String),
    #[error("player failed: {0}")]
    Player(String),
    #[error("download failed: {0}")]
    Download(String),
    #[error("history operation failed: {0}")]
    History(String),
    #[error("invalid input: {0}")]
    Input(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),
}
