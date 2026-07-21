# AllAnime scraping pipeline

This document describes the scraper implemented by `AllAnimeClient`. It is intended for contributors debugging an upstream change, not as a promise that every third-party source will remain available.

The implementation is split across these files:

- `src/client.rs`: HTTP workflow, GraphQL requests, bootstrap discovery, and provider resolution.
- `src/crypto.rs`: request authentication and episode-response decryption.
- `src/cipher.rs`: decoding obfuscated `sourceUrl` values and refreshing the byte map.
- `src/models.rs`: public result types and deterministic episode/source ordering.
- `tests/http.rs`: end-to-end scraper behavior against a mock HTTP server.

## 1. Search and episode discovery

Search and episode-list operations use ordinary GraphQL POST requests. Search supplies the selected `sub` or `dub` translation type and the `allowAdult` option. Episode discovery reads `availableEpisodesDetail`, accepts integer and fractional episode values, converts them to strings, and sorts them numerically.

These requests do not use the encrypted episode-source protocol described below.

## 2. Runtime crypto bootstrap

Episode source requests require metadata used by the current AllAnime/Mkissa frontend. The client fetches the configured bootstrap page, discovers its application JavaScript and chunks, and scans those assets for the current epoch, build ID, API endpoint, persisted-query hash material, and two key parts.

The key parts are decoded and combined through the XOR derivation in `src/crypto.rs`. Successfully discovered material is cached for 30 minutes. If bootstrap discovery fails, the client uses bundled current and legacy material so a temporary frontend/CDN failure does not immediately disable playback.

Dynamic material is tried with its advertised epoch and the adjacent epochs. This accounts for short rollout windows where the frontend CDN and GraphQL edge disagree about the active epoch.

## 3. Authenticated episode request

For every material candidate, the client:

1. Builds the episode variables from show ID, translation type, and episode string.
2. Places the current timestamp into a five-minute bucket.
3. Creates `aaReq` with AES-256-GCM using the candidate key and query hash.
4. Attempts the persisted GraphQL GET request with the candidate build ID.
5. Falls back to a full GraphQL POST using the hash of the complete episode query.

Rate-limit responses are recognized separately from crypto failures. The request is retried with the server-indicated delay, bounded to avoid an indefinite wait. A persistent rate limit is returned as `AniError::RateLimited` rather than being mislabeled as an unavailable episode.

## 4. Episode response decoding

`decode_episode_response` first accepts ordinary JSON responses. Encrypted `tobeparsed` responses are decoded using the current material-key GCM format, the fallback-secret GCM format, or the legacy CTR format when that material explicitly allows it.

The result is normalized until the `sourceUrls` value is found. AllAnime has returned that list at several nesting levels and has sometimes encoded the list itself as a JSON string, so callers must not assume one fixed response envelope.

## 5. Source URL decoding

Each source entry contains a provider name and `sourceUrl`. Values beginning with `--` are decoded using the byte-substitution map in `src/cipher.rs`.

The project includes a bundled map for offline resilience. `refresh-cipher-map` downloads the latest released Bash ani-cli script, validates its substitution table, and caches the result in the application state directory. Refreshing the map is useful when episode requests decrypt correctly but every decoded provider path is malformed.

## 6. Provider resolution

Sources are resolved concurrently and partial provider failures are allowed. The current resolver supports:

- Direct HTTP(S) MP4, HLS, `videoplayback`, fast4speed, and Wix media URLs.
- Protocol-relative URLs by normalizing them to HTTPS.
- Mp4Upload pages by extracting their player media URL and retaining the Mp4Upload referrer.
- OK.ru embeds by decoding the page's HTML-escaped `data-options`, reading inline
  `flashvars.metadata`, or posting `st.location` to `flashvars.metadataUrl` when
  OK.ru supplies metadata out of band.
- Filemoon aliases through the current details, challenge, P-256 attestation, and
  encrypted playback API flow. The playback envelope is decrypted in-process
  with its supplied AES-256-GCM key parts.
- Known embed pages with a directly exposed media URL or a simple iframe/redirect.
- Relative or same-host absolute `/apivtwo/` and `/apiv2/` clock endpoints.
- Legacy clock responses containing `links[].link`, `resolutionStr`, and `hls`.
- Newer nested clock HLS objects containing `url`, a HLS marker, and `hardsub_lang`; non-English hard-sub variants are ignored when the language is declared.
- Clock-provided `Referer` values, which are carried into every resulting `StreamLink`.
- Wix repackager URL sets, expanded into their advertised MP4 qualities.
- HLS master playlists, with relative variant paths resolved against the master URL.

Resolved links are deduplicated and then sorted by provider and resolution. A provider failure does not discard links returned by another provider. If nothing resolves, the error reports the number of extracted entries and sanitized provider-level failure categories without printing replayable signed media URLs.

## 7. Headers and playback

`StreamLink` stores request metadata as typed `referer`, `origin`, and extra headers. Players and downloaders receive these as individual process arguments. Do not concatenate them into shell command strings.

Provider headers can be required for both playlist expansion and final playback. When adding a provider, test the initial page/JSON request, the HLS master request, and the final media request separately.

The API/bootstrap referrer and media-provider referrer are intentionally separate. Crypto and GraphQL requests follow the active Mkissa frontend, while provider clock/media requests default to the current upstream ani-cli referrer (`https://youtu-chan.com/`) unless the clock response supplies a more specific `Referer`.

## Diagnosing a live failure

Start with the scriptable commands:

```console
ani-cli-rs search --json "title"
ani-cli-rs episodes --json SHOW_ID --mode sub
ani-cli-rs links --json SHOW_ID 1 --quality best
ani-cli-rs debug --refresh
ani-cli-rs refresh-cipher-map
```

Set `RUST_LOG=warn` or `RUST_LOG=debug` for provider categories and bootstrap diagnostics. Avoid posting full signed media URLs, cookies, authorization values, crypto debug secrets, or personal paths in issues.

A useful failure classification is:

- Search or episode list fails: ordinary GraphQL connectivity/schema problem.
- `AA_CRYPTO_*` or decryption failure: bootstrap, key, epoch, build, or response-format problem.
- Source entries decode to nonsense: cipher-map problem.
- Clock endpoint fails: provider proxy/network problem.
- Clock succeeds but yields no links: unsupported clock response shape.
- Links resolve but playback returns 403: missing or incorrect provider referrer/origin.

## Adding fixtures

Normal tests must never depend on live AllAnime. Add a minimal sanitized fixture or a Wiremock response that contains only the fields needed to reproduce the shape. Tests should cover:

- the new response shape;
- required headers;
- relative URL handling;
- partial provider failure;
- deduplication and quality ordering;
- errors that do not expose temporary media credentials.

Use live smoke tests only for manual verification after deterministic tests pass.
