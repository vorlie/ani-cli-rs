# ani-cli-rs 0.1.0

Initial release of a standalone Rust port of ani-cli for Windows, Linux, and macOS source builds. The executable is named `ani-cli-rs`, allowing it to coexist with the Bash `ani-cli` command.

## AllAnime support

- Added catalog search, sub/dub episode discovery, encrypted episode queries, and source resolution.
- Added runtime MKissa crypto and API discovery with AES-256-GCM request signing and encrypted-response decoding.
- Added controlled rate-limit retries and compatibility fallbacks for changing AllAnime builds.
- Added support for obfuscated source URLs, internal clock APIs, direct MP4 and HLS media, Mp4Upload, fast4speed, master playlists, and Wix multi-quality streams.
- Preserved provider-specific referer and origin headers for playback and downloads.
- Added runtime refresh and caching of the upstream ani-cli URL cipher map.

## CLI and desktop workflow

- Added familiar ani-cli flags for search, episode/range selection, dub mode, quality, history continuation, downloads, VLC, Syncplay, and attached playback.
- Added keyboard-friendly interactive menus with Back navigation, fuzzy filtering, and next/replay/previous actions.
- Added scriptable `search`, `episodes`, `links`, `play`, `download`, `debug`, and `refresh-cipher-map` subcommands with JSON output where applicable.
- Added Bash ani-cli-compatible tab-separated history storage and supported `ANI_CLI_*` environment variables.
- Added mpv, VLC, and Syncplay launching with arguments passed safely to child processes.
- Added resumable direct downloads and HLS downloads through yt-dlp with an ffmpeg fallback.

## Distribution

- Added Cargo aliases and packaging scripts for Windows and static musl Linux builds, including x86-64 and Linux ARM64 targets.
- Added checksum-verifying install and uninstall scripts for releases published under `vorlie/ani-cli-rs`.
- Official prebuilt releases are provided for Windows and Linux. macOS users can build locally with the included Cargo aliases, but official macOS assets are not currently published.

## Verification

- All 18 automated Rust tests pass.
- `cargo fmt --check` passes.
- `cargo clippy --all-targets -- -D warnings` passes.
- Live AllAnime validation resolves current episode sources successfully.

## Known omissions

Self-update, rofi/dmenu integration, Android/Termux and iSH adapters, intro skipping, system-journal logging, and next-release lookup are not included in 0.1.0.
