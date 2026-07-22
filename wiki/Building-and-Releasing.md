# Building and Releasing

## Development prerequisites

- current stable Rust with edition 2024 support;
- Git;
- platform linker/toolchain;
- optional external players/downloaders for manual smoke tests.

```console
git clone https://github.com/vorlie/ani-cli-rs.git
cd ani-cli-rs
cargo build
```

## Required checks

```console
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
```

Normal tests are deterministic and must not require live AllAnime. Use fixtures and Wiremock for network behavior. Live smoke checks are manual because provider URLs and crypto rotate.

## Release builds and target directories

Cargo automatically separates explicit targets:

```console
cargo release-windows
cargo release-linux
cargo release-linux-arm64
cargo release-macos
cargo release-macos-arm64
```

The Linux ARM64 alias is provided for local/contributor builds only. The maintainer does not publish or device-test Linux ARM64 release assets.

Examples:

```text
target/x86_64-pc-windows-msvc/release/ani-cli-rs.exe
target/x86_64-unknown-linux-musl/release/ani-cli-rs
target/aarch64-unknown-linux-musl/release/ani-cli-rs
```

Install the target first:

```console
rustup target add x86_64-unknown-linux-musl
```

The corresponding linker must also be available.

## Windows privacy remapping

The Windows release workflow remaps the builder's user-profile source path to `/build`. This reduces accidental username/path disclosure in panic and source-location strings. It is not code obfuscation and does not encrypt the binary.

## Package release assets

Windows:

```powershell
.\scripts\package-release.ps1
```

Linux:

```sh
./scripts/package-release.sh
./scripts/package-release.sh x86_64-unknown-linux-musl
```

Packages are written below `dist/` and contain the executable, README, and GPL license. Packaging also generates the `.sha256` sidecar expected by installers.

Windows releases may additionally include:

```text
ani-cli-rs-VERSION-windows-x64-setup.exe
ani-cli-rs-VERSION-windows-x64-setup.exe.sha256
```

## Release checklist

1. Confirm the version in `Cargo.toml` and the root package entry in `Cargo.lock`.
2. Run every required check from a clean tree.
3. Build/package Windows x64 and Linux x86-64 musl. Do not list Linux ARM64 as an official asset without a separately tested release policy.
4. Test `--version`, `--help`, search, one playback resolution, one direct download, and one HLS path.
5. Verify each archive locally.
6. Create the matching Git tag/release.
7. Upload every archive/installer together with its `.sha256` file.
8. Test both install scripts against the published release.
9. Confirm the built-in updater resolves assets from the exact selected tag.

Do not publish a release containing only the binary: the install scripts require the correctly named archive and checksum.

## macOS policy

Cargo aliases support local Intel and Apple Silicon builds, but official macOS assets are not published because hosted macOS CI minutes are limited and more expensive. Contributors may document successful local builds without implying official release support.

## SemVer and commits

- Patch: compatible bug and provider fixes.
- Minor: additive user-facing workflows, such as interactive download preflight.
- Major: incompatible CLI or public library API changes.

Prefer scoped conventional commit messages and isolate a release version bump when practical.
