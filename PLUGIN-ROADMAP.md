# ani-cli-rs external plugin roadmap

Status: proposed

This document describes a phased plan for adding external executable plugins to `ani-cli-rs`. The initial goal is provider extensibility: adding search, episode-list, and stream-resolution backends without rebuilding the main application.

External processes are preferred over in-process Rust dynamic libraries. They avoid Rust ABI instability, isolate crashes, permit plugins written in any language, and provide a protocol that AniPlay could adopt later.

## Goals

- Allow third-party catalog and streaming providers to integrate with the existing CLI workflow.
- Keep the main executable stable when a plugin crashes, hangs, or returns malformed data.
- Define a small, versioned, language-independent protocol.
- Preserve non-interactive JSON use and the familiar interactive workflow.
- Support Windows, Linux, and macOS source builds without requiring identical compiler toolchains.
- Make plugin discovery, diagnostics, updates, and removal understandable to terminal users.
- Leave room for metadata, subtitle, resolver, and integration plugins after provider support is stable.
- Make the provider protocol reusable by AniPlay where practical.

## Non-goals for the first release

- Loading arbitrary Rust traits from `.dll`, `.so`, or `.dylib` files.
- Installing plugins automatically from untrusted URLs.
- A centralized plugin marketplace.
- Executing plugin-provided UI code inside the terminal application.
- Perfect operating-system sandboxing on every platform.
- Allowing plugins to mutate ani-cli-rs history or configuration directly.
- Replacing both built-in Anikoto implementations with plugins immediately.

## Proposed layout

Each plugin lives in its own directory:

```text
plugins/
└── example-provider/
    ├── plugin.json
    ├── example-provider.exe
    ├── LICENSE
    └── README.md
```

Platform-native user plugin directories should be used by default:

- Windows: `%LOCALAPPDATA%\ani-cli-rs\plugins`
- Linux: `$XDG_DATA_HOME/ani-cli-rs/plugins`, falling back to `~/.local/share/ani-cli-rs/plugins`
- macOS: `~/Library/Application Support/ani-cli-rs/plugins`

An additional directory may be supplied through `ANI_CLI_PLUGIN_DIR`. The explicit directory takes priority over the platform directory, making portable and development setups easy to test.

## Manifest version 1

`plugin.json` is read before starting the executable:

```json
{
  "manifestVersion": 1,
  "id": "example-provider",
  "name": "Example Provider",
  "version": "1.0.0",
  "protocolVersion": 1,
  "type": "provider",
  "description": "Example anime catalog and streaming provider.",
  "homepage": "https://example.invalid/plugin",
  "license": "GPL-3.0-only",
  "executables": {
    "windows-x86_64": "example-provider.exe",
    "windows-aarch64": "example-provider-arm64.exe",
    "linux-x86_64": "example-provider",
    "linux-aarch64": "example-provider-arm64",
    "macos-x86_64": "example-provider-macos",
    "macos-aarch64": "example-provider-macos-arm64"
  },
  "capabilities": ["search", "episodes", "streams"],
  "permissions": {
    "networkDomains": ["api.example.invalid", "video.example.invalid"]
  }
}
```

Manifest validation must reject:

- Unknown manifest or incompatible protocol versions.
- Invalid or duplicate plugin IDs.
- Absolute executable paths or paths escaping the plugin directory.
- Missing executables for the current platform and architecture.
- Unsupported plugin types or capabilities.
- Invalid URLs and malformed network-domain declarations.
- Manifests larger than a conservative size limit.

The manifest describes intent; it is not an operating-system sandbox. A normal executable can still access anything permitted to the current user unless stronger isolation is added later.

## Protocol version 1

Plugins communicate through newline-delimited JSON on stdin and stdout.

