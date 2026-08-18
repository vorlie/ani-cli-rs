# ani-cli-rs 0.9.4
 
This release fixes video player launching issues.
 
## Fixes
 
- Fixed video player launching failures by adding HLS relay cache settings to mpv arguments
- Added mpv cache parameters (`--cache=yes`, `--cache-secs=120`, `--demuxer-max-bytes=512MiB`, `--demuxer-max-back-bytes=256MiB`) for relayed streams
- Enhanced player executable validation to provide better error messages when players are not found
- Improved process detachment behavior to match koto-cli's nohup behavior
- Fixed URL argument ordering to ensure the stream URL is passed as the last argument to mpv
- These changes resolve playback issues with Anikoto provider's HLS streams and relay mechanism

## Debug logging improvements

The application includes comprehensive debug logging through the `tracing` crate to help with troubleshooting and issue reporting:

- Player launch logging with executable details, process IDs, and command arguments
- Stream resolution logging with URLs, HLS status, and provider information
- HLS relay logging with bind addresses, upstream hosts, and resource registration
- Network request logging for provider API calls and stream fetching
- Error context logging with structured metadata for better debugging

Enable debug logging with:
```console
RUST_LOG=ani_cli_rs=debug,ani_cli=debug ani-cli-rs "anime title"
```

For full stream resolution and relay tracing:
```console
RUST_LOG=ani_cli_rs=trace,ani_cli=trace ani-cli-rs "anime title"
```

## Upgrade
 
Existing installations can update with:
 
```console
ani-cli-rs update
```
 
**Full changelog:** [0.9.3...0.9.4](https://github.com/vorlie/ani-cli-rs/compare/0.9.3...0.9.4)