# ani-cli-rs 0.7.0

This release adds Anikoto as an explicit second catalog while keeping AllAnime as the default for Bash ani-cli compatibility.

## Highlights

- Select Anikoto with `--provider anikoto`, `-p anikoto`, or `ANI_CLI_PROVIDER=anikoto`.
- Search the Anikoto recent catalog and AniList concurrently, merge stable identifiers, and retain adult filtering.
- Resolve native MegaPlay sources through explicit episode, embed-ID, AniList, and MAL candidates.
- Play or download MegaPlay HLS delivered by KotoCDN, including confirmed PNG-wrapped MPEG-TS segments, through a tokenized loopback relay.
- Preserve provider headers and subtitle metadata for mpv, VLC, Syncplay, yt-dlp, aria2, and FFmpeg workflows.
- Store Anikoto history in the existing tab-separated format through self-identifying `anikoto:` IDs. Mixed AllAnime/Anikoto continuation routes each entry independently.
- Return provider-aware catalog, mapping, rate-limit, extraction, and unavailable-source diagnostics.

AllAnime diagnostics remain available through `debug` and `refresh-cipher-map`; these commands intentionally reject an Anikoto selection. Searches are not combined, and playback never silently switches catalogs.

See [`docs/ANIKOTO-KOTOCDN.md`](docs/ANIKOTO-KOTOCDN.md) for the complete contributor workflow.

**Full changelog:** https://github.com/vorlie/ani-cli-rs/compare/0.6.0...0.7.0

---

# Previous release notes: ani-cli-rs 0.6.0

This release significantly improves the interactive download workflow. You can now search by anime title, select the correct anime or season, choose episodes, and verify the complete batch before downloading begins.

## What’s new

### Interactive anime and season selection

Run:

```console
ani-cli-rs --download "anime title"
```

Search results now clearly act as the anime/season picker, matching how AllAnime represents separate seasons.

After selecting a result, choose one or multiple episodes to download.

### Download preflight

Before creating any files, ani-cli-rs now resolves every selected episode sequentially and:

- Confirms that downloadable streams are available.
- Applies the requested `-q/--quality` setting to each episode.
- Filters out streams that cannot be downloaded.
- Caches resolved streams for the subsequent transfers.
- Preserves the selected episode order.

Resolving episodes sequentially also helps avoid unnecessary AllAnime rate limiting.

If an episode is unavailable:

- Interactive sessions show every unavailable episode and return to the episode picker.
- Back returns to the anime/season results.
- Explicit `-e/--episode`, `--range`, continuation, and non-interactive requests fail without starting a partial batch.
- No download files or history entries are created during a failed preflight.

Network, rate-limit, crypto, and malformed-response errors remain fatal and are reported directly.

### Scriptable downloads remain compatible

The scriptable interface is unchanged:

```console
ani-cli-rs download SHOW_ID EPISODE
```

It still requires an AllAnime/Mkissa show ID rather than an anime name. Use the legacy-compatible interface when you want interactive title and season selection:

```console
ani-cli-rs --download "anime title"
```

## Documentation

This release introduces a comprehensive project Wiki covering:

- Installation and PATH configuration
- CLI commands and environment variables
- Playback, players, and keyboard controls
- Downloads, aria2, yt-dlp, FFmpeg, and `.part` files
- Configuration and history compatibility
- Troubleshooting provider and network failures
- Security, privacy, antivirus detections, and checksums
- Building, packaging, and releasing
- AllAnime scraper architecture
- Contributor guidance and frequently asked questions

See the [ani-cli-rs Wiki](https://github.com/vorlie/ani-cli-rs/wiki).

The repository README has also been streamlined into a more concise project overview with direct links to the detailed Wiki pages.

## Platform notes

Official prebuilt releases are currently provided for:

- Windows x64
- Linux x86-64

Linux ARM64 and macOS remain source-buildable, but no maintainer-built or device-tested binaries are published for them.

The Linux installer now reports unsupported ARM64 systems clearly instead of attempting to download an unavailable release asset.

## Upgrade

Use the built-in updater:

```console
ani-cli-rs update
```

Or reinstall using the platform installation script. Release archives are accompanied by SHA-256 checksum files, which the installers verify automatically.

**Full changelog:** https://github.com/vorlie/ani-cli-rs/compare/0.5.4...0.6.0