- One complete JSON object per line.
- UTF-8 encoding only.
- `stdout` is reserved exclusively for protocol messages.
- Human-readable diagnostics go to `stderr`.
- Every request contains an ID copied into its response.
- Unknown fields are ignored when safe, allowing additive protocol evolution.
- The host limits line length and total buffered output.

### Startup handshake

The host starts the plugin and sends:

```json
{"protocol":1,"id":1,"method":"initialize","params":{"host":{"name":"ani-cli-rs","version":"0.2.0"},"locale":"en-US"}}
```

The plugin responds:

```json
{"protocol":1,"id":1,"result":{"plugin":{"id":"example-provider","version":"1.0.0"},"capabilities":["search","episodes","streams"]}}
```

The returned ID and version must match the manifest. A mismatch disables the plugin for that process and produces a clear diagnostic.

### Search

```json
{"protocol":1,"id":2,"method":"search","params":{"query":"frieren","mode":"sub","allowAdult":false}}
```

```json
{"protocol":1,"id":2,"result":[{"id":"show-123","name":"Frieren: Beyond Journey's End","episodes":28}]}
```

### Episode list

```json
{"protocol":1,"id":3,"method":"episodes","params":{"animeId":"show-123","mode":"sub"}}
```

```json
{"protocol":1,"id":3,"result":["1","2","3","4"]}
```

Episode identifiers remain strings to support specials, decimals, and provider-specific numbering.

### Stream resolution

```json
{"protocol":1,"id":4,"method":"streams","params":{"animeId":"show-123","episode":"1","mode":"sub"}}
```

```json
{"protocol":1,"id":4,"result":[{"url":"https://video.example.invalid/master.m3u8","resolution":"1080p","hls":true,"provider":"Example","downloadable":true,"headers":{"referer":"https://example.invalid/","origin":"https://example.invalid"}}]}
```

Stream results should map directly to the existing `StreamLink` model so built-in and external providers share quality selection, playback, download, and history behavior.

### Errors

Plugins return structured errors:

```json
{"protocol":1,"id":4,"error":{"code":"rate_limited","message":"Try again later.","retryAfterSeconds":5}}
```

Initial error codes:

- `invalid_request`
- `not_found`
- `unavailable`
- `rate_limited`
- `network`
- `provider_changed`
- `permission_required`
- `internal`

Unknown codes are treated as `internal` while preserving the plugin's safe human-readable message.

### Shutdown

The host sends a best-effort shutdown request before closing stdin:

```json
{"protocol":1,"id":5,"method":"shutdown","params":{}}
```

The host may terminate the process after a short grace period. Plugins must not assume that shutdown is always delivered after crashes or forced termination.

## Process lifecycle

The first implementation should start one process for the selected plugin and reuse it for the current CLI session.

Required controls:

- Startup timeout, initially 5 seconds.
- Search and episode-list timeout, initially 15 seconds.
- Stream-resolution timeout, initially 30 seconds.
- Maximum JSON line size, initially 1 MiB.
- Maximum stderr retained for an error report, initially 64 KiB.
- Process termination when stdout is malformed, the handshake fails, or a timeout expires.
- Concurrent draining of stderr so a verbose plugin cannot deadlock on a full pipe.
- Cleanup through a process guard when the CLI exits early.
- No shell command construction; executable paths and arguments are passed directly to the process API.

Protocol timeouts should be configurable for diagnostics, but plugins must not be allowed to disable them unilaterally.

## CLI integration

Proposed commands and options:

```console
ani-cli-rs plugins list
ani-cli-rs plugins inspect example-provider
ani-cli-rs plugins doctor example-provider
ani-cli-rs plugins enable example-provider
ani-cli-rs plugins disable example-provider
ani-cli-rs --provider example-provider "frieren"
ani-cli-rs search --provider example-provider --json "frieren"
```

Proposed environment variables:

- `ANI_CLI_PROVIDER`: default provider ID.
- `ANI_CLI_PLUGIN_DIR`: additional or portable plugin directory.
- `ANI_CLI_PLUGIN_TIMEOUT`: diagnostic timeout override with a documented upper bound.

