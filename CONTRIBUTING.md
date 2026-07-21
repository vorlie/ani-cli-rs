# Contributing to ani-cli-rs

Thank you for helping improve ani-cli-rs. Bug fixes, compatibility improvements, tests, documentation, and focused feature proposals are welcome.

## Before starting

- Search existing issues and pull requests.
- Keep changes scoped to one problem where practical.
- Open an issue before implementing a large CLI redesign, public API break, new network provider, or plugin architecture change.
- Do not commit credentials, cookies, complete signed media URLs, personal filesystem paths, or raw debug dumps containing private information.

## Development setup

Install the current stable Rust toolchain. The package uses Rust edition 2024.

```console
git clone https://github.com/vorlie/ani-cli-rs.git
cd ani-cli-rs
cargo build
```

Playback and HLS download tests may additionally require mpv, VLC, Syncplay, yt-dlp, or FFmpeg, depending on the workflow being changed.

## Required checks

Run these before submitting a pull request:

```console
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
```

Normal tests must remain deterministic and must not require live AllAnime access. Use fixtures or the mock HTTP server for scraper changes. Live endpoint checks should remain explicitly opt-in.

## Scraper changes

Read [`docs/ALLANIME-SCRAPING.md`](docs/ALLANIME-SCRAPING.md) before changing the request, crypto, cipher, or provider-resolution pipeline.

- Include sanitized fixtures for new response shapes and encryption paths.
- Test partial provider failures instead of assuming every extracted source works.
- Preserve required request headers in `StreamLink` rather than constructing shell command strings.
- Avoid logging complete temporary media URLs, authorization values, cookies, or other replayable credentials.
- Explain whether behavior matches Bash ani-cli, AniPlay, or a newly observed upstream change.

## CLI and platform changes

- Preserve `ani-cli [options] [query] [options]` compatibility where possible.
- Keep process arguments separate and platform-safe.
- Add parser or integration tests for new flags and environment variables.
- Document intentional Windows, Linux, or macOS differences.

## Commits and pull requests

Use concise, imperative commit messages. Conventional prefixes such as `fix:`, `feat:`, `docs:`, `test:`, and `chore:` are encouraged but not mandatory.

Fill out the pull-request template, link the relevant issue, and describe manual testing. Contributions are accepted under the repository's GPL-3.0-only license.
