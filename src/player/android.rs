use std::{
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
};

use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::{AniError, Result, StreamLink};

pub(super) fn android_intent_args(component: &str, url: &str, title: &str) -> Vec<String> {
    vec![
        "start".into(),
        "--user".into(),
        "0".into(),
        "-a".into(),
        "android.intent.action.VIEW".into(),
        "-d".into(),
        url.into(),
        "-n".into(),
        component.into(),
        "--es".into(),
        "title".into(),
        title.into(),
    ]
}

pub(super) fn android_intent_launcher() -> PathBuf {
    if let Some(executable) = std::env::var_os("ANI_CLI_PLAYER") {
        return PathBuf::from(executable);
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    ["termux-am-starter", "termux-am", "am"]
        .iter()
        .find_map(|name| find_in_path(name, &path))
        .unwrap_or_else(|| PathBuf::from("termux-am-starter"))
}

pub(super) async fn play_android(
    executable: &Path,
    launch_args: Vec<String>,
    stream: &StreamLink,
    title: &str,
    relay_active: bool,
) -> Result<Option<i32>> {
    let terminal = io::stdin().is_terminal();
    if relay_active && !terminal {
        return Err(AniError::PlayerAndroidTerminalRequired);
    }

    info!(
        title = %title,
        executable = %executable.display(),
        "launching Android activity for playback",
    );
    debug!(
        title = %title,
        stream_url = %stream.url,
        args = ?launch_args,
        "full Android intent command line",
    );
    let launch_result = Command::new(executable).args(launch_args).status().await;
    let code = match launch_result {
        Ok(status) if status.success() => {
            let code = status.code().unwrap_or(0);
            info!(
                title = %title,
                exit_code = code,
                "Android player activity returned successfully",
            );
            code
        }
        result => {
            let primary_error = match &result {
                Ok(status) => format!(
                    "Android activity launcher {} exited with {}",
                    executable.display(),
                    status.code().unwrap_or(1)
                ),
                Err(error) => format!(
                    "could not launch Android player through {}: {error}",
                    executable.display()
                ),
            };
            warn!(
                title = %title,
                executable = %executable.display(),
                error = %primary_error,
                "primary Android launch failed; trying termux-open fallback",
            );
            match launch_android_url_fallback(executable, &stream.url, stream.hls).await {
                Ok(code) => {
                    warn!(
                        title = %title,
                        "opened the stream through Android's default URL handler instead",
                    );
                    eprintln!(
                        "warning: {primary_error}; opened the stream through Android's default URL handler instead"
                    );
                    code
                }
                Err(fallback_error) => {
                    warn!(
                        title = %title,
                        error = %fallback_error,
                        "termux-open fallback also failed",
                    );
                    eprintln!("{primary_error}; {fallback_error}");
                    return Err(AniError::PlayerLaunchFailed {
                        executable: executable.display().to_string(),
                        reason: format!("Android launch failed: {}", fallback_error),
                    });
                }
            }
        }
    };

    if terminal {
        debug!(
            title = %title,
            "waiting for the Android player to finish before returning control to the TUI",
        );
        wait_for_android_player().await?;
    }
    Ok(Some(code))
}

fn find_in_path(executable: &str, path: &std::ffi::OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .map(|directory| directory.join(executable))
        .find(|candidate| candidate.is_file())
}

async fn launch_android_url_fallback(
    executable: &Path,
    url: &str,
    hls: bool,
) -> std::result::Result<i32, String> {
    if !is_termux_activity_launcher(executable) {
        return Err(
            "the configured custom launcher failed and cannot use the automatic Termux fallback"
                .into(),
        );
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut open_error = None;
    if let Some(opener) = find_in_path("termux-open", &path) {
        let status = Command::new(&opener)
            .args(["--view", "--content-type", android_media_type(hls), url])
            .status()
            .await;
        match status {
            Ok(status) if status.success() => return Ok(status.code().unwrap_or(0)),
            Ok(status) => {
                open_error = Some(format!(
                    "{} exited with {}",
                    opener.display(),
                    status.code().unwrap_or(1)
                ));
            }
            Err(error) => {
                open_error = Some(format!("could not run {}: {error}", opener.display()));
            }
        }
    }
    let opener = find_in_path("termux-open-url", &path).ok_or_else(|| {
        let prefix = open_error
            .map(|error| format!("{error}; "))
            .unwrap_or_default();
        format!(
            "{prefix}termux-open-url is unavailable; install or update the termux-tools package"
        )
    })?;
    let status = Command::new(&opener)
        .arg(url)
        .status()
        .await
        .map_err(|error| format!("could not run {}: {error}", opener.display()))?;
    if !status.success() {
        return Err(format!(
            "{} exited with {}",
            opener.display(),
            status.code().unwrap_or(1)
        ));
    }
    Ok(status.code().unwrap_or(0))
}

fn android_media_type(hls: bool) -> &'static str {
    if hls {
        "application/vnd.apple.mpegurl"
    } else {
        "video/mp4"
    }
}

fn is_termux_activity_launcher(executable: &Path) -> bool {
    executable
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "termux-am-starter" | "termux-am" | "am"))
}

