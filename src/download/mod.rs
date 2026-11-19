use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use reqwest::header;
use tokio::{fs::OpenOptions, io::AsyncWriteExt, process::Command};

use crate::{
    AniError, Result, StreamLink, relay_stream_without_hls_subtitles, requires_hls_relay,
    util::{http::apply_headers, sanitize_filename},
};

mod progress;
mod subtitles;

use progress::DownloadProgress;
use subtitles::attach_subtitles;

#[derive(Clone, Debug)]
pub struct DownloadOptions {
    pub directory: PathBuf,
    pub filename: String,
}

pub async fn download_stream(stream: &StreamLink, options: &DownloadOptions) -> Result<PathBuf> {
    if requires_hls_relay(stream) {
        // Downloads never need subtitles exposed as synthetic HLS renditions:
        // subtitle tracks are fetched separately (via the relayed sidecar
        // URLs on `local.subtitles`) and muxed into the output with FFmpeg.
        let (_relay, local) = relay_stream_without_hls_subtitles(stream).await?;
        let target = download_stream_inner(&local, options).await?;
        attach_subtitles(&local, &target).await?;
        return Ok(target);
    }
    let target = download_stream_inner(stream, options).await?;
    attach_subtitles(stream, &target).await?;
    Ok(target)
}

async fn download_stream_inner(stream: &StreamLink, options: &DownloadOptions) -> Result<PathBuf> {
    tokio::fs::create_dir_all(&options.directory).await?;
    let filename = sanitize_filename(&options.filename, "episode");
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
        Err(AniError::NoDownloadTool)
    } else {
        eprintln!("HLS download failures: {}", failures.join("; "));
        Err(AniError::HlsDownloadFailed {
            reason: failures.join("; "),
        })
    }
}

pub(super) async fn program_available(program: &str) -> bool {
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
        if let Some(config) = aria2_config_path() {
            args.extend([
                "--downloader-args".into(),
                format!("aria2c:--conf-path=\"{}\"", config.to_string_lossy()),
            ]);
        }
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
        "--summary-interval=0".into(),
        format!("--dir={}", directory.to_string_lossy()),
        format!("--out={filename}"),
    ];
    if let Some(config) = aria2_config_path() {
        args.push(format!("--conf-path={}", config.to_string_lossy()));
    }
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

fn aria2_config_path() -> Option<PathBuf> {
    let profile = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    aria2_config_path_from(profile)
}

fn aria2_config_path_from(profile: Option<std::ffi::OsString>) -> Option<PathBuf> {
    let path = PathBuf::from(profile?).join(".aria2").join("aria2.conf");
    path.is_file().then_some(path)
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
    let mut request = apply_headers(client.get(&stream.url), &stream.headers);
    if existing > 0 {
        request = request.header(header::RANGE, format!("bytes={existing}-"));
    }
    let response = request.send().await?;
    if !response.status().is_success() {
        return Err(AniError::DownloadFailed {
            reason: format!("media server returned {}", response.status()),
            source: None,
        });
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
            subtitles: vec![],
        }
    }

    #[test]
    fn sanitizes_windows_filename_characters() {
        assert_eq!(sanitize_filename("A: B?", "episode"), "A_ B_");
    }

    #[test]
    fn aria2_direct_arguments_enable_parallel_resume_and_headers() {
        let args = aria2_args(&test_stream(false), Path::new("downloads/episode.mp4.part"));

        assert!(args.contains(&"--continue=true".into()));
        assert!(args.contains(&"--max-connection-per-server=16".into()));
        assert!(args.contains(&"--split=16".into()));
        assert!(args.contains(&"--console-log-level=error".into()));
        assert!(args.contains(&"--download-result=hide".into()));
        assert!(args.contains(&"--summary-interval=0".into()));
        assert!(args.contains(&"--referer=https://example.com/watch".into()));
        assert!(args.contains(&"--header=Origin: https://example.com".into()));
        assert!(args.contains(&"--header=X-Test: value".into()));
        assert_eq!(
            args.last().map(String::as_str),
            Some("https://media.example/video")
        );
    }

    #[test]
    fn discovers_aria2_config_below_the_user_profile() {
        let profile = tempfile::tempdir().unwrap();
        let config_directory = profile.path().join(".aria2");
        std::fs::create_dir(&config_directory).unwrap();
        let config = config_directory.join("aria2.conf");
        std::fs::write(&config, "max-download-limit=1M\n").unwrap();

        assert_eq!(
            aria2_config_path_from(Some(profile.path().as_os_str().to_owned())),
            Some(config)
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
