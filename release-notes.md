# ani-cli-rs 0.4.0

`0.4.0` focuses on faster, more resilient downloads and better project support resources. aria2c can now accelerate direct and HLS transfers, downloader failures proceed through a real fallback chain, and the repository includes structured issue reporting, contribution guidance, and security documentation.

There are no intentional breaking changes to existing CLI flags, environment variables, history files, library APIs, or JSON subcommands.

## Highlights

### Faster parallel downloads with aria2

- Direct media downloads prefer aria2c when it is installed, using up to 16 connections and resumable partial files.
- HLS downloads let yt-dlp delegate transfers to aria2c while retaining 16 concurrent fragments.
- Provider-specific Referer, Origin, and additional request headers are passed to every downloader.
- A failed aria2c transfer falls back to the built-in Rust downloader for direct media.
- HLS failures retry through native yt-dlp and then FFmpeg instead of stopping after the first installed tool fails.
- Existing installations without aria2c continue to work without configuration changes.

Download acceleration depends on the media server supporting parallel range requests. When a server limits connections or bandwidth, aria2c may perform similarly to the existing downloaders.

### GitHub community and security resources

- Added structured forms for application bugs, provider breakage, and feature requests.
- Added a pull-request template with formatting, linting, testing, scraper-fixture, and privacy checks.
- Added contribution guidance for deterministic scraper tests, platform-safe process handling, and focused commits.
- Added a security policy directing exploitable reports to GitHub's private vulnerability-reporting flow.
- Added dedicated provider diagnostics prompts while warning reporters not to publish signed media URLs, credentials, or private paths.

### Documentation and release verification

- Added README quick links for installation, usage, diagnostics, contribution, security, and roadmap documentation.
- Documented why unsigned Windows builds and the AllAnime crypto/bootstrap workflow may trigger heuristic antivirus warnings.
- Added manual SHA-256 verification instructions and clarified what a matching checksum does and does not establish.
- Corrected the deliberately excluded feature list now that AnimeSchedule lookup is implemented.

## Download behavior

When aria2c is installed and available in `PATH`, the fallback order is:

```text
Direct media: aria2c → built-in resumable Rust downloader
HLS:         yt-dlp + aria2c → native yt-dlp → FFmpeg
```

No new downloader is mandatory. Users who want parallel transfers can install aria2 through their operating system's package manager.

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

- `ani-cli-rs-0.4.0-x86_64-pc-windows-msvc.zip`
- `ani-cli-rs-0.4.0-x86_64-pc-windows-msvc.zip.sha256`
- `ani-cli-rs-0.4.0-x86_64-unknown-linux-musl.tar.gz`
- `ani-cli-rs-0.4.0-x86_64-unknown-linux-musl.tar.gz.sha256`

Official macOS binaries are not published. macOS users may build from source with the included Cargo aliases.

## Verification

- 30 deterministic Rust tests pass.
- `cargo fmt --check` passes.
- `cargo check --all-targets` passes.
- `cargo clippy --all-targets -- -D warnings` passes.
- aria2c argument construction and yt-dlp delegation are covered by unit tests.

**Full changelog:** https://github.com/vorlie/ani-cli-rs/compare/0.3.0...0.4.0
