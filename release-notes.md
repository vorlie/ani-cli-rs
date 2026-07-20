# ani-cli-rs 0.5.0

`0.5.0` adds a built-in release checker and installer handoff while improving aria2 reliability for Mp4Upload downloads. Users can now check for updates or start the existing checksum-verifying installer directly from ani-cli-rs.

There are no intentional breaking changes to existing playback, download, history, library, or JSON interfaces.

## Highlights

### Built-in update checker

- Added Bash-compatible `-U/--update` to check for and install the latest release.
- Added `ani-cli-rs update` for the same scriptable workflow.
- Added `ani-cli-rs update --check` to report availability without changing the installation.
- Release versions are compared numerically, so versions such as `0.10.0` correctly sort after `0.9.0`.
- Installer scripts are downloaded from the exact GitHub release tag rather than the mutable `master` branch.
- Release tags are validated before being used to construct an installer URL.
- The existing installers continue to download the platform archive and verify its published SHA-256 checksum.

### Safe Windows replacement

- Windows updates are handed to PowerShell and continue after the running ani-cli-rs process exits, allowing the executable to be replaced safely.
- Temporary updater scripts remove themselves after a successful Windows installation.
- `ANI_CLI_RS_INSTALL_DIR` now overrides the installation directory on Windows as well as Unix.
- The Windows installer retains its explicit `-InstallDirectory` option.
- Updating does not require administrator privileges when using the default user-local installation directory.

Official macOS release binaries remain unavailable. `update --check` works on macOS, but installation directs users to rebuild from source.

### Mp4Upload download reliability

- Mp4Upload direct downloads now use four aria2 connections instead of sixteen.
- Other compatible direct providers retain the higher sixteen-connection limit.
- The lower Mp4Upload limit avoids repeated `403 Forbidden` responses from excess range requests while retaining parallel download performance.
- aria2 console logging is reduced and its final download-results table is hidden, while the live progress readout remains enabled.

## Usage

Check without installing:

```console
ani-cli-rs update --check
```

Install the latest release:

```console
ani-cli-rs update
```

The Bash-compatible form is also available:

```console
ani-cli-rs -U
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

The installers verify the downloaded release archive against its published SHA-256 checksum before installation.

## Release asset checklist

Upload each archive together with its generated `.sha256` file:

- `ani-cli-rs-0.5.0-x86_64-pc-windows-msvc.zip`
- `ani-cli-rs-0.5.0-x86_64-pc-windows-msvc.zip.sha256`
- `ani-cli-rs-0.5.0-x86_64-unknown-linux-musl.tar.gz`
- `ani-cli-rs-0.5.0-x86_64-unknown-linux-musl.tar.gz.sha256`

Official macOS binaries are not published. macOS users may build from source with the included Cargo aliases.

## Verification

- 35 deterministic Rust tests pass.
- `cargo fmt --check` passes.
- `cargo check --all-targets` passes.
- `cargo clippy --all-targets -- -D warnings` passes.
- The GitHub release check was smoke-tested against the live `vorlie/ani-cli-rs` repository.
- PowerShell installer syntax and the delayed Windows replacement path were validated.

**Full changelog:** https://github.com/vorlie/ani-cli-rs/compare/0.4.0...0.5.0