Built-in providers use stable IDs (`anikoto` and `anikoto2`). Provider-prefixed show IDs keep history entries self-routing without changing the legacy tab-separated file layout.

Interactive provider selection should appear only when more than one enabled provider is available. Existing users with no plugins installed should see no additional prompt.

## Security model

Executable plugins are trusted native code running as the current user. The initial release must communicate this clearly during manual installation and first enablement.

Minimum protections:

- Never discover loose executables without a valid manifest.
- Never execute a path outside the resolved plugin directory.
- Do not search the current working directory implicitly.
- Reject symlink or junction escapes after resolving the executable path.
- Pass no secrets or full environment dump to plugins.
- Start with a minimal documented environment where practical.
- Validate every response before converting it into public library models.
- Redact temporary media query strings and authorization headers from routine logs.
- Display the plugin ID in every plugin-originated error.
- Keep plugin auto-download and auto-update out of version 1.
- Document that domain permissions are advisory until requests are proxied through the host or an OS sandbox is implemented.

Future hardening may include:

- Optional SHA-256 hashes in manifests.
- Signed release metadata and publisher identity.
- Windows job objects, Linux namespaces/seccomp, and macOS sandbox profiles where maintainable.
- A host-proxied HTTP capability that can enforce domain declarations.
- WebAssembly plugins for portable parsers and resolvers requiring stronger isolation.

## Library architecture

Introduce an internal asynchronous provider abstraction implemented by built-in providers and external plugins:

```rust
trait Provider {
    async fn search(
        &self,
        query: &str,
        mode: TranslationType,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>>;

    async fn episodes(
        &self,
        anime_id: &str,
        mode: TranslationType,
    ) -> Result<Vec<String>>;

    async fn streams(
        &self,
        anime_id: &str,
        episode: &str,
        mode: TranslationType,
    ) -> Result<Vec<StreamLink>>;
}
```

The exact public API should be decided separately. The initial trait may remain internal until provider identity, error stability, and async-trait ergonomics are settled.

Suggested modules:

```text
src/plugins/
├── mod.rs
├── discovery.rs
├── manifest.rs
├── protocol.rs
├── process.rs
└── provider.rs
```

## Delivery phases

### Phase 0: protocol specification

- Finalize manifest and protocol version 1 schemas.
- Decide provider IDs and history migration behavior.
- Add JSON Schema documents or equivalent fixture validation.
- Publish a minimal reference plugin contract.
- Define which fields are stable and which are implementation details.

Exit criteria: a plugin author can implement the protocol without reading ani-cli-rs source code.

### Phase 1: discovery and diagnostics

- Resolve platform plugin directories.
- Parse and validate manifests without executing plugins.
- Add `plugins list`, `plugins inspect`, and `plugins doctor`.
- Report duplicate IDs, incompatible versions, missing binaries, and unsafe paths.
- Add enable/disable configuration.

Exit criteria: malformed installations are diagnosed precisely and no plugin code is executed during normal discovery.

### Phase 2: process protocol

- Implement the JSON-lines codec and request IDs.
- Add handshake, timeout, size-limit, stderr-draining, and shutdown behavior.
- Map structured plugin errors into ani-cli-rs error categories.
- Create a fake executable plugin used by integration tests.
- Verify paths and arguments on Windows and Unix platforms.

Exit criteria: the host can safely exercise a deterministic echo/reference plugin under success, timeout, crash, and malformed-output scenarios.

### Phase 3: provider integration

- Introduce the shared provider abstraction.
- Wrap both existing Anikoto clients as built-in providers.
- Implement external search, episodes, and streams methods.
- Add `--provider` and `ANI_CLI_PROVIDER`.
- Reuse existing quality selection, playback, downloads, and headers for plugin streams.
- Extend history with provider identity while preserving old tab-separated entries.

