# ani-cli-rs Wiki

`ani-cli-rs` is a cross-platform Rust port of [ani-cli](https://github.com/pystardust/ani-cli). It searches two independent Anikoto catalogs, resolves native episode sources, launches desktop or Android media players, downloads direct media and HLS streams, and preserves the familiar ani-cli command-line workflow.

The executable is named `ani-cli-rs`, so it can coexist with the Bash `ani-cli`. The project also exposes an `ani_cli` Rust library.

## Start here

- **New user:** [Installation](Installation) → [Getting Started](Getting-Started)
- **Looking for an option:** [CLI Reference](CLI-Reference)
- **Playback setup:** [Playback and Players](Playback-and-Players)
- **Downloading:** [Downloads](Downloads)
- **Something failed:** [Troubleshooting](Troubleshooting)
- **Checking a release:** [Security and Privacy](Security-and-Privacy)
- **Building or packaging:** [Building and Releasing](Building-and-Releasing)
- **Working on a provider:** [Provider Architecture](Provider-Architecture), the [Anikoto API/KotoCDN reference](https://github.com/vorlie/ani-cli-rs/blob/master/docs/ANIKOTO-KOTOCDN.md), and the [Anikoto.cz reference](https://github.com/vorlie/ani-cli-rs/blob/master/docs/ANIKOTO-CZ.md)

## Platform support

| Platform | Official binaries | Local source builds | Notes |
|---|---:|---:|---|
| Windows 10/11 x64 | Yes | Yes | Portable ZIP and optional per-user Inno Setup installer |
| Windows on ARM64 | x64 build | Yes | Windows 11 can run the published x64 build through emulation |
| Linux x86-64 | Yes | Yes | Official release uses musl for broad distribution compatibility |
| Linux ARM64 | No | Yes | Source build only; not device-tested by the maintainer |
| macOS Intel/Apple Silicon | No | Tested | Build locally; IINA is selected automatically |
| Android/Termux | No | Tested | Build locally; launches Android mpv or VLC through Termux intents |
| iSH | No | Not supported | No iOS player adapter |

## External programs

The scraper itself does not require curl, sed, OpenSSL, Botan, or fzf. Some actions use programs discovered through `PATH`:

- `mpv` for default desktop playback;
- Android mpv through Termux intents on Android;
- VLC with `--vlc`;
- Syncplay with `--syncplay`;
- `aria2c` for faster direct and delegated HLS downloads;
- `yt-dlp` and FFmpeg for HLS downloads.

See [Playback and Players](Playback-and-Players) and [Downloads](Downloads) for exact fallback behavior.

## Important upstream limitation

Both Anikoto catalogs and their third-party video hosts change independently. A released episode can have no usable native source because every copy is deleted, blocked, expired, or temporarily failing. ani-cli-rs reports provider-specific failures but cannot restore media that no upstream host currently serves.

## Project links

- [Repository](https://github.com/vorlie/ani-cli-rs)
- [Releases](https://github.com/vorlie/ani-cli-rs/releases)
- [Issues](https://github.com/vorlie/ani-cli-rs/issues)
- [Upstream Bash ani-cli](https://github.com/pystardust/ani-cli)
- License: GPL-3.0-only
