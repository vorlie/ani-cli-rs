use std::{io::IsTerminal, path::PathBuf, str::FromStr};

use ani_lib::{
    AniError, AnikotoClient, AnikotoCzClient, CatalogProvider, DownloadOptions, HistoryEntry,
    HistoryStore, Player, PlayerKind, PlayerOptions, Result, SearchOptions, SearchResult,
    StreamLink, TranslationType, choose_quality, download_stream, expand_episode_selection,
    provider_from_show_id,
};
use serde::Serialize;
use tracing::{debug, info};

use super::{
    cli::{Cli, Commands},
    schedule::display_next_episode_schedule,
    ui::{
        AfterPlayAction, SelectionPurpose, continue_selection, format_search_label,
        interactive_after_play, prompt_text, select_initial_episodes, select_search_result,
        should_offer_playback_controls,
    },
    updater,
};

#[cfg(debug_assertions)]
use super::showcase::{showcase_search, showcase_streams};

pub async fn run(cli: Cli) -> Result<()> {
    let effective_provider = cli
        .provider
        .or_else(|| cli.sort.map(|_| CatalogProvider::Anikoto2));

    if cli.update {
        return updater::run(false).await;
    }
    if let Some(Commands::Update { check }) = cli.command.as_ref() {
        return updater::run(*check).await;
    }
    // Spawn a background, best-effort check for newer releases and notify in-terminal.
    // Non-blocking and silently ignored on error.
    if std::io::stdout().is_terminal() {
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        tokio::spawn(async move {
            let _ = updater::check_and_notify_new_version(&current_version).await;
        });
    }
    if cli.next_episode_countdown {
        let query = if cli.query.is_empty() {
            if !std::io::stdin().is_terminal() {
                return Err(AniError::CommandRequiresQuery);
            }
            prompt_text("Search anime release schedule")?
        } else {
            cli.query.join(" ")
        };
        return display_next_episode_schedule(&query).await;
    }

    let clients = ProviderClients::new(cli.demo_mode())?;
    if let Some(command) = cli.command {
        return run_command(&clients, effective_provider, command).await;
    }
    let history = HistoryStore::platform_default()?;
    if cli.delete {
        history.clear().await?;
        println!("History deleted.");
        return Ok(());
    }
    let mode = if cli.dub {
        TranslationType::Dub
    } else {
        std::env::var("ANI_CLI_MODE")
            .ok()
            .as_deref()
            .map(TranslationType::from_str)
            .transpose()?
            .unwrap_or_default()
    };
    let player = build_legacy_player(&cli);
    let terminal = std::io::stdin().is_terminal();
    let mut use_continue = cli.continue_watching;
    let mut initial_query = if cli.query.is_empty() {
        None
    } else {
        Some(cli.query.join(" "))
    };
    loop {
        let (show, episodes, selected, prepared_downloads) = if use_continue {
            let Some((show, episodes, initial_episode)) =
                continue_selection(&clients, &history, mode, cli.select_nth).await?
            else {
                return Ok(());
            };
            let selection = cli.episode.clone().or(initial_episode).ok_or_else(|| {
                AniError::StreamUnavailable {
                    reason: "history entry has no next episode".to_string(),
                }
            })?;
            let selected = expand_episode_selection(&selection, &episodes)?;
            let prepared = if cli.download {
                Some(require_download_preflight(
                    preflight_downloads(&clients, &show, &selected, mode, &cli.quality).await?,
                )?)
            } else {
                None
            };
            (show, episodes, selected, prepared)
        } else {
            'search: loop {
                let query = if let Some(query) = initial_query.take() {
                    query
                } else {
                    prompt_text("Search anime")?
                };
                let results = clients
                    .search_with_options(
                        effective_provider.unwrap_or_default(),
                        &query,
                        mode,
                        SearchOptions {
                            allow_adult: cli.allow_adult,
                            sort: cli.sort,
                        },
                    )
                    .await?;
                'anime: loop {
                    let purpose = if cli.download {
                        SelectionPurpose::Download
                    } else {
                        SelectionPurpose::Watch
                    };
                    let Some(show) = select_search_result(&results, cli.select_nth, purpose)?
                    else {
                        continue 'search;
                    };
                    let episodes = clients.episodes(&show.id, show.provider, mode).await?;
                    'episode: loop {
                        let selection = if let Some(selection) = cli.episode.clone() {
                            selection
                        } else {
                            let Some(selection) =
                                select_initial_episodes(&episodes, cli.multi_selection, purpose)?
                            else {
                                if results.len() == 1 {
                                    continue 'search;
                                }
                                continue 'anime;
                            };
                            selection
                        };
                        let selected = expand_episode_selection(&selection, &episodes)?;
                        let prepared = if cli.download {
                            match preflight_downloads(
                                &clients,
                                &show,
                                &selected,
                                mode,
                                &cli.quality,
                            )
                            .await?
                            {
                                DownloadPreflight::Ready(prepared) => Some(prepared),
                                DownloadPreflight::Unavailable(failures)
                                    if can_retry_download_selection(
                                        cli.episode.is_some(),
                                        terminal,
                                        use_continue,
                                    ) =>
                                {
                                    print_preflight_failures(&failures);
                                    continue 'episode;
                                }
                                DownloadPreflight::Unavailable(failures) => {
                                    return Err(download_preflight_error(&failures));
                                }
                            }
                        } else {
                            None
                        };
                        break 'search (show, episodes, selected, prepared);
                    }
                }
            }
        };
        use_continue = false;
        let download_directory = std::env::var_os("ANI_CLI_DOWNLOAD_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let context = PlaybackContext {
            clients: &clients,
            history: &history,
            show: &show,
            episodes: &episodes,
            mode,
            download_directory,
            player: &player,
        };
        if let Some(prepared) = prepared_downloads {
            for episode in &prepared {
                execute_prepared_episode(&context, episode, true).await?;
            }
        } else {
            for episode in &selected {
                play_or_download(&context, episode, &cli.quality, false).await?;
            }
        }

        if should_offer_playback_controls(
            selected.len(),
            terminal,
            cli.download,
            cli.exit_after_play,
        ) && interactive_after_play(&context, &selected[0], &cli.quality).await?
            == AfterPlayAction::Search
        {
            initial_query = None;
            continue;
        }
        break;
    }
    Ok(())
}

