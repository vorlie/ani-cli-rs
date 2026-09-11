# Frequently Asked Questions

## Is this a fork of Bash ani-cli?

It is a standalone Rust port using Bash ani-cli as a compatibility baseline. The executable and repository are separate and use the `ani-cli-rs` name to avoid collisions.

## Are the commands the same?

Core compatibility flags and the interactive title workflow are intentionally familiar. ani-cli-rs also adds explicit scriptable subcommands and JSON output. Some platform-specific Bash features such as rofi/dmenu integration are intentionally excluded.

## Why does `download` reject an anime name?

`ani-cli-rs download SHOW_ID EPISODE` is the scriptable interface. Use:

```console
ani-cli-rs --download "anime name"
```

for interactive title/season/episode selection.

## Where do I find a show ID?

Run `ani-cli-rs search --json "title"` and use the returned provider-prefixed ID.

## Why is a season shown as another anime result?

Both catalogs commonly model seasons as separate show entries rather than one show containing a season list. Select the appropriately named result.

## Why does a download use `.part`?

It marks incomplete/resumable media. The file is renamed to `.mp4` after successful completion. See [Downloads](guides/downloads.md#why-files-use-part).

## Can I disable `.part`?

Not currently. Writing directly to the final name would make interrupted media appear complete and weaken safe resume/finalization behavior.

## Does my aria2 config work?

If present at `%USERPROFILE%\.aria2\aria2.conf` or `$HOME/.aria2/aria2.conf`, ani-cli-rs passes it explicitly. Safety-critical per-download arguments take precedence.

## Will a Linux binary built on Ubuntu work on Arch?

The official musl binary is intended to be portable across common distributions of the same CPU architecture. A normal glibc-linked build may depend on the builder's glibc version; use the musl release target for distribution.

## Why are there no macOS releases?

The project avoids consuming limited, higher-cost hosted macOS CI minutes. macOS source builds and automatic IINA playback are tested, but official assets are not promised.

## Is there an official Linux ARM64 build?

No. Linux ARM64 can be built from source, and a Cargo alias exists for contributor cross-builds, but the maintainer does not have ARM64 Linux hardware for device testing and does not publish an ARM64 release asset.

## Does Termux work?

Termux source builds are tested. ani-cli-rs requests mpv-android normally or Android VLC with `--vlc`. If the explicit activity bridge is unavailable, it uses Android's media handler through `termux-open`; the Android default or user selection then takes precedence over `--vlc`. Keep Termux running until relayed playback ends. A terminal VLC installed with `pkg install vlc` is not the Android application and is intentionally not selected.

## Why does antivirus flag the Windows executable?

It is currently unsigned and performs provider requests, local HLS relay traffic, downloads, and child-process launching. Those behaviors can trigger heuristics. Verify checksums, inspect the source, or build locally. A detection should still be evaluated rather than automatically dismissed.

## Why does one anime work while another fails?

Each episode can use different third-party hosts. Some copies are deleted, blocked, expired, or protected by changed provider protocols. ani-cli-rs cannot produce a stream when every upstream source is unavailable.

## When should I use `--ignore-host-lists`?

Use the `--ignore-host-lists` (or `-I`) flag when you encounter playback issues that might be caused by new or unrecognized streaming domains. This forces all HLS streams through the local relay regardless of the host domain, which can resolve issues with:

- New provider domains not yet in the HLS relay allowlist
- Temporary domain changes by streaming providers
- Provider switching to backup domains

```bash
ani-cli-rs --ignore-host-lists "anime title"
# or short form
ani-cli-rs -I "anime title"
```

You can also set this permanently via environment variable:

```bash
export ANI_CLI_IGNORE_HOST_LISTS=1
```

Note that forcing relay for all streams may slightly increase startup time compared to direct playback for known safe domains.

## Why does the player fail to start with "executable not found"?

ani-cli-rs now validates that the player executable exists before attempting to launch it. If mpv (or your configured player) is not installed or not in your system PATH, you'll see a clear error message with installation instructions.

Solutions:
- Install the player (mpv, VLC, etc.)
- Ensure the player directory is in your PATH
- Set the `ANI_CLI_PLAYER` environment variable to the full path
- Use the `--player` flag to specify the executable path directly

## Does `--allow-adult` bypass router filtering?

No. It only changes catalog filtering where the selected provider supplies adult metadata. DNS, FortiGuard, parental controls, antivirus, or ISP filtering still applies.

## Does ani-cli-rs require administrator/root access?

No. Official installers target per-user locations. External player/downloader installation may follow separate operating-system rules.

## Can Rust plugins be `.dll` or `.so` files?

The current roadmap prefers external executables with a versioned JSON-lines protocol. Rust has no stable ABI for safely loading arbitrary trait implementations across compiler versions.
