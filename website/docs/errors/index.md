# ani-cli-rs Error Codes

ani-cli-rs uses stable error codes to make failures easier to understand and report. Each error code is a stable identifier that will not change across versions.

## Error Code Scheme

Error codes follow the format `ACL-xxxx` where:
- `ACL` = `ani-cli`
- `xxxx` = numeric identifier

### Code Ranges

- **ACL-1xxx**: Network / provider errors
- **ACL-2xxx**: Search / anime / episode errors  
- **ACL-3xxx**: Streaming / source errors
- **ACL-4xxx**: Download errors
- **ACL-5xxx**: Playback errors
- **ACL-6xxx**: Filesystem / configuration errors
- **ACL-7xxx**: Authentication / external services
- **ACL-9xxx**: Internal / unexpected errors

---

## Network / Provider Errors (ACL-1xxx)

<a id="ACL-1001"></a>
### ACL-1001 — Provider unavailable

The selected provider could not be reached.

**Possible causes**
- Network connection problems
- Provider downtime
- DNS issues
- Provider blocking the request

**Try**
- Check your internet connection
- Try again later
- Try another provider

**Current mapping**: `AniError::ProviderUnavailable { provider, source }`

---

<a id="ACL-1002"></a>
### ACL-1002 — Provider request failed

A request to the provider failed during execution.

**Possible causes**
- Network timeout
- Connection interruption
- TLS/SSL certificate issues

**Try**
- Check your internet connection
- Try again later
- Check if the provider is experiencing issues

**Current mapping**: `AniError::ProviderRequestFailed { provider, source }`

---

<a id="ACL-1003"></a>
### ACL-1003 — Provider returned invalid response

The provider returned data that ani-cli-rs could not understand.

**Possible causes**
- Provider API changed
- Provider returned malformed data
- Provider returned unexpected data structure

**Try**
- Try another provider
- If the problem persists, report it with the error details

**Current mapping**: `AniError::ProviderInvalidResponse { provider, message, source }`

---

<a id="ACL-1004"></a>
### ACL-1004 — Provider rate limited

The provider has rate-limited requests from ani-cli-rs.

**Possible causes**
- Too many requests in a short time
- Provider throttling

**Try**
- Wait a few minutes and try again
- Try another provider

**Current mapping**: `AniError::ProviderRateLimited { provider, retry_after_seconds }`

---

<a id="ACL-1005"></a>
### ACL-1005 — Provider catalog error

The provider's catalog or search API returned an error.

**Possible causes**
- Provider catalog service down
- Invalid search parameters
- Provider internal error

**Try**
- Try another provider
- Check your search query
- Try again later

**Current mapping**: `AniError::ProviderCatalogError { provider, message, source }`

---

<a id="ACL-1006"></a>
### ACL-1006 — Provider URL validation failed

A provider URL failed security validation.

**Possible causes**
- URL uses HTTP instead of HTTPS
- URL contains credentials
- URL points to a literal IP address
- URL has invalid format

**Try**
- This is typically an internal error
- Report it if it persists

**Current mapping**: `AniError::ProviderUrlValidationFailed { reason }`

---

<a id="ACL-1007"></a>
### ACL-1007 — Provider response size exceeded

The provider returned a response that exceeded safety limits.

**Possible causes**
- Provider returned unusually large response
- Possible attack or malformed response

**Try**
- Try another provider
- Report this if it persists

**Current mapping**: `AniError::ProviderResponseSizeExceeded { provider, size, limit }`

---

## Search / Anime / Episode Errors (ACL-2xxx)

<a id="ACL-2001"></a>
### ACL-2001 — No search results found

No anime matching the search query was found.

**Possible causes**
- Search query doesn't match any anime
- Typo in the search query
- Anime not available on selected provider

**Try**
- Check your search query spelling
- Try a different search term
- Try another provider

**Current mapping**: `AniError::NoSearchResults`

---

<a id="ACL-2002"></a>
### ACL-2002 — Empty search query

Search query was empty.

**Possible causes**
- No search query provided
- Interactive input not available

**Try**
- Provide a search query
- Use interactive mode if available

**Current mapping**: `AniError::EmptySearchQuery`

---

<a id="ACL-2003"></a>
### ACL-2003 — No episodes available

No episodes are available for the selected anime.

**Possible causes**
- Anime has no episodes yet
- Episodes not available on selected provider
- Anime not fully released

**Try**
- Try another provider
- Check if the anime is released
- Try a different anime

