use std::path::{Path, PathBuf};

use tokio::process::Command;

use crate::{
    RequestHeaders, Result, StreamLink, SubtitleTrack,
    util::{
        http::{fetch_subtitle_bytes, subtitle_extension},
        sanitize_filename,
    },
};

use super::program_available;

const MAX_SUBTITLE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug)]
struct DownloadedSubtitle {
    path: PathBuf,
    label: String,
    default: bool,
}

pub(super) async fn attach_subtitles(stream: &StreamLink, target: &Path) -> Result<()> {
    if stream.subtitles.is_empty() {
        return Ok(());
    }
    let mut subtitles = Vec::new();
    for (index, track) in stream.subtitles.iter().enumerate() {
        match download_subtitle_track(track, &stream.headers, target, index).await {
            Ok(subtitle) => subtitles.push(subtitle),
            Err(error) => eprintln!("Could not download subtitle '{}': {error}", track.label),
        }
    }
    if subtitles.is_empty() {
        eprintln!("No provider subtitle tracks could be downloaded.");
        return Ok(());
    }
    if !program_available("ffmpeg").await {
        eprintln!(
            "FFmpeg is unavailable; saved {} subtitle track(s) beside the video.",
            subtitles.len()
        );
        return Ok(());
    }

    let stem = target
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("episode");
    let muxed = target.with_file_name(format!("{stem}.subtitled.part.mp4"));
    let args = subtitle_mux_args(target, &subtitles, &muxed);
    let status = Command::new("ffmpeg").args(&args).status().await;
    match status {
        Ok(status) if status.success() => {
            if tokio::fs::try_exists(target).await? {
                tokio::fs::remove_file(target).await?;
            }
            tokio::fs::rename(&muxed, target).await?;
            for subtitle in subtitles {
                let _ = tokio::fs::remove_file(subtitle.path).await;
            }
            eprintln!("Embedded provider subtitles into the downloaded MP4.");
        }
        Ok(status) => {
            let _ = tokio::fs::remove_file(&muxed).await;
            eprintln!(
                "FFmpeg subtitle muxing exited with {}; subtitle sidecars were kept.",
                status.code().unwrap_or(1)
            );
        }
        Err(error) => {
            let _ = tokio::fs::remove_file(&muxed).await;
            eprintln!(
                "Could not start FFmpeg for subtitle muxing ({error}); subtitle sidecars were kept."
            );
        }
    }
    Ok(())
}

async fn download_subtitle_track(
    track: &SubtitleTrack,
    headers: &RequestHeaders,
    target: &Path,
    index: usize,
) -> Result<DownloadedSubtitle> {
    let bytes = fetch_subtitle_bytes(track, headers, MAX_SUBTITLE_BYTES).await?;
    let stem = target
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("episode");
    let label = sanitize_filename(&track.label, "episode");
    let extension = subtitle_extension(&track.url);
    let path = target.with_file_name(format!("{stem}.{index}.{label}.{extension}"));
    tokio::fs::write(&path, bytes).await?;
    Ok(DownloadedSubtitle {
        path,
        label: track.label.clone(),
        default: track.default,
    })
}

fn subtitle_mux_args(
    target: &Path,
    subtitles: &[DownloadedSubtitle],
    output: &Path,
) -> Vec<String> {
    let mut args = vec![
        "-y".into(),
        "-loglevel".into(),
        "error".into(),
        "-stats".into(),
        "-i".into(),
        target.to_string_lossy().into_owned(),
    ];
    for subtitle in subtitles {
        args.extend(["-i".into(), subtitle.path.to_string_lossy().into_owned()]);
    }
    args.extend(["-map".into(), "0:v?".into(), "-map".into(), "0:a?".into()]);
    for index in 0..subtitles.len() {
        args.extend(["-map".into(), format!("{}:0", index + 1)]);
    }
    args.extend([
        "-c:v".into(),
        "copy".into(),
        "-c:a".into(),
        "copy".into(),
        "-c:s".into(),
        "mov_text".into(),
    ]);
    let default_index = subtitles.iter().position(|subtitle| subtitle.default);
    for (index, subtitle) in subtitles.iter().enumerate() {
        args.extend([
            format!("-metadata:s:s:{index}"),
            format!("title={}", subtitle.label),
            format!("-metadata:s:s:{index}"),
            format!("language={}", subtitle_language_code(&subtitle.label)),
            format!("-disposition:s:{index}"),
            if Some(index) == default_index || (default_index.is_none() && index == 0) {
                "default".into()
            } else {
                "0".into()
            },
        ]);
    }
    args.push(output.to_string_lossy().into_owned());
    args
}

