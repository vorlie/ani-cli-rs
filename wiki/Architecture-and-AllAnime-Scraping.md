# Architecture and AllAnime Scraping

This page is for contributors diagnosing scraper changes. It summarizes the pipeline; the repository's [`docs/ALLANIME-SCRAPING.md`](https://github.com/vorlie/ani-cli-rs/blob/master/docs/ALLANIME-SCRAPING.md) remains the detailed source-adjacent reference.

## Components

| Area | Responsibility |
|---|---|
| `client` | HTTP workflow, GraphQL, bootstrap discovery, provider resolution |
| `crypto` | `aaReq`, key derivation, GCM/CTR episode decoding |
| `cipher` | `--` source URL substitution map and refresh/cache |
| `models` | Public search/stream/header types, ordering and quality selection |
| `player` | Safe process arguments for mpv, VLC, and Syncplay |
| `download` | aria2/yt-dlp/FFmpeg/Rust transfer strategy |
| `history` | Bash-compatible `ani-hsts` persistence |

## Search and episode discovery

Search and episode lists use ordinary GraphQL requests. Search variables include translation type and `allowAdult`. Episode values may be integers or fractional labels and are sorted numerically.

Search results are show entries, not a nested season model. Consequently, interactive “season selection” means choosing the matching AllAnime search entry.

## Runtime bootstrap

Episode source requests require rotating frontend material. The client fetches the Mkissa bootstrap page, discovers application JavaScript/chunks, and scans them for:

- epoch and build ID;
- API URL and persisted-query material;
- two key components;
- current query hashes.

Key components are decoded and XOR-derived. Dynamic material is cached for 30 minutes. Bundled current and legacy material provides temporary resilience when frontend discovery fails.

## Authenticated episode requests

For each candidate material set, the client:

1. builds episode variables;
2. buckets the timestamp into five-minute intervals;
3. constructs AES-256-GCM `aaReq` authentication;
4. tries persisted GraphQL with dynamic/current/legacy builds;
5. falls back to the full GraphQL POST query.

Short adjacent epoch attempts handle frontend/API rollout skew. Rate-limit errors are recognized and retried with bounded delays rather than mislabeled as missing episodes.

## Response decoding

The response can be plaintext JSON or contain encrypted `tobeparsed` data. Supported decoding paths include:

- material-key AES-GCM;
- fallback-secret AES-GCM;
- legacy AES-CTR when the selected material permits it.

The source list may appear at different nesting levels or itself be JSON encoded as a string. Normalization must not assume one fixed envelope.

## Source URL cipher map

Source URLs beginning with `--` use a byte-substitution map. A bundled map provides offline resilience. `refresh-cipher-map` downloads the newest released Bash ani-cli script, validates its table, and caches it.

If episode decryption succeeds but every decoded URL is malformed, the map is a likely failure point.

## Provider resolution

Providers are resolved concurrently and partial failure is expected. Current handling includes:

- direct MP4/HLS/videoplayback/fast4speed/Wix URLs;
- protocol-relative URLs;
- relative and same-host `/apivtwo/` and `/apiv2/` clocks;
- legacy and nested English HLS clock response shapes;
- clock-provided referers;
- Wix quality expansion;
- HLS master-playlist variants;
- Mp4Upload embeds;
- OK.ru inline or remote metadata;
- Filemoon details/challenge/P-256 attestation/AES-GCM playback;
- simple iframe/redirect embeds exposing direct media.

Links are deduplicated and ranked by provider and resolution. Provider errors must never discard valid streams already returned by another source.

## Request headers

`StreamLink` carries typed referrer, origin, and extra headers. Provider headers must survive:

1. provider page/clock request;
2. master playlist expansion;
3. final playback or download.

Do not concatenate shell commands. Every external program argument is passed separately.

The Mkissa API/bootstrap referrer and media-provider referrer are intentionally distinct. Current default media referrer compatibility follows `https://youtu-chan.com/` unless a provider response supplies a more specific value.

## Safe diagnostics

Useful commands:

```console
ani-cli-rs search --json "title"
ani-cli-rs episodes --json SHOW_ID --mode sub
ani-cli-rs links --json SHOW_ID 1
ani-cli-rs debug --refresh
ani-cli-rs refresh-cipher-map
```

Log structural facts—host, redacted path family, response size, provider category—not replayable URLs, cookies, tokens, or personal paths.

## Testing upstream changes

- Add the smallest sanitized fixture reproducing the response shape.
- Use Wiremock for headers, fallback order, malformed data, and partial provider failure.
- Keep normal `cargo test` offline and deterministic.
- Test both successful and failing providers in one episode response.
- Verify final `StreamLink` headers, ordering, and deduplication.
- Perform live smoke tests only after deterministic checks pass.

An episode with all sources deleted is not a useful success fixture. Confirm at least one source is currently playable before concluding that a new parser works.