**Current mapping**: `AniError::NoEpisodesAvailable { anime }`

---

<a id="ACL-2004"></a>
### ACL-2004 — Invalid episode selection

Selected episode is not available or invalid.

**Possible causes**
- Episode number out of range
- Episode not available for selected mode (sub/dub)
- Invalid episode format

**Try**
- Check available episodes
- Try a different episode
- Try subtitle/dub mode toggle

**Current mapping**: `AniError::InvalidEpisodeSelection { episode, reason }`

---

<a id="ACL-2005"></a>
### ACL-2005 — Selection out of range

User selection is outside the valid range.

**Possible causes**
- Selected option doesn't exist
- List changed during selection

**Try**
- Try again with valid selection
- Refresh the list

**Current mapping**: `AniError::SelectionOutOfRange { max, selected }`

---

<a id="ACL-2006"></a>
### ACL-2006 — Command requires query

Command requires a query parameter but none was provided.

**Possible causes**
- Missing required argument
- Non-interactive terminal without query

**Try**
- Provide the required query
- Use interactive mode if available

**Current mapping**: `AniError::CommandRequiresQuery`

---

## Streaming / Source Errors (ACL-3xxx)

<a id="ACL-3001"></a>
### ACL-3001 — No playable sources found

No playable video sources were found for the selected episode.

**Possible causes**
- No video servers available
- All sources failed to resolve
- Provider has no working streams

**Try**
- Try another provider
- Try subtitle/dub mode toggle
- Try a different episode

**Current mapping**: `AniError::NoPlayableSources { anime, episode, mode }`

---

<a id="ACL-3002"></a>
### ACL-3002 — Source resolution failed

Failed to resolve video sources from provider.

**Possible causes**
- Provider source API changed
- Source extraction failed
- Invalid source data

**Try**
- Try another provider
- Try a different episode
- Report if it persists

**Current mapping**: `AniError::SourceResolutionFailed { provider, reason }`

---

<a id="ACL-3003"></a>
### ACL-3003 — Stream unavailable

The selected video stream is not available.

**Possible causes**
- Stream server down
- Stream removed
- Network issues

**Try**
- Try another quality/source
- Try another provider
- Check your connection

**Current mapping**: `AniError::StreamUnavailable { reason }`

---

<a id="ACL-3004"></a>
### ACL-3004 — Unsupported embed host

Provider uses an unsupported embed host.

**Possible causes**
- Provider added new embed host
- Embed host not whitelisted

**Try**
- Try another provider
- Report this for host whitelist update

**Current mapping**: `AniError::UnsupportedEmbedHost { host }`

---

<a id="ACL-3005"></a>
### ACL-3005 — No native streams available

Provider returned no supported native video streams.

**Possible causes**
- Provider changed stream format
- No compatible streams available

**Try**
- Try another provider
- Try a different episode

**Current mapping**: `AniError::NoNativeStreams { provider }`

---

## Download Errors (ACL-4xxx)

<a id="ACL-4001"></a>
### ACL-4001 — Download failed

General download failure.

**Possible causes**
- Network issues during download
- Server not responding
- Connection interrupted

**Try**
- Check your internet connection
- Try again later
- Try another source/provider

**Current mapping**: `AniError::DownloadFailed { reason, source }`

---

<a id="ACL-4002"></a>
### ACL-4002 — No download tool available

No suitable download tool (yt-dlp or FFmpeg) found for HLS downloads.

**Possible causes**
- yt-dlp not installed
- FFmpeg not installed
- Tools not in PATH

**Try**
- Install yt-dlp or FFmpeg
- Ensure tools are in your PATH
- Use non-HLS sources if available

**Current mapping**: `AniError::NoDownloadTool`

---

<a id="ACL-4003"></a>
### ACL-4003 — HLS download failed

HLS stream download failed.

**Possible causes**
- yt-dlp/FFmpeg failed
- HLS stream corrupted
- Network issues

**Try**
- Check yt-dlp/FFmpeg installation
- Try another source
- Check your connection

**Current mapping**: `AniError::HlsDownloadFailed { reason }`

---

<a id="ACL-4004"></a>
### ACL-4004 — Subtitle download failed

Failed to download subtitle track.

**Possible causes**
- Subtitle server not responding
- Invalid subtitle URL
- Network issues

**Try**
- Try without subtitles
- Try another source
- Check your connection

**Current mapping**: `AniError::SubtitleDownloadFailed { track, reason }`

---

