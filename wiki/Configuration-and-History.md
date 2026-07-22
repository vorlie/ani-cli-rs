# Configuration and History

ani-cli-rs uses command-line options for one-off choices and environment variables for defaults. It does not currently maintain a general application configuration file.

## Environment variables

| Variable | Purpose |
|---|---|
| `ANI_CLI_MODE` | Default translation mode: `sub` or `dub` |
| `ANI_CLI_PLAYER` | Default mpv-compatible executable path |
| `ANI_CLI_DOWNLOAD_DIR` | Compatibility-mode download directory |
| `ANI_CLI_QUALITY` | Default quality (`best`, `worst`, or resolution) |
| `ANI_CLI_HIST_DIR` | Directory containing the `ani-hsts` history file |
| `ANI_CLI_ALLOW_ADULT` | Enable adult search results |
| `ANI_CLI_MULTI_SELECTION` | Open episode multi-select directly |
| `ANI_CLI_NO_DETACH` | Wait for the player |
| `ANI_CLI_EXIT_AFTER_PLAY` | Propagate attached player failures |
| `ANI_CLI_RS_INSTALL_DIR` | Installer/uninstaller target directory |
| `ANI_CLI_RS_PROFILE` | Unix profile modified by install/uninstall scripts |

Boolean variables recognize `1`, `true`, or `yes` where consumed as application booleans.

## PowerShell examples

Current process only:

```powershell
$env:ANI_CLI_QUALITY = "720p"
$env:ANI_CLI_DOWNLOAD_DIR = "$HOME\Downloads\Anime"
ani-cli-rs "title"
```

Persistent user variable:

```powershell
[Environment]::SetEnvironmentVariable("ANI_CLI_QUALITY", "720p", "User")
```

Open a new terminal after changing persistent variables.

## Unix shell examples

One command:

```sh
ANI_CLI_QUALITY=720p ani-cli-rs "title"
```

Persistent shell configuration:

```sh
export ANI_CLI_QUALITY=720p
export ANI_CLI_DOWNLOAD_DIR="$HOME/Downloads/Anime"
```

Place exports in the profile loaded by your shell.

## History format

The `ani-hsts` file uses the Bash ani-cli-compatible tab-separated format:

```text
EPISODE<TAB>SHOW_ID<TAB>TITLE
```

Example:

```text
4	SyR2K6bGYfKSE6YMm	Example Anime
```

One entry is retained per show ID. Successful playback/download updates the current episode through an atomic temporary-file rewrite.

## History location

For predictable sharing with Bash ani-cli, set:

```powershell
$env:ANI_CLI_HIST_DIR = "$HOME\.local\state\ani-cli"
```

```sh
export ANI_CLI_HIST_DIR="$HOME/.local/state/ani-cli"
```

ani-cli-rs appends `ani-hsts` to that directory. Without the variable, the `directories` crate chooses the platform-native application state directory, falling back to local application data when the OS has no dedicated state directory.

## Continue and clear

```console
ani-cli-rs --continue
ani-cli-rs --delete
```

`--continue` lists entries that have a published next episode. `--delete` rewrites the history as empty; it does not delete downloaded media.

## Scraper state

Crypto bootstrap material and the validated URL cipher map use a separate platform-native `ani-cli-rs` state directory. Dynamic crypto material is cached for a limited time because upstream values rotate. `debug --refresh` bypasses the normal cached bootstrap lookup.

## Logging

```powershell
$env:RUST_LOG = "warn"
ani-cli-rs "title"
```

```sh
RUST_LOG=debug ani-cli-rs "title"
```

Debug logs can expose provider names, response categories, and local executable paths. The application redacts complete signed provider URLs in routine scraper diagnostics, but review logs before sharing them publicly.
