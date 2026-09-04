use eframe::egui;

use egui_material_icons::icons::*;

use crate::{
    AniError, CatalogProvider, StreamLink,
    gui::state::{GuiMessage, GuiState, LoadingState},
    relay_stream_without_hls_subtitles, requires_hls_relay,
};

const ACCENT: egui::Color32 = egui::Color32::from_rgb(255, 145, 45);
const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(255, 165, 70);
const ACCENT_MUTED: egui::Color32 = egui::Color32::from_rgb(110, 70, 35);

pub struct AniGuiApp {
    state: GuiState,
}

impl AniGuiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_material_icons::initialize(&cc.egui_ctx);

        Self::configure_style(&cc.egui_ctx);

        Self {
            state: GuiState::new(),
        }
    }

    fn configure_style(ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();

        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.combo_width = 140.0;

        style.visuals = egui::Visuals::dark();

        // General colors.
        style.visuals.hyperlink_color = ACCENT;
        style.visuals.selection.bg_fill = ACCENT_MUTED;
        style.visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);

        // Inactive widgets.
        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(31, 31, 31);
        style.visuals.widgets.inactive.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 55, 55));
        style.visuals.widgets.inactive.fg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(205, 205, 205));

        // Hovered widgets.
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(45, 39, 32);
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ACCENT_HOVER);

        // Active / pressed widgets.
        style.visuals.widgets.active.bg_fill = ACCENT_MUTED;
        style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, ACCENT);
        style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

        // Non-interactive surfaces.
        style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(24, 24, 24);
        style.visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45));

        ctx.set_style(style);
    }

    // -------------------------------------------------------------------------
    // Async operations
    // -------------------------------------------------------------------------

    fn perform_search(&self, query: String) {
        let tx = self.state.message_sender();
        let anikoto_client = self.state.anikoto_client.clone();
        let anikoto_cz_client = self.state.anikoto_cz_client.clone();
        let provider = self.state.provider;
        let translation = self.state.translation;

        self.state.runtime.spawn(async move {
            let result = match provider {
                CatalogProvider::Anikoto => {
                    if let Some(client) = anikoto_client {
                        client.search(&query, translation).await
                    } else {
                        Err(AniError::Provider("Anikoto client not available".into()))
                    }
                }

                CatalogProvider::Anikoto2 => {
                    if let Some(client) = anikoto_cz_client {
                        client.search(&query, translation).await
                    } else {
                        Err(AniError::Provider("Anikoto.cz client not available".into()))
                    }
                }
            };

            match result {
                Ok(results) => {
                    let _ = tx.send(GuiMessage::SearchResults(results));
                }

                Err(e) => {
                    let _ = tx.send(GuiMessage::Error(format!("Search failed: {}", e)));
                }
            }
        });
    }

    fn load_episodes(&self, show_id: String) {
        let tx = self.state.message_sender();
        let anikoto_client = self.state.anikoto_client.clone();
        let anikoto_cz_client = self.state.anikoto_cz_client.clone();
        let provider = self.state.provider;
        let translation = self.state.translation;

        self.state.runtime.spawn(async move {
            let result = match provider {
                CatalogProvider::Anikoto => {
                    if let Some(client) = anikoto_client {
                        client.episodes(&show_id, translation).await
                    } else {
                        Err(AniError::Provider("Anikoto client not available".into()))
                    }
                }

                CatalogProvider::Anikoto2 => {
                    if let Some(client) = anikoto_cz_client {
                        client.episodes(&show_id, translation).await
                    } else {
                        Err(AniError::Provider("Anikoto.cz client not available".into()))
                    }
                }
            };

            match result {
                Ok(episodes) => {
                    let _ = tx.send(GuiMessage::EpisodesLoaded(episodes));
                }

                Err(e) => {
                    let _ = tx.send(GuiMessage::Error(format!("Failed to load episodes: {}", e)));
                }
            }
        });
    }

    fn load_streams(&self, show_id: String, episode: String) {
        let tx = self.state.message_sender();
        let anikoto_client = self.state.anikoto_client.clone();
        let anikoto_cz_client = self.state.anikoto_cz_client.clone();
        let provider = self.state.provider;
        let translation = self.state.translation;

        self.state.runtime.spawn(async move {
            let result = match provider {
                CatalogProvider::Anikoto => {
                    if let Some(client) = anikoto_client {
                        client.streams(&show_id, &episode, translation).await
                    } else {
                        Err(AniError::Provider("Anikoto client not available".into()))
                    }
                }

                CatalogProvider::Anikoto2 => {
                    if let Some(client) = anikoto_cz_client {
                        client.streams(&show_id, &episode, translation).await
                    } else {
                        Err(AniError::Provider("Anikoto.cz client not available".into()))
                    }
                }
            };

            match result {
                Ok(mut streams) => {
                    crate::models::sort_streams(&mut streams);

                    let _ = tx.send(GuiMessage::StreamsLoaded(streams));
                }

                Err(e) => {
                    let _ = tx.send(GuiMessage::Error(format!("Failed to load streams: {}", e)));
                }
            }
        });
    }

    fn start_playback(&self, stream: StreamLink, title: String) {
        let tx = self.state.message_sender();
        let player = self.state.player.clone();
        let stream_ref = stream.clone();

        self.state.runtime.spawn(async move {
            let result = if requires_hls_relay(&stream_ref) {
                match relay_stream_without_hls_subtitles(&stream_ref).await {
                    Ok((relay, local)) => {
                        let _ = tx.send(GuiMessage::RelayStarted(relay));

                        player.play(&local, &title).await
                    }

                    Err(e) => {
                        let _ = tx.send(GuiMessage::Error(format!(
                            "Failed to start HLS relay: {}",
                            e
                        )));

                        return;
                    }
                }
            } else {
                player.play(&stream_ref, &title).await
            };

            match result {
                Ok(_) => {
                    let _ = tx.send(GuiMessage::PlayerStarted);
                }

                Err(e) => {
                    let _ = tx.send(GuiMessage::Error(format!("Failed to start player: {}", e)));
                }
            }
        });
    }

    // -------------------------------------------------------------------------
    // Message handling
    // -------------------------------------------------------------------------

    fn process_messages(&mut self) {
        while let Ok(message) = self.state.message_rx.try_recv() {
            match message {
                GuiMessage::SearchResults(results) => {
                    self.state.search_results = results;
                    self.state.loading_state = LoadingState::Idle;
                }

                GuiMessage::EpisodesLoaded(episodes) => {
                    self.state.episodes = episodes;
                    self.state.loading_state = LoadingState::Idle;
                }

                GuiMessage::StreamsLoaded(streams) => {
                    self.state.streams = streams;
                    self.state.loading_state = LoadingState::Idle;
                }

                GuiMessage::Error(error) => {
                    self.state.set_error(error);
                }

                GuiMessage::PlayerStarted => {
                    self.state.loading_state = LoadingState::Idle;
                }

                GuiMessage::RelayStarted(relay) => {
                    self.state.active_relay = Some(relay);
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Header
    // -------------------------------------------------------------------------

    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading(egui::RichText::new("ani-cli-rs").strong().size(22.0));

                ui.label(egui::RichText::new("Anime streaming client").small().weak());
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new(ICON_SETTINGS).size(19.0))
                            .frame(false),
                    )
                    .on_hover_text("Settings")
                    .clicked()
                {
                    // Settings later.
                }
            });
        });

        ui.add_space(14.0);
    }

    // -------------------------------------------------------------------------
    // Search
    // -------------------------------------------------------------------------

    fn render_search(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(ICON_SEARCH).size(18.0).color(ACCENT));

            ui.label(egui::RichText::new("Search").strong().size(16.0));
        });

        ui.add_space(6.0);

        let searching = self.state.loading_state == LoadingState::Searching;

        ui.horizontal(|ui| {
            let available_width = ui.available_width();

            let button_width = 44.0;
            let spacing = ui.spacing().item_spacing.x;
            let text_width = (available_width - button_width - spacing).max(100.0);

            let response = ui.add_sized(
                [text_width, 36.0],
                egui::TextEdit::singleline(&mut self.state.search_query)
                    .hint_text("Search anime...")
                    .vertical_align(egui::Align::Center),
            );

            let enter_pressed =
                response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));

            let can_search = !searching && !self.state.search_query.trim().is_empty();

            let search_icon = if searching {
                ICON_PROGRESS_ACTIVITY
            } else {
                ICON_SEARCH
            };

            let clicked = ui
                .add_enabled(
                    can_search,
                    egui::Button::new(egui::RichText::new(search_icon).size(20.0))
                        .min_size(egui::vec2(button_width, 36.0)),
                )
                .clicked();

            if can_search && (clicked || enter_pressed) {
                let query = self.state.search_query.trim().to_string();

                self.state.error_message = None;
                self.state.loading_state = LoadingState::Searching;

                self.perform_search(query);
            }
        });

        ui.add_space(9.0);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(ICON_SOURCE).size(17.0).weak());

            ui.label(egui::RichText::new("Provider").small().weak());

            let controls_enabled = self.state.loading_state == LoadingState::Idle
                || self.state.loading_state == LoadingState::Searching;

            ui.add_enabled_ui(controls_enabled, |ui| {
                egui::ComboBox::from_id_salt("provider_selector")
                    .selected_text(match self.state.provider {
                        CatalogProvider::Anikoto => "Anikoto",
                        CatalogProvider::Anikoto2 => "Anikoto.cz",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.state.provider,
                            CatalogProvider::Anikoto,
                            "Anikoto",
                        );

                        ui.selectable_value(
                            &mut self.state.provider,
                            CatalogProvider::Anikoto2,
                            "Anikoto.cz",
                        );
                    });
            });

            ui.add_space(10.0);

            ui.label(egui::RichText::new(ICON_SUBTITLES).size(17.0).weak());

            ui.label(egui::RichText::new("Translation").small().weak());

            ui.add_enabled_ui(controls_enabled, |ui| {
                egui::ComboBox::from_id_salt("translation_selector")
                    .selected_text(match self.state.translation {
                        crate::TranslationType::Sub => "Sub",
                        crate::TranslationType::Dub => "Dub",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.state.translation,
                            crate::TranslationType::Sub,
                            "Sub",
                        );

                        ui.selectable_value(
                            &mut self.state.translation,
                            crate::TranslationType::Dub,
                            "Dub",
                        );
                    });
            });
        });
    }

    // -------------------------------------------------------------------------
    // Error
    // -------------------------------------------------------------------------

    fn render_error(&mut self, ui: &mut egui::Ui) {
        let Some(error) = self.state.error_message.clone() else {
            return;
        };

        ui.add_space(10.0);

        egui::Frame::group(ui.style())
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(ICON_ERROR_OUTLINE)
                            .size(18.0)
                            .color(egui::Color32::LIGHT_RED),
                    );

                    ui.label(
                        egui::RichText::new("Error")
                            .strong()
                            .color(egui::Color32::LIGHT_RED),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button(egui::RichText::new(ICON_CLOSE).size(17.0))
                            .on_hover_text("Clear error")
                            .clicked()
                        {
                            self.state.clear_error();
                        }
                    });
                });

                ui.add_space(4.0);

                ui.label(egui::RichText::new(error).color(egui::Color32::LIGHT_RED));
            });
    }

    // -------------------------------------------------------------------------
    // Search results
    // -------------------------------------------------------------------------

    fn render_results(&mut self, ui: &mut egui::Ui) {
        ui.add_space(14.0);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(ICON_SEARCH).size(18.0).color(ACCENT));

            ui.label(egui::RichText::new("Results").strong().size(16.0));

            if !self.state.search_results.is_empty() {
                ui.label(
                    egui::RichText::new(format!("{} found", self.state.search_results.len()))
                        .small()
                        .weak(),
                );
            }
        });

        ui.add_space(6.0);

        if self.state.loading_state == LoadingState::Searching {
            self.render_loading_card(ui, ICON_PROGRESS_ACTIVITY, "Searching...");

            return;
        }

        if self.state.search_results.is_empty() {
            egui::Frame::group(ui.style())
                .inner_margin(18.0)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new(ICON_SEARCH_OFF).size(26.0).weak());

                        ui.add_space(6.0);

                        ui.label(egui::RichText::new("No search results").strong());

                        ui.add_space(3.0);

                        ui.label(egui::RichText::new("Search for an anime to get started.").weak());
                    });
                });

            return;
        }

        egui::ScrollArea::vertical()
            .max_height(230.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let results = self.state.search_results.clone();

                for result in results {
                    let selected = self
                        .state
                        .selected_show
                        .as_ref()
                        .map(|show| show.id == result.id)
                        .unwrap_or(false);

                    let provider = match result.provider {
                        CatalogProvider::Anikoto => "Anikoto",
                        CatalogProvider::Anikoto2 => "Anikoto.cz",
                    };

                    let row_text = format!(
                        "{}\n{} episodes  •  {}",
                        result.name, result.episodes, provider
                    );

                    let response = ui.add_sized(
                        [ui.available_width(), 58.0],
                        egui::Button::new(egui::RichText::new(row_text).size(14.0))
                            .selected(selected),
                    );

                    if response.clicked() {
                        self.select_show(result);
                    }

                    ui.add_space(5.0);
                }
            });
    }

    fn select_show(&mut self, result: crate::SearchResult) {
        self.state.selected_show = Some(result.clone());

        self.state.episodes.clear();
        self.state.selected_episode = None;

        self.state.streams.clear();
        self.state.selected_stream = None;

        self.state.error_message = None;

        self.state.provider = result.provider;

        self.state.loading_state = LoadingState::LoadingEpisodes;

        self.load_episodes(result.id);
    }

    // -------------------------------------------------------------------------
    // Selected anime / playback
    // -------------------------------------------------------------------------

    fn render_selection(&mut self, ui: &mut egui::Ui) {
        let Some(show) = self.state.selected_show.clone() else {
            return;
        };

        ui.add_space(14.0);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(ICON_PLAYLIST_PLAY)
                    .size(18.0)
                    .color(ACCENT),
            );

            ui.label(egui::RichText::new("Selected").strong().size(16.0));
        });

        ui.add_space(6.0);

        egui::Frame::group(ui.style())
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(ICON_MOVIE).size(20.0).color(ACCENT));

                    ui.label(egui::RichText::new(&show.name).strong().size(15.0));
                });

                ui.add_space(2.0);

                ui.label(
                    egui::RichText::new(format!(
                        "{} episodes  •  {}",
                        show.episodes,
                        match show.provider {
                            CatalogProvider::Anikoto => "Anikoto",
                            CatalogProvider::Anikoto2 => "Anikoto.cz",
                        }
                    ))
                    .small()
                    .weak(),
                );

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    // Episode
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(ICON_LIST_ALT).size(16.0).weak());

                            ui.label(egui::RichText::new("Episode").small().weak());
                        });

                        let loading = self.state.loading_state == LoadingState::LoadingEpisodes;

                        ui.add_enabled_ui(!loading, |ui| {
                            egui::ComboBox::from_id_salt("episode_selector")
                                .width(150.0)
                                .selected_text(
                                    self.state
                                        .selected_episode
                                        .as_deref()
                                        .unwrap_or("Select episode"),
                                )
                                .show_ui(ui, |ui| {
                                    let episodes = self.state.episodes.clone();

                                    for episode in episodes {
                                        let selected =
                                            self.state.selected_episode.as_ref() == Some(&episode);

                                        if ui.selectable_label(selected, &episode).clicked() {
                                            self.select_episode(episode);
                                        }
                                    }
                                });
                        });
                    });

                    ui.add_space(14.0);

                    // Source
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(ICON_SOURCE).size(16.0).weak());

                            ui.label(egui::RichText::new("Source").small().weak());
                        });

                        let loading = self.state.loading_state == LoadingState::LoadingStreams;

                        ui.add_enabled_ui(!loading, |ui| {
                            let selected_text = self
                                .state
                                .selected_stream
                                .as_ref()
                                .map(|stream| stream.provider.as_str())
                                .unwrap_or("Select source");

                            egui::ComboBox::from_id_salt("source_selector")
                                .width(150.0)
                                .selected_text(selected_text)
                                .show_ui(ui, |ui| {
                                    let streams = self.state.streams.clone();

                                    for stream in streams {
                                        let selected = self
                                            .state
                                            .selected_stream
                                            .as_ref()
                                            .map(|selected| selected.url == stream.url)
                                            .unwrap_or(false);

                                        let label = stream.provider.clone();

                                        if ui.selectable_label(selected, label).clicked() {
                                            self.state.selected_stream = Some(stream);
                                        }
                                    }
                                });
                        });
                    });

                    ui.add_space(14.0);

                    // Quality
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(ICON_HD).size(16.0).weak());

                            ui.label(egui::RichText::new("Quality").small().weak());
                        });

                        let loading = self.state.loading_state == LoadingState::LoadingStreams;

                        ui.add_enabled_ui(!loading, |ui| {
                            let selected_text = self
                                .state
                                .selected_stream
                                .as_ref()
                                .map(|stream| stream.resolution.as_str())
                                .unwrap_or("Select quality");

                            egui::ComboBox::from_id_salt("quality_selector")
                                .width(110.0)
                                .selected_text(selected_text)
                                .show_ui(ui, |ui| {
                                    let streams = self.state.streams.clone();

                                    for stream in streams {
                                        let selected = self
                                            .state
                                            .selected_stream
                                            .as_ref()
                                            .map(|selected| selected.url == stream.url)
                                            .unwrap_or(false);

                                        let label =
                                            format!("{} • {}", stream.resolution, stream.provider);

                                        if ui.selectable_label(selected, label).clicked() {
                                            self.state.selected_stream = Some(stream);
                                        }
                                    }
                                });
                        });
                    });
                });

                ui.add_space(12.0);

                let can_play = self.state.selected_stream.is_some()
                    && self.state.selected_episode.is_some()
                    && self.state.loading_state == LoadingState::Idle;

                ui.add_enabled_ui(can_play, |ui| {
                    let width = ui.available_width();

                    let button = egui::Button::new(
                        egui::RichText::new(format!("{}  Play Episode", ICON_PLAY_ARROW))
                            .strong()
                            .size(15.0),
                    );

                    if ui.add_sized([width, 42.0], button).clicked() {
                        if let (Some(stream), Some(show)) = (
                            self.state.selected_stream.clone(),
                            self.state.selected_show.clone(),
                        ) {
                            self.state.error_message = None;

                            self.state.loading_state = LoadingState::StartingPlayer;

                            self.start_playback(stream, show.name);
                        }
                    }
                });
            });
    }

    fn select_episode(&mut self, episode: String) {
        self.state.selected_episode = Some(episode.clone());

        self.state.streams.clear();
        self.state.selected_stream = None;

        if let Some(show) = &self.state.selected_show {
            self.state.loading_state = LoadingState::LoadingStreams;

            self.load_streams(show.id.clone(), episode);
        }
    }

    // -------------------------------------------------------------------------
    // Loading / status
    // -------------------------------------------------------------------------

    fn render_loading_card(&self, ui: &mut egui::Ui, icon: &str, text: &str) {
        egui::Frame::group(ui.style())
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(icon).size(26.0).color(ACCENT));

                    ui.add_space(6.0);

                    ui.label(egui::RichText::new(text).weak());
                });
            });
    }

    fn render_status(&self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.separator();

        ui.horizontal(|ui| {
            let (indicator, text, active) = match self.state.loading_state {
                LoadingState::Idle => (ICON_CHECK_CIRCLE, "Ready", false),

                LoadingState::Searching => (ICON_PROGRESS_ACTIVITY, "Searching...", true),

                LoadingState::LoadingEpisodes => {
                    (ICON_PROGRESS_ACTIVITY, "Loading episodes...", true)
                }

                LoadingState::LoadingStreams => {
                    (ICON_PROGRESS_ACTIVITY, "Resolving streams...", true)
                }

                LoadingState::StartingPlayer => (ICON_PLAY_ARROW, "Starting player...", true),
            };

            ui.label(egui::RichText::new(indicator).size(16.0).color(if active {
                ACCENT
            } else {
                egui::Color32::from_rgb(100, 100, 100)
            }));

            ui.label(egui::RichText::new(text).small().weak());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let provider = match self.state.provider {
                    CatalogProvider::Anikoto => "Anikoto",
                    CatalogProvider::Anikoto2 => "Anikoto.cz",
                };

                ui.label(
                    egui::RichText::new(format!("{}  {}", ICON_SOURCE, provider))
                        .small()
                        .weak(),
                );
            });
        });
    }
}

// -----------------------------------------------------------------------------
// eframe
// -----------------------------------------------------------------------------

impl eframe::App for AniGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_messages();

        if self.state.loading_state != LoadingState::Idle {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_width(ui.available_width());

            self.render_header(ui);
            self.render_error(ui);
            self.render_search(ui);
            self.render_results(ui);
            self.render_selection(ui);
            self.render_status(ui);
        });
    }
}
