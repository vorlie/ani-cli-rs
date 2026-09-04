use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::{
    AnikotoClient, AnikotoCzClient, CatalogProvider, HlsRelay, Player, PlayerOptions, SearchResult,
    StreamLink, TranslationType,
};

#[derive(Clone, Debug, PartialEq)]
pub enum LoadingState {
    Idle,
    Searching,
    LoadingEpisodes,
    LoadingStreams,
    StartingPlayer,
}

pub enum GuiMessage {
    SearchResults(Vec<SearchResult>),
    EpisodesLoaded(Vec<String>),
    StreamsLoaded(Vec<StreamLink>),
    Error(String),
    PlayerStarted,
    RelayStarted(HlsRelay),
}

pub struct GuiState {
    // UI state
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub selected_show: Option<SearchResult>,
    pub episodes: Vec<String>,
    pub selected_episode: Option<String>,
    pub streams: Vec<StreamLink>,
    pub selected_stream: Option<StreamLink>,
    pub translation: TranslationType,
    pub provider: CatalogProvider,
    pub loading_state: LoadingState,
    pub error_message: Option<String>,

    // Async communication
    pub message_tx: mpsc::UnboundedSender<GuiMessage>,
    pub message_rx: mpsc::UnboundedReceiver<GuiMessage>,

    // Tokio runtime for async operations
    pub runtime: Runtime,

    // HLS relay to keep alive during playback
    pub active_relay: Option<HlsRelay>,

    // Library clients
    pub anikoto_client: Option<AnikotoClient>,
    pub anikoto_cz_client: Option<AnikotoCzClient>,
    pub player: Player,
}

impl GuiState {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");

        Self {
            search_query: String::new(),
            search_results: Vec::new(),
            selected_show: None,
            episodes: Vec::new(),
            selected_episode: None,
            streams: Vec::new(),
            selected_stream: None,
            translation: TranslationType::Sub,
            provider: CatalogProvider::Anikoto,
            loading_state: LoadingState::Idle,
            error_message: None,
            message_tx: tx,
            message_rx: rx,
            runtime,
            active_relay: None,
            anikoto_client: AnikotoClient::new().ok(),
            anikoto_cz_client: AnikotoCzClient::new().ok(),
            player: Player::new(PlayerOptions::default_player()),
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        self.loading_state = LoadingState::Idle;
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub fn message_sender(&self) -> mpsc::UnboundedSender<GuiMessage> {
        self.message_tx.clone()
    }
}