<a id="ACL-4005"></a>
### ACL-4005 — Subtitle size exceeded

Subtitle track exceeds safety size limit (16 MiB).

**Possible causes**
- Malformed subtitle file
- Subtitle file too large

**Try**
- Try another source
- Try without subtitles
- Report if it persists

**Current mapping**: `AniError::SubtitleSizeExceeded { size, limit }`

---

<a id="ACL-4006"></a>
### ACL-4006 — Download output error

Failed to write downloaded content to disk.

**Possible causes**
- Disk full
- Permission issues
- Invalid path

**Try**
- Check disk space
- Check write permissions
- Use a different download directory

**Current mapping**: `AniError::DownloadOutputError { path, reason }`

---

## Playback Errors (ACL-5xxx)

<a id="ACL-5001"></a>
### ACL-5001 — Player not found

Configured player executable not found.

**Possible causes**
- Player not installed
- Player not in PATH
- Incorrect player path in configuration

**Try**
- Install the player
- Check player installation
- Update player path in configuration

**Current mapping**: `AniError::PlayerNotFound { executable }`

---

<a id="ACL-5002"></a>
### ACL-5002 — Player launch failed

Failed to launch the media player.

**Possible causes**
- Player execution failed
- Invalid player arguments
- Permission issues

**Try**
- Check player installation
- Try a different player
- Check player configuration

**Current mapping**: `AniError::PlayerLaunchFailed { executable, reason }`

---

<a id="ACL-5003"></a>
### ACL-5003 — Player exited with error

Media player exited with a non-zero status code.

**Possible causes**
- Player encountered an error
- Invalid media file
- Player configuration issue

**Try**
- Try a different player
- Try a different source
- Check player logs

**Current mapping**: `AniError::PlayerExitedWithError { executable, exit_code }`

---

<a id="ACL-5004"></a>
### ACL-5004 — Android terminal required

Android playback requires an interactive terminal.

**Possible causes**
- Running in non-interactive mode on Android
- Termux session not interactive

**Try**
- Use interactive terminal on Android
- Use download mode instead

**Current mapping**: `AniError::AndroidTerminalRequired`

---

<a id="ACL-5005"></a>
### ACL-5005 — General player error

General media player error.

**Possible causes**
- Player-specific issues
- Media format issues
- Player configuration problems

**Try**
- Try a different player
- Check player documentation
- Try a different source

**Current mapping**: `AniError::GeneralPlayerError { reason }`

---

## Filesystem / Configuration Errors (ACL-6xxx)

<a id="ACL-6001"></a>
### ACL-6001 — I/O error

General filesystem I/O error.

**Possible causes**
- Permission issues
- Disk full
- Invalid path
- Filesystem errors

**Try**
- Check file permissions
- Check disk space
- Verify paths are correct
- Check filesystem health

**Current mapping**: `AniError::IoError { source, context }`

---

<a id="ACL-6002"></a>
### ACL-6002 — History state directory error

Could not determine or create state directory for history.

**Possible causes**
- No valid state directory found
- Permission issues
- Invalid configuration

**Try**
- Check configuration
- Check directory permissions
- Verify app data directory

**Current mapping**: `AniError::HistoryStateDirectoryError`

---

<a id="ACL-6003"></a>
### ACL-6003 — History operation failed

History read/write operation failed.

**Possible causes**
- History file corrupted
- Permission issues
- Invalid history data

**Try**
- Check history file permissions
- Clear history if corrupted
- Check app data directory

**Current mapping**: `AniError::HistoryOperationFailed { operation, reason }`

---

<a id="ACL-6004"></a>
### ACL-6004 — Invalid history entry

History entry contains invalid data.

**Possible causes**
- History file corrupted
- Invalid data format

**Try**
- Clear history file
- Rebuild history

**Current mapping**: `AniError::InvalidHistoryEntry { reason }`

---

## Authentication / External Services Errors (ACL-7xxx)

<a id="ACL-7001"></a>
### ACL-7001 — Update check failed

Failed to check for updates.

**Possible causes**
- Network issues
- GitHub API unavailable
- Invalid version format

**Try**
- Check your connection
- Try again later
- Manually check for updates

**Current mapping**: `AniError::UpdateCheckFailed { reason }`

---

<a id="ACL-7002"></a>
### ACL-7002 — Invalid release tag

GitHub returned an unsafe or invalid release tag.

**Possible causes**
- GitHub API issue
- Invalid release format

**Try**
- Try again later
- Manually check releases

