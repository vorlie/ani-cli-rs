# Installation

`ani-cli-rs` can be installed directly via Cargo from [crates.io](https://crates.io/crates/ani-cli-rs), downloaded as prebuilt binaries from [GitHub Releases](https://github.com/vorlie/ani-cli-rs/releases), or built from source.

Prebuilt assets are provided for Windows x64 and Linux x86-64. Linux ARM64, tested macOS, and tested Termux users build from source or install via Cargo.

---

## Cargo (crates.io)

### As a CLI Binary
To globally install the executable:

```sh
cargo install ani-cli-rs

```

> **Note:** Ensure Cargo's binary directory (`~/.cargo/bin` or `%USERPROFILE%\.cargo\bin`) is in your system's `PATH`.

### As a Library

To add `ani-cli-rs` to your own Rust project:

```sh
cargo add ani-cli-rs

```

Or add it to your `Cargo.toml`:

```toml
[dependencies]
ani-cli-rs = "0.9.6"

```

---

## Windows

### Portable installation

Open PowerShell and run:

```powershell
Invoke-WebRequest [https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/install.ps1](https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/install.ps1) -OutFile install.ps1
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

### Inno Setup wizard

If the release contains the setup asset:

```powershell
.\install.ps1 -UseSetup

```

This launches a per-user installer with an uninstall entry. It does not require administrator privileges. The portable and Setup installations use different installation management, so use the matching uninstall method.

---

## Linux

```sh
curl -fsSL [https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/install.sh](https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/install.sh) -o install.sh
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
git clone [https://github.com/vorlie/ani-cli-rs.git](https://github.com/vorlie/ani-cli-rs.git)
cd ani-cli-rs
cargo build --release --locked

```

The local `cargo release-linux-arm64` alias is intended for contributors with a suitable cross-linker or ARM64 build environment; its existence does not indicate an official binary.

---

## macOS

No official macOS release assets are provided. The source build and automatic IINA selection have been tested. Install the Rust toolchain and build locally:

```sh
git clone [https://github.com/vorlie/ani-cli-rs.git](https://github.com/vorlie/ani-cli-rs.git)
cd ani-cli-rs
cargo build --release --locked

```

The binary is placed at `target/release/ani-cli-rs`. Copy it to a directory on your `PATH` if desired.

IINA is the default player on macOS. Install it with `brew install --cask iina` and ensure its `iina` command is on `PATH`.

---

## Android with Termux

Termux source builds are tested, but no Android release binary is published. Install a current Termux build from a trusted source, then run:

```sh
pkg update
pkg install rust
git clone [https://github.com/vorlie/ani-cli-rs.git](https://github.com/vorlie/ani-cli-rs.git)
cd ani-cli-rs
cargo build --release --locked
install -Dm755 target/release/ani-cli-rs "$PREFIX/bin/ani-cli-rs"

```

Install an Android HLS-capable video player such as mpv-android or VLC. Do not use `pkg install vlc` for this integration: that installs a terminal VLC build rather than the Android application.

Run `ani-cli-rs "title"` to request mpv-android or `ani-cli-rs --vlc "title"` to request Android VLC. ani-cli-rs searches for `termux-am-starter`, `termux-am`, and `am`, in that order. If the explicit activity bridge is incompatible with the installed Termux app, it falls back to `termux-open` with the stream's media type, then `termux-open-url`. On this fallback path Android's chosen/default media handler takes precedence, so `--vlc` cannot guarantee VLC. `ANI_CLI_PLAYER` can override the activity-launcher executable if a device requires a different path.

---

## Build from source on any supported platform

Install a current stable Rust toolchain, then:

```console
git clone [https://github.com/vorlie/ani-cli-rs.git](https://github.com/vorlie/ani-cli-rs.git)
cd ani-cli-rs
cargo build --release --locked

```

Windows output: `target\release\ani-cli-rs.exe`

Unix output: `target/release/ani-cli-rs`

Rust edition 2024 requires a recent toolchain. If Cargo says the `edition2024` feature is unavailable, update through rustup and verify that the rustup binaries appear before a distribution-installed `/usr/bin/cargo` in `PATH`.

---

## Uninstall

### Cargo

```sh
cargo uninstall ani-cli-rs

```

### Portable Windows

```powershell
Invoke-WebRequest [https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/uninstall.ps1](https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/uninstall.ps1) -OutFile uninstall.ps1
.\uninstall.ps1
Remove-Item .\uninstall.ps1

```

### Linux

```sh
curl -fsSL [https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/uninstall.sh](https://raw.githubusercontent.com/vorlie/ani-cli-rs/master/scripts/uninstall.sh) -o uninstall.sh
sh uninstall.sh
rm uninstall.sh

```

For an Inno Setup installation, use Windows Installed Apps or the generated uninstaller.

---

## Verify the installation

```console
ani-cli-rs --version
ani-cli-rs --help

```

If the command is not found, see [Troubleshooting: installed but not found](https://www.google.com/search?q=../support/troubleshooting.md%23installed-but-command-not-found).

