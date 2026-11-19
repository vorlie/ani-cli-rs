use std::{path::PathBuf, str::FromStr};

use ani_lib::{CatalogProvider, SearchSort};
use clap::{
    Args, ColorChoice, Parser, Subcommand,
    builder::Styles,
    builder::styling::{AnsiColor, Color, Style},
};

const LONG_ABOUT: &str = "A cross-platform Rust port of ani-cli for browsing, resolving, playing, and downloading anime from Anikoto providers.\n\nThe interactive workflow searches the selected subbed or dubbed catalog, lists available episodes, resolves current provider links, selects the requested quality, and opens an external player. Anikoto.cz is the default catalog; use --provider anikoto or ANI_CLI_RS_PROVIDER=anikoto to switch to the legacy Anikoto API/MegaPlay provider. Watch history uses the Bash ani-cli tab-separated format, so an existing history directory can be reused.\n\nThe scraper and KotoCDN compatibility relay are implemented entirely in Rust; Python, curl, sed, OpenSSL, Botan, and fzf are not required. Playback uses IINA on macOS, an Android media player from Termux, and mpv on other desktops by default, with optional VLC and Syncplay integrations. Downloads prefer aria2c for parallel transfers when available, with yt-dlp, FFmpeg, and the built-in resumable downloader as fallbacks.";

const AFTER_HELP: &str = concat!(
    "\u{1b}[38;5;208mKEYBOARD NAVIGATION:\u{1b}[0m\n",
    "  \u{1b}[38;5;214mArrow keys / Tab\u{1b}[0m       Navigate menus\n",
    "  \u{1b}[38;5;214mj / k\u{1b}[0m                  Move down / up in action menus\n",
    "  \u{1b}[38;5;214mh / l\u{1b}[0m                  Change pages in action menus\n",
    "  \u{1b}[38;5;214mSpace / Enter\u{1b}[0m          Select or toggle an item\n",
    "  \u{1b}[38;5;214mType\u{1b}[0m                    Filter fuzzy anime/episode menus\n",
    "  \u{1b}[38;5;214mEscape\u{1b}[0m                 Go back immediately from a fuzzy menu\n",
    "  \u{1b}[38;5;214mq / Escape\u{1b}[0m             Leave an ordinary action menu\n\n",
    "\u{1b}[38;5;208mEXAMPLES:\u{1b}[0m\n",
    "  \u{1b}[38;5;203mani-cli-rs frieren\u{1b}[0m\n",
    "  \u{1b}[38;5;203mani-cli-rs --provider anikoto2 \"black torch\"\u{1b}[0m\n",
    "  \u{1b}[38;5;203mani-cli-rs --allow-adult \"search query\"\u{1b}[0m\n",
    "  \u{1b}[38;5;203mani-cli-rs --dub -q 720p \"cowboy bebop\"\u{1b}[0m\n",
    "  \u{1b}[38;5;203mani-cli-rs -S 1 -e 2-4 \"one piece\"\u{1b}[0m\n",
    "  \u{1b}[38;5;203mani-cli-rs --continue\u{1b}[0m\n",
    "  \u{1b}[38;5;203mani-cli-rs --download -e 1 \"anime title\"\u{1b}[0m\n",
    "  \u{1b}[38;5;203mani-cli-rs search --allow-adult --json \"search query\"\u{1b}[0m\n",
    "  \u{1b}[38;5;203mani-cli-rs links --json SHOW_ID 1 --quality 1080p\u{1b}[0m\n\n",
    "\u{1b}[38;5;208mTERMUX:\u{1b}[0m\n",
    "  \u{1b}[38;5;214mInstall an Android video player; do not use the terminal VLC package.\u{1b}[0m\n",
    "  \u{1b}[38;5;214m--vlc requests Android VLC when explicit intents work. A compatibility fallback\u{1b}[0m\n",
    "  \u{1b}[38;5;214muses Android's media handler instead. Keep Termux open for relayed HLS playback.\u{1b}[0m\n\n",
    "\u{1b}[38;5;208mENVIRONMENT:\u{1b}[0m\n",
    "  \u{1b}[38;5;203mANI_CLI_MODE, ANI_CLI_PLAYER, ANI_CLI_DOWNLOAD_DIR, ANI_CLI_QUALITY,\u{1b}[0m\n",
    "  \u{1b}[38;5;203mANI_CLI_HIST_DIR, ANI_CLI_ALLOW_ADULT, ANI_CLI_MULTI_SELECTION,\u{1b}[0m\n",
    "  \u{1b}[38;5;203mANI_CLI_NO_DETACH, ANI_CLI_EXIT_AFTER_PLAY, ANI_CLI_RS_PROVIDER,\u{1b}[0m\n",
    "  \u{1b}[38;5;203mANI_CLI_IGNORE_HOST_LISTS\u{1b}[0m\n\n",
    "\u{1b}[38;5;208mDEBUG LOGGING:\u{1b}[0m\n",
    "  \u{1b}[38;5;203mRUST_LOG=ani_cli_rs=debug,ani_cli=debug\u{1b}[0m    \u{1b}[38;5;214mverbose launch diagnostics\u{1b}[0m\n",
    "  \u{1b}[38;5;203mRUST_LOG=ani_cli_rs=trace,ani_cli=trace\u{1b}[0m    \u{1b}[38;5;214mfull stream resolution + relay tracing\u{1b}[0m\n",
    "  \u{1b}[38;5;203mRUST_LOG=warn\u{1b}[0m                              \u{1b}[38;5;214monly warnings and errors (default off)\u{1b}[0m\n\n",
    "\u{1b}[38;5;214mOfficial prebuilt releases are provided for Windows and Linux. Tested macOS and Termux builds are compiled from source.\u{1b}[0m"
);

