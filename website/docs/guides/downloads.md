# Downloads

Both Anikoto providers use the same preflight and downloader fallback order. Select Anikoto.cz with `--provider anikoto2`; self-identifying `anikoto:` and `anikoto2:` IDs route automatically.

MegaPlay/KotoCDN HLS is passed through ani-cli-rs's tokenized loopback relay for the lifetime of yt-dlp/FFmpeg so wrapped transport-stream fragments can be consumed without transcoding.

## Interactive download by anime name

```console
ani-cli-rs --download "anime title"
```

Catalogs generally expose seasons as separate search results. Select the intended anime/season entry, then one or more episodes.

From version 0.6.0, ani-cli-rs preflights the complete selection before starting a transfer:

1. resolve each episode sequentially;
2. discard streams not marked downloadable;
3. apply the requested quality independently to each episode;
4. if every episode resolves, cache the selected links and start downloads in episode order;
5. if an episode is unavailable, start no downloads and update no history.

Prompted terminal sessions return to the episode picker after availability failures. Explicit `-e/--episode`, `-r/--range`, continuation selections, and non-terminal runs fail instead of prompting.

Preflight means provider resolution, not an extra HTTP `HEAD` request. Some hosts reject probes that are not real media requests.

## Scriptable download by show ID

```console
ani-cli-rs download SHOW_ID 1 --title "Anime title" --output ./Downloads
```

This subcommand does not search by name and does not open selectors. Obtain the provider-prefixed ID with `search --json`.

For automation, `links --json` can also be used to inspect the exact stream selected for an episode, including subtitle tracks exposed by the provider.

```console
ani-cli-rs links SHOW_ID 1 --mode sub -q 1080p --json
```

The JSON output contains the resolved video URL, resolution, HLS status, provider, downloadability, required request headers, and any subtitle tracks exposed by the provider.

Resolved URLs may be signed or temporary. Avoid posting complete `links --json` output in public issues.

## Quality selection

```console
ani-cli-rs -d -q best "title"
ani-cli-rs -d -q worst "title"
ani-cli-rs -d -q 720p "title"
```

If an explicit resolution is missing, quality selection falls back to the best available downloadable stream for that episode.

For scriptable downloads, quality can be passed directly to the `download` or `links` subcommand:

```console
ani-cli-rs download SHOW_ID 1 -q 1080p --title "Anime title"
ani-cli-rs links SHOW_ID 1 -q 1080p --json
```

## Download directory

Compatibility workflow:

```powershell
$env:ANI_CLI_DOWNLOAD_DIR = "$HOME\Downloads\Anime"
ani-cli-rs -d "title"
```

```sh
ANI_CLI_DOWNLOAD_DIR="$HOME/Downloads/Anime" ani-cli-rs -d "title"
```

Scriptable workflow uses `--output`.

## Downloader order

### Direct media

1. `aria2c` with parallel connections and resume support;
2. built-in Rust streaming downloader if aria2c is missing or fails.

Providers known to reject excessive parallel requests, including Mp4Upload, receive a lower connection limit.

### HLS

1. yt-dlp with aria2c as external downloader when both exist;
2. yt-dlp without aria2c;
3. FFmpeg;
4. error if none succeeds.

Install the tools through the operating system and ensure their executable names are in `PATH`.

## Subtitle tracks

Subtitle availability depends on the provider and the selected stream. Some streams expose external subtitle tracks, while others provide no subtitle metadata at all.

When subtitle tracks are exposed, `links --json` includes them in the `subtitles` array:

```json
{
  "label": "English (CR English)",
  "url": "https://example.invalid/subtitles/track.vtt",
  "default": true
}
```

The exact URL is provider-generated and may be temporary.

### Anikoto subtitle tracks

Native MegaPlay sources can provide several external subtitle tracks. ani-cli-rs attempts to download every subtitle track exposed by the selected source; there is intentionally no language picker before downloading.

When FFmpeg is available, downloaded subtitles are embedded into the final MP4 as selectable `mov_text` tracks. Each track receives:

* its complete provider label as the track title;
* an ISO 639 language tag when the provider label identifies a supported language;
* the provider-designated default status, or the first track as a fallback default.

Language tags allow players such as VLC to display `English`, `Polish`, or another language instead of generic names such as `Track 1`. Unknown provider labels remain embedded with their original title and an undetermined-language tag.

If FFmpeg is unavailable or subtitle muxing fails, the completed video is preserved and downloaded subtitles remain beside it as sidecar files. The CLI reports this fallback.

Streams without external subtitle metadata are downloaded unchanged.

## Downloading a subtitle separately

If a provider exposes subtitles but the normal download does not produce the subtitle file you want, use `links --json` to retrieve the subtitle metadata and download the track separately.

For example, in PowerShell:

