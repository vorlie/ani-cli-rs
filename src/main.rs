use std::{io::IsTerminal, path::PathBuf, str::FromStr};

use ani_cli::{
    AllAnimeClient, AniError, DownloadOptions, HistoryEntry, HistoryStore, Player, PlayerKind,
    PlayerOptions, Result, SearchOptions, SearchResult, TranslationType, choose_quality,
    download_stream, expand_episode_selection,
};
use clap::{Args, Parser, Subcommand};
use dialoguer::{FuzzySelect, Input, MultiSelect, Select, theme::ColorfulTheme};
use serde::{Deserialize, Serialize};

const LONG_ABOUT: &str = "A cross-platform Rust port of ani-cli for browsing, resolving, playing, and downloading anime from AllAnime.\n\nThe interactive workflow searches the subbed or dubbed catalog, lists available episodes, resolves current provider links, selects the requested quality, and opens an external player. Watch history uses the Bash ani-cli tab-separated format, so an existing history directory can be reused.\n\nThe scraper performs HTTP and cryptography internally; curl, sed, OpenSSL, Botan, and fzf are not required. Playback uses mpv by default, with optional VLC and Syncplay integrations. HLS downloads prefer yt-dlp and fall back to ffmpeg, while direct media downloads are handled internally.";

const AFTER_HELP: &str = "KEYBOARD NAVIGATION:\n  Arrow keys / Tab       Navigate menus\n  j / k                  Move down / up in action menus\n  h / l                  Change pages in action menus\n  Space / Enter          Select or toggle an item\n  Type                    Filter fuzzy anime/episode menus\n  Escape                 Go back immediately from a fuzzy menu\n  q / Escape             Leave an ordinary action menu\n\nEXAMPLES:\n  ani-cli-rs frieren\n  ani-cli-rs --allow-adult \"search query\"\n  ani-cli-rs --dub -q 720p \"cowboy bebop\"\n  ani-cli-rs -S 1 -e 2-4 \"one piece\"\n  ani-cli-rs --continue\n  ani-cli-rs --download -e 1 \"anime title\"\n  ani-cli-rs search --allow-adult --json \"search query\"\n  ani-cli-rs links --json SHOW_ID 1 --quality 1080p\n  ani-cli-rs debug --refresh\n\nENVIRONMENT:\n  ANI_CLI_MODE, ANI_CLI_PLAYER, ANI_CLI_DOWNLOAD_DIR, ANI_CLI_QUALITY,\n  ANI_CLI_HIST_DIR, ANI_CLI_ALLOW_ADULT, ANI_CLI_MULTI_SELECTION,\n  ANI_CLI_NO_DETACH, ANI_CLI_EXIT_AFTER_PLAY\n\nOfficial prebuilt releases are provided for Windows and Linux. macOS users can build from source.";

