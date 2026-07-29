# Anikoto, MegaPlay, and KotoCDN Resolution

Contributor notes for the native Anikoto implementation in ani-cli-rs 0.7.0, with comparisons to AniPlay 1.16.2.

Last verified: 2026-07-22

## Purpose

This document explains how an Anikoto catalog result becomes a native MegaPlay stream and how KotoCDN's disguised HLS segments are made usable by ordinary desktop players and download tools.

ani-cli-rs is native-only: it does not expose MegaPlay iframe URLs as playable CLI streams. Candidate embeds are used only to obtain the numeric player ID needed by `stream/getSources`. If all native candidates fail, the command returns a provider-aware unavailable-source error.

It also describes why the native stream is needed for Watch Together and which parts of the workflow can be reused when adding another provider.

Although this document follows AniPlay's implementation, it is also intended as a reference for other projects integrating Anikoto, MegaPlay, KotoCDN, or a similar catalog-to-embed-to-native-media workflow. The provider layering, identifier strategy, fallback model, header handling, diagnostics, security guidance, and integration checklist are project-agnostic. AniPlay-specific Electron and Watch Together code should be treated as worked examples and adapted to the target application's runtime and player architecture.

The ani-cli-rs implementation lives primarily in:

- `src/anikoto.rs`
- `src/hls_relay.rs`
- `src/main.rs`
- `src/player.rs`
- `src/download.rs`

Later sections also discuss the related AniPlay implementation as a worked Electron/Watch Together reference. Those paths are:

- `ani-cli-gui/electron/providers/anikoto.ts`
- `ani-cli-gui/electron/media-headers.ts`
- `ani-cli-gui/electron/main.ts`
- `ani-cli-gui/src/lib/watch-together-content.ts`
- `ani-cli-gui/src/pages/PlayerPage.tsx`

## Important terminology

These services have different responsibilities:

| Layer | Current service | Purpose |
| --- | --- | --- |
| Metadata/catalog | Anikoto API | Recent titles, series metadata, episode numbers, embed IDs, and embed URLs |
| Player/embed | MegaPlay | Hosts the iframe player and exposes player source metadata |
| Media delivery | KotoCDN and other MegaPlay-associated hosts | Serves HLS manifests, segments, and subtitles |
| Coordination | AniPlay Watch Together Worker | Shares stable content identifiers and playback state, but never media URLs |

KotoCDN should not be treated as an anime catalog provider. It is one possible delivery host returned by MegaPlay. Delivery domains can change independently of Anikoto and MegaPlay.

## Upstream documentation

- Anikoto API: <https://anikotoapi.site/>
- MegaPlay API and embed documentation: <https://megaplay.buzz/api>

The documented Anikoto endpoints are:

```text
GET https://anikotoapi.site/recent-anime?page=PAGE&per_page=COUNT
GET https://anikotoapi.site/series/SERIES_ID
```

The documented MegaPlay embed forms are:

```text
https://megaplay.buzz/stream/s-2/EPISODE_EMBED_ID/sub
https://megaplay.buzz/stream/s-2/EPISODE_EMBED_ID/dub
https://megaplay.buzz/stream/ani/ANILIST_ID/EPISODE/sub
https://megaplay.buzz/stream/mal/MAL_ID/EPISODE/sub
```

MegaPlay documents iframe playback and outbound `postMessage` progress events. The native `stream/getSources` request described below is an observed implementation detail, not a stable public API guarantee. Treat it as replaceable provider-specific code.

## Complete resolution flow

```text
Search term
    |
    +-- Anikoto /recent-anime
    |
    +-- AniList search metadata
            |
            v
Encoded AniPlay Anikoto ID
            |
            +-- Anikoto /series/{id}
            |       |
            |       +-- episode number
            |       +-- episode_embed_id
            |       +-- embed_url.sub / embed_url.dub
            |
            v
Ordered MegaPlay embed candidates
            |
            +------------------------------+
            |                              |
            v                              v
Return iframe fallback              Fetch embed HTML
                                           |
                                           +-- extract data-id
                                           |
                                           v
                              GET /stream/getSources?id=...
                                           |
                                           +-- sources/file URL
                                           +-- caption tracks
                                           +-- intro/outro metadata
                                           |
                                           v
                               Native HLS/MP4 StreamLink
                                           |
                                           v
                           Apply MegaPlay/KotoCDN headers
```

