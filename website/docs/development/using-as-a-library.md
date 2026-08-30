# Using ani-cli-rs as a Rust Library

`ani-cli-rs` is primarily distributed as a command-line application, but its
core functionality is also exposed as a reusable Rust library crate.

The library is published under the crate name `ani-cli-rs` and is exposed to
Rust code through the `ani_lib` library target.

This allows other Rust applications to use the provider clients, stream
resolution, HLS relay, downloading, history, and player-related functionality
without depending on the CLI application itself.

> **Note:** The library API is still under active development. Public APIs may
> change between releases while the library is being stabilized.

## Crate setup

The package is named `ani-cli-rs`, while its library target is named
`ani_lib`:

```toml
[package]
name = "ani-cli-rs"
version = "0.9.4"

[lib]
name = "ani_lib"
path = "src/lib.rs"
```

Add it to your application's `Cargo.toml`:

```toml
[dependencies]
ani-cli-rs = "0.9.6"
```

Then import functionality from `ani_lib`:

```rust
use ani_lib::{
    AnikotoCzClient,
    SearchResult,
    StreamLink,
    TranslationType,
};
```

## Repository dependency

If you are developing against a version of `ani-cli-rs` that has not yet been
published to crates.io, Cargo can use a Git repository instead:

```toml
[dependencies]
ani-cli-rs = {
    git = "https://github.com/vorlie/ani-cli-rs.git"
}
```

For applications that need a specific revision, pin the dependency to a
commit:

```toml
[dependencies]
ani-cli-rs = {
    git = "https://github.com/vorlie/ani-cli-rs.git",
    rev = "a430c6e6a203dcf9fac8817e132bffcbd4bac11f"
}
```

Pinning a commit is useful when your application depends on an API that is
still changing. It also ensures that another developer or CI system builds
against exactly the same revision.

## What is exposed by the library?

The crate currently exposes the following main components:

| API                        | Purpose                              |
| -------------------------- | ------------------------------------ |
| `AnikotoClient`            | Anikoto API catalog client           |
| `AnikotoCzClient`          | Anikoto.cz catalog client            |
| `SearchResult`             | Search result metadata               |
| `StreamLink`               | Resolved media stream metadata       |
| `SubtitleTrack`            | Subtitle track metadata              |
| `TranslationType`          | Sub/dub selection                    |
| `HlsRelay`                 | Local HLS relay lifetime             |
| `relay_stream`             | Create a local HLS relay             |
| `download_stream`          | Download a resolved stream           |
| `DownloadOptions`          | Download configuration               |
| `HistoryStore`             | Persistent playback history          |
| `Player`                   | External player integration          |
| `PlayerKind`               | Supported player selection           |
| `PlayerOptions`            | Player configuration                 |
| `choose_quality`           | Select an appropriate stream quality |
| `expand_episode_selection` | Expand episode selections            |
| `I18n` / `Locale`          | Localization support                 |
| `AniError` / `Result`      | Library error handling               |

The exact public API should be treated as version-dependent while the library
is still being stabilized.

---

# Basic provider usage

A simple application can create an `AnikotoCzClient`, search for an anime, and
then retrieve its episodes and streams.

```rust
use ani_lib::{
    AnikotoCzClient,
    TranslationType,
};

#[tokio::main]
async fn main() -> ani_lib::Result<()> {
    let client = AnikotoCzClient::new()?;

    let results = client
        .search("Mushoku Tensei", TranslationType::Sub)
        .await?;

    for result in results {
        println!("{} ({})", result.name, result.id);
    }

    Ok(())
}
```

The client is asynchronous, so applications should run it inside a Tokio
runtime.

## Searching

Search results are returned as `SearchResult` values.

A typical workflow is:

```text
search
  ↓
SearchResult
  ↓
episodes
  ↓
StreamLink
  ↓
playback / download
```

For example:

```rust
let results = client
    .search("Mushoku Tensei", TranslationType::Sub)
    .await?;

let show = results
    .first()
    .ok_or_else(|| ani_lib::AniError::NotFound(
        "No results found".into()
    ))?;
```

Applications should generally keep the complete `SearchResult` rather than
only storing its display name, since the provider-specific ID is required for
subsequent requests.

---

# Getting episodes

Once a `SearchResult` has been selected, retrieve its available episodes:

```rust
let episodes = client
    .episodes(&show.id, TranslationType::Sub)
    .await?;

for episode in &episodes {
    println!("Episode {episode}");
}
```

The library represents episode identifiers as strings:

```rust
Vec<String>
```

