# ani-cli-rs 0.9.0

This release adds tested Android/Termux playback support, including native Android player handoff, protected HLS relaying, and external subtitle tracks.

## Highlights

- Added source-build support for Android through Termux.
- Added Android `VIEW` intent launching for mpv-android and VLC.
- Added compatibility fallbacks through media-typed `termux-open` and `termux-open-url`.
- Added Android playback for protected MegaPlay/KotoCDN HLS streams through the existing loopback relay.
- Added external WebVTT/SRT subtitles as standard HLS subtitle renditions.
- Added generated subtitle media playlists with duration derived from the final subtitle cue.
- Added Termux-specific help, installation instructions, usage guidance, and troubleshooting.

## Android player behavior

Normal playback requests mpv-android. `--vlc` requests the Android VLC application when Termux's explicit activity bridge works.

Some Termux installations expose a socket-backed `termux-am` that cannot connect. ani-cli-rs now detects a failed launch and falls back to Android's media handler. On this fallback path, Android's selected or default application takes precedence, so `--vlc` becomes a preference rather than a guarantee.

For protected HLS, keep Termux running while watching. Return to Termux and press Enter only after playback finishes; pressing Enter early shuts down the local relay.

## Subtitles

Separate provider subtitles are wrapped in HLS subtitle media playlists and exposed to compatible Android players. Select the track from the player's subtitle menu if it is not enabled automatically.

Device testing confirmed relayed playback and selectable external subtitles with mpv-android, VLC, Amnis, and Samsung Video Player. Compatibility can still vary between Android versions and player builds. Embedded and burned-in subtitles are unaffected.

## Installation

No Android binary is published. Build from source inside Termux:

```sh
pkg update
pkg install git rust termux-tools
git clone https://github.com/vorlie/ani-cli-rs.git
cd ani-cli-rs
cargo build --release --locked
install -Dm755 target/release/ani-cli-rs "$PREFIX/bin/ani-cli-rs"
```

Install an Android media player separately. Do not use `pkg install vlc` for Android handoff; that installs a terminal VLC build rather than the Android application.

```sh
ani-cli-rs "cyberpunk"
ani-cli-rs --vlc "cyberpunk"
```

Official prebuilt releases remain available for Windows x64 and Linux x86-64. macOS and Termux remain tested source-build platforms.

## Upgrade

Existing Windows and Linux installations can update with:

```console
ani-cli-rs update
```

Termux users should pull and rebuild:

```sh
cd ~/ani-cli-rs
git pull --ff-only
cargo build --release --locked
install -Dm755 target/release/ani-cli-rs "$PREFIX/bin/ani-cli-rs"
```

**Full changelog:** [0.8.0...0.9.0](https://github.com/vorlie/ani-cli-rs/compare/0.8.0...0.9.0)