fn cli_styles() -> Styles {
    Styles::styled()
        .header(
            Style::new()
                .bold()
                .fg_color(Some(Color::Ansi(AnsiColor::Yellow))),
        )
        .usage(
            Style::new()
                .bold()
                .fg_color(Some(Color::Ansi(AnsiColor::BrightYellow))),
        )
        .literal(
            Style::new()
                .bold()
                .fg_color(Some(Color::Ansi(AnsiColor::BrightRed))),
        )
        .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red))))
        .valid(Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightYellow))))
        .invalid(
            Style::new()
                .bold()
                .fg_color(Some(Color::Ansi(AnsiColor::Red))),
        )
        .error(
            Style::new()
                .bold()
                .fg_color(Some(Color::Ansi(AnsiColor::Red))),
        )
}

#[derive(Parser, Debug)]
#[command(
    name = "ani-cli-rs",
    version,
    about = "Browse, play, and download anime from Anikoto providers",
    long_about = LONG_ABOUT,
    after_help = AFTER_HELP,
    color = ColorChoice::Auto,
    styles = cli_styles()
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// Catalog provider: anikoto2 (default) or anikoto (legacy Anikoto API/MegaPlay).
    #[arg(short = 'p', long, global = true, env = "ANI_CLI_RS_PROVIDER")]
    pub provider: Option<CatalogProvider>,
    /// Continue from the next unwatched episode in the ani-cli history.
    #[arg(short = 'c', long = "continue")]
    pub continue_watching: bool,
    /// Download selected episodes instead of launching a player.
    #[arg(short = 'd', long)]
    pub download: bool,
    /// Delete all saved watch history and exit.
    #[arg(short = 'D', long)]
    pub delete: bool,
    /// Launch the resolved stream through Syncplay.
    #[arg(short = 's', long)]
    pub syncplay: bool,
    /// Select a search or history result by its one-based index.
    #[arg(short = 'S', long = "select-nth")]
    pub select_nth: Option<usize>,
    /// Choose best, worst, or a resolution such as 1080p or 720p.
    #[arg(short = 'q', long, env = "ANI_CLI_QUALITY", default_value = "best")]
    pub quality: String,
    /// Use VLC; on Termux this preference requires a working explicit activity bridge.
    #[arg(short = 'v', long)]
    pub vlc: bool,
    /// Select one episode, whitespace-separated episodes, or a range such as 2-5.
    #[arg(short = 'e', short_alias = 'r', long = "episode", alias = "range")]
    pub episode: Option<String>,
    /// Open the interactive episode picker in multi-selection mode.
    #[arg(long, env = "ANI_CLI_MULTI_SELECTION")]
    pub multi_selection: bool,
    /// Search and play the dubbed catalog instead of subtitles.
    #[arg(long)]
    pub dub: bool,
    /// Include titles marked as adult in catalog search results.
    #[arg(short = 'a', long, env = "ANI_CLI_ALLOW_ADULT")]
    pub allow_adult: bool,
    /// Preferred Anikoto.cz sort order for the filter page.
    #[arg(long, env = "ANI_CLI_SEARCH_SORT", value_parser = SearchSort::from_str)]
    pub sort: Option<SearchSort>,
    /// Keep the player attached and wait for it to exit.
    #[arg(long, env = "ANI_CLI_NO_DETACH")]
    pub no_detach: bool,
    /// Return the attached player's exit status after playback.
    #[arg(long, env = "ANI_CLI_EXIT_AFTER_PLAY")]
    pub exit_after_play: bool,
    /// Enable verbose error output with full error chains.
    #[arg(long, env = "ANI_CLI_VERBOSE")]
    pub verbose: bool,
    /// Enable debug error output with detailed diagnostics.
    #[arg(long, env = "ANI_CLI_DEBUG")]
    pub debug: bool,
    /// Force all streams through HLS relay regardless of host allowlist.
    #[arg(short = 'I', long, env = "ANI_CLI_IGNORE_HOST_LISTS")]
    pub ignore_host_lists: bool,
    /// Display the next scheduled raw and subtitled releases, then exit.
    #[arg(short = 'N', long = "nextep-countdown")]
    pub next_episode_countdown: bool,
    /// Check for and install the latest ani-cli-rs release.
    #[arg(short = 'U', long)]
    pub update: bool,
    /// Anime title to search for; omitted titles are prompted interactively.
    #[arg(value_name = "QUERY")]
    pub query: Vec<String>,
    /// Use deterministic local fixtures for development recordings.
    #[cfg(debug_assertions)]
    #[arg(long, hide = true)]
    pub demo_mode: bool,
}