This is intentional because provider episode identifiers are not guaranteed
to always be simple integers.

For example, an application should not assume that every episode can safely be
parsed with:

```rust
episode.parse::<u32>()
```

---

# Resolving streams

After selecting an episode, request its available streams:

```rust
let streams = client
    .streams(
        &show.id,
        "1",
        TranslationType::Sub,
    )
    .await?;

for stream in &streams {
    println!(
        "{} - {} ({})",
        stream.resolution,
        stream.provider,
        if stream.hls { "HLS" } else { "Direct" }
    );
}
```

The resulting `StreamLink` contains the information needed by a player or
downloader.

Conceptually:

```text
StreamLink
├── URL
├── resolution
├── provider
├── HLS/direct classification
├── downloadability
├── request headers
└── optional subtitle tracks
```

Applications should use the `StreamLink` returned by the library rather than
attempting to reconstruct provider URLs themselves.

---

# Subtitles

Streams can contain optional subtitle tracks.

The public model is:

```rust
pub struct SubtitleTrack {
    pub label: String,
    pub url: String,
    pub default: bool,
}
```

For example:

```rust
for subtitle in &stream.subtitles {
    println!(
        "{}: {}{}",
        subtitle.label,
        subtitle.url,
        if subtitle.default {
            " (default)"
        } else {
            ""
        }
    );
}
```

The subtitle URL should be treated as provider-controlled data. Applications
should not assume that it uses the same host as the media stream.

If a player requires subtitles to be loaded through a local relay, the
application is responsible for providing that relay path.

---

# HLS playback

HLS streams require additional handling when consumed by applications that
cannot directly access the provider's media resources.

`ani-cli-rs` exposes:

```rust
pub use hls_relay::{
    HlsRelay,
    relay_stream,
    relay_stream_without_hls_subtitles,
};
```

A relay can be created from a `StreamLink`:

```rust
use ani_lib::relay_stream;

let (relay, local_stream) = relay_stream(&stream).await?;

println!("Local playback URL: {}", local_stream.url);
```

The important detail is that the returned `HlsRelay` must remain alive while
the stream is being played:

```rust
let (relay, local_stream) = relay_stream(&stream).await?;

// Keep `relay` alive while the player is using `local_stream`.
play(local_stream.url).await?;

// Relay is dropped after playback finishes.
drop(relay);
```

Dropping the relay shuts down its local server.

## Why is a relay needed?

Some provider HLS resources require request-specific handling and cannot be
consumed directly by every embedded player or WebView.

The relay provides a loopback URL such as:

```text
http://127.0.0.1:<port>/...
```

while keeping the upstream provider request handling inside the library.

The relay binds to loopback rather than exposing itself as an internet-facing
proxy.

---

# Example: application playback pipeline

A desktop application can combine the APIs into a complete playback flow:

```rust
use ani_lib::{
    relay_stream,
    AnikotoCzClient,
    TranslationType,
};

#[tokio::main]
async fn main() -> ani_lib::Result<()> {
    let client = AnikotoCzClient::new()?;

    let results = client
        .search("Mushoku Tensei", TranslationType::Sub)
        .await?;

    let show = results
        .first()
        .ok_or_else(|| ani_lib::AniError::NotFound(
            "No anime found".into()
        ))?;

    let episodes = client
        .episodes(&show.id, TranslationType::Sub)
        .await?;

    let episode = episodes
        .first()
        .ok_or_else(|| ani_lib::AniError::NotFound(
            "No episodes found".into()
        ))?;

    let streams = client
        .streams(
            &show.id,
            episode,
            TranslationType::Sub,
        )
        .await?;

    let stream = streams
        .first()
        .ok_or_else(|| ani_lib::AniError::NotFound(
            "No streams found".into()
        ))?;

    if stream.hls {
        let (_relay, local_stream) = relay_stream(stream).await?;

        println!("Play: {}", local_stream.url);

        // Keep `_relay` alive until playback finishes.
    } else {
        println!("Play: {}", stream.url);
    }

    Ok(())
}
```

A real application would normally add quality selection, error handling,
subtitle handling, and player integration around this flow.

---

# Using ani-cli-rs from Tauri

One practical use case for the library is embedding it into a Rust desktop
application.

For example, **Kioku** uses `ani_lib` from its Tauri backend to provide its
playback functionality.

Its dependency can be pinned directly to a Git revision during development:

```toml
[dependencies]
ani_lib = {
    package = "ani-cli-rs",
    git = "https://github.com/vorlie/ani-cli-rs.git",
    rev = "a430c6e6a203dcf9fac8817e132bffcbd4bac11f"
}
```

