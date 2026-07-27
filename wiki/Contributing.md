# Contributing

Bug fixes, provider compatibility updates, tests, documentation, and focused feature proposals are welcome.

## Before coding

1. Search existing issues and pull requests.
2. Reproduce on the latest release or `master` where safe.
3. Keep one change focused on one problem.
4. Open an issue before a large CLI redesign, public API break, new provider, or plugin architecture change.

## Setup

```console
git clone https://github.com/vorlie/ani-cli-rs.git
cd ani-cli-rs
cargo build
```

Create a descriptive branch:

```console
git switch -c fix/provider-response-shape
git switch -c feat/interactive-download-selection
```

## Required validation

```console
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
```

Do not make normal tests depend on live providers. Use fixtures or Wiremock.

## Good commit boundaries

Prefer commits that are independently understandable and testable:

```text
refactor(cli): separate resolution from media actions
feat(download): preflight episodes before transfers
docs(download): explain interactive selection
chore(release): bump version to 0.6.0
```

Avoid mixing formatting, unrelated cleanup, generated artifacts, and the actual fix.

## Scraper changes

Read [Provider Architecture](Provider-Architecture) and the relevant source-adjacent provider document first.

- Include sanitized response fixtures.
- Preserve partial-provider success.
- Carry provider headers in `StreamLink`.
- Explain whether behavior matches Bash ani-cli, AniPlay, or a newly observed upstream change.
- Never log complete signed URLs or active authentication values.

## CLI/platform changes

- Preserve `ani-cli [options] [query] [options]` compatibility where practical.
- Keep the scriptable subcommands non-interactive.
- Pass process arguments separately.
- Add parser/integration tests for flags and environment variables.
- Document intentional differences among Windows, Linux, and macOS.

## Pull requests

Describe:

- the user-visible problem;
- the chosen behavior;
- tests run;
- manual platform/provider checks;
- known limitations;
- linked issues.

All contributions are accepted under GPL-3.0-only.

## Plugin work

The repository's plugin roadmap proposes versioned external executable plugins rather than Rust dynamic libraries. Do not begin provider-plugin integration before the protocol, history/provider identity, process limits, and trust model are settled.