**Current mapping**: `AniError::InvalidReleaseTag { tag }`

---

<a id="ACL-7003"></a>
### ACL-7003 — Installer execution failed

Update installer failed to execute.

**Possible causes**
- Installer execution failed
- Permission issues
- Invalid installer

**Try**
- Run installer manually
- Check permissions
- Try manual update

**Current mapping**: `AniError::InstallerExecutionFailed { exit_code }`

---

<a id="ACL-7004"></a>
### ACL-7004 — Platform not supported

Update not supported for current platform.

**Possible causes**
- No official releases for platform
- Platform-specific limitations

**Try**
- Build from source
- Use alternative update method

**Current mapping**: `AniError::PlatformNotSupported { platform }`

---

## Internal / Unexpected Errors (ACL-9xxx)

<a id="ACL-9001"></a>
### ACL-9001 — Internal error

ani-cli-rs encountered an unexpected internal error.

**Possible causes**
- Bug in ani-cli-rs
- Unexpected state
- Internal invariant violation

**Try**
- Run again with `--verbose` flag
- Report the issue with full error details
- Include the error code and output

**Current mapping**: `AniError::InternalError { message }`

---

<a id="ACL-9002"></a>
### ACL-9002 — JSON parsing error

Failed to parse JSON data.

**Possible causes**
- Invalid JSON format
- Unexpected JSON structure
- Encoding issues

**Try**
- Try again later
- Report if it persists

**Current mapping**: `AniError::JsonParsingError { source, context }`

---

<a id="ACL-9003"></a>
### ACL-9003 — URL parsing error

Failed to parse URL.

**Possible causes**
- Invalid URL format
- Malformed URL
- Encoding issues

**Try**
- Try again later
- Report if it persists

**Current mapping**: `AniError::UrlParsingError { source, context }`

---

<a id="ACL-9004"></a>
### ACL-9004 — Invalid input

General invalid input error.

**Possible causes**
- Invalid user input
- Invalid configuration
- Invalid parameters

**Try**
- Check your input
- Check configuration
- Try with valid parameters

**Current mapping**: `AniError::InvalidInput { input, reason }`

---

## Error Code Stability

Once an error code is released, its meaning will not change. New error codes may be added, but existing codes will remain stable. This allows:

- Documentation to remain accurate
- Scripts to rely on specific error codes
- Support triage to use error codes effectively
- Library consumers to handle errors predictably

## Using Error Codes Programmatically

Library consumers can use error codes for programmatic error handling:

```rust
use ani_lib::{AniError, ErrorCode};

match result {
    Err(error) if error.code() == ErrorCode::NoPlayableSources => {
        // Handle no sources case
        println!("No sources available, trying fallback...");
    }
    Err(error) if error.code() == ErrorCode::ProviderUnavailable => {
        // Handle provider unavailable case
        println!("Provider unavailable, switching providers...");
    }
    Err(error) => {
        // Handle other errors
        eprintln!("Error: {}", error);
    }
    Ok(_) => println!("Success!")
}
```

## Error Output Examples

### Normal mode (default)
Compact, user-friendly error messages with helpful suggestions.

```
error[ACL-3001]: No playable sources found

No playable sources were found for episode 7.

help: Try another provider or try again later.
docs: https://vorlie.github.io/ani-cli-rs/errors#ACL-3001
```

### Verbose mode (`--verbose`)
Includes additional context information and error chains.

```
error[ACL-3001]: No playable sources found

No playable sources were found for episode 7.

Provider: Anikoto
Episode: 7
Mode: sub

Caused by:
  provider returned no playable streams

help: Try another provider or try again later.
docs: https://vorlie.github.io/ani-cli-rs/errors#ACL-3001
```

### Debug mode (`--debug`)
Full diagnostic information with detailed error chains and internal state.

```
error[ACL-3001]

Debug information:
  provider = anikoto
  episode = 7
  error_code = ACL-3001
  error_slug = no-playable-sources

Error chain:
  [0]: No playable sources found
  [1]: provider returned no playable streams

User message:
  No playable sources found
  No playable sources were found for episode 7.

Help:
  Try another provider or try again later.

Documentation: https://vorlie.github.io/ani-cli-rs/errors/no-playable-sources
```

## Reporting Errors

When reporting an error, please include:

1. The error code (e.g., `ACL-3001`)
2. The complete error message
3. ani-cli-rs version
4. Operating system
5. Provider being used (if applicable)
6. `--verbose` output if available

This helps in quickly identifying and resolving issues.
