# Provider Architecture

ani-cli-rs ships two independent catalog clients:

| CLI value | Catalog | Native source path | ID prefix |
|---|---|---|---|
| `anikoto` (default) | `anikotoapi.site` plus optional AniList enrichment | MegaPlay | `anikoto:` |
| `anikoto2` | `anikoto.cz` website AJAX workflow | MegaPlay/VidTube | `anikoto2:` |

AllAnime/Mkissa and its rotating cryptographic bootstrap are no longer part of
the application.

## Module boundaries

| Module | Responsibility |
|---|---|
| `providers/` (`anikoto`, `anikoto_cz`, `common`) | Anikoto API/AniList & Anikoto.cz catalog resolution, MegaPlay/VidTube extraction, and shared HTTP/HLS/cache helpers |
| `hls_relay` | Tokenized loopback playlist rewriting and PNG-wrapped TS removal |
| `models` | Provider IDs, search results, stream metadata, quality/episode ordering |
| `error/` (`mod`, `code`, `report`) | Structured `AniError` variants, `ACL-xxxx` error codes, and `ErrorReport` formatting |
| `player/` (`mod`, `android`) | Safe external-player arguments, Termux/Android intent launching, and relay lifetime |
| `download/` (`mod`, `progress`, `subtitles`) | Downloader fallback order, progress rendering, FFmpeg subtitle muxing, and relay lifetime |
| `history` | Legacy tab-separated history with self-routing provider IDs |
| `util/` | Internal HTTP header/subtitle/URL helpers and filename/env utilities |
| `bin_app/` | CLI-only `clap` definitions, `dialoguer` TUI prompts, workflow orchestration, updater, schedule, and logging |

## Routing

Searches use the explicitly selected provider or the `anikoto` default.
Subsequent commands inspect provider-prefixed IDs:

```text
anikoto:<base64url metadata>  -> Anikoto API client
anikoto2:<base64url metadata> -> Anikoto.cz client
```

This also makes mixed-provider history continuation deterministic. Raw numeric
IDs and raw slugs are intentionally interpreted only by the provider selected
with `--provider`.

Providers never silently fall back to each other. A failure remains attributed
to the catalog the user selected.

## Shared stream contract

Both clients return `StreamLink` values containing:

- validated native media URL;
- quality label and HLS/direct-media classification;
- provider/server label;
- downloadability;
- narrowly scoped referer, origin, and user-agent headers;
- optional validated subtitle tracks.

Quality ordering, player launch, downloads, and history updates operate on this
shared type and do not need provider-specific branches.

## KotoCDN relay

MegaPlay-associated HLS can contain MPEG-TS fragments hidden behind a PNG
prefix. `hls_relay` binds only to `127.0.0.1` on an ephemeral port, rewrites
registered playlist resources, validates every upstream URL, and removes a
prefix only when three MPEG-TS sync bytes appear 188 bytes apart.

The player/downloader remains attached for the relay lifetime. Dropping
`HlsRelay` signals its accept loop to stop.

This is not an internet-facing proxy and cannot fetch arbitrary caller URLs.

## Provider references

- [Anikoto API, MegaPlay, and KotoCDN](https://github.com/vorlie/ani-cli-rs/blob/master/docs/ANIKOTO-KOTOCDN.md)
- [Anikoto.cz workflow](https://github.com/vorlie/ani-cli-rs/blob/master/docs/ANIKOTO-CZ.md)

## Testing

Normal tests use parser fixtures and Wiremock. Live smoke tests are opt-in:

```sh
ANI_CLI_LIVE_ANIKOTO=1 cargo test live_anikoto_smoke_test_is_opt_in --lib
ANI_CLI_LIVE_ANIKOTO2=1 cargo test live_anikoto_cz_smoke_test_is_opt_in --lib
```

Never print complete resolved media URLs in test output or issue reports.
