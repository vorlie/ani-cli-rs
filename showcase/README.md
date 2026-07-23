# ani-cli-rs showcase generator

This directory records a deterministic ani-cli-rs terminal walkthrough with [VHS](https://github.com/charmbracelet/vhs). The tape exercises the real `search`, `episodes`, `links`, quality-selection, JSON, subtitle metadata, and diagnostics command paths against local Rust fixtures.

## Generate the showcase

The supported recording host is Windows 10/11 with WSL. WSL needs:

- A current Rust toolchain with `cargo`.
- FFmpeg.
- `curl` and `sha256sum`.

From the repository root, run:

```powershell
.\showcase\showcase.ps1
```

The first run downloads pinned Linux builds of VHS 0.11.0 and ttyd 1.7.7 into the ignored `showcase/.tools` directory. Their SHA-256 hashes are checked before execution. VHS may also download its pinned headless Chromium build into the WSL user cache.

The command builds the debug CLI in WSL, records the tape, and validates:

- A 60–90 second, 1280 x 720 MP4 at `showcase/output/ani-cli-rs-showcase.mp4`.
- Five PNG screenshots under `showcase/output/screenshots`.
- A README GIF no larger than 10 MiB at `docs/assets/ani-cli-rs-showcase.gif`.

The tools, MP4, screenshots, and intermediate output are ignored by Git. Only the README GIF is tracked.

## Fixture safety

The hidden `--demo-mode` flag exists only in debug builds. Release builds do not compile the flag or fixture backend.

When enabled, provider clients are not constructed. Search results, episodes, stream URLs, request headers, subtitles, and crypto diagnostics come from fixed in-process values. The placeholder `.invalid` URLs are printed for demonstration and are never fetched by the tape. Playback, downloads, updates, history, and external programs are not invoked.

The recorder's first-run downloads are development tooling traffic. The running CLI does not contact AllAnime, Anikoto, MegaPlay, AniList, GitHub APIs, or media hosts.

## Editing the tape

The source is [`ani-cli-rs.tape`](ani-cli-rs.tape). It uses:

- Explicit provider flags so developer environment variables cannot alter the recording.
- Screen-based waits with ten-second limits instead of timing-only assumptions.
- A fixed terminal size, AniPlay color palette, Cascadia Mono, and deterministic pauses.
- Independent screenshots after every major command.

Run `vhs validate showcase/ani-cli-rs.tape` inside a WSL environment containing VHS when changing tape syntax. Re-run the PowerShell generator after changes so the tracked GIF matches the tape.