fn subtitle_language_code(label: &str) -> &'static str {
    let normalized = label.to_ascii_lowercase();
    let label = normalized
        .split(['(', '[', '-', '_'])
        .next()
        .unwrap_or(&normalized)
        .trim();
    match label {
        "english" | "en" => "eng",
        "polish" | "polski" | "pl" => "pol",
        "spanish" | "español" | "espanol" | "es" => "spa",
        "portuguese" | "português" | "portugues" | "pt" => "por",
        "french" | "français" | "francais" | "fr" => "fra",
        "german" | "deutsch" | "de" => "deu",
        "italian" | "italiano" | "it" => "ita",
        "japanese" | "日本語" | "ja" => "jpn",
        "korean" | "한국어" | "ko" => "kor",
        "chinese" | "中文" | "zh" => "zho",
        "arabic" | "العربية" | "ar" => "ara",
        "russian" | "русский" | "ru" => "rus",
        "ukrainian" | "українська" | "uk" => "ukr",
        "turkish" | "türkçe" | "turkce" | "tr" => "tur",
        "indonesian" | "bahasa indonesia" | "id" => "ind",
        "vietnamese" | "tiếng việt" | "tieng viet" | "vi" => "vie",
        "thai" | "ไทย" | "th" => "tha",
        "hindi" | "हिन्दी" | "hi" => "hin",
        "dutch" | "nederlands" | "nl" => "nld",
        "czech" | "čeština" | "cestina" | "cs" => "ces",
        "romanian" | "română" | "romana" | "ro" => "ron",
        "hungarian" | "magyar" | "hu" => "hun",
        _ => "und",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtitle_mux_maps_tracks_and_selects_the_provider_default() {
        let subtitles = vec![
            DownloadedSubtitle {
                path: "English.vtt".into(),
                label: "English".into(),
                default: false,
            },
            DownloadedSubtitle {
                path: "Polish.ass".into(),
                label: "Polish".into(),
                default: true,
            },
        ];
        let args = subtitle_mux_args(
            Path::new("episode.mp4"),
            &subtitles,
            Path::new("episode.subtitled.part.mp4"),
        );
        assert!(args.windows(2).any(|args| args == ["-map", "1:0"]));
        assert!(args.windows(2).any(|args| args == ["-map", "2:0"]));
        assert!(
            args.windows(2)
                .any(|args| args == ["-disposition:s:1", "default"])
        );
        assert!(args.contains(&"-c:s".into()));
        assert!(args.contains(&"mov_text".into()));
        assert!(args.contains(&"language=eng".into()));
        assert!(args.contains(&"language=pol".into()));
    }

    #[test]
    fn subtitle_extensions_are_restricted_to_supported_text_formats() {
        assert_eq!(
            subtitle_extension("https://cdn.example/subtitle.srt?token=1"),
            "srt"
        );
        assert_eq!(
            subtitle_extension("https://cdn.example/subtitle.ass"),
            "ass"
        );
        assert_eq!(subtitle_extension("http://127.0.0.1:1/r/token"), "vtt");
    }

    #[test]
    fn subtitle_labels_map_to_mp4_language_codes() {
        assert_eq!(subtitle_language_code("English (CC)"), "eng");
        assert_eq!(subtitle_language_code("Português-BR"), "por");
        assert_eq!(subtitle_language_code("Unknown Provider Label"), "und");
    }
}
