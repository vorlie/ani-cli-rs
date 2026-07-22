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

Run `ani-cli-rs search --json "title"`, or copy the segment following `/anime/` in a Mkissa URL.

## Why is a season shown as another anime result?

AllAnime commonly models seasons as separate show entries rather than one show containing a season list. Select the appropriately named result.

## Why does a download use `.part`?

It marks incomplete/resumable media. The file is renamed to `.mp4` after successful completion. See [Downloads](Downloads#why-files-use-part).

## Can I disable `.part`?

Not currently. Writing directly to the final name would make interrupted media appear complete and weaken safe resume/finalization behavior.

## Does my aria2 config work?

If present at `%USERPROFILE%\.aria2\aria2.conf` or `$HOME/.aria2/aria2.conf`, ani-cli-rs passes it explicitly. Safety-critical per-download arguments take precedence.

## Will a Linux binary built on Ubuntu work on Arch?

The official musl binary is intended to be portable across common distributions of the same CPU architecture. A normal glibc-linked build may depend on the builder's glibc version; use the musl release target for distribution.

## Why are there no macOS releases?

The project avoids consuming limited, higher-cost hosted macOS CI minutes. macOS remains source-buildable, but official assets are not promised.

## Is there an official Linux ARM64 build?

No. Linux ARM64 can be built from source, and a Cargo alias exists for contributor cross-builds, but the maintainer does not have ARM64 Linux hardware for device testing and does not publish an ARM64 release asset.

## Why does antivirus flag the Windows executable?

It is currently unsigned and performs cryptography, dynamic frontend inspection, downloads, and child-process launching. Those behaviors can trigger heuristics. Verify checksums, inspect the source, or build locally. A detection should still be evaluated rather than automatically dismissed.

## Why does one anime work while another fails?

Each episode can use different third-party hosts. Some copies are deleted, blocked, expired, or protected by changed provider protocols. ani-cli-rs cannot produce a stream when every upstream source is unavailable.

## What are `AA_CRYPTO_STALE` and `AA_CRYPTO_CROSS_KEY`?

They indicate disagreement between the rotating frontend crypto material and the episode API. Retry after a short delay, use `debug --refresh`, and update to the latest release if the issue persists.

## Does `--allow-adult` bypass router filtering?

No. It only changes the AllAnime search variable. DNS, FortiGuard, parental controls, antivirus, or ISP filtering still applies.

## Does ani-cli-rs require administrator/root access?

No. Official installers target per-user locations. External player/downloader installation may follow separate operating-system rules.

## Can Rust plugins be `.dll` or `.so` files?

The current roadmap prefers external executables with a versioned JSON-lines protocol. Rust has no stable ABI for safely loading arbitrary trait implementations across compiler versions.