```powershell
$links = ani-cli-rs links "SHOW_ID" 10 --mode sub -q 1080p --json |
    ConvertFrom-Json

$link = $links |
    Where-Object { $_.downloadable -and $_.resolution -eq "1080p" } |
    Select-Object -First 1

if ($null -eq $link) {
    throw "No downloadable 1080p stream was exposed."
}

$subtitle = $link.subtitles |
    Where-Object { $_.label -match '^English' } |
    Select-Object -First 1

if ($null -eq $subtitle) {
    throw "No English subtitle was exposed by the provider."
}

curl.exe -L `
    -H "Referer: $($link.headers.referer)" `
    -H "User-Agent: $($link.headers.extra.'User-Agent')" `
    -o "Episode 10.en.vtt" `
    $subtitle.url
```

The important part is using the request headers returned alongside the stream. Some subtitle hosts reject requests that omit the provider's `Referer` or `User-Agent`.

This approach is useful when you want to choose a specific language rather than keeping every track exposed by the provider.

## WebVTT and SRT sidecars

Providers may expose subtitles as WebVTT (`.vtt`). The format does not need to be converted for normal playback: Jellyfin and modern media players can use WebVTT sidecars directly.

For example:

```text
Kimi ga Shinu made Koi wo Shitai Episode 10.mp4
Kimi ga Shinu made Koi wo Shitai Episode 10.en.vtt
```

For broader compatibility or archival purposes, WebVTT can be converted to SRT with FFmpeg:

```console
ffmpeg -i "Episode 10.en.vtt" "Episode 10.en.srt"
```

A sidecar subtitle is often the simplest option when the original video should remain untouched.

## Embedding subtitles into MKV

For a library such as Jellyfin, you can also mux the video and subtitles into an MKV container.

```console
ffmpeg -i "Episode 10.mp4" ^
       -i "Episode 10.en.vtt" ^
       -map 0 -map 1 ^
       -c copy ^
       -c:s srt ^
       -metadata:s:s:0 language=eng ^
       -metadata:s:s:0 title="English" ^
       "Episode 10.mkv"
```

On PowerShell, the same command can be written using backticks:

```powershell
ffmpeg -i "Episode 10.mp4" `
       -i "Episode 10.en.vtt" `
       -map 0 -map 1 `
       -c copy `
       -c:s srt `
       -metadata:s:s:0 language=eng `
       -metadata:s:s:0 title="English" `
       "Episode 10.mkv"
```

The video and audio streams are copied without re-encoding. Only the subtitle stream is converted to the Matroska-compatible SRT format.

For multiple subtitle files, add another `-i`, `-map`, and subtitle metadata entry for each track.

Do not use `mov_text` when creating an MKV. `mov_text` is an MP4 subtitle codec; MKV should use formats such as SRT or ASS.

## Embedding subtitles into MP4

If keeping MP4 is important, WebVTT can instead be converted to MP4-compatible `mov_text`:

```powershell
ffmpeg -i "Episode 10.mp4" `
       -i "Episode 10.en.vtt" `
       -map 0 -map 1 `
       -c copy `
       -c:s mov_text `
       -metadata:s:s:0 language=eng `
       -metadata:s:s:0 title="English" `
       "Episode 10.subbed.mp4"
```

This also copies the original video and audio without re-encoding.

## Why files use `.part`

Direct downloads are written as `Title Episode N.mp4.part`. After successful completion, ani-cli-rs atomically renames the file to `Title Episode N.mp4`.

This is intentional:

* incomplete media never looks finished;
* interrupted downloads can resume;
* the final filename appears only after successful finalization.

There is currently no option to write directly to the final name. If `.part` remains, the transfer was interrupted or finalization failed. Rerun the same download to resume when the provider link and downloader permit it.

Do not confuse the media `.part` file with aria2's own temporary `.aria2` control file.

## aria2 configuration

ani-cli-rs explicitly loads an existing config from:

* Windows: `%USERPROFILE%\.aria2\aria2.conf`
* Linux/macOS: `$HOME/.aria2/aria2.conf`

The file is optional. Required command-line arguments for output naming, resume behavior, connection safety, and provider headers override conflicting config entries.

Example configuration:

```ini
console-log-level=warn
summary-interval=0
file-allocation=none
```

Do not force a global split/connection count that conflicts with hosts enforcing low parallel limits.

## HTTP 403 from aria2

A few failed connection attempts do not necessarily mean the whole aria2 download failed; segmented downloads can retry and still complete. Check the final status and whether ani-cli-rs successfully renamed `.part` to `.mp4`.

Persistent 403 errors can mean:

* the signed URL expired;
* the provider binds the URL to another IP;
* required referrer/origin headers were lost;
* the provider rejects the configured connection count.

Rerun source resolution rather than reusing an old URL. If aria2 fails, ani-cli-rs attempts its documented fallback where possible.
