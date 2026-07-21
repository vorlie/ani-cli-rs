# ani-cli-rs 0.5.2

`0.5.2` fixes Windows installation and in-app updates on PowerShell environments where `Get-FileHash` is unavailable.

There are no breaking changes to playback, downloads, history, library APIs, or JSON output.

## Windows installer compatibility

- Replaced the installer's dependency on `Get-FileHash` with an in-process .NET SHA-256 implementation.
- Replaced its dependency on `Expand-Archive` with .NET ZIP extraction so the installer does not fail at the next step on older PowerShell installations.
- Applied the SHA-256 compatibility fix to Windows release packaging.
- Updated the manual Windows verification example to use the built-in `certutil` utility.

Users on `0.5.1` can install this patch with:

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

The installers verify the downloaded release archive against its published SHA-256 checksum before installation.

## Release asset checklist

Upload each archive together with its generated `.sha256` file:

- `ani-cli-rs-0.5.2-x86_64-pc-windows-msvc.zip`
- `ani-cli-rs-0.5.2-x86_64-pc-windows-msvc.zip.sha256`
- `ani-cli-rs-0.5.2-x86_64-unknown-linux-musl.tar.gz`
- `ani-cli-rs-0.5.2-x86_64-unknown-linux-musl.tar.gz.sha256`

Official macOS binaries are not published. macOS users may build from source with the included Cargo aliases.

## Verification

- 36 deterministic Rust tests pass.
- Both PowerShell scripts pass parser validation.
- The .NET SHA-256 fallback was verified against PowerShell's reference digest.
- The .NET ZIP extraction API was verified as available.
- `cargo fmt --check` passes.
- `cargo check --all-targets` passes.
- `cargo clippy --all-targets -- -D warnings` passes.

**Full changelog:** https://github.com/vorlie/ani-cli-rs/compare/0.5.1...0.5.2
