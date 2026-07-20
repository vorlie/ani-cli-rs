use std::{
    io::{IsTerminal, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use futures_util::StreamExt;
use reqwest::header;
use tokio::{fs::OpenOptions, io::AsyncWriteExt, process::Command};

use crate::{AniError, Result, StreamLink};

#[derive(Clone, Debug)]
pub struct DownloadOptions {
    pub directory: PathBuf,
    pub filename: String,
}

pub async fn download_stream(stream: &StreamLink, options: &DownloadOptions) -> Result<PathBuf> {
    tokio::fs::create_dir_all(&options.directory).await?;
    let filename = sanitize_filename(&options.filename);
    let target = options.directory.join(format!("{filename}.mp4"));
    if stream.hls {
        download_hls(stream, &target).await?;
        return Ok(target);
    }

    let partial = target.with_extension("mp4.part");
    match run_tool("aria2c", &aria2_args(stream, &partial)).await {
        ToolAttempt::Success => match finalize_partial(&partial, &target).await {
            Ok(()) => return Ok(target),
            Err(error) => eprintln!(
                "aria2c finished but its output could not be finalized ({error}); falling back to the built-in downloader..."
            ),
        },
        ToolAttempt::Failed(error) => {
            eprintln!("aria2c failed ({error}); falling back to the built-in downloader...");
        }
        ToolAttempt::Unavailable => {}
    }
    download_direct(stream, &target).await?;
    Ok(target)
}

async fn download_hls(stream: &StreamLink, target: &Path) -> Result<()> {
    let mut failures = Vec::new();
    if program_available("aria2c").await {
        match run_tool("yt-dlp", &yt_dlp_args(stream, target, true)).await {
            ToolAttempt::Success => return Ok(()),
            ToolAttempt::Failed(error) => {
                eprintln!("yt-dlp with aria2c failed ({error}); retrying with yt-dlp...");
                failures.push(error);
            }
            ToolAttempt::Unavailable => {}
        }
    }

    match run_tool("yt-dlp", &yt_dlp_args(stream, target, false)).await {
        ToolAttempt::Success => return Ok(()),
        ToolAttempt::Failed(error) => {
            eprintln!("yt-dlp failed ({error}); falling back to FFmpeg...");
            failures.push(error);
        }
        ToolAttempt::Unavailable => {}
    }
    match run_tool("ffmpeg", &ffmpeg_args(stream, target)).await {
        ToolAttempt::Success => return Ok(()),
        ToolAttempt::Failed(error) => failures.push(error),
        ToolAttempt::Unavailable => {}
    }

    if failures.is_empty() {
        Err(AniError::Download(
            "HLS downloads require yt-dlp or ffmpeg in PATH".into(),
        ))
    } else {
        Err(AniError::Download(format!(
            "all available HLS downloaders failed: {}",
            failures.join("; ")
        )))
    }
}

async fn program_available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .await
        .is_ok()
}

#[derive(Debug, PartialEq, Eq)]
enum ToolAttempt {
    Unavailable,
    Success,
    Failed(String),
}

async fn run_tool(program: &str, args: &[String]) -> ToolAttempt {
    if !program_available(program).await {
        return ToolAttempt::Unavailable;
    }
    eprintln!("Downloading with {program} (progress is reported by {program})...");
    match Command::new(program).args(args).status().await {
        Ok(status) if status.success() => ToolAttempt::Success,
        Ok(status) => ToolAttempt::Failed(format!(
            "{program} exited with {}",
            status.code().unwrap_or(1)
        )),
        Err(error) => ToolAttempt::Failed(format!("could not start {program}: {error}")),
    }
}

fn yt_dlp_args(stream: &StreamLink, target: &Path, aria2: bool) -> Vec<String> {
    let mut args = Vec::new();
    if aria2 {
        args.extend(["--downloader".into(), "aria2c".into()]);
    }
    append_yt_dlp_headers(&mut args, stream);
    args.extend([
        "--no-skip-unavailable-fragments".into(),
        "--fragment-retries".into(),
        "infinite".into(),
        "--progress".into(),
        "-N".into(),
        "16".into(),
        "-o".into(),
        target.to_string_lossy().into_owned(),
        stream.url.clone(),
    ]);
    args
}

fn append_yt_dlp_headers(args: &mut Vec<String>, stream: &StreamLink) {
    if let Some(referer) = &stream.headers.referer {
        args.extend(["--referer".into(), referer.clone()]);
    }
    if let Some(origin) = &stream.headers.origin {
        args.extend(["--add-headers".into(), format!("Origin:{origin}")]);
    }
    for (name, value) in &stream.headers.extra {
        args.extend(["--add-headers".into(), format!("{name}:{value}")]);
    }
}

fn ffmpeg_args(stream: &StreamLink, target: &Path) -> Vec<String> {
    let mut args = vec!["-y".into(), "-extension_picky".into(), "0".into()];
    if let Some(referer) = &stream.headers.referer {
        args.extend(["-referer".into(), referer.clone()]);
    }
    let mut headers = Vec::new();
    if let Some(origin) = &stream.headers.origin {
        headers.push(format!("Origin: {origin}"));
    }
    headers.extend(
        stream
            .headers
            .extra
            .iter()
            .map(|(name, value)| format!("{name}: {value}")),
    );
    if !headers.is_empty() {
        args.extend(["-headers".into(), format!("{}\r\n", headers.join("\r\n"))]);
    }
    args.extend([
        "-loglevel".into(),
        "error".into(),
        "-stats".into(),
        "-i".into(),
        stream.url.clone(),
        "-c".into(),
        "copy".into(),
        target.to_string_lossy().into_owned(),
    ]);
    args
}

fn aria2_args(stream: &StreamLink, partial: &Path) -> Vec<String> {
    let directory = partial.parent().unwrap_or_else(|| Path::new("."));
    let filename = partial
        .file_name()
        .unwrap_or(partial.as_os_str())
        .to_string_lossy();
    let connections = aria2_connection_count(stream);
    let mut args = vec![
        "--continue=true".into(),
        format!("--max-connection-per-server={connections}"),
        format!("--split={connections}"),
        "--min-split-size=1M".into(),
        "--file-allocation=none".into(),
        "--auto-file-renaming=false".into(),
        "--allow-overwrite=true".into(),
        "--console-log-level=error".into(),
        "--download-result=hide".into(),
        "--summary-interval=1".into(),
        format!("--dir={}", directory.to_string_lossy()),
        format!("--out={filename}"),
    ];
    if let Some(referer) = &stream.headers.referer {
        args.push(format!("--referer={referer}"));
    }
    if let Some(origin) = &stream.headers.origin {
        args.push(format!("--header=Origin: {origin}"));
    }
    args.extend(
        stream
            .headers
            .extra
            .iter()
            .map(|(name, value)| format!("--header={name}: {value}")),
    );
    args.push(stream.url.clone());
    args
}

fn aria2_connection_count(stream: &StreamLink) -> u8 {
    let provider_is_mp4upload = stream.provider.to_ascii_lowercase().contains("mp4upload");
    let host_is_mp4upload = url::Url::parse(&stream.url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .is_some_and(|host| host == "mp4upload.com" || host.ends_with(".mp4upload.com"));
    if provider_is_mp4upload || host_is_mp4upload {
        4
    } else {
        16
    }
}

async fn download_direct(stream: &StreamLink, target: &Path) -> Result<()> {
    let partial = target.with_extension("mp4.part");
    let existing = tokio::fs::metadata(&partial)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    let client = reqwest::Client::builder().build()?;
    let mut request = client.get(&stream.url);
    if let Some(referer) = &stream.headers.referer {
        request = request.header(header::REFERER, referer);
    }
    if let Some(origin) = &stream.headers.origin {
        request = request.header(header::ORIGIN, origin);
    }
    for (name, value) in &stream.headers.extra {
        request = request.header(name, value);
    }
    if existing > 0 {
        request = request.header(header::RANGE, format!("bytes={existing}-"));
    }
    let response = request.send().await?;
    if !response.status().is_success() {
        return Err(AniError::Download(format!(
            "media server returned {}",
            response.status()
        )));
    }
    let append = existing > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    let initial = if append { existing } else { 0 };
    let total = response_total(&response, initial);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(&partial)
        .await?;
    let mut progress = DownloadProgress::new(initial, total);
    let mut chunks = response.bytes_stream();
    while let Some(chunk) = chunks.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        progress.advance(chunk.len() as u64);
    }
    file.flush().await?;
    progress.finish();
    drop(file);
    finalize_partial(&partial, target).await?;
    Ok(())
}

async fn finalize_partial(partial: &Path, target: &Path) -> Result<()> {
    if tokio::fs::try_exists(target).await? {
        tokio::fs::remove_file(target).await?;
    }
    tokio::fs::rename(partial, target).await?;
    let mut control = partial.as_os_str().to_os_string();
    control.push(".aria2");
    let control = PathBuf::from(control);
    if tokio::fs::try_exists(&control).await? {
        tokio::fs::remove_file(control).await?;
    }
    Ok(())
}

fn response_total(response: &reqwest::Response, initial: u64) -> Option<u64> {
    response
        .headers()
        .get(header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit_once('/'))
        .and_then(|(_, total)| total.parse().ok())
        .or_else(|| response.content_length().map(|length| initial + length))
}

struct DownloadProgress {
    initial: u64,
    downloaded: u64,
    total: Option<u64>,
    started: Instant,
    last_draw: Instant,
    last_width: usize,
    terminal: bool,
    finished: bool,
}

impl DownloadProgress {
    fn new(initial: u64, total: Option<u64>) -> Self {
        let now = Instant::now();
        let mut progress = Self {
            initial,
            downloaded: initial,
            total,
            started: now,
            last_draw: now.checked_sub(Duration::from_secs(1)).unwrap_or(now),
            last_width: 0,
            terminal: std::io::stderr().is_terminal(),
            finished: false,
        };
        progress.draw(false);
        progress
    }

    fn advance(&mut self, bytes: u64) {
        self.downloaded += bytes;
        let interval = if self.terminal {
            Duration::from_millis(125)
        } else {
            Duration::from_secs(5)
        };
        if self.last_draw.elapsed() >= interval {
            self.draw(false);
        }
    }

    fn finish(&mut self) {
        self.finished = true;
        self.draw(true);
    }

    fn draw(&mut self, finished: bool) {
        let elapsed = self.started.elapsed();
        let transferred = self.downloaded.saturating_sub(self.initial);
        let speed = if elapsed.as_secs_f64() > 0.0 {
            transferred as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };
        let line = progress_line(self.downloaded, self.total, speed, finished);
        let padding = " ".repeat(self.last_width.saturating_sub(line.len()));
        if self.terminal {
            eprint!("\r{line}{padding}");
            if finished {
                eprintln!();
            }
            let _ = std::io::stderr().flush();
        } else {
            eprintln!("{line}");
        }
        self.last_width = line.len();
        self.last_draw = Instant::now();
    }
}

impl Drop for DownloadProgress {
    fn drop(&mut self) {
        if self.terminal && !self.finished {
            eprintln!();
        }
    }
}

fn progress_line(downloaded: u64, total: Option<u64>, speed: f64, finished: bool) -> String {
    let state = if finished {
        "Downloaded"
    } else {
        "Downloading"
    };
    let speed_text = format!("{}/s", format_bytes(speed));
    match total {
        Some(total) if total > 0 => {
            let percent = (downloaded as f64 / total as f64 * 100.0).min(100.0);
            let remaining = total.saturating_sub(downloaded);
            let eta = if speed > 0.0 {
                format_duration(Duration::from_secs_f64(remaining as f64 / speed))
            } else {
                "--:--".into()
            };
            format!(
                "{state}: {percent:5.1}%  {} / {}  {speed_text}  ETA {eta}",
                format_bytes(downloaded as f64),
                format_bytes(total as f64)
            )
        }
        _ => format!("{state}: {}  {speed_text}", format_bytes(downloaded as f64)),
    }
}

fn format_bytes(bytes: f64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes.max(0.0);
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let hours = seconds / 3600;
    let minutes = seconds % 3600 / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn sanitize_filename(value: &str) -> String {
    let value: String = value
        .chars()
        .map(|c| if "<>:\"/\\|?*\0".contains(c) { '_' } else { c })
        .collect();
    let value = value.trim().trim_end_matches(['.', ' ']);
    if value.is_empty() {
        "episode".into()
    } else {
        value.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RequestHeaders;

    fn test_stream(hls: bool) -> StreamLink {
        StreamLink {
            url: "https://media.example/video".into(),
            resolution: "1080p".into(),
            hls,
            provider: "Example".into(),
            downloadable: true,
            headers: RequestHeaders {
                referer: Some("https://example.com/watch".into()),
                origin: Some("https://example.com".into()),
                extra: [("X-Test".into(), "value".into())].into(),
            },
        }
    }

    #[test]
    fn sanitizes_windows_filename_characters() {
        assert_eq!(sanitize_filename("A: B?"), "A_ B_");
    }

    #[test]
    fn formats_progress_with_speed_and_eta() {
        assert_eq!(
            progress_line(512 * 1024, Some(1024 * 1024), 128.0 * 1024.0, false),
            "Downloading:  50.0%  512.0 KiB / 1.0 MiB  128.0 KiB/s  ETA 00:04"
        );
    }

    #[test]
    fn formats_long_download_eta() {
        assert_eq!(format_duration(Duration::from_secs(3723)), "01:02:03");
    }

    #[test]
    fn aria2_direct_arguments_enable_parallel_resume_and_headers() {
        let args = aria2_args(&test_stream(false), Path::new("downloads/episode.mp4.part"));

        assert!(args.contains(&"--continue=true".into()));
        assert!(args.contains(&"--max-connection-per-server=16".into()));
        assert!(args.contains(&"--split=16".into()));
        assert!(args.contains(&"--console-log-level=error".into()));
        assert!(args.contains(&"--download-result=hide".into()));
        assert!(args.contains(&"--referer=https://example.com/watch".into()));
        assert!(args.contains(&"--header=Origin: https://example.com".into()));
        assert!(args.contains(&"--header=X-Test: value".into()));
        assert_eq!(
            args.last().map(String::as_str),
            Some("https://media.example/video")
        );
    }

    #[test]
    fn aria2_limits_mp4upload_parallel_connections() {
        let mut stream = test_stream(false);
        stream.url = "https://a4.mp4upload.com:183/d/token/video.mp4".into();
        stream.provider = "Mp4Upload".into();

        let args = aria2_args(&stream, Path::new("episode.mp4.part"));

        assert!(args.contains(&"--max-connection-per-server=4".into()));
        assert!(args.contains(&"--split=4".into()));
    }

    #[test]
    fn yt_dlp_can_delegate_hls_downloads_to_aria2() {
        let args = yt_dlp_args(&test_stream(true), Path::new("episode.mp4"), true);

        assert!(
            args.windows(2)
                .any(|args| args == ["--downloader", "aria2c"])
        );
        assert!(
            args.windows(2)
                .any(|args| args == ["--referer", "https://example.com/watch"])
        );
        assert!(
            args.windows(2)
                .any(|args| args == ["--add-headers", "Origin:https://example.com"])
        );
    }
}