#[derive(Parser, Debug)]
#[command(
    name = "ani-cli-rs",
    version,
    about = "Browse, play, and download anime from AllAnime",
    long_about = LONG_ABOUT,
    after_help = AFTER_HELP
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Continue from the next unwatched episode in the ani-cli history.
    #[arg(short = 'c', long = "continue")]
    continue_watching: bool,
    /// Download selected episodes instead of launching a player.
    #[arg(short = 'd', long)]
    download: bool,
    /// Delete all saved watch history and exit.
    #[arg(short = 'D', long)]
    delete: bool,
    /// Launch the resolved stream through Syncplay.
    #[arg(short = 's', long)]
    syncplay: bool,
    /// Select a search or history result by its one-based index.
    #[arg(short = 'S', long = "select-nth")]
    select_nth: Option<usize>,
    /// Choose best, worst, or a resolution such as 1080p or 720p.
    #[arg(short = 'q', long, env = "ANI_CLI_QUALITY", default_value = "best")]
    quality: String,
    /// Use VLC instead of the default mpv player.
    #[arg(short = 'v', long)]
    vlc: bool,
    /// Select one episode, whitespace-separated episodes, or a range such as 2-5.
    #[arg(short = 'e', short_alias = 'r', long = "episode", alias = "range")]
    episode: Option<String>,
    /// Open the interactive episode picker in multi-selection mode.
    #[arg(long, env = "ANI_CLI_MULTI_SELECTION")]
    multi_selection: bool,
    /// Search and play the dubbed catalog instead of subtitles.
    #[arg(long)]
    dub: bool,
    /// Include titles marked as adult in AllAnime search results.
    #[arg(short = 'a', long, env = "ANI_CLI_ALLOW_ADULT")]
    allow_adult: bool,
    /// Keep the player attached and wait for it to exit.
    #[arg(long, env = "ANI_CLI_NO_DETACH")]
    no_detach: bool,
    /// Return the attached player's exit status after playback.
    #[arg(long, env = "ANI_CLI_EXIT_AFTER_PLAY")]
    exit_after_play: bool,
    /// Display the next scheduled raw and subtitled releases, then exit.
    #[arg(short = 'N', long = "nextep-countdown")]
    next_episode_countdown: bool,
    /// Anime title to search for; omitted titles are prompted interactively.
    #[arg(value_name = "QUERY")]
    query: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Search the AllAnime catalog without opening the interactive player.
    Search(SearchArgs),
    /// List the available episodes for an AllAnime show ID.
    Episodes(EpisodesArgs),
    /// Resolve and display playable sources for one episode.
    Links(LinksArgs),
    /// Resolve one episode and launch it in a media player.
    Play(ActionArgs),
    /// Resolve and download one episode.
    Download(ActionArgs),
    /// Display the active AllAnime crypto/bootstrap diagnostics.
    Debug {
        /// Discard cached crypto material and fetch the bootstrap again.
        #[arg(long)]
        refresh: bool,
    },
    /// Download, validate, and cache the latest upstream URL cipher map.
    RefreshCipherMap,
}

