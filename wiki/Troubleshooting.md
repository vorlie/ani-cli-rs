# Troubleshooting

Start by identifying which stage failed:

1. installation or `PATH`;
2. search/episode discovery;
3. provider server/native-source resolution;
4. KotoCDN relay handling;
5. player launch;
6. download transfer/finalization.

The distinction matters: reinstalling will not repair a provider outage, and changing providers will not fix a missing player executable.

## Installed but command not found

Windows portable installations default to:

```text
C:\Users\USER\.local\bin\ani-cli-rs.exe
```

Check:

```powershell
Test-Path "$HOME\.local\bin\ani-cli-rs.exe"
[Environment]::GetEnvironmentVariable("Path", "User")
Get-Command ani-cli-rs.exe -ErrorAction SilentlyContinue
```

Open a new PowerShell or Command Prompt after installation. The current terminal does not automatically inherit persistent `PATH` changes made by another process.

Linux:

```sh
ls -l "$HOME/.local/bin/ani-cli-rs"
printf '%s\n' "$PATH"
command -v ani-cli-rs
```

Source the profile modified by the installer or open a new shell.

## Windows installer says the architecture is unsupported

Current installer scripts deliberately select the published x64 Windows archive on Windows, including Windows 11 ARM64 systems that can emulate x64. Download the newest `install.ps1`; older cached copies may still contain architecture detection that rejects some systems.

Useful system information:

```powershell
[Environment]::Is64BitOperatingSystem
[Environment]::Is64BitProcess
$env:PROCESSOR_ARCHITECTURE
$env:PROCESSOR_ARCHITEW6432
Get-CimInstance Win32_OperatingSystem | Select-Object Caption, Version, BuildNumber, OSArchitecture
```

## Rust says edition 2024 is unavailable

Your shell is using an old Cargo. Verify:

```sh
which cargo
which rustc
cargo --version
rustc --version
```

After installing through rustup:

```sh
source "$HOME/.cargo/env"
rustup update stable
```

Ensure `$HOME/.cargo/bin` appears before `/usr/bin` in `PATH`.

## Musl target cannot find `core` or `std`

Install the exact target:

```sh
rustup target add x86_64-unknown-linux-musl
```

or:

```sh
rustup target add aarch64-unknown-linux-musl
```

Also install the corresponding musl linker/toolchain through your distribution.

## Search works but sources fail

Retry once, or select the other catalog with `--provider anikoto2` or `--provider anikoto`. Common categories:

- `rate limit reached`: wait before retrying;
- `catalog error`: the selected catalog endpoint failed;
- `no mapping`: the catalog could not map the selected episode to an embed;
- `native source resolution failed`: returned servers were unsupported, deleted, or changed;
- `provider playlist exceeds ... limit`: the response violated relay safety bounds.

Repeated rapid retries can prolong upstream throttling.

## A title works but another does not

Different episodes use different third-party hosts. One show can resolve through MegaPlay or VidTube while another has only unsupported or deleted embeds. Include the show ID, episode, mode, version, and sanitized provider categories in a bug report.

Do not include complete signed media URLs.

## HTTP 403 during playback or download

Likely causes:

- expired signed URL;
- missing provider referrer/origin;
- URL bound to another public IP;
- router, DNS, antivirus, or corporate filtering;
- too many parallel connections.

Resolve the source again. Test the other catalog, another network, or a VPN only if allowed by your network policy. FortiGuard and similar filters may classify catalog or media hosts as adult/streaming content even when ani-cli-rs itself works correctly.

## mpv, VLC, Syncplay, aria2c, yt-dlp, or FFmpeg not found

Verify the executable directly:

```powershell
Get-Command mpv.exe, vlc.exe, syncplay.exe, aria2c.exe, yt-dlp.exe, ffmpeg.exe -ErrorAction SilentlyContinue
```

```sh
command -v mpv vlc syncplay aria2c yt-dlp ffmpeg
```

## Termux opens terminal VLC or cannot find an Android player

`pkg install vlc` installs a terminal program and does not expose the separately installed Android VLC application. Install an Android player app and ensure the standard Termux tools are current:

```sh
pkg install termux-tools
command -v termux-am-starter termux-am am termux-open termux-open-url
```

Run `ani-cli-rs "title"` to request mpv-android or `ani-cli-rs --vlc "title"` to request Android VLC. If the explicit activity launcher reports a missing socket, ani-cli-rs uses media-typed `termux-open` before falling back to `termux-open-url`. Android's chosen/default media handler controls the fallback, so `--vlc` cannot force VLC there. If Android still opens a browser, clear that browser's default link association and select an installed video player. If the activity launcher is installed at a custom path, set `ANI_CLI_PLAYER` to that launcher.

## Termux playback stops after returning to the terminal

Protected HLS is served through a loopback relay owned by the running ani-cli-rs process. Leave Termux running in the background while watching and press Enter only after playback has ended. Pressing Enter early shuts down the relay and makes the player lose its playlist and segments.

## Termux video plays without external subtitles

Open the Android player's subtitle menu and select the provider track. ani-cli-rs exposes separate provider subtitles as standard HLS subtitle renditions. Device testing confirmed them with mpv-android, VLC, Amnis, and Samsung Video Player, but behavior can still differ between Android versions and player builds. Burned-in subtitles are unaffected.

If a player reports a 30-minute subtitle rendition for an unusual movie or special, the provider subtitle could not be parsed for its final cue time and ani-cli-rs used its bounded fallback duration. Include the provider name, title, episode, and sanitized `links --json` output in a bug report.

Install only the tools needed by your workflow. Direct downloads can fall back to Rust; HLS requires yt-dlp or FFmpeg.

## `.part` remains after a download

The transfer did not finalize. Rerun the same command to resume where supported. Check free disk space, file permissions, antivirus quarantine, whether another program holds the final filename, and the final aria2/ani-cli-rs error. See [Downloads](Downloads#why-files-use-part).

## Debugging without leaking secrets

Safe report template:

```text
ani-cli-rs version:
OS and architecture:
Command shape (redact title if needed):
Show ID and episode:
Sub or dub:
Provider failure categories:
Expected behavior:
Actual behavior:
```

Never attach cookies, authorization headers, active signed URLs, raw decrypted episode payloads, or personal paths unless carefully sanitized.