async fn wait_for_android_player() -> Result<()> {
    tokio::task::spawn_blocking(|| {
        println!(
            "Opened the Android player. Return to Termux and press Enter after playback ends."
        );
        print!("Waiting for Android player... ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok::<(), io::Error>(())
    })
    .await
    .map_err(|error| {
        eprintln!("Android playback prompt failed: {error}");
        AniError::PlayerLaunchFailed {
            executable: "android".to_string(),
            reason: format!("playback prompt failed: {}", error),
        }
    })??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Player, PlayerKind, PlayerOptions, RequestHeaders};

    #[test]
    fn android_mpv_arguments_use_an_explicit_view_intent() {
        let player = Player::new(PlayerOptions {
            executable: "termux-am-starter".into(),
            kind: PlayerKind::AndroidMpv,
            no_detach: true,
            exit_after_play: false,
            force_hls_relay: false,
        });
        let stream = StreamLink {
            url: "http://127.0.0.1:43123/stream-token".into(),
            resolution: "1080p".into(),
            hls: true,
            provider: "Anikoto".into(),
            downloadable: true,
            headers: RequestHeaders::default(),
            subtitles: vec![],
        };

        assert_eq!(
            player.command_args(&stream, "Anime Episode 1"),
            vec![
                "start",
                "--user",
                "0",
                "-a",
                "android.intent.action.VIEW",
                "-d",
                "http://127.0.0.1:43123/stream-token",
                "-n",
                "is.xyz.mpv/.MPVActivity",
                "--es",
                "title",
                "Anime Episode 1",
            ]
        );
    }

    #[test]
    fn android_vlc_arguments_target_the_android_app_not_terminal_vlc() {
        let player = Player::new(PlayerOptions {
            executable: "am".into(),
            kind: PlayerKind::AndroidVlc,
            no_detach: true,
            exit_after_play: false,
            force_hls_relay: false,
        });
        let stream = StreamLink {
            url: "https://media.example/episode.m3u8".into(),
            resolution: "720p".into(),
            hls: true,
            provider: "Anikoto".into(),
            downloadable: true,
            headers: RequestHeaders::default(),
            subtitles: vec![],
        };

        assert!(
            player.command_args(&stream, "Episode").contains(
                &"org.videolan.vlc/org.videolan.vlc.gui.video.VideoPlayerActivity".into()
            )
        );
    }

    #[test]
    fn android_launcher_lookup_prefers_the_first_available_candidate() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("termux-am"), "").unwrap();
        let path = std::env::join_paths([directory.path()]).unwrap();

        assert_eq!(
            find_in_path("termux-am", &path),
            Some(directory.path().join("termux-am"))
        );
        assert_eq!(find_in_path("termux-am-starter", &path), None);
    }

    #[test]
    fn only_termux_activity_launchers_allow_the_url_opener_fallback() {
        assert!(is_termux_activity_launcher(Path::new("termux-am-starter")));
        assert!(is_termux_activity_launcher(Path::new(
            "/data/data/com.termux/files/usr/bin/termux-am"
        )));
        assert!(is_termux_activity_launcher(Path::new("am")));
        assert!(!is_termux_activity_launcher(Path::new(
            "/data/local/tmp/custom-launcher"
        )));
    }

    #[test]
    fn android_fallback_uses_specific_media_types() {
        assert_eq!(android_media_type(true), "application/vnd.apple.mpegurl");
        assert_eq!(android_media_type(false), "video/mp4");
    }
}
