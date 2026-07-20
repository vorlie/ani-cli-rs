# ani-cli-rs 0.2.0

`0.2.0` is the first feature update to the Rust ani-cli port. It improves AllAnime rollout resilience, adds adult-search controls and live download progress, and makes Windows builds and HTTPS handling more reliable.

There are no intentional breaking changes to the existing CLI flags, history format, or JSON subcommands.

## Highlights

### Adult search toggle

- Added `-a` / `--allow-adult` to interactive searches and the scriptable `search` subcommand.
- Added `ANI_CLI_ALLOW_ADULT` for persistent configuration.
- Added `SearchOptions` and `AllAnimeClient::search_with_options` to the library API; the existing `search` method remains available and keeps adult results disabled by default.

### Download progress

- Direct downloads now show percentage, transferred size, total size, average speed, and ETA.
- Resumed `.part` downloads include existing bytes in the displayed progress.
- Interactive terminals update progress in place, while redirected output emits periodic progress lines.
- yt-dlp progress is explicitly enabled, and FFmpeg retains its native bitrate and speed statistics.
- Failed direct downloads leave the terminal on a clean line and preserve the resumable partial file.

### AllAnime reliability

- Updated bundled crypto material for AllAnime epoch `6884`, build `48`.
- Retained the legacy epoch `4128` material for builds `12` and `9`.
- Added adjacent-epoch attempts to tolerate short bootstrap/API rollout mismatches.
- Full GraphQL requests are attempted immediately after a failed persisted query for each crypto candidate, reducing unnecessary requests before a viable fallback.
- Expanded tests for current and legacy key derivation, adult search variables, and fallback behavior.

### Windows and networking

- HTTPS now uses Windows-native certificate roots together with bundled public WebPKI roots, improving compatibility on filtered or unusually configured networks.
- Network errors now include their underlying TLS and transport causes for more useful diagnostics.
- `cargo release-windows` and the Windows packaging script remap local profile paths, preventing the builder's username from being embedded in release binaries through Rust source-location strings.

### Installer and documentation fixes

- Fixed the Linux installer when GitHub returns compact, single-line release JSON.
- Corrected raw installation links to use the repository's `master` branch.
- Improved documentation for adult search, release builds, and download status output.
- AllAnime debug JSON exports are ignored by Git to prevent accidental commits.

## Installation

### Linux

```sh
curl -fsSL https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/install.sh -o install.sh
sh install.sh
```

### Windows PowerShell

```powershell
Invoke-WebRequest https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/install.ps1 -OutFile install.ps1
.\install.ps1
```

The installers verify the downloaded archive against its published SHA-256 checksum before installing it into a user-local directory and adding that directory to `PATH` when necessary.

## Release asset checklist

Upload each archive together with its generated `.sha256` file:

- `ani-cli-rs-0.2.0-x86_64-pc-windows-msvc.zip`
- `ani-cli-rs-0.2.0-x86_64-pc-windows-msvc.zip.sha256`
- `ani-cli-rs-0.2.0-x86_64-unknown-linux-musl.tar.gz`
- `ani-cli-rs-0.2.0-x86_64-unknown-linux-musl.tar.gz.sha256`

Official macOS binaries are not published. macOS users can build from source with the included Cargo aliases.

## Verification

- 22 deterministic Rust tests pass.
- `cargo fmt --all -- --check` passes.
- `cargo clippy --locked --all-targets -- -D warnings` passes.
- Windows and Linux release packaging produce versioned archives and checksum files.

**Full changelog:** https://github.com/vorlie/ani-cli-rs/compare/0.1.0...v0.2.0
