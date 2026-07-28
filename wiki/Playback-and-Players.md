# Playback and Players

Anikoto API/MegaPlay is the default catalog. Select the independent Anikoto.cz catalog with `--provider anikoto2` or `ANI_CLI_PROVIDER=anikoto2`. For KotoCDN HLS from either provider, ani-cli-rs starts a loopback-only relay and remains attached to the player until it exits; this translates confirmed PNG-wrapped transport-stream segments.

## mpv

mpv is the default player on Linux and Windows. Install it through your operating system and ensure `mpv` (`mpv.exe` on Windows) is in `PATH`.

```console
ani-cli-rs "title"
```

ani-cli-rs passes the stream URL, media title, and provider referrer as separate process arguments. mpv receives `--tls-verify=no` for compatibility with third-party media hosts.

Override the executable:

```powershell
$env:ANI_CLI_PLAYER = "C:\Program Files\mpv\mpv.exe"
ani-cli-rs "title"
```

```sh
ANI_CLI_PLAYER=/opt/mpv/bin/mpv ani-cli-rs "title"
```

## IINA on macOS

IINA is the tested default player on macOS. Install it with `brew install --cask iina` and ensure its `iina` command is on `PATH`. ani-cli-rs passes provider headers and subtitles through IINA's mpv-compatible options.

`ANI_CLI_PLAYER` overrides the platform's default executable, so it must point to an IINA-compatible command on macOS or an mpv-compatible command on Linux and Windows.

## VLC

```console
ani-cli-rs --vlc "title"
```

VLC receives `--play-and-exit`, the media title, and `--http-referrer` when required. The executable defaults to `vlc.exe` on Windows and `vlc` elsewhere.

## Android players from Termux

On Android, ani-cli-rs launches an installed Android player through a `VIEW` intent. Normal playback requests mpv-android (`is.xyz.mpv`); `--vlc` requests the Android VLC application instead of a terminal VLC executable.

Ensure the standard Termux tools are installed:

```sh
pkg install termux-tools
```

ani-cli-rs searches for `termux-am-starter`, `termux-am`, and `am` and first attempts to target the requested Android player directly. Some Termux app/package combinations expose a socket-backed `termux-am` that cannot connect. In that case ani-cli-rs automatically uses media-typed `termux-open`, falling back again to `termux-open-url` when necessary. Android's chosen/default media handler controls which app opens on this fallback path; `--vlc` is therefore a preference, not a guarantee. The media type helps Android choose a video player instead of a web browser.

HLS playback is routed through ani-cli-rs's loopback relay on Android. After launching the player, the terminal intentionally waits:

```text
Opened the Android player. Return to Termux and press Enter after playback ends.
```

This keeps protected playlists, segments, and provider headers available to the Android app. Return to Termux and press Enter only after playback ends; the normal next/replay/previous menu then continues. Non-interactive Android HLS playback is rejected because exiting immediately would destroy the required relay.

Android intent APIs do not accept the same mpv/VLC command-line options as desktop executables. For relayed HLS, ani-cli-rs wraps provider WebVTT/SRT tracks in timed subtitle media playlists and publishes them as standard HLS renditions. Select the track from the Android player's subtitle menu if it is not enabled automatically. Device testing confirmed relayed playback and selectable external subtitles with mpv-android, VLC, Amnis, and Samsung Video Player. Compatibility can still vary between Android versions and player builds. Embedded and burned-in subtitles remain unchanged.

## Syncplay

```console
ani-cli-rs --syncplay "title"
```

The executable defaults to `syncplay.exe` on Windows and `syncplay` elsewhere. ani-cli-rs supplies the stream and mpv-compatible referrer/title options to Syncplay.

Syncplay coordinates playback state; it does not make an unavailable provider source available.

## Attached and detached playback

By default the player is detached after it starts. Use:

```console
ani-cli-rs --no-detach "title"
```

Attached playback allows ani-cli-rs to wait for player completion before continuing. Use `--exit-after-play` when the player's non-zero exit should also fail the CLI command.

Environment equivalents:

```text
ANI_CLI_NO_DETACH=true
ANI_CLI_EXIT_AFTER_PLAY=true
```

Accepted true values are `1`, `true`, and `yes`, case-insensitively.

## Interactive post-playback controls

After a single episode is launched in an interactive terminal, the action menu can include:

- Next episode;
- Replay;
- Previous episode;
- Select another episode;
- Change quality;
- Quit.

Unavailable directions are hidden. Multi-episode batches, downloads, non-terminal runs, and `--exit-after-play` do not open this menu.

## Referrer-related failures

Many media hosts return HTTP 403 without the correct `Referer` or `Origin`. `StreamLink` carries these headers from resolution into player arguments. If a URL works in ani-cli-rs but not when pasted into a browser, that does not necessarily indicate a bug: the browser request may lack the required header, or the URL may be bound to an IP/expiry time.

For diagnosis, use `links --json` but redact signed query strings before sharing output.