Exit criteria: a reference external provider completes interactive playback, scripted JSON output, downloading, and history continuation.

### Phase 4: author tooling

- Publish a small Rust protocol SDK with no dependency on internal scraper code.
- Publish language-neutral schemas and example request transcripts.
- Add a plugin conformance test command.
- Provide a template provider repository and release packaging examples.
- Document stdout/stderr rules and common deadlock mistakes.

Exit criteria: a third party can build and package a provider without copying private host implementation details.

### Phase 5: shared AniPlay compatibility

- Review protocol fields against AniPlay's provider and `StreamLink` models.
- Add host capability negotiation rather than assuming identical features.
- Define optional fields for subtitles, embed-only streams, and browser fallback.
- Build one reference plugin used unchanged by both hosts.
- Keep host-specific playback and download behavior outside the plugin.

Exit criteria: one packaged provider can serve both ani-cli-rs and AniPlay through the same protocol version.

### Phase 6: distribution and trust

- Define an optional signed plugin-index format.
- Add checksum verification for manually requested installations.
- Establish publisher and revocation metadata.
- Design updates as explicit user actions with downgrade support.
- Evaluate whether a curated registry is worth its moderation and security cost.

Exit criteria: distribution can be introduced without silently executing newly downloaded code.

## Test plan

Unit tests:

- Manifest parsing, platform selection, path traversal, and duplicate IDs.
- Request and response serialization.
- Protocol version and capability negotiation.
- Response validation for search results, episodes, streams, and headers.
- Structured error mapping and safe message truncation.
- History compatibility with and without provider identity.

Integration tests with fake plugin executables:

- Successful handshake and all provider methods.
- Startup failure, non-zero exit, crash, and early EOF.
- Startup and request timeouts.
- Invalid UTF-8, malformed JSON, oversized lines, and mismatched IDs.
- Protocol output mistakenly written to stderr and logs mistakenly written to stdout.
- Large stderr output without deadlock.
- Graceful and forced shutdown.
- Executable paths containing spaces and Unicode.
- Windows `.exe` and Unix executable-bit behavior.
- Multiple plugins with conflicting IDs.
- Non-interactive JSON output containing no plugin log noise.

Manual smoke tests:

- Windows 11 PowerShell and Command Prompt.
- Ubuntu/Debian and a non-Debian Linux distribution.
- x86-64 and ARM64 where release hardware or emulation is available.
- Interactive playback, direct download, HLS download, and history continuation.

## Compatibility policy

- `manifestVersion` changes only when manifest interpretation becomes incompatible.
- `protocolVersion` changes only for incompatible wire changes.
- Additive optional fields do not require a protocol bump.
- Hosts advertise their version and supported capabilities during initialization.
- Plugins must fail clearly when a required capability is unavailable.
- At least one previous protocol version should be considered for support after version 2 exists, but no promise is made until the maintenance cost is measured.
- Provider IDs are permanent once published because they become part of configuration and history.

## Open decisions

- Whether plugin processes may make network requests directly or should eventually use host-proxied HTTP.
- Whether provider identity should extend the legacy history line or use a sidecar metadata file.
- Whether a process remains alive for the whole interactive session or is restarted after idle time.
- How plugins declare adult-content behavior and whether the host enforces opt-in independently.
- Whether subtitle tracks belong in protocol version 1 or an early additive extension.
- Whether plugin configuration is generic JSON, a typed schema, or host-managed key/value settings.
- Whether an external provider may request browser fallback when it cannot return direct streams.
- How AniPlay should present trust prompts for the same executable plugins.

## Recommended first milestone

The first implementation milestone should stop after Phase 2. It should deliver discovery, diagnostics, and a hardened process protocol with a fake reference plugin, but should not yet route normal playback through third-party code.

This creates a reviewable security and compatibility boundary before provider selection changes the user-facing CLI and history model.
