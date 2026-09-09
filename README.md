
<div align="center">

# ani-cli-rs

**A fast, cross-platform anime CLI written in Rust.**

Search, stream, and download anime directly from your terminal with native playback, multiple Anikoto catalogs, and support for MegaPlay/KotoCDN sources.

[![Documentation](https://img.shields.io/badge/docs-online-blue?style=flat-square)](https://vorlie.github.io/ani-cli-rs/)
[![GitHub](https://img.shields.io/github/stars/vorlie/ani-cli-rs?style=flat-square)](https://github.com/vorlie/ani-cli-rs)
[![License](https://img.shields.io/github/license/vorlie/ani-cli-rs?style=flat-square)](LICENSE)
[![Discord](https://img.shields.io/discord/1499791569870655669?style=flat-square\&logo=discord\&logoColor=white)](https://discord.gg/9SXX6ddpNR)

Crates:
[![Crates.io Version](https://img.shields.io/crates/v/ani-cli-rs?style=flat-square)](https://crates.io/crates/ani-cli-rs)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/ani-cli-rs?style=flat-square)](https://crates.io/crates/ani-cli-rs)

Github:
[![Latest Release](https://img.shields.io/github/v/release/vorlie/ani-cli-rs?style=flat-square)](https://github.com/vorlie/ani-cli-rs/releases)
[![Downloads](https://img.shields.io/github/downloads/vorlie/ani-cli-rs/total?style=flat-square)](https://github.com/vorlie/ani-cli-rs/releases)

[**Documentation**](https://vorlie.github.io/ani-cli-rs/) ·
[**Installation**](https://vorlie.github.io/ani-cli-rs/guides/installation/) ·
[**CLI Reference**](https://vorlie.github.io/ani-cli-rs/reference/cli/) ·
[**Contributing**](CONTRIBUTING.md) ·
[**Discord**](https://discord.gg/9SXX6ddpNR)

</div>

![Automated ani-cli-rs showcase](docs/assets/ani-cli-rs-showcase.gif)

---

## What is ani-cli-rs?

`ani-cli-rs` is an independent Rust implementation inspired by [ani-cli](https://github.com/pystardust/ani-cli).

It provides the familiar interactive terminal experience across **Windows, Linux, macOS, and Termux**, while using a native Rust implementation for searching, provider resolution, playback, downloads, and local HLS handling.

Unlike the original Bash project, the executable is intentionally named `ani-cli-rs` and the underlying functionality is also available as the `ani_lib` Rust library.

### Highlights

* 🦀 Native Rust implementation
* 🖥️ Windows, Linux, macOS, and Termux support
* 🔎 Interactive anime and episode search
* 🌐 Two independent Anikoto catalogs
* 🎬 Native MegaPlay source extraction
* 📡 Local KotoCDN HLS relay
* 💬 External provider subtitle support
* ⬇️ Parallel and fallback download support
* 📺 mpv, IINA, VLC, Android mpv, and Syncplay
* 🤖 Scriptable commands with JSON output
* 📚 Reusable Rust library API

---

## Community

ani-cli-rs is built in the open and improved through code, bug reports, documentation, testing, and provider work.

<div align="center">

**⭐ Star the project if you find it useful**

[![GitHub stars](https://img.shields.io/github/stars/vorlie/ani-cli-rs?style=for-the-badge)](https://github.com/vorlie/ani-cli-rs)
[![GitHub forks](https://img.shields.io/github/forks/vorlie/ani-cli-rs?style=for-the-badge)](https://github.com/vorlie/ani-cli-rs)
[![GitHub contributors](https://img.shields.io/github/contributors/vorlie/ani-cli-rs?style=for-the-badge)](https://github.com/vorlie/ani-cli-rs/graphs/contributors)

</div>

### Contributors

Thank you to everyone who has contributed code, fixes, ideas, documentation, testing, and feedback.

---

## Quick start

### Search and watch

```console
ani-cli-rs "frieren"
```

### Common commands

```console
ani-cli-rs --dub -q 720p "cowboy bebop"

ani-cli-rs -e 2-4 "one piece"

ani-cli-rs --continue

ani-cli-rs --download "anime title"

ani-cli-rs --allow-adult "search query"

ani-cli-rs --provider anikoto2 "black torch"
```

### Termux

```sh
ani-cli-rs "cyberpunk"

ani-cli-rs --vlc "cyberpunk"
```

See the **[CLI reference](https://vorlie.github.io/ani-cli-rs/reference/cli/)** for all commands, options, environment variables, JSON workflows, and keyboard controls.

---

## Features

### Playback

* Interactive anime, season, and episode selection
* Subbed and dubbed results
* Quality selection
* Episode ranges and multi-selection
* Playback history and continuation
* Next, previous, replay, and episode-selection controls
* mpv, IINA, VLC, Android mpv, and Syncplay
* Optional `--exit-after-play` and `--no-detach` modes

### Providers

* Independent Anikoto API and Anikoto.cz catalogs
* Native MegaPlay source extraction
* Transparent KotoCDN HLS handling
* Provider subtitles exposed as normal HLS subtitle tracks
* Provider-aware IDs and history entries
* Explicit provider selection without silent catalog switching

### Downloads

* Batch episode resolution before transfers begin
* aria2 parallel downloads
* yt-dlp and FFmpeg fallbacks
* HLS downloads
* Provider subtitle embedding into MP4 where supported

### Automation

Commands can return machine-readable JSON for scripting and integrations:

```console
ani-cli-rs search --json "frieren"

ani-cli-rs episodes --json SHOW_ID --mode sub

ani-cli-rs links --json SHOW_ID 1 --quality 1080p

ani-cli-rs play SHOW_ID 1 --title "Frieren" --no-detach

ani-cli-rs download SHOW_ID 1 --output ./downloads

ani-cli-rs update --check
```

`episodes`, `links`, `play`, and `download` expect the show ID returned by `search`.

---

## Installation

Official releases are currently provided for:

| Platform     | Distribution     |
| ------------ | ---------------- |
| Windows x64  | Prebuilt release |
| Linux x86-64 | Prebuilt release |
| macOS        | Source build     |
| Termux       | Source build     |
| Linux ARM64  | Source build     |

Prebuilt macOS, Android/Termux, and Linux ARM64 binaries are not currently published.

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

For the Inno Setup installer:

```powershell
.\install.ps1 -UseSetup
```

### Android / Termux

```sh
pkg update
pkg install git rust termux-tools

git clone https://github.com/vorlie/ani-cli-rs.git
cd ani-cli-rs

cargo build --release --locked
install -Dm755 target/release/ani-cli-rs "$PREFIX/bin/ani-cli-rs"
```

Install an Android video player such as mpv-android or VLC from an Android app source.

> **Do not use `pkg install vlc`.**
> That installs a terminal VLC build rather than the Android VLC application used by this integration.

The installation scripts verify release archives against their published SHA-256 checksums and install to `~/.local/bin` by default.

For PATH configuration, custom installation locations, uninstalling, and source builds, see the **[installation guide](https://vorlie.github.io/ani-cli-rs/guides/installation/)**.

---

## Requirements

### Desktop

* **Linux / Windows:** [mpv](https://mpv.io/) for default playback
* **macOS:** [IINA](https://iina.io/) for tested out-of-the-box playback

  * `brew install --cask iina`

Optional:

* [VLC](https://www.videolan.org/vlc/)
* [Syncplay](https://syncplay.pl/)
* [aria2](https://aria2.github.io/)
* [yt-dlp](https://github.com/yt-dlp/yt-dlp)
* [FFmpeg](https://ffmpeg.org/)

Desktop programs must be available through `PATH`.

### Termux

Tested Android playback options include:

* mpv-android
* VLC
* Amnis
* Samsung Video Player

On Termux, normal playback uses mpv-android. `--vlc` requests Android VLC when the explicit activity bridge works; otherwise ani-cli-rs falls back to Android's media handler through `termux-open`.

See **[Playback and Players](https://vorlie.github.io/ani-cli-rs/guides/playback-and-players/)** and **[Downloads](https://vorlie.github.io/ani-cli-rs/guides/downloads/)** for details.

---

## CLI compatibility

The primary interface follows Bash ani-cli conventions:

```text
ani-cli-rs [OPTIONS] [QUERY]
```

Common compatibility options include:

```text
-c, --continue           Continue from history
-d, --download           Download instead of play
-D, --delete             Delete history
-s, --syncplay           Use Syncplay
-S, --select-nth         Select the nth search result
-q, --quality            best, worst, or a resolution
-v, --vlc                Use VLC
-e, --episode            Episode or range
-r, --range              Episode range alias
-a, --allow-adult        Include adult results
-N, --nextep-countdown   Show release timing
-U, --update             Update from GitHub Releases
-p, --provider           anikoto or anikoto2

    --dub                Use dubbed results
    --multi-selection    Select multiple episodes
    --no-detach          Keep the player attached
    --exit-after-play    Skip the post-playback menu
```

Run:

```console
ani-cli-rs --help
```

for the authoritative list.

---

## Rust library

The provider and playback functionality is also available as the `ani_lib` Rust library.

This allows applications to integrate ani-cli-rs provider resolution directly instead of spawning the CLI as a subprocess.

```toml
[dependencies]
ani-cli-rs = "0.9.6"
```

Example:

```rust
use ani_lib::{
    AnikotoCzClient,
    TranslationType,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AnikotoCzClient::new()?;

    let results = client
        .search("Frieren", TranslationType::Sub)
        .await?;

    for result in results {
        println!("{}", result.name);
    }

    Ok(())
}
```

The library exposes the APIs used by the CLI for:

* Provider and catalog access
* Search and stream resolution
* Playback
* Downloads
* History
* HLS relay
* Internationalization
* Shared data models

### Public API

The current public API includes:

* `AnikotoClient`
* `AnikotoClientBuilder`
* `AnikotoCzClient`
* `AnikotoCzClientBuilder`
* `SearchResult`
* `StreamLink`
* `SubtitleTrack`
* `TranslationType`
* `SearchOptions`
* `CatalogProvider`
* `HlsRelay`
* `relay_stream`
* `relay_stream_without_hls_subtitles`
* `download_stream`
* `HistoryStore`
* `HistoryEntry`
* `Player`
* `PlayerKind`
* `PlayerOptions`
* `I18n`
* `Locale`

For application integration, see the **[Rust library integration guide](https://vorlie.github.io/ani-cli-rs/development/using-as-a-library/)**.

---

## Kioku

**[Kioku](https://github.com/vorlie/kioku)** is a native desktop AniList manager built around `ani-cli-rs`.

It combines AniList library management with integrated anime search and media playback without requiring a separate player application.

> [!NOTE]
> Kioku is currently in early development and is **not feature-complete**. Expect incomplete features, UI changes, and breaking changes.

---

## Provider architecture

ani-cli-rs currently supports two independent catalogs:

```text
Anikoto API
    └── anikoto:<id>

Anikoto.cz
    └── anikoto2:<slug>
```

The prefixes are preserved through search, episode resolution, playback, and history so the correct catalog can be selected automatically.

The catalogs are intentionally **not merged**, and playback never silently switches between them.

To change the default interactive provider:

```sh
ANI_CLI_RS_PROVIDER=anikoto2 ani-cli-rs "black torch"
```

Provider internals are documented in:

* [`docs/ANIKOTO-KOTOCDN.md`](docs/ANIKOTO-KOTOCDN.md)
* [`docs/ANIKOTO-CZ.md`](docs/ANIKOTO-CZ.md)

---

## Security and diagnostics

Official Windows binaries are currently unsigned and may trigger SmartScreen or heuristic antivirus warnings.

ani-cli-rs performs network requests, launches media players and downloaders, and temporarily creates a loopback HLS relay for KotoCDN playback. These behaviors may produce misleading automated classifications.

Release installers verify published checksums.

For diagnostic logging:

```powershell
$env:RUST_LOG = "ani_cli_rs=debug,ani_cli=debug"
ani-cli-rs "cyberpunk edgerunners"
```

```bash
RUST_LOG=ani_cli_rs=debug,ani_cli=debug ani-cli-rs "cyberpunk edgerunners"
```

Use `trace` instead of `debug` for full stream-resolution and relay tracing.

See **[Security and Privacy](https://vorlie.github.io/ani-cli-rs/development/security/)** for manual verification and privacy details.

---

## Development

ani-cli-rs uses Rust 2024 and requires a current stable Rust toolchain.

```console
cargo build --release

cargo fmt --check

cargo check --all-targets

cargo clippy --all-targets -- -D warnings

cargo test
```

The repository also contains a deterministic showcase generator that produces the terminal recording, screenshots, and README GIF without contacting anime providers:

```powershell
.\showcase\showcase.ps1
```

See **[Building and Releasing](https://vorlie.github.io/ani-cli-rs/development/building/)** and the **[showcase guide](showcase/README.md)** for more information.

---

## Documentation

The complete documentation is available at:

**https://vorlie.github.io/ani-cli-rs/**

It covers:

* Installation
* Configuration
* Playback and players
* Downloads
* CLI commands
* JSON automation
* Provider internals
* HLS relay behavior
* Library integration
* Development
* Security and privacy
* Troubleshooting

Documentation lives alongside the source code in [`website/docs/`](website/docs/index.md), so documentation changes can be reviewed together with code changes.

---

## Troubleshooting

When something goes wrong, first identify which stage failed:

```text
Installation / PATH
        ↓
Search / episode discovery
        ↓
Provider resolution
        ↓
Source resolution
        ↓
KotoCDN / HLS relay
        ↓
Player launch
        ↓
Download transfer
        ↓
Finalization
```

This distinction matters: reinstalling ani-cli-rs will not fix a provider outage, and changing players will not fix an episode that failed during source resolution.

See the **[Troubleshooting guide](https://vorlie.github.io/ani-cli-rs/support/troubleshooting/)** for diagnostic steps and logging information.

---

## Scope

ani-cli-rs currently targets:

* Windows
* Linux
* macOS source builds
* Termux source builds

The following are intentionally outside the current scope:

* iSH adapters
* rofi/dmenu integration
* Intro skipping
* System-journal logging

---

## Media hosting and legal notice

`ani-cli-rs` does **not** host, mirror, or distribute media.

The application retrieves search and stream information from third-party providers and passes resolved streams to the user's selected playback or download tool.

For HLS sources requiring local processing, ani-cli-rs can create a temporary loopback relay on the user's device. This relay is local and is not an internet-facing media proxy.

Users are responsible for complying with applicable laws and the terms of service of the services they access.

See [`DMCA.md`](DMCA.md) for the project's copyright and takedown policy.

---

## Contributing

Contributions are welcome.

You can help by:

* Reporting bugs
* Improving documentation
* Adding tests
* Improving provider handling
* Fixing playback or download issues
* Improving cross-platform support
* Contributing to the Rust library
* Reviewing pull requests

See [`CONTRIBUTING.md`](CONTRIBUTING.md) to get started.

Security issues should be reported according to [`SECURITY.md`](SECURITY.md).

---

## License

Licensed under [GPL-3.0-only](LICENSE).