pub enum ProviderClients {
    Live {
        anikoto: AnikotoClient,
        anikoto2: AnikotoCzClient,
    },
    #[cfg(debug_assertions)]
    Showcase,
}

impl ProviderClients {
    fn new(_showcase: bool) -> Result<Self> {
        #[cfg(debug_assertions)]
        if _showcase {
            return Ok(Self::Showcase);
        }
        Ok(Self::Live {
            anikoto: AnikotoClient::new()?,
            anikoto2: AnikotoCzClient::new()?,
        })
    }

    async fn search_with_options(
        &self,
        provider: CatalogProvider,
        query: &str,
        mode: TranslationType,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        match self {
            Self::Live { anikoto, anikoto2 } => match provider {
                CatalogProvider::Anikoto => anikoto.search_with_options(query, mode, options).await,
                CatalogProvider::Anikoto2 => {
                    anikoto2.search_with_options(query, mode, options).await
                }
            },
            #[cfg(debug_assertions)]
            Self::Showcase => Ok(showcase_search(provider, query, options)),
        }
    }

    pub async fn episodes(
        &self,
        show_id: &str,
        selected: CatalogProvider,
        mode: TranslationType,
    ) -> Result<Vec<String>> {
        match self {
            Self::Live { anikoto, anikoto2 } => match routed_provider(show_id, selected) {
                CatalogProvider::Anikoto => anikoto.episodes(show_id, mode).await,
                CatalogProvider::Anikoto2 => anikoto2.episodes(show_id, mode).await,
            },
            #[cfg(debug_assertions)]
            Self::Showcase => Ok((1..=12).map(|episode| episode.to_string()).collect()),
        }
    }

