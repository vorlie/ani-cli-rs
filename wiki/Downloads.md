# Downloads

Anikoto downloads use the same preflight and downloader fallback order as AllAnime. Select the catalog with `--provider anikoto`; self-identifying `anikoto:` IDs returned by search route automatically. MegaPlay/KotoCDN HLS is passed through ani-cli-rs's tokenized loopback relay for the lifetime of yt-dlp/FFmpeg so wrapped transport-stream fragments can be consumed without transcoding.

Native MegaPlay subtitle tracks are downloaded after the video. When FFmpeg is available, they are embedded into the MP4 as selectable `mov_text` tracks and the provider's default track is retained. Without FFmpeg—or when muxing fails—the subtitle files remain beside the video as sidecars rather than being discarded.

## Interactive download by anime name

```console
ani-cli-rs --download "anime title"
```

AllAnime generally exposes seasons as separate search results. Select the intended anime/season entry, then one or more episodes.

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

This subcommand does not search by name and does not open selectors. Obtain the ID with `search --json` or from a Mkissa `/anime/SHOW_ID` URL.

## Quality selection

```console
ani-cli-rs -d -q best "title"
ani-cli-rs -d -q worst "title"
ani-cli-rs -d -q 720p "title"
```

If an explicit resolution is missing, quality selection falls back to the best available downloadable stream for that episode.

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

## Why files use `.part`

Direct downloads are written as `Title Episode N.mp4.part`. After successful completion, ani-cli-rs atomically renames the file to `Title Episode N.mp4`.

This is intentional:

- incomplete media never looks finished;
- interrupted downloads can resume;
- the final filename appears only after successful finalization.

There is currently no option to write directly to the final name. If `.part` remains, the transfer was interrupted or finalization failed. Rerun the same download to resume when the provider link and downloader permit it.

Do not confuse the media `.part` file with aria2's own temporary `.aria2` control file.

## aria2 configuration

ani-cli-rs explicitly loads an existing config from:

- Windows: `%USERPROFILE%\.aria2\aria2.conf`
- Linux/macOS: `$HOME/.aria2/aria2.conf`

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

- the signed URL expired;
- the provider binds the URL to another IP;
- required referrer/origin headers were lost;
- the provider rejects the configured connection count.

Rerun source resolution rather than reusing an old URL. If aria2 fails, ani-cli-rs attempts its documented fallback where possible.
