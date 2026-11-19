# ani-cli-rs 0.12.0

## Added

- Added automated GitHub Actions release builds (`.github/workflows/release.yml`) for Windows (`x86_64-pc-windows-msvc` ZIP and Inno Setup installer), Linux (`x86_64-unknown-linux-musl`), and macOS (`aarch64-apple-darwin` and `x86_64-apple-darwin`).
- Enabled macOS binary packaging in `scripts/package-release.sh`, installation in `scripts/install.sh`, and self-updating (`ani-cli-rs -U` / `ani-cli-rs update`) on macOS.
- Exposed `ani_lib::anikoto2_total_episodes` so CLI/library consumers can read Anikoto.cz episode totals without decoding provider IDs directly.

## Changed

- Modularized the CLI binary (`src/bin_app/`) and library (`src/error/`, `src/providers/`, `src/player/`, `src/download/`, `src/util/`), consolidating duplicated HTTP, HLS expansion, caching, and subtitle handling logic.
- Consolidated release version checking on `semver::Version`.
- Replaced raw `eprintln!` debug/diagnostic output in `ani_lib` with `tracing::debug!` so library consumers control log verbosity cleanly.

## Verification

- Verified formatting (`cargo fmt --check`), linting (`cargo clippy --all-targets`), CLI help (`cargo run -- --help`), and the full test suite (`cargo test --all-targets`: 100/100 passing on Windows).