    pub async fn streams(
        &self,
        show_id: &str,
        selected: CatalogProvider,
        episode: &str,
        mode: TranslationType,
    ) -> Result<Vec<StreamLink>> {
        match self {
            Self::Live { anikoto, anikoto2 } => match routed_provider(show_id, selected) {
                CatalogProvider::Anikoto => anikoto.streams(show_id, episode, mode).await,
                CatalogProvider::Anikoto2 => anikoto2.streams(show_id, episode, mode).await,
            },
            #[cfg(debug_assertions)]
            Self::Showcase => Ok(showcase_streams(selected, episode)),
        }
    }
}

fn routed_provider(show_id: &str, selected: CatalogProvider) -> CatalogProvider {
    if show_id.starts_with("anikoto:") || show_id.starts_with("anikoto2:") {
        provider_from_show_id(show_id)
    } else {
        selected
    }
}

async fn run_command(
    clients: &ProviderClients,
    selected_provider: Option<CatalogProvider>,
    command: Commands,
) -> Result<()> {
    let provider = selected_provider.unwrap_or_default();
    match command {
        Commands::Search(args) => {
            let mode = TranslationType::from_str(&args.mode)?;
            let values = clients
                .search_with_options(
                    provider,
                    &args.query,
                    mode,
                    SearchOptions {
                        allow_adult: args.allow_adult,
                        sort: args.sort,
                    },
                )
                .await?;
            output(&values, args.json, |value| {
                format!("{}\t{}", value.id, format_search_label(value))
            })?;
        }
        Commands::Episodes(args) => {
            let values = clients
                .episodes(
                    &args.show_id,
                    provider,
                    TranslationType::from_str(&args.mode)?,
                )
                .await?;
            output(&values, args.json, |value| value.clone())?;
        }
        Commands::Links(args) => {
            let values = clients
                .streams(
                    &args.show_id,
                    provider,
                    &args.episode,
                    TranslationType::from_str(&args.mode)?,
                )
                .await?;
            if let Some(quality) = args.quality {
                let value = choose_quality(&values, &quality).ok_or_else(|| {
                    AniError::NoPlayableSources {
                        anime: None,
                        episode: Some(args.episode.clone()),
                        mode: None,
                    }
                })?;
                output(std::slice::from_ref(value), args.json, |value| {
                    format!("{}\t{}\t{}", value.resolution, value.provider, value.url)
                })?;
            } else {
                output(&values, args.json, |value| {
                    format!("{}\t{}\t{}", value.resolution, value.provider, value.url)
                })?;
            }
        }
        Commands::Play(args) => {
            let mode = TranslationType::from_str(&args.mode)?;
            let streams = clients
                .streams(&args.show_id, provider, &args.episode, mode)
                .await?;
            let stream = choose_quality(&streams, &args.quality).ok_or_else(|| {
                AniError::NoPlayableSources {
                    anime: Some(args.title.clone()),
                    episode: Some(args.episode.clone()),
                    mode: Some(mode.to_string()),
                }
            })?;
            let mut options = PlayerOptions::default_player();
            if let Some(executable) = args.player {
                options.executable = executable;
            }
            options.no_detach |= args.no_detach;
            options.exit_after_play = false;
            options.force_hls_relay = args.ignore_host_lists;
            Player::new(options)
                .play(stream, &format!("{} Episode {}", args.title, args.episode))
                .await?;
        }
        Commands::Download(args) => {
            let streams = clients
                .streams(
                    &args.show_id,
                    provider,
                    &args.episode,
                    TranslationType::from_str(&args.mode)?,
                )
                .await?;
            let stream = choose_quality(&streams, &args.quality).ok_or_else(|| {
                AniError::NoPlayableSources {
                    anime: Some(args.title.clone()),
                    episode: Some(args.episode.clone()),
                    mode: Some(args.mode.clone()),
                }
            })?;
            let options = DownloadOptions {
                directory: args.output.unwrap_or_else(|| PathBuf::from(".")),
                filename: format!("{} Episode {}", args.title, args.episode),
            };
            println!(
                "Saved {}",
                download_stream(stream, &options).await?.display()
            );
        }
        Commands::Update { .. } => unreachable!("update commands are handled before client setup"),
    }
    Ok(())
}

