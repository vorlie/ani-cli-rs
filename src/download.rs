use std::path::{Path, PathBuf};

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
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(&partial)
        .await?;
    let mut chunks = response.bytes_stream();
    while let Some(chunk) = chunks.next().await {
        file.write_all(&chunk?).await?;
    }
    file.flush().await?;
    drop(file);
    if tokio::fs::try_exists(target).await? {
        tokio::fs::remove_file(target).await?;
    }
    tokio::fs::rename(partial, target).await?;
    Ok(())
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
}
