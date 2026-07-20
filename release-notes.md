# ani-cli-rs 0.3.0

`0.3.0` improves interactive session continuity and closes several compatibility gaps with Bash ani-cli. Playback controls now remain available after explicitly selecting an episode, the episode picker can select multiple episodes, legacy options work after search terms, and `-N/--nextep-countdown` provides the upstream release-schedule lookup.

There are no intentional breaking changes to existing flags, environment variables, history files, library APIs, or JSON subcommands.

## Highlights

### Persistent playback controls

- The playback menu now remains open after a single episode selected with `-e/--episode` finishes.
- Users can continue to the next episode, replay, go to the previous episode, select another episode, or change quality without restarting ani-cli-rs.
- Next and previous actions are only shown when the corresponding episode exists.
- The current quality is displayed in the action menu.
- `--exit-after-play` continues to skip the post-play menu for scripts and other non-interactive workflows.

### Multi-episode selection

- Added a visible **Select multiple episodes** entry to the interactive episode picker.
- Space toggles episodes, Enter confirms the selection, and Escape returns to the previous menu.
- Added `--multi-selection` to open the multi-select picker directly.
- Added Bash-compatible `ANI_CLI_MULTI_SELECTION` configuration.
- Explicit `-e/--episode` values and ranges retain their existing non-interactive behavior.

### Bash-style argument ordering

- Options may now appear before, after, or between search terms.
- Commands such as `ani-cli-rs frieren -q 1080p -e 2` now behave like their Bash ani-cli equivalents.
- Multi-word queries remain intact when flags are interspersed.

### Next-release schedule lookup

- Added `-N/--nextep-countdown` compatibility using AnimeSchedule.
- Displays English and Japanese titles, upcoming raw and subtitled release timestamps, and the current series status.
- Schedule mode exits without contacting AllAnime or launching a player.
- When used interactively without a query, ani-cli-rs prompts for an anime title; non-interactive use requires a query.

Despite the upstream option name, this command reports the next scheduled episode release. It does not automatically start the next locally available episode.

### Plugin architecture roadmap

- Added `PLUGIN-ROADMAP.md`, proposing external executable provider plugins instead of unstable in-process Rust dynamic libraries.
- The roadmap covers discovery, a versioned JSON-lines protocol, process isolation, provider integration, security, testing, and possible AniPlay interoperability.
- This release does not load or execute plugins; the document describes future work only.

## Examples

```console
# Flags can follow the query
ani-cli-rs frieren -q 1080p -e 2

# Open the multi-episode picker directly
ani-cli-rs --multi-selection "cowboy bebop"

# Display upcoming release information
ani-cli-rs -N "one piece"
```

Multi-selection can also be enabled persistently:

```sh
export ANI_CLI_MULTI_SELECTION=true
```

```powershell
$env:ANI_CLI_MULTI_SELECTION = "true"
```

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

The installers verify the release archive against its published SHA-256 checksum before installing it into a user-local directory and adding that directory to `PATH` when necessary.

Official macOS binaries are not published. macOS users may build from source with the included Cargo aliases.

## Verification

- 28 deterministic Rust tests pass.
- `cargo fmt --check` passes.
- `cargo check --all-targets` passes.
- `cargo clippy --all-targets -- -D warnings` passes.
- The AnimeSchedule integration was smoke-tested against its live API.

**Full changelog:** https://github.com/vorlie/ani-cli-rs/compare/0.2.0...0.3.0