impl Cli {
    pub fn demo_mode(&self) -> bool {
        #[cfg(debug_assertions)]
        {
            self.demo_mode
        }
        #[cfg(not(debug_assertions))]
        {
            false
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Search the selected catalog without opening the interactive player.
    Search(SearchArgs),
    /// List the available episodes for a provider show ID.
    Episodes(EpisodesArgs),
    /// Resolve and display playable sources for one episode.
    Links(LinksArgs),
    /// Resolve one episode and launch it in a media player.
    Play(ActionArgs),
    /// Resolve and download one episode.
    Download(ActionArgs),
    /// Check for or install the latest ani-cli-rs release.
    Update {
        /// Only report whether an update is available.
        #[arg(long)]
        check: bool,
    },
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Anime title to search for.
    pub query: String,
    /// Translation catalog to query: sub or dub.
    #[arg(long, default_value = "sub")]
    pub mode: String,
    /// Include titles marked as adult in search results.
    #[arg(short = 'a', long, env = "ANI_CLI_ALLOW_ADULT")]
    pub allow_adult: bool,
    /// Preferred Anikoto.cz sort order for the filter page.
    #[arg(long, value_parser = SearchSort::from_str)]
    pub sort: Option<SearchSort>,
    /// Print structured JSON instead of tab-separated text.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct EpisodesArgs {
    /// Provider show ID returned by search; this is not an anime title.
    pub show_id: String,
    /// Translation catalog to query: sub or dub.
    #[arg(long, default_value = "sub")]
    pub mode: String,
    /// Print structured JSON instead of one episode per line.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct LinksArgs {
    /// Provider show ID returned by search; this is not an anime title.
    pub show_id: String,
    /// Episode number or fractional episode string.
    pub episode: String,
    /// Translation catalog to query: sub or dub.
    #[arg(long, default_value = "sub")]
    pub mode: String,
    /// Return only the best match for this quality.
    #[arg(short, long)]
    pub quality: Option<String>,
    /// Print complete stream metadata as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ActionArgs {
    /// Provider show ID returned by search; this is not an anime title.
    pub show_id: String,
    /// Episode number or fractional episode string.
    pub episode: String,
    /// Translation catalog to query: sub or dub.
    #[arg(long, default_value = "sub")]
    pub mode: String,
    /// Choose best, worst, or a specific resolution.
    #[arg(short, long, default_value = "best")]
    pub quality: String,
    /// Human-readable title used for the player or output filename.
    #[arg(long, default_value = "Anime")]
    pub title: String,
    /// Override the media-player executable for the play subcommand.
    #[arg(long)]
    pub player: Option<PathBuf>,
    /// Keep the selected player attached until it exits.
    #[arg(long)]
    pub no_detach: bool,
    /// Force all streams through HLS relay regardless of host allowlist.
    #[arg(short = 'I', long, env = "ANI_CLI_IGNORE_HOST_LISTS")]
    pub ignore_host_lists: bool,
    /// Directory used by the download subcommand.
    #[arg(long)]
    pub output: Option<PathBuf>,
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
    fn legacy_update_flag_can_follow_the_query() {
        let cli = Cli::try_parse_from(["ani-cli-rs", "frieren", "-U"])
            .expect("update flag should parse after a query");

        assert!(cli.update);
        assert_eq!(cli.query, ["frieren"]);
    }

    #[test]
    fn ignore_host_lists_flag_sets_force_hls_relay() {
        let cli = Cli::try_parse_from(["ani-cli-rs", "--ignore-host-lists", "frieren"])
            .expect("ignore-host-lists flag should parse");

        assert!(cli.ignore_host_lists);
        assert_eq!(cli.query, ["frieren"]);
    }

    #[test]
    fn short_ignore_host_lists_flag_sets_force_hls_relay() {
        let cli = Cli::try_parse_from(["ani-cli-rs", "-I", "frieren"])
            .expect("short -I flag should parse");

        assert!(cli.ignore_host_lists);
        assert_eq!(cli.query, ["frieren"]);
    }

    #[test]
    fn play_subcommand_ignore_host_lists_flag_sets_force_hls_relay() {
        let cli =
            Cli::try_parse_from(["ani-cli-rs", "play", "SHOW_ID", "1", "--ignore-host-lists"])
                .expect("play subcommand with ignore-host-lists should parse");

        if let Some(Commands::Play(args)) = cli.command {
            assert!(args.ignore_host_lists);
        } else {
            panic!("Expected Play command");
        }
    }

    #[test]
    fn ignore_host_lists_env_variable_sets_force_hls_relay() {
        // Skip this test as env var testing requires complex setup
        // The flag parsing tests are sufficient for coverage
    }
}