The iframe is retained even when native extraction succeeds. Normal playback can therefore fall back to the provider player, while Watch Together can select the controllable native source.

## 1. Searching and creating a stable internal ID

AniPlay combines two result sources:

- `GET /recent-anime` from Anikoto;
- AniList GraphQL search results.

Anikoto results provide the provider's series ID. AniList improves title, image, episode-count, MAL-ID, and AniList-ID coverage. Results are merged and deduplicated by AniList ID when possible, then by normalized title.

AniPlay stores the available identifiers in a Base64URL-encoded JSON object prefixed with `anikoto:`:

```json
{
  "anilistId": "183661",
  "malId": "60226",
  "anikotoId": "8370",
  "title": "Example title",
  "episodes": 12
}
```

Conceptually:

```ts
const showId = `anikoto:${Buffer.from(JSON.stringify(metadata), 'utf8').toString('base64url')}`
```

This avoids depending on only one mapping system. A result may have an Anikoto series ID, AniList ID, MAL ID, or some combination of them.

Numeric IDs are also accepted by the decoder and interpreted as Anikoto series IDs for compatibility.

## 2. Loading the episode list

When an Anikoto series ID exists, AniPlay requests:

```text
GET https://anikotoapi.site/series/{anikotoId}
```

For each episode it reads:

```json
{
  "number": 1,
  "episode_embed_id": "835403",
  "embed_url": {
    "sub": "https://megaplay.buzz/stream/s-2/835403/sub",
    "dub": "https://megaplay.buzz/stream/s-2/835403/dub"
  }
}
```

Field names are normalized defensively because provider payloads can vary. Episode numbers are converted to strings and sorted numerically.

If `/series/{id}` is unavailable but AniList supplied an episode count, AniPlay can still produce a synthetic `1..N` episode list and try the AniList/MAL embed routes.

Search and series requests are cached for five minutes. Failed promises are removed from the cache so a temporary upstream failure does not poison later attempts.

## 3. Building embed candidates

For a selected episode and language, AniPlay creates candidates in this order:

1. the explicit `embed_url.sub` or `embed_url.dub` returned by Anikoto;
2. `/stream/s-2/{episode_embed_id}/{language}`;
3. `/stream/ani/{anilistId}/{episode}/{language}`;
4. `/stream/mal/{malId}/{episode}/{language}`.

Duplicate URLs are removed without changing the order.

The explicit series response is preferred because it is the closest mapping to the provider's current catalog. AniList and MAL routes are fallbacks; MegaPlay warns that not every title is mapped through both external databases.

## 4. Preserving the iframe fallback

The first usable candidate becomes an embed `StreamLink`:

```ts
{
  url: embedUrl,
  resolution: 'Embed',
  hls: false,
  provider: 'MegaPlay · Embed',
  downloadable: false,
  embed: true,
}
```

This fallback is important because native extraction is more fragile than the documented embed interface. If the provider changes its internal player or blocks native access, users can still use the iframe.

## 5. Extracting MegaPlay's internal source ID

Native resolution begins by requesting the embed page with a browser User-Agent and a MegaPlay referer:

```http
GET /stream/s-2/{episode_embed_id}/{language}
Host: megaplay.buzz
Referer: https://megaplay.buzz/
User-Agent: Mozilla/5.0 ...
Accept: text/html,application/json,text/plain,*/*
```

The current player markup contains a numeric `data-id` on the player element:

```html
<div id="megaplay-player" data-id="124325"></div>
```

AniPlay extracts it with a deliberately narrow expression:

```ts
/\bdata-id=["'](\d+)["']/i
```

Do not assume that the catalog's `episode_embed_id` and this internal `data-id` are the same. They belong to different layers and can have different values.

## 6. Requesting the source metadata

The internal ID is passed to the player source endpoint:

```text
GET https://megaplay.buzz/stream/getSources?id={dataId}
```

Required request context:

```http
Accept: application/json,text/plain,*/*
Referer: https://megaplay.buzz/stream/.../sub
Origin: https://megaplay.buzz
User-Agent: Mozilla/5.0 ...
```

A current response resembles this shape:

```json
{
  "sources": {
    "file": "https://MEDIA_HOST/.../master.m3u8"
  },
  "tracks": [
    {
      "file": "https://SUBTITLE_HOST/.../eng.vtt",
      "label": "English",
      "kind": "captions",
      "default": true
    }
  ],
  "intro": { "start": 81, "end": 170 },
  "outro": { "start": 1315, "end": 1404 }
}
```

