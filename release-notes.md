# ani-cli-rs 0.5.1

`0.5.1` is a small quality-of-life patch for downloads and command-line documentation. It keeps aria2's useful live status while removing its noisy expanded progress summaries, and makes the required AllAnime/Mkissa show ID much clearer in scriptable commands.

There are no intentional breaking changes to playback, downloads, history, library APIs, or JSON output.

## Changes

### Quieter aria2 progress

- Disabled aria2's expanded periodic progress-summary blocks.
- Kept the compact live progress line with downloaded size, percentage, speed, connections, and ETA.
- Kept the reduced four-connection limit for Mp4Upload downloads introduced in `0.5.0`.

### Clearer show ID documentation

- Clarified that the `episodes`, `links`, `play`, and `download` subcommands accept an AllAnime/Mkissa show ID, not an anime title.
- Documented how to copy the ID from the final segment of a Mkissa anime URL. For example, the show ID in `https://mkissa.to/anime/SyR2K6bGYfKSE6YMm` is `SyR2K6bGYfKSE6YMm`.
- Documented `ani-cli-rs search --json "anime title"` as a scriptable way to obtain a show ID.
- Clarified that `--title` controls display text and the output filename; it does not replace the show ID.
- Added a tested help-text regression check for the `download` command.

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

The installers verify the downloaded release archive against its published SHA-256 checksum before installation.

## Release asset checklist

Upload each archive together with its generated `.sha256` file:

- `ani-cli-rs-0.5.1-x86_64-pc-windows-msvc.zip`
- `ani-cli-rs-0.5.1-x86_64-pc-windows-msvc.zip.sha256`
- `ani-cli-rs-0.5.1-x86_64-unknown-linux-musl.tar.gz`
- `ani-cli-rs-0.5.1-x86_64-unknown-linux-musl.tar.gz.sha256`

Official macOS binaries are not published. macOS users may build from source with the included Cargo aliases.

## Verification

- 36 deterministic Rust tests pass.
- `cargo fmt --check` passes.
- `cargo check --all-targets` passes.
- `cargo clippy --all-targets -- -D warnings` passes.

**Full changelog:** https://github.com/vorlie/ani-cli-rs/compare/0.5.0...0.5.1
