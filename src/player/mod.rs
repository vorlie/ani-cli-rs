use std::{
    fmt,
    path::PathBuf,
    process::Stdio,
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::{
    AniError, Result, StreamLink, relay_stream, relay_stream_without_hls_subtitles,
    requires_hls_relay,
    util::{
        env_bool,
        http::{fetch_subtitle_bytes, subtitle_extension},
        sanitize_filename,
    },
};

mod android;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayerKind {
    Mpv,
    Iina,
    Vlc,
    AndroidMpv,
    AndroidVlc,
    Syncplay,
    Custom,
}

impl fmt::Display for PlayerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Mpv => "mpv",
            Self::Iina => "iina",
            Self::Vlc => "vlc",
            Self::AndroidMpv => "android-mpv",
            Self::AndroidVlc => "android-vlc",
            Self::Syncplay => "syncplay",
            Self::Custom => "custom",
        };
        f.write_str(label)
    }
}

#[derive(Clone, Debug)]
pub struct PlayerOptions {
    pub executable: PathBuf,
    pub kind: PlayerKind,
    pub no_detach: bool,
    pub exit_after_play: bool,
    pub force_hls_relay: bool,
}

impl PlayerOptions {
    pub fn default_player() -> Self {
        if cfg!(target_os = "android") {
            Self::default_android_mpv()
        } else if cfg!(target_os = "macos") {
            Self::default_iina()
        } else {
            Self::default_mpv()
        }
    }