fn output<T: Serialize>(values: &[T], json: bool, text: impl Fn(&T) -> String) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(values)?);
    } else {
        for value in values {
            println!("{}", text(value));
        }
    }
    Ok(())
}

fn build_legacy_player(cli: &Cli) -> Player {
    let mut options = PlayerOptions::default_player();
    options.no_detach |= cli.no_detach;
    options.exit_after_play |= cli.exit_after_play;
    options.force_hls_relay = cli.ignore_host_lists;
    if cli.vlc {
        select_vlc_player(&mut options, cfg!(target_os = "android"), cfg!(windows));
    }
    if cli.syncplay {
        options.executable = PathBuf::from(if cfg!(windows) {
            "syncplay.exe"
        } else {
            "syncplay"
        });
        options.kind = PlayerKind::Syncplay;
    }
    let player = Player::new(options);
    info!(
        executable = %player.describe(),
        vlc = cli.vlc,
        syncplay = cli.syncplay,
        no_detach = cli.no_detach,
        exit_after_play = cli.exit_after_play,
        ignore_host_lists = cli.ignore_host_lists,
        "selected external player",
    );
    player
}

fn select_vlc_player(options: &mut PlayerOptions, android: bool, windows: bool) {
    if android {
        options.kind = PlayerKind::AndroidVlc;
        return;
    }
    options.executable = PathBuf::from(if windows { "vlc.exe" } else { "vlc" });
    options.kind = PlayerKind::Vlc;
}

pub struct PlaybackContext<'a> {
    pub clients: &'a ProviderClients,
    pub history: &'a HistoryStore,
    pub show: &'a SearchResult,
    pub episodes: &'a [String],
    pub mode: TranslationType,
    pub download_directory: PathBuf,
    pub player: &'a Player,
}

#[derive(Debug)]
struct PreparedEpisode {
    episode: String,
    stream: StreamLink,
}

#[derive(Debug)]
struct UnavailableDownload {
    episode: String,
    reason: String,
}

#[derive(Debug)]
enum DownloadPreflight {
    Ready(Vec<PreparedEpisode>),
    Unavailable(Vec<UnavailableDownload>),
}

async fn preflight_downloads(
    clients: &ProviderClients,
    show: &SearchResult,
    episodes: &[String],
    mode: TranslationType,
    quality: &str,
) -> Result<DownloadPreflight> {
    let mut results = Vec::with_capacity(episodes.len());
    for episode in episodes {
        println!("Checking {} episode {episode} sources...", show.name);
        let result = async {
            let streams = clients
                .streams(&show.id, show.provider, episode, mode)
                .await?;
            let stream = choose_download_stream(&streams, quality).ok_or_else(|| {
                AniError::NoPlayableSources {
                    anime: Some(show.name.clone()),
                    episode: Some(episode.clone()),
                    mode: Some(mode.to_string()),
                }
            })?;
            Ok(PreparedEpisode {
                episode: episode.clone(),
                stream,
            })
        }
        .await;
        results.push((episode.clone(), result));
    }
    collect_download_preflight(results)
}

fn choose_download_stream(streams: &[StreamLink], quality: &str) -> Option<StreamLink> {
    let downloadable = streams
        .iter()
        .filter(|stream| stream.downloadable)
        .cloned()
        .collect::<Vec<_>>();
    choose_quality(&downloadable, quality).cloned()
}

fn collect_download_preflight(
    results: Vec<(String, Result<PreparedEpisode>)>,
) -> Result<DownloadPreflight> {
    let mut prepared = Vec::with_capacity(results.len());
    let mut unavailable = Vec::new();
    for (episode, result) in results {
        match result {
            Ok(value) => prepared.push(value),
            Err(
                AniError::NoEpisodesAvailable { .. }
                | AniError::NoPlayableSources { .. }
                | AniError::UnavailableNoEpisodes
                | AniError::UnavailableNoStreams,
            ) => {
                unavailable.push(UnavailableDownload {
                    episode,
                    reason: "no downloadable sources".to_string(),
                });
            }
            Err(error) => return Err(error),
        }
    }
    if unavailable.is_empty() {
        Ok(DownloadPreflight::Ready(prepared))
    } else {
        Ok(DownloadPreflight::Unavailable(unavailable))
    }
}

