# ani-cli-rs 0.5.3

`0.5.3` makes the Windows installation layout consistent with Linux while preserving existing installations during in-app updates.

There are no breaking changes to playback, downloads, history, library APIs, or JSON output.

## Windows installation path

- Changed the default Windows installation directory from `%LOCALAPPDATA%\Programs\ani-cli-rs\bin` to `$HOME\.local\bin`.
- Updated the Windows uninstaller to use the same default and respect `ANI_CLI_RS_INSTALL_DIR`.
- Kept in-app updates pinned to the directory containing the running executable. Installations made with an older version are therefore upgraded in place instead of leaving a stale executable earlier in `PATH`.
- Kept `ANI_CLI_RS_INSTALL_DIR` and the PowerShell `-InstallDirectory` parameter available for custom locations.

Users on an earlier release can install this patch with:

```console
ani-cli-rs update
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

Fresh Windows installations are placed in `$HOME\.local\bin`. The installer adds that directory to the user-level `PATH` when needed.

## Release asset checklist

Upload each archive together with its generated `.sha256` file:

- `ani-cli-rs-0.5.3-x86_64-pc-windows-msvc.zip`
- `ani-cli-rs-0.5.3-x86_64-pc-windows-msvc.zip.sha256`
- `ani-cli-rs-0.5.3-x86_64-unknown-linux-musl.tar.gz`
- `ani-cli-rs-0.5.3-x86_64-unknown-linux-musl.tar.gz.sha256`

Official macOS binaries are not published. macOS users may build from source with the included Cargo aliases.

## Verification

- 36 deterministic Rust tests pass.
- Windows installer, uninstaller, and packaging scripts pass parser validation.
- `cargo fmt --check` passes.
- `cargo check --all-targets` passes.
- `cargo clippy --all-targets -- -D warnings` passes.

**Full changelog:** https://github.com/vorlie/ani-cli-rs/compare/0.5.2...0.5.3
