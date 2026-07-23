# ani-cli-rs

A cross-platform Rust port of [ani-cli](https://github.com/pystardust/ani-cli) with AllAnime and Anikoto/MegaPlay catalogs.

`ani-cli-rs` provides the familiar interactive ani-cli experience on Windows and Linux while keeping its executable name distinct from the Bash project. It also exposes the scraper as an `ani_cli` Rust library.

![Automated ani-cli-rs showcase](docs/assets/ani-cli-rs-showcase.gif)

## Quick links

- [Install](#installation)
- [Quick start](#quick-start)
- [Requirements](#requirements)
- [CLI compatibility](#cli-compatibility)
- [Scriptable commands](#scriptable-commands)
- [Build and test](#build-and-test)
- [Showcase generator](showcase/README.md)
- [Troubleshooting](https://github.com/vorlie/ani-cli-rs/wiki/Troubleshooting)
- [Complete documentation](https://github.com/vorlie/ani-cli-rs/wiki)
- [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md) · [License](LICENSE)

## Features

- Interactive anime/season and episode selection
- Subbed and dubbed AllAnime or Anikoto search and playback
- Native MegaPlay source extraction and transparent KotoCDN HLS unwrapping
- mpv, IINA, VLC, and Syncplay support
- Quality selection, episode ranges, history, and continuation
- Preflighted batch downloads with aria2, yt-dlp, FFmpeg, and built-in fallbacks
- Scriptable commands with JSON output
- Native Rust HTTP and cryptography-no curl, sed, OpenSSL, Botan, or fzf dependency

## Installation

Official releases are provided for Windows x64 and Linux x86-64. Linux ARM64 and macOS users can build from source, but maintainer-built binaries are not currently published for those platforms.

### Linux

```sh
curl -fsSL https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/install.sh -o install.sh
sh install.sh
rm install.sh
```

### Windows PowerShell

Portable installation:

```powershell
Invoke-WebRequest https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/install.ps1 -OutFile install.ps1
.\install.ps1
Remove-Item .\install.ps1
```

To use the Inno Setup installer instead:

```powershell
.\install.ps1 -UseSetup
```

The scripts verify release archives against their published SHA-256 checksums and install to the user-local `~/.local/bin` directory by default. See the [installation guide](https://github.com/vorlie/ani-cli-rs/wiki/Installation) for PATH help, custom locations, uninstalling, and source builds.

## Requirements

- Linux and Windows: [mpv](https://mpv.io/) for default playback
- macOS: [IINA](https://iina.io/) for default playback (`brew install --cask iina`)
- Optional: VLC or Syncplay for alternate playback
- Optional: [aria2](https://aria2.github.io/) for faster parallel downloads
- Optional: yt-dlp and FFmpeg for HLS downloads, fallbacks, and embedding provider subtitles into MP4 files

External programs must be available through `PATH`. See [Playback and Players](https://github.com/vorlie/ani-cli-rs/wiki/Playback-and-Players) and [Downloads](https://github.com/vorlie/ani-cli-rs/wiki/Downloads) for setup details.

## Quick start

Search interactively:

```console
ani-cli-rs "frieren"
```

Common examples:

```console
ani-cli-rs --dub -q 720p "cowboy bebop"
ani-cli-rs -e 2-4 "one piece"
ani-cli-rs --continue
ani-cli-rs --download "anime title"
ani-cli-rs --allow-adult "search query"
ani-cli-rs --provider anikoto "dandadan"
```

Interactive playback remains open after an episode and offers next, replay, previous, episode-selection, and quality controls. Add `--exit-after-play` to exit immediately instead.

In download mode, search results act as the anime/season picker. All selected episodes are resolved before the first transfer begins, preventing unavailable episodes from leaving a partially downloaded batch.

## CLI compatibility

The primary interface follows Bash ani-cli conventions:

```text
ani-cli-rs [OPTIONS] [QUERY]
```

Supported compatibility flags include:

```text
-c, --continue            Continue from history
-d, --download            Download instead of play
-D, --delete              Delete history
-s, --syncplay            Use Syncplay
-S, --select-nth          Select the nth search result
-q, --quality             best, worst, or a resolution
-v, --vlc                 Use VLC
-e, --episode             Episode or range
-r, --range               Episode range alias
-a, --allow-adult         Include adult results
-N, --nextep-countdown    Show release timing
-U, --update              Update from GitHub Releases
-p, --provider            allanime (default) or anikoto
    --dub                  Use dubbed results
    --multi-selection      Select multiple episodes
    --no-detach            Keep the player attached
    --exit-after-play      Skip the post-playback menu
```

Run `ani-cli-rs --help` for the authoritative list. Environment variables and keyboard controls are documented in the [CLI reference](https://github.com/vorlie/ani-cli-rs/wiki/CLI-Reference).

## Scriptable commands

```console
ani-cli-rs search --json "frieren"
ani-cli-rs -p anikoto search --json "frieren"
ani-cli-rs episodes --json SHOW_ID --mode sub
ani-cli-rs links --json SHOW_ID 1 --quality 1080p
ani-cli-rs play SHOW_ID 1 --title "Frieren" --no-detach
ani-cli-rs download SHOW_ID 1 --output ./downloads
ani-cli-rs debug --refresh
ani-cli-rs refresh-cipher-map
ani-cli-rs update --check
```

`episodes`, `links`, `play`, and `download` require the **show ID returned by `search`**, not an anime title. AllAnime also accepts the ID in a Mkissa URL. For this URL:

```text
https://mkissa.to/anime/SyR2K6bGYfKSE6YMm
```

the show ID is `SyR2K6bGYfKSE6YMm`. Anikoto search returns self-identifying IDs beginning with `anikoto:`; those IDs route automatically. Raw numeric Anikoto series IDs require `--provider anikoto`. Use `ani-cli-rs --download "anime title"` when you want interactive name and season selection.

Set `ANI_CLI_PROVIDER=anikoto` to make Anikoto the interactive/search default. AllAnime remains the compatibility default. Provider catalogs are intentionally not combined and playback never silently crosses between them.

See the [CLI reference](https://github.com/vorlie/ani-cli-rs/wiki/CLI-Reference) for every command and JSON workflow.

## Security and antivirus notices

Official Windows binaries are currently unsigned and may trigger SmartScreen or heuristic antivirus warnings. AllAnime support includes runtime bootstrap inspection and cryptographic decoding, which can also produce misleading “obfuscation” labels in automated behavior reports.

Release installers verify published checksums. Users who prefer not to run prebuilt binaries can inspect the tagged source and build it with `cargo build --release --locked`. Read [Security and Privacy](https://github.com/vorlie/ani-cli-rs/wiki/Security-and-Privacy) for the full explanation and manual verification steps.

## Build and test

The project uses Rust 2024 and therefore requires a current stable Rust toolchain.

```console
cargo build --release
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
```

On Windows with WSL available, `.\showcase\showcase.ps1` generates the deterministic terminal MP4, screenshots, and README GIF without contacting anime providers. See the [showcase guide](showcase/README.md).

Target-specific Cargo aliases and release packaging are covered in [Building and Releasing](https://github.com/vorlie/ani-cli-rs/wiki/Building-and-Releasing).

## Documentation

The [project Wiki](https://github.com/vorlie/ani-cli-rs/wiki) contains the full user and contributor documentation. Its source is tracked in [`wiki/`](wiki/Home.md) so documentation changes can be reviewed with code changes.

Provider internals are documented in [`docs/ALLANIME-SCRAPING.md`](docs/ALLANIME-SCRAPING.md) and [`docs/ANIKOTO-KOTOCDN.md`](docs/ANIKOTO-KOTOCDN.md).

## Scope

This project targets desktop Windows, Linux, and macOS source builds. rofi/dmenu integration, Android/Termux and iSH adapters, intro skipping, and system-journal logging are not currently included.

## License

Licensed under [GPL-3.0-only](LICENSE).