fn can_retry_download_selection(explicit_episode: bool, terminal: bool, continuing: bool) -> bool {
    !explicit_episode && terminal && !continuing
}

fn print_preflight_failures(failures: &[UnavailableDownload]) {
    eprintln!("No downloads started. These episodes have no downloadable sources:");
    for failure in failures {
        eprintln!("  Episode {}: {}", failure.episode, failure.reason);
    }
    eprintln!("Choose episodes again, or go back to select another anime / season.");
}

fn download_preflight_error(failures: &[UnavailableDownload]) -> AniError {
    let details = failures
        .iter()
        .map(|failure| format!("episode {} ({})", failure.episode, failure.reason))
        .collect::<Vec<_>>()
        .join(", ");
    eprintln!("Download preflight failures: {details}");
    AniError::NoPlayableSources {
        anime: None,
        episode: None,
        mode: None,
    }
}

fn require_download_preflight(preflight: DownloadPreflight) -> Result<Vec<PreparedEpisode>> {
    match preflight {
        DownloadPreflight::Ready(prepared) => Ok(prepared),
        DownloadPreflight::Unavailable(failures) => Err(download_preflight_error(&failures)),
    }
}

async fn prepare_episode(
    context: &PlaybackContext<'_>,
    episode: &str,
    quality: &str,
) -> Result<PreparedEpisode> {
    println!("Fetching {} episode {episode}...", context.show.name);
    let streams = context
        .clients
        .streams(
            &context.show.id,
            context.show.provider,
            episode,
            context.mode,
        )
        .await?;
    let stream =
        choose_quality(&streams, quality)
            .cloned()
            .ok_or_else(|| AniError::NoPlayableSources {
                anime: Some(context.show.name.clone()),
                episode: Some(episode.to_string()),
                mode: Some(context.mode.to_string()),
            })?;
    Ok(PreparedEpisode {
        episode: episode.into(),
        stream,
    })
}

pub async fn play_or_download(
    context: &PlaybackContext<'_>,
    episode: &str,
    quality: &str,
    download: bool,
) -> Result<()> {
    debug!(
        show = %context.show.name,
        show_id = %context.show.id,
        episode,
        quality,
        download,
        "resolving episode stream",
    );
    let prepared = prepare_episode(context, episode, quality).await?;
    debug!(
        show = %context.show.name,
        episode = %prepared.episode,
        stream_url = %prepared.stream.url,
        stream_hls = prepared.stream.hls,
        resolution = %prepared.stream.resolution,
        provider = %prepared.stream.provider,
        "stream resolved",
    );
    execute_prepared_episode(context, &prepared, download).await
}

async fn execute_prepared_episode(
    context: &PlaybackContext<'_>,
    prepared: &PreparedEpisode,
    download: bool,
) -> Result<()> {
    let title = format!(
        "{} Episode {}",
        clean_title(&context.show.name),
        prepared.episode
    );
    if download {
        info!(title = %title, "downloading episode stream");
        let path = download_stream(
            &prepared.stream,
            &DownloadOptions {
                directory: context.download_directory.clone(),
                filename: title,
            },
        )
        .await?;
        println!("Saved {}", path.display());
    } else {
        info!(
            title = %title,
            player = %context.player.describe(),
            stream_url = %prepared.stream.url,
            stream_hls = prepared.stream.hls,
            "handing off episode stream to the player",
        );
        context.player.play(&prepared.stream, &title).await?;
    }
    context
        .history
        .update(HistoryEntry {
            episode: prepared.episode.clone(),
            show_id: context.show.id.clone(),
            title: context.show.name.clone(),
        })
        .await?;
    Ok(())
}