Never commit or share a real returned media URL. It may be temporary, IP-sensitive, or contain provider-specific identifiers.

## 7. Parsing sources and subtitles

Provider payloads are often inconsistent, so AniPlay accepts several shapes:

- a source string;
- `{ file: URL }`;
- `{ url: URL }`;
- `{ src: URL }`;
- arrays of sources;
- nested `sources`, `source`, or `links` objects.

Resolution labels are read from `label` or `quality`; otherwise the result is labeled `Auto`.

A source is considered HLS when its URL contains `.m3u8`. Caption entries are collected from `tracks`, `captions`, or `subtitles`, restricted to caption/subtitle-like kinds, deduplicated by URL, and attached to every parsed stream.

The result becomes a native `StreamLink`:

```ts
{
  url: 'https://MEDIA_HOST/.../master.m3u8',
  resolution: 'Auto',
  hls: true,
  provider: 'MegaPlay',
  downloadable: true,
  subtitles: [/* normalized tracks */],
}
```

## 8. Why KotoCDN needs special headers

The source metadata currently often points to a host such as:

```text
megap.kotocdn.site
```

Requesting the same HLS manifest without browser/provider context can return HTTP 403. AniPlay classifies KotoCDN as a MegaPlay-associated media host and sends:

```http
Referer: https://megaplay.buzz/
Origin: https://megaplay.buzz
User-Agent: a normal browser User-Agent
```

The allowlist currently covers the following MegaPlay-associated domain families:

```text
megaplay.buzz
mewstream.buzz
lostproject.club
voltara.click
kotocdn.site
```

Host matching must accept the exact domain or a real subdomain:

```ts
hostname === domain || hostname.endsWith(`.${domain}`)
```

Do not use a loose substring or plain `endsWith(domain)` check without the dot boundary. For example, `evilmegaplay.buzz` must not be trusted as a MegaPlay host.

## 9. Electron request interception and CORS

The renderer uses `hls.js`, but the media request originates from AniPlay's renderer rather than from the MegaPlay iframe. Electron therefore applies the provider headers in `session.defaultSession.webRequest.onBeforeSendHeaders`.

AniPlay also normalizes CORS response headers in `onHeadersReceived`, limited to explicitly allowlisted provider media URL patterns. This allows `hls.js` to read manifests and segments from the renderer.

Important security rule:

> Never apply provider referers, origins, or permissive CORS rewriting to `*://*/*`.

Keep interception restricted to known media domains. A global rule can leak provider context to unrelated requests and weaken the renderer's network boundary.

The same host classifier is used by the downloader so FFmpeg receives the correct headers for native Anikoto HLS streams.

### Disguised HLS segments during downloads

Some KotoCDN media playlists currently point at MPEG-TS segments wrapped with a small, valid PNG prefix and served as `image/png`. Browser playback can tolerate this provider behavior, but FFmpeg probes the segment as a PNG video. The resulting stream has no usable dimensions, so MP4 stream-copy fails while writing its header.

AniPlay handles this only for recognized MegaPlay/KotoCDN HLS downloads. A tokenized relay bound to `127.0.0.1` rewrites playlist-owned URLs, preserves the upstream provider headers, scans only the beginning of disguised segments, removes the PNG prefix when three 188-byte MPEG-TS packet sync bytes confirm the payload, and streams the remaining bytes as `video/mp2t` to FFmpeg.

The relay does not transcode or buffer the complete episode. It accepts no caller-provided destination URL, uses an ephemeral port and random path token, and closes when FFmpeg finishes or fails. Keep those restrictions if adapting this workaround elsewhere; a generic local HTTP proxy would create a much larger security surface.

## 10. Native-resolution fallback behavior

Native extraction is enabled by default in AniPlay 1.16.2. It can be disabled for debugging or emergency compatibility:

```text
ANIPLAY_ANIKOTO_NATIVE=false
```

Accepted opt-out values are `0`, `false`, and `no`, case-insensitively.

Resolution behavior is:

1. create the iframe fallback when an embed candidate exists;
2. if native resolution is disabled, return the iframe;
3. otherwise try the ordered embed candidates;
4. stop after the first candidate that produces supported native streams;
5. if every native attempt fails, return the iframe rather than failing playback;
6. fail only when neither an embed nor a supported native stream exists.