The important distinction here is:

```toml
package = "ani-cli-rs"
```

refers to the Cargo package name, while:

```rust
use ani_lib::...;
```

refers to the library target exposed by that package.

## Tauri command example

A Tauri backend can expose the provider API to a frontend:

```rust
#[tauri::command]
pub async fn playback_search(
    query: String,
    translation: TranslationType,
) -> Result<Vec<SearchResult>, String> {
    let client = AnikotoCzClient::new()
        .map_err(|error| error.to_string())?;

    client
        .search(&query, translation)
        .await
        .map_err(|error| error.to_string())
}
```

The frontend can then invoke the command without needing to know anything
about the provider implementation.

This gives the application a clean separation:

```text
┌─────────────────────────────┐
│          Kioku UI           │
│       React / TypeScript    │
└──────────────┬──────────────┘
               │ Tauri commands
               ▼
┌─────────────────────────────┐
│       Kioku Rust backend    │
│        playback.rs          │
└──────────────┬──────────────┘
               │ ani_lib
               ▼
┌─────────────────────────────┐
│          ani_lib             │
│                             │
│  Anikoto clients             │
│  Stream resolution           │
│  HLS relay                   │
│  Downloads                   │
│  Player integration          │
└─────────────────────────────┘
```

This is preferable to duplicating provider extraction logic inside the
application.

---

# Keeping the dependency reproducible

When using the Git dependency during development, there are two common
approaches.

## Track a branch

```toml
[dependencies]
ani_lib = {
    package = "ani-cli-rs",
    git = "https://github.com/vorlie/ani-cli-rs.git",
    branch = "master"
}
```

This is convenient when actively developing both projects.

However, the dependency can change whenever the branch moves.

## Pin a commit

```toml
[dependencies]
ani_lib = {
    package = "ani-cli-rs",
    git = "https://github.com/vorlie/ani-cli-rs.git",
    rev = "a430c6e6a203dcf9fac8817e132bffcbd4bac11f"
}
```

This is preferable for reproducible builds when depending on an unreleased
library API.

Once a compatible library version is published, applications should generally
prefer the versioned crate dependency:

```toml
[dependencies]
ani-cli-rs = "0.9"
```

---

# API stability

The library target is currently considered a developing API.

This means applications using `ani_lib` should expect that:

* public types may gain or lose fields;
* functions may change signatures;
* provider-specific behavior may change;
* error variants may change;
* playback and relay APIs may evolve;
* provider implementations can change independently of the public API.

For applications that need a stable build, pin a known-good release or Git
commit rather than tracking the development branch indefinitely.

The CLI and library share the same underlying provider implementation, so
provider changes can affect both.

---

# Recommended integration pattern

For a desktop media application, the recommended architecture is:

```text
Application
│
├── Search
│   └── AnikotoCzClient::search()
│
├── Series selection
│   └── AnikotoCzClient::episodes()
│
├── Episode selection
│   └── AnikotoCzClient::streams()
│
├── Quality/source selection
│   └── StreamLink
│
├── Playback
│   ├── direct stream
│   └── relay_stream() for HLS
│
├── Subtitles
│   └── StreamLink.subtitles
│
└── Download
    └── download_stream()
```

Keep provider-specific logic inside `ani_lib` whenever possible.

The application should primarily deal with the shared models:

```rust
SearchResult
StreamLink
SubtitleTrack
TranslationType
```

This makes it possible to change the provider implementation without
requiring the UI or playback layer to understand how media URLs are extracted.

---

# Kioku

[Kioku](https://github.com/vorlie/kioku) is one example of an application
currently integrating `ani_lib`.

Kioku uses the library from its Tauri backend rather than implementing the
Anikoto provider logic itself. Its playback system uses the shared
`SearchResult` and `StreamLink` models and the library's HLS relay for streams
that require local handling.

Kioku is currently **not a finished application** and should be considered a
development/integration example rather than a reference implementation of a
stable `ani_lib` API.

Its integration is nevertheless useful for demonstrating how the library can
be embedded into a larger Rust application with a non-CLI frontend.

---

# Related documentation

* [Provider Architecture](architecture.md)
* [Anikoto API, MegaPlay, and KotoCDN](../development/anikoto-kotocdn.md)
* [Anikoto.cz workflow](../development/anikoto-cz.md)
* [Building and Releasing](building.md)
* [Playback and Players](../guides/playback-and-players.md)