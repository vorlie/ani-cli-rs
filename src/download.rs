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
        if run_hls_tool(
            "yt-dlp",
            &[
                "--referer",
                stream.headers.referer.as_deref().unwrap_or(""),
                "--no-skip-unavailable-fragments",
                "--fragment-retries",
                "infinite",
                "--progress",
                "-N",
                "16",
                "-o",
                &target.to_string_lossy(),
                &stream.url,
            ],
        )
        .await?
        {
            return Ok(target);
        }
        if run_hls_tool(
            "ffmpeg",
            &[
                "-extension_picky",
                "0",
                "-referer",
                stream.headers.referer.as_deref().unwrap_or(""),
                "-loglevel",
                "error",
                "-stats",
                "-i",
                &stream.url,
                "-c",
                "copy",
                &target.to_string_lossy(),
            ],
        )
        .await?
        {
            return Ok(target);
        }
        return Err(AniError::Download(
            "HLS downloads require yt-dlp or ffmpeg in PATH".into(),
        ));
    }
    download_direct(stream, &target).await?;
    Ok(target)
}

async fn run_hls_tool(program: &str, args: &[&str]) -> Result<bool> {
    let available = Command::new(program)
        .arg("--version")
        .output()
        .await
        .is_ok();
    if !available {
        return Ok(false);
    }
    eprintln!("Downloading with {program} (progress is reported by {program})...");
    let status = Command::new(program)
        .args(args)
        .status()
        .await
        .map_err(|e| AniError::Download(format!("could not start {program}: {e}")))?;
    if !status.success() {
        return Err(AniError::Download(format!(
            "{program} exited with {}",
            status.code().unwrap_or(1)
        )));
    }
    Ok(true)
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
    if tokio::fs::try_exists(target).await? {
        tokio::fs::remove_file(target).await?;
    }
    tokio::fs::rename(partial, target).await?;
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
}