This is intentional graceful degradation. A private extraction endpoint changing should not immediately remove all Anikoto playback.

## 11. Watch Together behavior

An iframe can send progress events to its parent, but MegaPlay does not document a parent-to-iframe API for reliable play, pause, and seek commands. Outbound progress events alone are insufficient for synchronized playback.

AniPlay therefore considers only non-embed links controllable:

```ts
links.some((link) => !link.embed)
```

When a room matches the current provider, show, episode, and translation type, the player automatically switches from the iframe to the first native source. The existing HTML video synchronization can then:

- apply authoritative play and pause state;
- seek guests to the host position;
- correct small drift using temporary playback-rate changes;
- report buffering/readiness;
- recover the current position after reconnecting.

If a participant resolves only the iframe, AniPlay marks them not ready and displays a warning that Anikoto cannot remain synchronized on that network.

The Worker receives only stable identifiers:

```json
{
  "provider": "anikoto",
  "showId": "anikoto:BASE64URL_METADATA",
  "animeName": "Example title",
  "episode": "1",
  "translationType": "sub",
  "aniListMediaId": 183661
}
```

Every participant resolves their own current media URL. This avoids putting temporary URLs, cookies, or provider headers into room storage and allows region-specific delivery hosts.

## Manual diagnostics

These examples inspect response shapes. They intentionally avoid printing or sharing final media URLs.

### Check the Anikoto catalog

```sh
curl -fsS 'https://anikotoapi.site/recent-anime?page=1&per_page=2' | jq '{ok, pagination, rows: (.data | length)}'
```

### Check a series response

```sh
curl -fsS 'https://anikotoapi.site/series/SERIES_ID' \
  | jq '{ok, episodeCount: (.data.episodes | length), first: (.data.episodes[0] | {number, episode_embed_id, embed_url})}'
```

### Find the internal player ID

```sh
curl -fsS \
  -A 'Mozilla/5.0' \
  -H 'Referer: https://megaplay.buzz/' \
  'https://megaplay.buzz/stream/s-2/EPISODE_EMBED_ID/sub' \
  | rg -o "data-id=['\"][0-9]+['\"]" \
  | head -n 1
```

### Inspect source metadata without displaying URLs

```sh
curl -fsS \
  -A 'Mozilla/5.0' \
  -H 'Accept: application/json,text/plain,*/*' \
  -H 'Origin: https://megaplay.buzz' \
  -H 'Referer: https://megaplay.buzz/stream/s-2/EPISODE_EMBED_ID/sub' \
  'https://megaplay.buzz/stream/getSources?id=INTERNAL_DATA_ID' \
  | jq '{sourceShape: (.sources | type), trackCount: ((.tracks // []) | length), hasIntro: has("intro"), hasOutro: has("outro")}'
```

Respect upstream rate limits. Do not put these commands in a tight loop.

## Common failures

| Symptom | Likely cause | Check |
| --- | --- | --- |
| Anikoto returns 429 | Catalog requests are too frequent | Cache search/series responses and back off |
| Anikoto returns 403 | Heavy API use or network/IP restriction | Stop retrying, wait, and test another network |
| Embed URL works but native extraction fails | MegaPlay changed its internal markup or source endpoint | Recheck `data-id` and the `getSources` response shape |
| `data-id` is missing | Player markup changed or an error/challenge page was returned | Log status, content type, final URL, and a short sanitized HTML signature |
| `getSources` returns 403 | Missing/incorrect `Referer`, `Origin`, or browser User-Agent | Compare the request with the header set above |
| HLS manifest returns 403 | Delivery host was not classified or provider headers were lost | Check the final hostname and Electron request interception |
| Manifest loads but segments fail | Segments use another unregistered domain | Inspect the master/media playlist hostnames and extend the narrow allowlist |
| FFmpeg reports `dimensions not set` for a MegaPlay download | A segment was detected as its PNG wrapper instead of the appended MPEG-TS payload | Inspect the first bytes without logging the signed URL; retain the scoped HLS relay and MPEG-TS sync validation |
| Video plays but subtitles do not | Subtitle host lacks headers/CORS handling or track shape changed | Inspect `tracks`, `captions`, and `subtitles` fields |
| Watch Together remains “loading” | Only the iframe resolved for that participant | Confirm that at least one returned link has `embed !== true` |
| Host can play but a guest cannot | Region/CDN/network differences | Each client resolves locally; test the guest network and avoid sharing the host URL |