    pub fn default_mpv() -> Self {
        let executable = std::env::var_os("ANI_CLI_PLAYER")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(if cfg!(windows) { "mpv.exe" } else { "mpv" }));
        Self {
            executable,
            kind: PlayerKind::Mpv,
            no_detach: env_bool("ANI_CLI_NO_DETACH"),
            exit_after_play: env_bool("ANI_CLI_EXIT_AFTER_PLAY"),
            force_hls_relay: false,
        }
    }

    pub fn default_iina() -> Self {
        let executable = std::env::var_os("ANI_CLI_PLAYER")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("iina"));
        Self {
            executable,
            kind: PlayerKind::Iina,
            no_detach: env_bool("ANI_CLI_NO_DETACH"),
            exit_after_play: env_bool("ANI_CLI_EXIT_AFTER_PLAY"),
            force_hls_relay: false,
        }
    }

    pub fn default_android_mpv() -> Self {
        Self {
            executable: android::android_intent_launcher(),
            kind: PlayerKind::AndroidMpv,
            no_detach: true,
            exit_after_play: env_bool("ANI_CLI_EXIT_AFTER_PLAY"),
            force_hls_relay: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    options: PlayerOptions,
}

impl Player {
    pub fn new(options: PlayerOptions) -> Self {
        Self { options }
    }

    pub fn command_args(&self, stream: &StreamLink, title: &str) -> Vec<String> {
        self.command_args_inner(stream, title, self.options.no_detach)
    }

    /// Human-readable summary of the configured player (executable + kind).
    /// Useful for debug logs and error messages.
    pub fn describe(&self) -> String {
        format!(
            "{} ({}) [no_detach={}, exit_after_play={}, force_hls_relay={}]",
            self.options.executable.display(),
            self.options.kind,
            self.options.no_detach,
            self.options.exit_after_play,
            self.options.force_hls_relay,
        )
    }

    fn command_args_inner(&self, stream: &StreamLink, title: &str, attached: bool) -> Vec<String> {
        let referer = stream.headers.referer.as_deref().unwrap_or("");
        match self.options.kind {
            PlayerKind::Mpv => {
                let mut args = mpv_options(stream, title, referer);
                // Ensure URL is passed last
                args.push(stream.url.clone());
                args
            }
            PlayerKind::Iina => {
                let mut args = vec!["--no-stdin".into()];
                if attached {
                    args.push("--keep-running".into());
                }
                args.push(stream.url.clone());
                args.push("--".into());
                args.extend(mpv_options(stream, title, referer));
                args
            }
            PlayerKind::Vlc => {
                let mut args = vec!["--play-and-exit".into(), format!("--meta-title={title}")];
                if !referer.is_empty() {
                    args.push(format!("--http-referrer={referer}"));
                }
                if let Some(agent) = stream.headers.extra.get("User-Agent") {
                    args.push(format!("--http-user-agent={agent}"));
                }
                for track in &stream.subtitles {
                    args.push(format!("--sub-file={}", track.url));
                }
                args.push(stream.url.clone());
                args
            }
            PlayerKind::AndroidMpv => {
                android::android_intent_args("is.xyz.mpv/.MPVActivity", &stream.url, title)
            }
            PlayerKind::AndroidVlc => android::android_intent_args(
                "org.videolan.vlc/org.videolan.vlc.gui.video.VideoPlayerActivity",
                &stream.url,
                title,
            ),
            PlayerKind::Syncplay => {
                let mut args = vec![
                    stream.url.clone(),
                    "--".into(),
                    "--tls-verify=no".into(),
                    format!("--force-media-title={title}"),
                ];
                if !referer.is_empty() {
                    args.push(format!("--referrer={referer}"));
                }
                append_mpv_headers(&mut args, stream);
                for track in &stream.subtitles {
                    args.push(format!("--sub-file={}", track.url));
                }
                args
            }
            PlayerKind::Custom => vec![stream.url.clone()],
        }
    }

    pub async fn play(&self, stream: &StreamLink, title: &str) -> Result<Option<i32>> {
        info!(
            title = %title,
            player = %self.describe(),
            stream_url = %stream.url,
            hls = stream.hls,
            subtitles = stream.subtitles.len(),
            "playback requested",
        );
        if self.options.force_hls_relay
            || crate::requires_hls_relay(stream)
            || (self.is_android_player() && stream.hls)
        {
            debug!(
                title = %title,
                player = %self.options.kind.to_string(),
                android_player = self.is_android_player(),
                hls = stream.hls,
                "stream requires the loopback HLS relay",
            );
            // Android players receive a single intent URL and cannot be given
            // an explicit `--sub-file`, so they need subtitles exposed as
            // synthetic HLS renditions. Desktop players already receive
            // subtitles via `--sub-file`, and wrapping a long subtitle file as
            // a single oversized HLS segment produces unreliable cue timing
            // in some HLS demuxers (see issue #18).
            let (_relay, local) = if self.is_android_player() {
                relay_stream(stream).await?
            } else {
                relay_stream_without_hls_subtitles(stream).await?
            };
            debug!(
                title = %title,
                local_url = %local.url,
                "HLS relay is serving the rewritten stream URL to the player",
            );
            return self.play_inner(&local, title, true).await;
        }
        self.play_inner(stream, title, false).await
    }

    async fn play_inner(
        &self,
        stream: &StreamLink,
        title: &str,
        force_attached: bool,
    ) -> Result<Option<i32>> {
        if self.is_android_player() {
            let launch_args = self.command_args_inner(stream, title, true);
            return android::play_android(
                &self.options.executable,
                launch_args,
                stream,
                title,
                force_attached,
            )
            .await;
        }

        // Validate that the player executable exists before attempting to launch
        let needs_validation = self.options.executable.components().count() > 1;
        if needs_validation {
            if !self.options.executable.exists() {
                eprintln!(
                    "Player executable not found: {}. Please install the player or set ANI_CLI_PLAYER environment variable.",
                    self.options.executable.display()
                );
                return Err(AniError::PlayerNotFound {
                    executable: self.options.executable.display().to_string(),
                });
            }
        } else {
            use which::which;
            if which(&self.options.executable).is_err() {
                eprintln!(
                    "Player executable '{}' not found in PATH. Please install the player or set ANI_CLI_PLAYER environment variable.",
                    self.options.executable.display()
                );
                return Err(AniError::PlayerNotFound {
                    executable: self.options.executable.display().to_string(),
                });
            }
        }

        let mut command = Command::new(&self.options.executable);
        let attached = self.options.no_detach || force_attached;
        let args = if self.options.kind == PlayerKind::Vlc {
            self.vlc_command_args(stream, title, attached).await?
        } else {
            self.command_args_inner(stream, title, attached)
        };
        info!(
            title = %title,
            executable = %self.options.executable.display(),
            kind = %self.options.kind,
            attached,
            "launching external player",
        );
        debug!(
            title = %title,
            stream_url = %stream.url,
            stream_hls = stream.hls,
            args = ?args,
            "full player command line",
        );
        command.args(args);
        if attached {
            match command.status().await {
                Ok(status) => {
                    let code = status.code().unwrap_or(1);
                    info!(
                        title = %title,
                        exit_code = code,
                        success = status.success(),
                        "player exited",
                    );
                    if !status.success() && self.options.exit_after_play {
                        eprintln!("Player exited with {code}");
                        return Err(AniError::PlayerExitFailed);
                    }
                    Ok(Some(code))
                }
                Err(error) => {
                    warn!(
                        title = %title,
                        executable = %self.options.executable.display(),
                        error = %error,
                        "failed to launch player in attached mode",
                    );
                    eprintln!(
                        "Could not launch {}: {error}",
                        self.options.executable.display()
                    );
                    Err(AniError::PlayerLaunchFailed {
                        executable: self.options.executable.display().to_string(),
                        reason: format!("attached mode launch failed: {}", error),
                    })
                }
            }
        } else {
            command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            match command.spawn() {
                Ok(child) => {
                    info!(
                        title = %title,
                        pid = child.id().unwrap_or(0),
                        executable = %self.options.executable.display(),
                        "player launched in the background",
                    );
                    let _ = child.id();
                    Ok(None)
                }
                Err(error) => {
                    warn!(
                        title = %title,
                        executable = %self.options.executable.display(),
                        error = %error,
                        "failed to launch player in detached mode",
                    );
                    eprintln!(
                        "Could not launch {}: {error}",
                        self.options.executable.display()
                    );
                    Err(AniError::PlayerLaunchFailed {
                        executable: self.options.executable.display().to_string(),
                        reason: format!("detached mode launch failed: {}", error),
                    })
                }
            }
        }
    }

    async fn vlc_command_args(
        &self,
        stream: &StreamLink,
        title: &str,
        _attached: bool,
    ) -> Result<Vec<String>> {
        let referer = stream.headers.referer.as_deref().unwrap_or("");
        let mut args = vec!["--play-and-exit".into(), format!("--meta-title={title}")];
        if !referer.is_empty() {
            args.push(format!("--http-referrer={referer}"));
        }
        if let Some(agent) = stream.headers.extra.get("User-Agent") {
            args.push(format!("--http-user-agent={agent}"));
        }
        for subtitle_path in prepare_vlc_subtitles(stream).await? {
            args.push(format!("--sub-file={subtitle_path}"));
        }
        args.push(stream.url.clone());
        Ok(args)
    }

    fn is_android_player(&self) -> bool {
        matches!(
            self.options.kind,
            PlayerKind::AndroidMpv | PlayerKind::AndroidVlc
        )
    }
}

async fn prepare_vlc_subtitles(stream: &StreamLink) -> Result<Vec<String>> {
    if stream.subtitles.is_empty() {
        return Ok(Vec::new());
    }

    let temp_dir = std::env::temp_dir().join(format!(
        "ani-cli-rs-vlc-subtitles-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    tokio::fs::create_dir_all(&temp_dir).await?;

    let mut local_paths = Vec::new();
    for (index, track) in stream.subtitles.iter().enumerate() {
        let path = temp_dir.join(format!(
            "{index}.{}.{}",
            sanitize_filename(&track.label, "subtitle"),
            subtitle_extension(&track.url)
        ));
        let bytes = fetch_subtitle_bytes(track, &stream.headers, 16 * 1024 * 1024).await?;
        tokio::fs::write(&path, bytes).await?;
        local_paths.push(path.to_string_lossy().into_owned());
    }

    Ok(local_paths)
}

fn mpv_options(stream: &StreamLink, title: &str, referer: &str) -> Vec<String> {
    let mut args = vec![
        "--tls-verify=no".into(),
        format!("--force-media-title={title}"),
    ];
    if !referer.is_empty() {
        args.push(format!("--referrer={referer}"));
    }
    append_mpv_headers(&mut args, stream);
    for track in &stream.subtitles {
        args.push(format!("--sub-file={}", track.url));
    }
    if let Some(track) = stream.subtitles.iter().find(|track| track.default) {
        args.push(format!("--slang={}", track.label));
    }
    if stream.hls && requires_hls_relay(stream) {
        args.push("--cache=yes".into());
        args.push("--cache-secs=120".into());
        args.push("--demuxer-max-bytes=512MiB".into());
        args.push("--demuxer-max-back-bytes=256MiB".into());
    }
    args
}

fn append_mpv_headers(args: &mut Vec<String>, stream: &StreamLink) {
    let mut headers = Vec::new();
    if let Some(origin) = &stream.headers.origin
        && safe_header_value(origin)
    {
        headers.push(format!("Origin: {origin}"));
    }
    headers.extend(
        stream
            .headers
            .extra
            .iter()
            .filter(|(name, value)| safe_header_value(name) && safe_header_value(value))
            .map(|(name, value)| format!("{name}: {value}")),
    );
    if !headers.is_empty() {
        args.push(format!("--http-header-fields={}", headers.join(",")));
    }
}

fn safe_header_value(value: &str) -> bool {
    !value.contains(['\r', '\n'])
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::{RequestHeaders, SubtitleTrack};

    #[test]
    fn mpv_arguments_preserve_referrer_as_one_argument() {
        let player = Player::new(PlayerOptions {
            executable: "mpv".into(),
            kind: PlayerKind::Mpv,
            no_detach: true,
            exit_after_play: false,
            force_hls_relay: false,
        });
        let stream = StreamLink {
            url: "https://media/a.m3u8".into(),
            resolution: "1080p".into(),
            hls: true,
            provider: "Default".into(),
            downloadable: true,
            headers: RequestHeaders {
                referer: Some("https://ref.example".into()),
                ..Default::default()
            },
            subtitles: vec![],
        };
        assert!(
            player
                .command_args(&stream, "Anime Episode 1")
                .contains(&"--referrer=https://ref.example".into())
        );
    }

    #[test]
    fn iina_arguments_put_stream_before_raw_mpv_options() {
        let player = Player::new(PlayerOptions {
            executable: "iina".into(),
            kind: PlayerKind::Iina,
            no_detach: false,
            exit_after_play: false,
            force_hls_relay: false,
        });
        let stream = StreamLink {
            url: "https://media/a.m3u8".into(),
            resolution: "1080p".into(),
            hls: true,
            provider: "Default".into(),
            downloadable: true,
            headers: RequestHeaders {
                referer: Some("https://ref.example".into()),
                origin: Some("https://origin.example".into()),
                ..Default::default()
            },
            subtitles: vec![SubtitleTrack {
                label: "English".into(),
                url: "https://media/subtitles.vtt".into(),
                default: true,
            }],
        };

        assert_eq!(
            player.command_args(&stream, "Anime Episode 1"),
            vec![
                "--no-stdin",
                "https://media/a.m3u8",
                "--",
                "--tls-verify=no",
                "--force-media-title=Anime Episode 1",
                "--referrer=https://ref.example",
                "--http-header-fields=Origin: https://origin.example",
                "--sub-file=https://media/subtitles.vtt",
                "--slang=English",
            ]
        );
    }

    #[test]
    fn forced_attached_iina_keeps_cli_running() {
        let player = Player::new(PlayerOptions {
            executable: "iina".into(),
            kind: PlayerKind::Iina,
            no_detach: false,
            exit_after_play: false,
            force_hls_relay: false,
        });
        let stream = StreamLink {
            url: "https://media/a.m3u8".into(),
            resolution: "1080p".into(),
            hls: true,
            provider: "Default".into(),
            downloadable: true,
            headers: RequestHeaders::default(),
            subtitles: vec![],
        };

        assert_eq!(
            &player.command_args_inner(&stream, "Anime", true)[..3],
            ["--no-stdin", "--keep-running", "https://media/a.m3u8"]
        );
    }

    #[tokio::test]
    async fn vlc_downloads_remote_subtitles_to_local_files() {
        use wiremock::{
            Mock, MockServer, ResponseTemplate,
            matchers::{method, path},
        };

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/subtitle.vtt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("WEBVTT\n\n00:00.000 --> 00:02.000\nHello from the subtitle."),
            )
            .mount(&server)
            .await;

        let stream = StreamLink {
            url: "https://media.example/episode.m3u8".into(),
            resolution: "720p".into(),
            hls: false,
            provider: "Anikoto".into(),
            downloadable: true,
            headers: RequestHeaders::default(),
            subtitles: vec![SubtitleTrack {
                label: "English".into(),
                url: format!("{}/subtitle.vtt", server.uri()),
                default: true,
            }],
        };

        let local_subtitles = prepare_vlc_subtitles(&stream).await.unwrap();

        assert_eq!(local_subtitles.len(), 1);
        let local_path = Path::new(&local_subtitles[0]);
        assert!(local_path.is_absolute());
        assert!(!local_path.to_string_lossy().starts_with("http://"));
        assert!(!local_path.to_string_lossy().starts_with("https://"));
        assert!(
            tokio::fs::read_to_string(local_path)
                .await
                .unwrap()
                .contains("Hello from the subtitle")
        );
    }

    #[test]
    fn platform_default_selects_expected_player() {
        let options = PlayerOptions::default_player();
        if cfg!(target_os = "android") {
            assert_eq!(options.kind, PlayerKind::AndroidMpv);
        } else if cfg!(target_os = "macos") {
            assert_eq!(options.kind, PlayerKind::Iina);
            if std::env::var_os("ANI_CLI_PLAYER").is_none() {
                assert_eq!(options.executable, PathBuf::from("iina"));
            }
        } else {
            assert_eq!(options.kind, PlayerKind::Mpv);
        }
    }
}
