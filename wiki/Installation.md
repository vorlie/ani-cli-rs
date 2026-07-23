# Installation

Official releases are published at <https://github.com/vorlie/ani-cli-rs/releases>. Prebuilt assets are provided for Windows x64 and Linux x86-64. Linux ARM64 and macOS users build from source.

## Windows: portable installation

Open PowerShell and run:

```powershell
Invoke-WebRequest https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/install.ps1 -OutFile install.ps1
.\install.ps1
Remove-Item .\install.ps1
```

The installer:

1. reads the latest GitHub release;
2. downloads the x64 Windows ZIP and its `.sha256` file;
3. verifies SHA-256 using .NET cryptography, including on older PowerShell versions without `Get-FileHash`;
4. installs `ani-cli-rs.exe` to `%USERPROFILE%\.local\bin` by default;
5. adds that directory to the user `PATH` unless `-NoPathUpdate` is supplied.

Open a new terminal after installation so it receives the updated user `PATH`.

Custom directory:

```powershell
.\install.ps1 -InstallDirectory "D:\Tools\bin"
```

Skip the `PATH` update:

```powershell
.\install.ps1 -NoPathUpdate
```

## Windows: Inno Setup wizard

If the release contains the setup asset:

```powershell
.\install.ps1 -UseSetup
```

This launches a per-user installer with an uninstall entry. It does not require administrator privileges. The portable and Setup installations use different installation management, so use the matching uninstall method.

## Linux

```sh
curl -fsSL https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/install.sh -o install.sh
sh install.sh
rm install.sh
```

The script installs the official Linux x86-64 archive, verifies its published SHA-256 checksum, installs to `$HOME/.local/bin` by default, and adds that directory to `$HOME/.profile` when needed. On Linux ARM64 it exits with source-build instructions because no official ARM64 asset is published.

Custom locations:

```sh
ANI_CLI_RS_INSTALL_DIR="$HOME/bin" ANI_CLI_RS_PROFILE="$HOME/.bashrc" sh install.sh
```

Official Linux archives use musl so one release can work across common distributions such as Ubuntu, Debian, Arch, Fedora, and Alpine. Kernel and CPU architecture compatibility still apply.

### Linux ARM64

Linux ARM64 is source-buildable but is not device-tested by the maintainer and has no official release archive:

```sh
git clone https://github.com/vorlie/ani-cli-rs.git
cd ani-cli-rs
cargo build --release --locked
```

The local `cargo release-linux-arm64` alias is intended for contributors with a suitable cross-linker or ARM64 build environment; its existence does not indicate an official binary.

## macOS

No official macOS release assets are provided. Install the Rust toolchain and build locally:

```sh
git clone https://github.com/vorlie/ani-cli-rs.git
cd ani-cli-rs
cargo build --release --locked
```

The binary is placed at `target/release/ani-cli-rs`. Copy it to a directory on your `PATH` if desired.

IINA is the default player on macOS. Install it with `brew install --cask iina` and ensure its `iina` command is on `PATH`.

## Build from source on any supported desktop

Install a current stable Rust toolchain, then:

```console
git clone https://github.com/vorlie/ani-cli-rs.git
cd ani-cli-rs
cargo build --release --locked
```

Windows output: `target\release\ani-cli-rs.exe`  
Unix output: `target/release/ani-cli-rs`

Rust edition 2024 requires a recent toolchain. If Cargo says the `edition2024` feature is unavailable, update through rustup and verify that the rustup binaries appear before a distribution-installed `/usr/bin/cargo` in `PATH`.

## Uninstall

Portable Windows:

```powershell
Invoke-WebRequest https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/uninstall.ps1 -OutFile uninstall.ps1
.\uninstall.ps1
Remove-Item .\uninstall.ps1
```

Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/uninstall.sh -o uninstall.sh
sh uninstall.sh
rm uninstall.sh
```

For an Inno Setup installation, use Windows Installed Apps or the generated uninstaller.

## Verify the installation

```console
ani-cli-rs --version
ani-cli-rs --help
```

If the command is not found, see [Troubleshooting: installed but not found](Troubleshooting#installed-but-command-not-found).
