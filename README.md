# ani-cli-rs

A cross-platform Rust port of [ani-cli](https://github.com/pystardust/ani-cli) focused on the current AllAnime workflow. It provides both the distinctly named `ani-cli-rs` executable and an `ani_cli` library, allowing it to coexist with the Bash `ani-cli`. Derived work is licensed under GPL-3.0-only; see the repository-level `LICENSE`.

## Quick links

- [Build](#build) · [Release packaging](#target-specific-and-standalone-releases) · [Install](#install-from-github-releases)
- [Antivirus and verification](#antivirus-and-release-verification) · [Compatible workflow](#compatible-workflow) · [Keyboard navigation](#keyboard-navigation)
- [Scriptable commands](#scriptable-commands) · [Library API](#library) · [Diagnostics and tests](#diagnostics-and-tests)
- [Deliberately excluded features](#deliberately-excluded-from-v1)

## Build

```console
cargo build --release
```

The binary is written to `target/release/ani-cli-rs` (`ani-cli-rs.exe` on Windows). The scraper uses native Rust HTTP and cryptography and does not require curl, sed, OpenSSL, Botan, or fzf.

### Target-specific and standalone releases

Explicit Cargo targets are kept separate automatically:

```console
cargo release-windows
cargo release-linux
cargo release-linux-arm64
```

Local macOS builds are also available for users and contributors, although they are not published as official release assets:

```console
cargo release-macos
cargo release-macos-arm64
```

For example, the portable Linux build is placed in `target/x86_64-unknown-linux-musl/release/ani-cli-rs`, while Windows uses `target/x86_64-pc-windows-msvc/release/ani-cli-rs.exe`.

The Windows release command and packaging script remap the builder's user-profile path to `/build`, preventing local usernames from being embedded in Rust panic and source-location strings.

The packaging scripts additionally create a standalone archive under `dist/` containing only the executable, README, and license:

```powershell
.\scripts\package-release.ps1
```

```sh
./scripts/package-release.sh
```

The shell script selects a musl Linux target from the host architecture. An explicit Linux target can also be supplied, such as `./scripts/package-release.sh x86_64-unknown-linux-musl`. Install the selected target with `rustup target add <target>` first; musl builds also require the corresponding musl linker toolchain.

Official prebuilt releases are provided for Windows and Linux only. macOS is not included because hosted macOS CI consumes a limited, higher-cost GitHub Actions minute allocation. The code remains portable, so macOS users may build it locally with `cargo build --release`, but no macOS release archive or installer support is promised.

Each package is accompanied by a `.sha256` file. Upload both files to the matching release in `vorlie/ani-cli-rs`; the installers refuse archives that do not match the published checksum.

### Install from GitHub Releases

Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/install.sh -o install.sh
sh install.sh
rm install.sh
```

Windows PowerShell:

```powershell
Invoke-WebRequest https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/install.ps1 -OutFile install.ps1
.\install.ps1
Remove-Item .\install.ps1
```

The Unix installer uses `~/.local/bin` and updates `~/.profile` only when needed. The Windows installer uses `%LOCALAPPDATA%\Programs\ani-cli-rs\bin` and updates the user-level `PATH`. Override these with `ANI_CLI_RS_INSTALL_DIR` on Unix or `-InstallDirectory` on Windows.

Uninstall with the corresponding `scripts/uninstall.sh` or `scripts/uninstall.ps1` script.

### Antivirus and release verification

Official Windows binaries are currently unsigned and may trigger Microsoft SmartScreen or heuristic antivirus detections. VirusTotal's behavior report for older releases has also described parts of the executable as obfuscated. A detection should be investigated rather than dismissed automatically, but that label can be caused by behavior that is expected and visible in this repository:

- The AllAnime client performs AES-GCM, AES-CTR, XOR, SHA-256, and Base64 operations to construct and decode provider requests.
- It downloads and scans AllAnime/Mkissa JavaScript bootstrap data because the provider changes its runtime crypto material.
- It resolves media URLs and starts external programs such as mpv, VLC, Syncplay, yt-dlp, or FFmpeg.
- The optional installer adds a user-local binary directory to the user's `PATH`; the application itself does not require administrator privileges.

Release archives include a separate `.sha256` file. The provided installers verify this checksum automatically. To verify a downloaded Windows archive manually:

```powershell
(Get-FileHash .\ani-cli-rs-0.3.0-x86_64-pc-windows-msvc.zip -Algorithm SHA256).Hash.ToLowerInvariant()
Get-Content .\ani-cli-rs-0.3.0-x86_64-pc-windows-msvc.zip.sha256
```

The two hashes must match. A matching checksum confirms that the archive is identical to the file published with the GitHub release; it does not replace reviewing the source or trusting the release publisher. Users who prefer not to run a prebuilt executable can inspect the tagged source and build it locally with `cargo build --release --locked`.

Playback requires `mpv` by default. `--vlc` uses VLC and `--syncplay` uses Syncplay. HLS downloads use `yt-dlp`, falling back to `ffmpeg`; direct MP4 downloads are handled internally with resumable `.part` files. Downloads report transferred size, speed, and ETA when available.

## Compatible workflow

```console
ani-cli-rs frieren
ani-cli-rs --allow-adult "search query"
ani-cli-rs --dub -q 720p "cowboy bebop"
ani-cli-rs -S 1 -e 2-4 "one piece"
ani-cli-rs --continue
ani-cli-rs --download -e 1 "anime title"
ani-cli-rs --delete
```

Supported compatibility flags are `-c/--continue`, `-d/--download`, `-D/--delete`, `-s/--syncplay`, `-S/--select-nth`, `-q/--quality`, `-v/--vlc`, `-e/--episode`, `-r/--range`, `-a/--allow-adult`, `-N/--nextep-countdown`, `--dub`, `--multi-selection`, `--no-detach`, and `--exit-after-play`.

Supported environment variables are `ANI_CLI_MODE`, `ANI_CLI_PLAYER`, `ANI_CLI_DOWNLOAD_DIR`, `ANI_CLI_QUALITY`, `ANI_CLI_HIST_DIR`, `ANI_CLI_ALLOW_ADULT`, `ANI_CLI_MULTI_SELECTION`, `ANI_CLI_NO_DETACH`, and `ANI_CLI_EXIT_AFTER_PLAY`.

History remains compatible with ani-cli's tab-separated `ani-hsts` format. Set `ANI_CLI_HIST_DIR` to share an existing history directory.

`-N/--nextep-countdown QUERY` matches Bash ani-cli's release-schedule mode: it displays AnimeSchedule's next raw and subtitled release timestamps and exits without contacting AllAnime.

After a single episode is launched from an interactive terminal, ani-cli-rs keeps the session open with controls for next, replay, previous, episode selection, and quality changes. This menu is also shown when the episode was supplied through `-e/--episode`; use `--exit-after-play` to skip it.

### Keyboard navigation

- Arrow keys navigate every menu; `Tab` and `Shift+Tab` also move down and up.
- Plain action menus additionally accept `j`/`k` to move, `h`/`l` to change pages, and Space or Enter to select.
- Fuzzy anime and episode menus accept typing immediately and include a visible Back row.
- The episode picker includes a multi-selection mode; use Space to toggle episodes and Enter to confirm. Set `ANI_CLI_MULTI_SELECTION=true` or pass `--multi-selection` to open it directly.
- Escape goes back immediately from a fuzzy menu: episodes return to anime results, and anime results return to search. `q` remains available as a filter character for titles containing that letter.
- In action and quality menus, `q` or Escape returns or exits without treating cancellation as an error.
- Non-interactive options such as `--select-nth` and `--episode` retain their existing behavior.

## Scriptable commands

```console
ani-cli-rs search --json "frieren"
ani-cli-rs search --allow-adult --json "search query"
ani-cli-rs episodes --json SHOW_ID --mode sub
ani-cli-rs links --json SHOW_ID 1 --quality 1080p
ani-cli-rs play SHOW_ID 1 --title "Frieren" --no-detach
ani-cli-rs download SHOW_ID 1 --output ./downloads
ani-cli-rs debug --refresh
ani-cli-rs refresh-cipher-map
```

`debug` reports dynamic/fallback crypto bootstrap material. `refresh-cipher-map` validates and caches the URL decoder from the latest upstream ani-cli release. Set `RUST_LOG=warn` or `RUST_LOG=debug` for scraper diagnostics.

## Library

```rust,no_run
use ani_cli::{AllAnimeClient, SearchOptions, TranslationType};

#[tokio::main]
async fn main() -> ani_cli::Result<()> {
    let client = AllAnimeClient::new()?;
    let shows = client.search_with_options(
        "search query",
        TranslationType::Sub,
        SearchOptions { allow_adult: true },
    ).await?;
    let episodes = client.episodes(&shows[0].id, TranslationType::Sub).await?;
    let streams = client.streams(&shows[0].id, &episodes[0], TranslationType::Sub).await?;
    println!("{}", streams[0].url);
    Ok(())
}
```

## Diagnostics and tests

Normal tests use local crypto/provider fixtures and do not contact AllAnime:

```console
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Live endpoints are intentionally not part of deterministic test runs because AllAnime's bootstrap and source URLs change frequently.

## Deliberately excluded from v1

Self-update, rofi/dmenu, Android/Termux and iSH adapters, intro skipping, and system-journal logging are outside desktop-core parity.