pub fn adjacent_episode(episodes: &[String], current: &str, delta: isize) -> Result<String> {
    let index = episodes
        .iter()
        .position(|value| value == current)
        .ok_or_else(|| AniError::InvalidEpisodeSelection {
            episode: current.to_string(),
            reason: "current episode is not in the available episode list".to_string(),
        })? as isize
        + delta;
    episodes
        .get(index as usize)
        .cloned()
        .ok_or(AniError::NoEpisodesAvailable { anime: None })
}

fn clean_title(value: &str) -> String {
    value
        .split('(')
        .next()
        .unwrap_or(value)
        .trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace())
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vlc_selection_uses_the_android_activity_launcher_on_termux() {
        let mut options = PlayerOptions {
            executable: "termux-am-starter".into(),
            kind: PlayerKind::AndroidMpv,
            no_detach: true,
            exit_after_play: false,
            force_hls_relay: false,
        };

        select_vlc_player(&mut options, true, false);

        assert_eq!(options.kind, PlayerKind::AndroidVlc);
        assert_eq!(options.executable, PathBuf::from("termux-am-starter"));
    }

    #[test]
    fn vlc_selection_keeps_desktop_executable_behavior() {
        let mut options = PlayerOptions::default_mpv();

        select_vlc_player(&mut options, false, true);

        assert_eq!(options.kind, PlayerKind::Vlc);
        assert_eq!(options.executable, PathBuf::from("vlc.exe"));
    }

    fn test_stream(resolution: &str, downloadable: bool) -> StreamLink {
        StreamLink {
            url: format!("https://media.example/{resolution}.mp4"),
            resolution: resolution.into(),
            hls: false,
            provider: "Test".into(),
            downloadable,
            headers: ani_lib::RequestHeaders::default(),
            subtitles: vec![],
        }
    }

    fn prepared(episode: &str) -> PreparedEpisode {
        PreparedEpisode {
            episode: episode.into(),
            stream: test_stream("720p", true),
        }
    }

    #[test]
    fn download_quality_ignores_non_downloadable_streams() {
        let streams = vec![test_stream("1080p", false), test_stream("720p", true)];

        assert_eq!(
            choose_download_stream(&streams, "best").unwrap().resolution,
            "720p"
        );
        assert_eq!(
            choose_download_stream(&streams, "1080p")
                .unwrap()
                .resolution,
            "720p"
        );
    }

    #[test]
    fn successful_download_preflight_preserves_episode_order() {
        let preflight = collect_download_preflight(vec![
            ("1".into(), Ok(prepared("1"))),
            ("2.5".into(), Ok(prepared("2.5"))),
            ("10".into(), Ok(prepared("10"))),
        ])
        .unwrap();
        let DownloadPreflight::Ready(values) = preflight else {
            panic!("expected a ready download batch");
        };

        assert_eq!(
            values
                .iter()
                .map(|value| value.episode.as_str())
                .collect::<Vec<_>>(),
            ["1", "2.5", "10"]
        );
    }

    #[test]
    fn download_preflight_aggregates_only_unavailable_episodes() {
        let preflight = collect_download_preflight(vec![
            ("1".into(), Ok(prepared("1"))),
            ("2".into(), Err(AniError::UnavailableNoEpisodes)),
            ("4".into(), Err(AniError::UnavailableNoEpisodes)),
        ])
        .unwrap();
        let DownloadPreflight::Unavailable(values) = preflight else {
            panic!("expected unavailable episodes");
        };

        assert_eq!(
            values
                .iter()
                .map(|value| value.episode.as_str())
                .collect::<Vec<_>>(),
            ["2", "4"]
        );
    }

    #[test]
    fn download_preflight_keeps_non_availability_errors_fatal() {
        let error = collect_download_preflight(vec![(
            "1".into(),
            Err(AniError::Network("offline".into())),
        )])
        .unwrap_err();

        assert!(matches!(error, AniError::Network(_)));
    }

    #[test]
    fn only_prompted_terminal_downloads_retry_selection() {
        assert!(can_retry_download_selection(false, true, false));
        assert!(!can_retry_download_selection(true, true, false));
        assert!(!can_retry_download_selection(false, false, false));
        assert!(!can_retry_download_selection(false, true, true));
    }
}
