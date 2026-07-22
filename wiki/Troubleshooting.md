# Troubleshooting

Start by identifying which stage failed:

1. installation or `PATH`;
2. search/episode discovery;
3. crypto/bootstrap decoding;
4. provider source resolution;
5. player launch;
6. download transfer/finalization.

The distinction matters: reinstalling will not repair a provider outage, and refreshing crypto will not fix a missing player executable.

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

Try:

```console
ani-cli-rs debug --refresh
ani-cli-rs refresh-cipher-map
```

Then retry. Common categories:

- `AA_CRYPTO_STALE`: frontend and API crypto material disagree or cached material is stale;
- `AA_CRYPTO_CROSS_KEY`: build/epoch material does not match the API response;
- `Too many requests`: AllAnime rate limiting; wait before retrying;
- `released but no supported sources resolved`: episode metadata exists, but all extracted providers failed or are unsupported;
- `provider reports video unavailable`: the host explicitly reports deletion/blocking;
- `provider returned an HTTP error`: an AllAnime proxy or video host returned a failing status.

The client retries bounded rate limits and tries dynamic, bundled, legacy, persisted-query, and full-query paths. Repeated rapid retries can prolong upstream throttling.

## A title works but another does not

Different episodes use different third-party hosts. One show can resolve through Wix or OK.ru while another has only deleted or protected Filemoon/VidGuard copies. Include the show ID, episode, mode, version, and sanitized provider categories in a bug report.

Do not include complete signed media URLs.

## HTTP 403 during playback or download

Likely causes:

- expired signed URL;
- missing provider referrer/origin;
- URL bound to another public IP;
- router, DNS, antivirus, or corporate filtering;
- too many parallel connections.

Resolve the source again. Test another network or VPN only if allowed by your network policy. FortiGuard and similar filters may classify Mkissa/AllAnime or media hosts as adult/streaming content even when ani-cli-rs itself works correctly.

## mpv, VLC, Syncplay, aria2c, yt-dlp, or FFmpeg not found

Verify the executable directly:

```powershell
Get-Command mpv.exe, vlc.exe, syncplay.exe, aria2c.exe, yt-dlp.exe, ffmpeg.exe -ErrorAction SilentlyContinue
```

```sh
command -v mpv vlc syncplay aria2c yt-dlp ffmpeg
```

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