#[derive(Args, Debug)]
struct SearchArgs {
    /// Anime title to search for.
    query: String,
    /// Translation catalog to query: sub or dub.
    #[arg(long, default_value = "sub")]
    mode: String,
    /// Include titles marked as adult in search results.
    #[arg(short = 'a', long, env = "ANI_CLI_ALLOW_ADULT")]
    allow_adult: bool,
    /// Print structured JSON instead of tab-separated text.
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct EpisodesArgs {
    /// AllAnime show identifier returned by search.
    show_id: String,
    /// Translation catalog to query: sub or dub.
    #[arg(long, default_value = "sub")]
    mode: String,
    /// Print structured JSON instead of one episode per line.
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct LinksArgs {
    /// AllAnime show identifier returned by search.
    show_id: String,
    /// Episode number or fractional episode string.
    episode: String,
    /// Translation catalog to query: sub or dub.
    #[arg(long, default_value = "sub")]
    mode: String,
    /// Return only the best match for this quality.
    #[arg(short, long)]
    quality: Option<String>,
    /// Print complete stream metadata as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct ActionArgs {
    /// AllAnime show identifier returned by search.
    show_id: String,
    /// Episode number or fractional episode string.
    episode: String,
    /// Translation catalog to query: sub or dub.
    #[arg(long, default_value = "sub")]
    mode: String,
    /// Choose best, worst, or a specific resolution.
    #[arg(short, long, default_value = "best")]
    quality: String,
    /// Human-readable title used for the player or output filename.
    #[arg(long, default_value = "Anime")]
    title: String,
    /// Override the media-player executable for the play subcommand.
    #[arg(long)]
    player: Option<PathBuf>,
    /// Keep the selected player attached until it exits.
    #[arg(long)]
    no_detach: bool,
    /// Directory used by the download subcommand.
    #[arg(long)]
    output: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();
    if let Err(error) = run(Cli::parse()).await {
        eprintln!("\x1b[31merror:\x1b[0m {error}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    if cli.next_episode_countdown {
        let query = if cli.query.is_empty() {
            if !std::io::stdin().is_terminal() {
                return Err(AniError::Input(
                    "--nextep-countdown requires an anime query".into(),
                ));
            }
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Search anime release schedule")
                .interact_text()
                .map_err(dialog_error)?
        } else {
            cli.query.join(" ")
        };
        return display_next_episode_schedule(&query).await;
    }

    let client = AllAnimeClient::new()?;
    if let Some(command) = cli.command {
        return run_command(&client, command).await;
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
    let (show, episodes, selected) = if cli.continue_watching {
        let Some((show, episodes, initial_episode)) =
            continue_selection(&client, &history, mode, cli.select_nth).await?
        else {
            return Ok(());
        };
        let selection = cli
            .episode
            .clone()
            .or(initial_episode)
            .ok_or_else(|| AniError::Unavailable("history entry has no next episode".into()))?;
        let selected = expand_episode_selection(&selection, &episodes)?;
        (show, episodes, selected)
    } else {
        let mut initial_query = if cli.query.is_empty() {
            None
        } else {
            Some(cli.query.join(" "))
        };
        'search: loop {
            let query = if let Some(query) = initial_query.take() {
                query
            } else {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Search anime")
                    .interact_text()
                    .map_err(dialog_error)?
            };
            let results = client
                .search_with_options(
                    &query,
                    mode,
                    SearchOptions {
                        allow_adult: cli.allow_adult,
                    },
                )
                .await?;
            'anime: loop {
                let Some(show) = select_search_result(&results, cli.select_nth)? else {
                    continue 'search;
                };
                let episodes = client.episodes(&show.id, mode).await?;
                let selection = if let Some(selection) = cli.episode.clone() {
                    selection
                } else {
                    let Some(selection) = select_initial_episodes(&episodes, cli.multi_selection)?
                    else {
                        if results.len() == 1 {
                            continue 'search;
                        }
                        continue 'anime;
                    };
                    selection
                };
                let selected = expand_episode_selection(&selection, &episodes)?;
                break 'search (show, episodes, selected);
            }
        }
    };
    let player = build_legacy_player(&cli);
    let download_directory = std::env::var_os("ANI_CLI_DOWNLOAD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let context = PlaybackContext {
        client: &client,
        history: &history,
        show: &show,
        episodes: &episodes,
        mode,
        download_directory,
        player: &player,
    };
    for episode in &selected {
        play_or_download(&context, episode, &cli.quality, cli.download).await?;
    }

    if should_offer_playback_controls(
        selected.len(),
        std::io::stdin().is_terminal(),
        cli.download,
        cli.exit_after_play,
    ) {
        interactive_after_play(&context, &selected[0], &cli.quality).await?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ScheduleSearchResponse {
    #[serde(default)]
    anime: Vec<ScheduleAnime>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleAnime {
    title: String,
    status: String,
    jpn_time: Option<String>,
    sub_time: Option<String>,
    #[serde(default)]
    names: ScheduleNames,
}

#[derive(Debug, Default, Deserialize)]
struct ScheduleNames {
    english: Option<String>,
    native: Option<String>,
}

async fn display_next_episode_schedule(query: &str) -> Result<()> {
    let response = reqwest::Client::builder()
        .user_agent(concat!("ani-cli-rs/", env!("CARGO_PKG_VERSION")))
        .build()?
        .get("https://animeschedule.net/api/v3/anime")
        .query(&[("q", query)])
        .send()
        .await?
        .error_for_status()?
        .json::<ScheduleSearchResponse>()
        .await?;
    if response.anime.is_empty() {
        return Err(AniError::Unavailable(
            "no AnimeSchedule results found".into(),
        ));
    }
    for anime in &response.anime {
        for line in schedule_lines(anime) {
            println!("{line}");
        }
        println!("---");
    }
    Ok(())
}

fn schedule_lines(anime: &ScheduleAnime) -> Vec<String> {
    let mut lines = Vec::with_capacity(5);
    if let Some(title) = anime.names.english.as_deref() {
        lines.push(format!("English Title: {title}"));
    } else {
        lines.push(format!("English Title: {}", anime.title));
    }
    if let Some(title) = anime.names.native.as_deref() {
        lines.push(format!("Japanese Title: {title}"));
    }
    if anime.status != "Finished" {
        if let Some(time) = anime
            .jpn_time
            .as_deref()
            .filter(|time| valid_release_time(time))
        {
            lines.push(format!("Next Raw Release: {time}"));
        }
        if let Some(time) = anime
            .sub_time
            .as_deref()
            .filter(|time| valid_release_time(time))
        {
            lines.push(format!("Next Sub Release: {time}"));
        }
    }
    lines.push(format!("Status:  {}", anime.status));
    lines
}

fn valid_release_time(time: &str) -> bool {
    !(time.starts_with("0001-") || time.starts_with("0002-"))
}

async fn run_command(client: &AllAnimeClient, command: Commands) -> Result<()> {
    match command {
        Commands::Search(args) => {
            let values = client
                .search_with_options(
                    &args.query,
                    TranslationType::from_str(&args.mode)?,
                    SearchOptions {
                        allow_adult: args.allow_adult,
                    },
                )
                .await?;
            output(&values, args.json, |value| {
                format!("{}\t{} ({} episodes)", value.id, value.name, value.episodes)
            })?;
        }
        Commands::Episodes(args) => {
            let values = client
                .episodes(&args.show_id, TranslationType::from_str(&args.mode)?)
                .await?;
            output(&values, args.json, |value| value.clone())?;
        }
        Commands::Links(args) => {
            let values = client
                .streams(
                    &args.show_id,
                    &args.episode,
                    TranslationType::from_str(&args.mode)?,
                )
                .await?;
            if let Some(quality) = args.quality {
                let value = choose_quality(&values, &quality)
                    .ok_or_else(|| AniError::Unavailable("no streams".into()))?;
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
            let streams = client.streams(&args.show_id, &args.episode, mode).await?;
            let stream = choose_quality(&streams, &args.quality)
                .ok_or_else(|| AniError::Unavailable("no streams".into()))?;
            let options = PlayerOptions {
                executable: args
                    .player
                    .unwrap_or_else(|| PlayerOptions::default_mpv().executable),
                kind: PlayerKind::Mpv,
                no_detach: args.no_detach,
                exit_after_play: false,
            };
            Player::new(options)
                .play(stream, &format!("{} Episode {}", args.title, args.episode))
                .await?;
        }
        Commands::Download(args) => {
            let streams = client
                .streams(
                    &args.show_id,
                    &args.episode,
                    TranslationType::from_str(&args.mode)?,
                )
                .await?;
            let stream = choose_quality(&streams, &args.quality)
                .ok_or_else(|| AniError::Unavailable("no streams".into()))?;
            let options = DownloadOptions {
                directory: args.output.unwrap_or_else(|| PathBuf::from(".")),
                filename: format!("{} Episode {}", args.title, args.episode),
            };
            println!(
                "Saved {}",
                download_stream(stream, &options).await?.display()
            );
        }
        Commands::Debug { refresh } => println!(
            "{}",
            serde_json::to_string_pretty(&client.crypto_debug(refresh).await?)?
        ),
        Commands::RefreshCipherMap => println!(
            "{}",
            serde_json::to_string_pretty(&client.refresh_cipher_map().await?)?
        ),
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

fn select_search_result(
    results: &[SearchResult],
    nth: Option<usize>,
) -> Result<Option<SearchResult>> {
    if results.is_empty() {
        return Err(AniError::Unavailable("no search results".into()));
    }
    let index = if let Some(index) = nth {
        index
            .checked_sub(1)
            .filter(|index| *index < results.len())
            .ok_or_else(|| AniError::Input(format!("selection {index} is out of range")))?
    } else if results.len() == 1 {
        return Ok(Some(results[0].clone()));
    } else {
        let mut items = vec!["← Back to search".to_owned()];
        items.extend(
            results
                .iter()
                .map(|value| format!("{} ({} episodes)", value.name, value.episodes)),
        );
        let Some(index) = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select anime (Esc: back, type to filter)")
            .items(&items)
            .default(1)
            .interact_opt()
            .map_err(dialog_error)?
        else {
            return Ok(None);
        };
        if index == 0 {
            return Ok(None);
        }
        index - 1
    };
    Ok(Some(results[index].clone()))
}

fn select_episode(episodes: &[String]) -> Result<Option<String>> {
    if episodes.is_empty() {
        return Err(AniError::Unavailable(
            "show has no episodes in this translation".into(),
        ));
    }
    let mut items = vec!["← Back to anime results".to_owned()];
    items.extend(episodes.iter().cloned());
    let index = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select episode (Esc: back, type to filter)")
        .items(&items)
        .default(episodes.len())
        .interact_opt()
        .map_err(dialog_error)?;
    Ok(index.and_then(|index| index.checked_sub(1).map(|index| episodes[index].clone())))
}

fn select_initial_episodes(episodes: &[String], multi: bool) -> Result<Option<String>> {
    if episodes.is_empty() {
        return Err(AniError::Unavailable(
            "show has no episodes in this translation".into(),
        ));
    }
    if multi {
        return select_multiple_episodes(episodes);
    }

    let mut items = vec![
        "← Back to anime results".to_owned(),
        "☑ Select multiple episodes".to_owned(),
    ];
    items.extend(episodes.iter().cloned());
    let Some(index) = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select episode (Esc: back, type to filter)")
        .items(&items)
        .default(episodes.len() + 1)
        .interact_opt()
        .map_err(dialog_error)?
    else {
        return Ok(None);
    };

    match index {
        0 => Ok(None),
        1 => select_multiple_episodes(episodes),
        index => Ok(Some(episodes[index - 2].clone())),
    }
}

fn select_multiple_episodes(episodes: &[String]) -> Result<Option<String>> {
    let Some(selected) = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select episodes (Space: toggle, Enter: confirm, Esc: back)")
        .items(episodes)
        .interact_opt()
        .map_err(dialog_error)?
    else {
        return Ok(None);
    };
    Ok(multiple_episode_selection(episodes, &selected))
}

fn multiple_episode_selection(episodes: &[String], selected: &[usize]) -> Option<String> {
    (!selected.is_empty()).then(|| {
        selected
            .iter()
            .filter_map(|index| episodes.get(*index))
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    })
}

async fn continue_selection(
    client: &AllAnimeClient,
    history: &HistoryStore,
    mode: TranslationType,
    nth: Option<usize>,
) -> Result<Option<(SearchResult, Vec<String>, Option<String>)>> {
    let mut candidates = Vec::new();
    for entry in history.entries().await? {
        let episodes = client.episodes(&entry.show_id, mode).await?;
        if let Some(position) = episodes.iter().position(|value| value == &entry.episode)
            && let Some(next) = episodes.get(position + 1)
        {
            let next = next.clone();
            candidates.push((
                SearchResult {
                    id: entry.show_id,
                    name: entry.title,
                    episodes: episodes.len() as f64,
                },
                episodes,
                next,
            ));
        }
    }
    if candidates.is_empty() {
        return Err(AniError::Unavailable(
            "no unwatched series in history".into(),
        ));
    }
    let index = if let Some(index) = nth {
        index
            .checked_sub(1)
            .filter(|index| *index < candidates.len())
            .ok_or_else(|| AniError::Input("history selection is out of range".into()))?
    } else {
        let mut items = vec!["← Cancel".to_owned()];
        items.extend(
            candidates
                .iter()
                .map(|(show, _, next)| format!("{} - episode {next}", show.name)),
        );
        let Some(index) = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Continue anime (Esc: cancel, type to filter)")
            .items(&items)
            .default(1)
            .interact_opt()
            .map_err(dialog_error)?
        else {
            return Ok(None);
        };
        if index == 0 {
            return Ok(None);
        }
        index - 1
    };
    let (show, episodes, next) = candidates.remove(index);
    Ok(Some((show, episodes, Some(next))))
}

fn build_legacy_player(cli: &Cli) -> Player {
    let mut options = PlayerOptions::default_mpv();
    options.no_detach |= cli.no_detach;
    options.exit_after_play |= cli.exit_after_play;
    if cli.vlc {
        options.executable = PathBuf::from(if cfg!(windows) { "vlc.exe" } else { "vlc" });
        options.kind = PlayerKind::Vlc;
    }
    if cli.syncplay {
        options.executable = PathBuf::from(if cfg!(windows) {
            "syncplay.exe"
        } else {
            "syncplay"
        });
        options.kind = PlayerKind::Syncplay;
    }
    Player::new(options)
}

struct PlaybackContext<'a> {
    client: &'a AllAnimeClient,
    history: &'a HistoryStore,
    show: &'a SearchResult,
    episodes: &'a [String],
    mode: TranslationType,
    download_directory: PathBuf,
    player: &'a Player,
}

async fn play_or_download(
    context: &PlaybackContext<'_>,
    episode: &str,
    quality: &str,
    download: bool,
) -> Result<()> {
    println!("Fetching {} episode {episode}...", context.show.name);
    let streams = context
        .client
        .streams(&context.show.id, episode, context.mode)
        .await?;
    let stream = choose_quality(&streams, quality)
        .ok_or_else(|| AniError::Unavailable("no streams".into()))?;
    let title = format!("{} Episode {episode}", clean_title(&context.show.name));
    if download {
        let path = download_stream(
            stream,
            &DownloadOptions {
                directory: context.download_directory.clone(),
                filename: title,
            },
        )
        .await?;
        println!("Saved {}", path.display());
    } else {
        context.player.play(stream, &title).await?;
    }
    context
        .history
        .update(HistoryEntry {
            episode: episode.into(),
            show_id: context.show.id.clone(),
            title: context.show.name.clone(),
        })
        .await?;
    Ok(())
}

async fn interactive_after_play(
    context: &PlaybackContext<'_>,
    first: &str,
    initial_quality: &str,
) -> Result<()> {
    let mut episode = first.to_owned();
    let mut quality = initial_quality.to_owned();
    loop {
        let actions = playback_actions(context.episodes, &episode);
        let labels: Vec<_> = actions
            .iter()
            .map(|action| action.label(&quality))
            .collect();
        let Some(choice) = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "Episode {episode} controls · {} (j/k: move, q/Esc: quit)",
                context.show.name
            ))
            .items(&labels)
            .default(0)
            .interact_opt()
            .map_err(dialog_error)?
        else {
            break;
        };
        match actions[choice] {
            PlaybackAction::Next => episode = adjacent_episode(context.episodes, &episode, 1)?,
            PlaybackAction::Replay => {}
            PlaybackAction::Previous => {
                episode = adjacent_episode(context.episodes, &episode, -1)?;
            }
            PlaybackAction::Select => {
                let Some(selected) = select_episode(context.episodes)? else {
                    continue;
                };
                episode = selected;
            }
            PlaybackAction::ChangeQuality => {
                let streams = context
                    .client
                    .streams(&context.show.id, &episode, context.mode)
                    .await?;
                let choices: Vec<_> = streams
                    .iter()
                    .map(|value| format!("{} · {}", value.resolution, value.provider))
                    .collect();
                let Some(index) = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select quality (j/k: move, q/Esc: back)")
                    .items(&choices)
                    .interact_opt()
                    .map_err(dialog_error)?
                else {
                    continue;
                };
                quality = streams[index].resolution.clone();
            }
            PlaybackAction::Quit => break,
        }
        play_or_download(context, &episode, &quality, false).await?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaybackAction {
    Next,
    Replay,
    Previous,
    Select,
    ChangeQuality,
    Quit,
}

impl PlaybackAction {
    fn label(self, quality: &str) -> String {
        match self {
            Self::Next => "Next episode".into(),
            Self::Replay => "Replay episode".into(),
            Self::Previous => "Previous episode".into(),
            Self::Select => "Choose another episode".into(),
            Self::ChangeQuality => format!("Change quality (current: {quality})"),
            Self::Quit => "Quit ani-cli-rs".into(),
        }
    }
}

fn playback_actions(episodes: &[String], current: &str) -> Vec<PlaybackAction> {
    let position = episodes.iter().position(|episode| episode == current);
    let mut actions = Vec::with_capacity(6);
    if position.is_some_and(|index| index + 1 < episodes.len()) {
        actions.push(PlaybackAction::Next);
    }
    actions.push(PlaybackAction::Replay);
    if position.is_some_and(|index| index > 0) {
        actions.push(PlaybackAction::Previous);
    }
    actions.extend([
        PlaybackAction::Select,
        PlaybackAction::ChangeQuality,
        PlaybackAction::Quit,
    ]);
    actions
}

fn should_offer_playback_controls(
    selected_count: usize,
    terminal: bool,
    download: bool,
    exit_after_play: bool,
) -> bool {
    selected_count == 1 && terminal && !download && !exit_after_play
}

fn adjacent_episode(episodes: &[String], current: &str, delta: isize) -> Result<String> {
    let index = episodes
        .iter()
        .position(|value| value == current)
        .ok_or_else(|| AniError::Input("current episode is not in episode list".into()))?
        as isize
        + delta;
    episodes
        .get(index as usize)
        .cloned()
        .ok_or_else(|| AniError::Unavailable("episode is out of range".into()))
}

fn clean_title(value: &str) -> String {
    value
        .split('(')
        .next()
        .unwrap_or(value)
        .trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace())
        .into()
}
fn dialog_error(error: dialoguer::Error) -> AniError {
    AniError::Input(format!("interactive selection failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_options_can_follow_the_query() {
        let cli = Cli::try_parse_from([
            "ani-cli-rs",
            "cowboy",
            "bebop",
            "--dub",
            "-q",
            "1080p",
            "-e",
            "2-4",
        ])
        .expect("legacy arguments should parse");

        assert_eq!(cli.query, ["cowboy", "bebop"]);
        assert!(cli.dub);
        assert_eq!(cli.quality, "1080p");
        assert_eq!(cli.episode.as_deref(), Some("2-4"));
    }

    #[test]
    fn legacy_options_can_be_interspersed_with_query_words() {
        let cli = Cli::try_parse_from([
            "ani-cli-rs",
            "--allow-adult",
            "cyberpunk",
            "-q",
            "720p",
            "edgerunners",
            "--no-detach",
        ])
        .expect("interspersed legacy arguments should parse");

        assert_eq!(cli.query, ["cyberpunk", "edgerunners"]);
        assert!(cli.allow_adult);
        assert!(cli.no_detach);
        assert_eq!(cli.quality, "720p");
    }

    #[test]
    fn multiple_episode_selection_preserves_episode_order() {
        let episodes = vec!["1".into(), "2".into(), "2.5".into(), "10".into()];

        assert_eq!(
            multiple_episode_selection(&episodes, &[0, 2, 3]).as_deref(),
            Some("1 2.5 10")
        );
        assert_eq!(multiple_episode_selection(&episodes, &[]), None);
    }

    #[test]
    fn next_episode_schedule_formats_ongoing_and_finished_titles() {
        let ongoing = ScheduleAnime {
            title: "Example".into(),
            status: "Ongoing".into(),
            jpn_time: Some("2026-07-24T15:00:00Z".into()),
            sub_time: Some("2026-07-24T16:00:00Z".into()),
            names: ScheduleNames {
                english: Some("Example Anime".into()),
                native: Some("例".into()),
            },
        };
        assert_eq!(
            schedule_lines(&ongoing),
            [
                "English Title: Example Anime",
                "Japanese Title: 例",
                "Next Raw Release: 2026-07-24T15:00:00Z",
                "Next Sub Release: 2026-07-24T16:00:00Z",
                "Status:  Ongoing",
            ]
        );

        let finished = ScheduleAnime {
            title: "Old Anime".into(),
            status: "Finished".into(),
            jpn_time: Some("2020-01-01T00:00:00Z".into()),
            sub_time: Some("0001-01-01T00:00:00Z".into()),
            names: ScheduleNames::default(),
        };
        assert_eq!(
            schedule_lines(&finished),
            ["English Title: Old Anime", "Status:  Finished"]
        );
    }

    #[test]
    fn playback_actions_hide_unavailable_episode_directions() {
        let episodes = vec!["1".into(), "2".into(), "3".into()];
        assert_eq!(
            playback_actions(&episodes, "1"),
            vec![
                PlaybackAction::Next,
                PlaybackAction::Replay,
                PlaybackAction::Select,
                PlaybackAction::ChangeQuality,
                PlaybackAction::Quit,
            ]
        );
        assert_eq!(
            playback_actions(&episodes, "3"),
            vec![
                PlaybackAction::Replay,
                PlaybackAction::Previous,
                PlaybackAction::Select,
                PlaybackAction::ChangeQuality,
                PlaybackAction::Quit,
            ]
        );
    }

    #[test]
    fn playback_controls_remain_available_for_one_interactive_episode() {
        assert!(should_offer_playback_controls(1, true, false, false));
        assert!(!should_offer_playback_controls(2, true, false, false));
        assert!(!should_offer_playback_controls(1, false, false, false));
        assert!(!should_offer_playback_controls(1, true, true, false));
        assert!(!should_offer_playback_controls(1, true, false, true));
    }
}
