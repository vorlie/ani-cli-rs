# Anikoto.cz provider workflow

This document describes the native Rust implementation behind
`--provider anikoto2`. It is a contributor reference for ani-cli-rs and other
projects integrating the same catalog. The website endpoints are private
implementation details and may change without notice.

The implementation lives in:

- `src/anikoto_cz.rs` for catalog and source resolution;
- `src/hls_relay.rs` for KotoCDN playlist and segment compatibility;
- `src/player.rs` and `src/download.rs` for relay lifetime management.

No Python helper, nginx service, or externally reachable proxy is used.

## Complete flow

```text
search query
    -> GET /ajax/anime/search?keyword=...
    -> parse /watch/<slug> results
    -> GET /watch/<slug>
    -> read #watch-main data-id and canonical data-url
    -> GET /ajax/episode/list/<id>?style=grid&vrf=
    -> select SUB/H-SUB or DUB episode
    -> GET canonical episode page
    -> GET /ajax/server/list?servers=<episode token>
    -> GET /ajax/server?get=<server token>
    -> MegaPlay or VidTube embed
    -> read numeric data-id
    -> GET <embed origin>/stream/getSources?id=<data-id>
    -> validate native media and subtitle URLs
    -> expand HLS master qualities
    -> play/download through the scoped KotoCDN relay
```

Search IDs are Base64URL JSON prefixed with `anikoto2:`:

```json
{
  "slug": "black-torch-1d364",
  "title": "Black Torch",
  "episodes": null
}
```

The prefix makes history and scriptable commands self-routing. A raw slug is
accepted only when the caller explicitly selects `--provider anikoto2`.

## Requests and cookies

`AnikotoCzClient` uses a dedicated `reqwest::Client` with:

- a coherent Chromium user agent;
- an in-memory cookie jar;
- bounded redirects and a 15-second timeout;
- `Referer`, `Accept-Language`, `Accept`, and `X-Requested-With` headers;
- a response-size cap before parsing provider HTML or JSON.

HTTP 429 is returned as a provider rate-limit error. Failed requests are not
aggressively retried.

## Episode and language handling

Episode anchors expose:

- `data-num`: display number, including fractional episodes;
- `data-slug`: canonical episode-page suffix;
- `data-ids`: token used by the server-list endpoint;
- `data-mal` and `data-timestamp`: optional mapper inputs;
- `data-sub` and `data-dub`: strict language availability.

SUB mode includes both soft-sub and hard-sub groups. DUB mode includes only
dub groups. The client never silently changes language.

The optional `mapper.nekostream.site` response can add Vidstream,
Kiwi-Stream, or Vibe-Stream candidates when MAL and timestamp metadata exist.
Mapper failure does not discard the website's own server list.

## Native source extraction

The `/ajax/server` exchange returns an embed URL. Native extraction is
currently restricted to exact `megaplay.buzz`/`vidtube.site` hosts and their
real subdomains. Other embeds are reported as unsupported rather than passed
to an external player as if they were direct media.

Supported embeds expose:

```html
<div data-id="123456"></div>
```

The numeric ID is sent to the embed origin's `/stream/getSources` endpoint
with the embed URL as referer and its origin as `Origin`. Nested
`sources`/`source`/`links` objects and `file`/`url`/`src` values are normalized
defensively. Subtitle tracks are read from `tracks`, `captions`, and
`subtitles`.

Only validated HTTPS URLs without credentials or literal-IP hosts become
`StreamLink` values. Provider headers remain attached to each stream and are
passed as separate player/downloader arguments.

## KotoCDN compatibility relay

Some KotoCDN HLS fragments are valid MPEG-TS data preceded by a small PNG
payload. Passing those URLs directly to FFmpeg/mpv can make the segment appear
to be an image and terminate playback.

ani-cli-rs starts a temporary Hyper server on `127.0.0.1` and registers only
resources discovered from the selected playlist. It:

- uses an ephemeral port and unguessable resource tokens;
- rewrites master/media playlists and `URI=` attributes;
- forwards the stream's required provider headers;
- rejects credentials and non-HTTPS upstream resources;
- caps playlist size and registered resource count;
- strips a PNG prefix only after finding three MPEG-TS sync bytes exactly
  188 bytes apart;
- corrects segment MIME types for GET and HEAD requests;
- shuts down when the attached player or downloader exits.

This is a short-lived loopback compatibility proxy, not a public relay. It
does not listen on LAN interfaces.

## Safe diagnostics

Useful issue information:

- provider stage that failed;
- HTTP status and final hostname;
- selected language and server label;
- number of native sources and subtitle tracks;
- whether a master playlist contained variants;
- whether the loopback relay started.

Never publish complete embed/media URLs, signed paths, query tokens, cookies,
or raw provider response bodies.

## Manual verification

```console
ani-cli-rs -p anikoto2 search --json "black torch"
ani-cli-rs episodes ANIKOTO2_ID --mode sub
ani-cli-rs links ANIKOTO2_ID 1 --json
ani-cli-rs play ANIKOTO2_ID 1 --title "Black Torch" --no-detach
```

The `links --json` result contains temporary URLs and must be sanitized before
sharing.

The opt-in Rust smoke test performs search, episode discovery, and native
source resolution without printing URLs:

```sh
ANI_CLI_LIVE_ANIKOTO2=1 cargo test live_anikoto_cz_smoke_test_is_opt_in --lib
```

Normal `cargo test` remains deterministic.