## Logging safely

Useful diagnostics:

- request stage: catalog, series, embed HTML, source metadata, manifest;
- response status and content type;
- final hostname after redirects;
- whether an internal `data-id` was found;
- number and type of parsed sources;
- number of subtitle tracks;
- whether fallback embed playback remains available.

Do not log:

- complete media URLs;
- query tokens or signed paths;
- cookies;
- authorization headers;
- full HTML/JSON bodies in normal logs;
- room host capability tokens.

For debug builds, sanitize URLs down to scheme and hostname wherever possible.

## Tests in AniPlay

Relevant deterministic tests:

- `electron/providers/anikoto.test.ts`
  - catalog normalization;
  - series/episode parsing;
  - `data-id` extraction;
  - source and subtitle parsing;
  - native-resolution default and opt-out behavior;
  - embed fallback URL construction.
- `electron/media-headers.test.ts`
  - exact/subdomain host classification;
  - lookalike-host rejection;
  - KotoCDN Electron URL patterns.
- `electron/downloads/download-utils.test.ts`
  - KotoCDN/MegaPlay FFmpeg headers.
- `electron/downloads/hls-mime-proxy.test.ts`
  - playlist URL rewriting;
  - disguised MIME correction;
  - PNG-wrapper/MPEG-TS boundary detection;
  - rejection of non-HTTPS upstream resources.
- `src/lib/watch-together-content.test.ts`
  - native versus embed-only controllability;
  - Anikoto synchronization-warning rules;
  - stable room content without media URLs.

Live provider tests should remain opt-in. Public provider responses, CDN domains, and media URLs change too often for deterministic CI.

## Adding another provider

Use this checklist when integrating another embed-based source.

### Catalog layer

- Identify the official or de facto catalog/search endpoint.
- Determine stable show and episode identifiers.
- Keep provider IDs separate from AniList/MAL IDs.
- Normalize episode numbers and translation/audio modes.
- Cache catalog responses according to the provider's documented limits.

### Embed layer

- Preserve a documented iframe/browser fallback when possible.
- Generate candidates from the strongest provider-specific identifier first.
- Validate embed URLs and protocols before returning them.
- Record the precise referer/origin expected by the embed.

### Native source layer

- Determine whether a documented native API exists before inspecting private player calls.
- If private extraction is necessary, isolate it in one provider module.
- Parse response shapes defensively instead of assuming one JSON layout.
- Support HLS and direct MP4 separately.
- Normalize subtitles, quality labels, and required headers.
- Deduplicate results.
- Do not hardcode a current CDN hostname as the provider identity.

### Playback/network layer

- Add only the provider's known delivery domains to request interception.
- Match exact domains and subdomains safely.
- Apply provider headers to manifests, segments, subtitles, and downloads.
- Scope any CORS rewriting to the narrow media-domain allowlist.
- Test redirects because the final CDN may differ from the initial URL.

### Watch Together layer

- Share stable content IDs, never resolved media URLs.
- Resolve sources independently on every participant.
- Mark iframe-only playback as uncontrollable unless the provider documents a secure bidirectional control protocol.
- Keep guests unready until native media metadata is loaded.
- Provide a visible fallback/error state when a participant cannot resolve the controllable source.

### Failure behavior

- Distinguish catalog failure, mapping failure, embed failure, extraction failure, and CDN failure.
- Retain a lower-capability fallback where it is safe.
- Do not retry 403/429 responses aggressively.
- Bound timeouts and candidate attempts.
- Avoid turning one broken source into a failure for unrelated providers.

### Verification

- Unit-test every parser with sanitized fixtures.
- Test hostile/lookalike hostnames for header rules.
- Test missing fields, malformed JSON, empty sources, and partial subtitle data.
- Test iframe-only fallback behavior.
- Test that room payloads contain no stream URL or headers.
- Perform a manual two-client Watch Together test on separate networks when possible.

## Maintenance warning

The catalog endpoints and embed URL formats are documented upstream. The internal source endpoint, HTML `data-id`, JSON shape, and current KotoCDN delivery hostname may change without notice.

Keep the implementation layered so that a change to any one of these can be repaired independently:

```text
Anikoto catalog parser
        !=
MegaPlay embed builder
        !=
MegaPlay native extractor
        !=
KotoCDN/media request policy
        !=
Watch Together synchronization
```

That separation is the most reusable lesson from this provider integration.
