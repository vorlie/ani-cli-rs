use std::{
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    process::Stdio,
};

use tokio::process::Command;

use crate::{
    AniError, Result, StreamLink, relay_stream, relay_stream_without_hls_subtitles,
    requires_hls_relay,
};

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

#[derive(Clone, Debug)]
pub struct PlayerOptions {
    pub executable: PathBuf,
    pub kind: PlayerKind,
    pub no_detach: bool,
    pub exit_after_play: bool,
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
        }
    }

    pub fn default_android_mpv() -> Self {
        Self {
            executable: android_intent_launcher(),
            kind: PlayerKind::AndroidMpv,
            no_detach: true,
            exit_after_play: env_bool("ANI_CLI_EXIT_AFTER_PLAY"),
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

    fn command_args_inner(&self, stream: &StreamLink, title: &str, attached: bool) -> Vec<String> {
        let referer = stream.headers.referer.as_deref().unwrap_or("");
        match self.options.kind {
            PlayerKind::Mpv => {
                let mut args = mpv_options(stream, title, referer);
                args.push(stream.url.clone());
                args
            }
            PlayerKind::Iina => {
                let mut args = vec!["--no-stdin".into()];
                if attached {
                    args.push("--keep-running".into());
                }
                args.push(stream.url.clone());
                args.extend(iina_mpv_options(stream, title, referer));
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
                android_intent_args("is.xyz.mpv/.MPVActivity", &stream.url, title)
            }
            PlayerKind::AndroidVlc => android_intent_args(
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
        if requires_hls_relay(stream) || (self.is_android_player() && stream.hls) {
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
            return self.play_android(stream, title, force_attached).await;
        }
        let mut command = Command::new(&self.options.executable);
        let attached = self.options.no_detach || force_attached;
        command.args(self.command_args_inner(stream, title, attached));
        if attached {
            let status = command.status().await.map_err(|e| {
                AniError::Player(format!(
                    "could not launch {}: {e}",
                    self.options.executable.display()
                ))
            })?;
            let code = status.code().unwrap_or(1);
            if !status.success() && self.options.exit_after_play {
                return Err(AniError::Player(format!("player exited with {code}")));
            }
            Ok(Some(code))
        } else {
            command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            command.spawn().map_err(|e| {
                AniError::Player(format!(
                    "could not launch {}: {e}",
                    self.options.executable.display()
                ))
            })?;
            Ok(None)
        }
    }

    fn is_android_player(&self) -> bool {
        matches!(
            self.options.kind,
            PlayerKind::AndroidMpv | PlayerKind::AndroidVlc
        )
    }

    async fn play_android(
        &self,
        stream: &StreamLink,
        title: &str,
        relay_active: bool,
    ) -> Result<Option<i32>> {
        let terminal = io::stdin().is_terminal();
        if relay_active && !terminal {
            return Err(AniError::Player(
                "Android HLS playback requires an interactive Termux terminal so the local relay can remain active"
                    .into(),
            ));
        }

        let launch_result = Command::new(&self.options.executable)
            .args(self.command_args_inner(stream, title, true))
            .status()
            .await;
        let code = match launch_result {
            Ok(status) if status.success() => status.code().unwrap_or(0),
            result => {
                let primary_error = match result {
                    Ok(status) => format!(
                        "Android activity launcher {} exited with {}",
                        self.options.executable.display(),
                        status.code().unwrap_or(1)
                    ),
                    Err(error) => format!(
                        "could not launch Android player through {}: {error}",
                        self.options.executable.display()
                    ),
                };
                match launch_android_url_fallback(&self.options.executable, &stream.url, stream.hls)
                    .await
                {
                    Ok(code) => {
                        eprintln!(
                            "warning: {primary_error}; opened the stream through Android's default URL handler instead"
                        );
                        code
                    }
                    Err(fallback_error) => {
                        return Err(AniError::Player(format!(
                            "{primary_error}; {fallback_error}"
                        )));
                    }
                }
            }
        };

        if terminal {
            wait_for_android_player().await?;
        }
        Ok(Some(code))
    }
}

fn android_intent_args(component: &str, url: &str, title: &str) -> Vec<String> {
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

fn android_intent_launcher() -> PathBuf {
    if let Some(executable) = std::env::var_os("ANI_CLI_PLAYER") {
        return PathBuf::from(executable);
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    ["termux-am-starter", "termux-am", "am"]
        .iter()
        .find_map(|name| find_in_path(name, &path))
        .unwrap_or_else(|| PathBuf::from("termux-am-starter"))
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
    .map_err(|error| AniError::Player(format!("Android playback prompt failed: {error}")))??;
    Ok(())
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
    args
}

fn iina_mpv_options(stream: &StreamLink, title: &str, referer: &str) -> Vec<String> {
    mpv_options(stream, title, referer)
        .into_iter()
        .filter_map(|option| {
            option
                .strip_prefix("--")
                .map(|option| format!("--mpv-{option}"))
        })
        .collect()
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

fn env_bool(name: &str) -> bool {
    std::env::var(name)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RequestHeaders, SubtitleTrack};
    #[test]
    fn mpv_arguments_preserve_referrer_as_one_argument() {
        let player = Player::new(PlayerOptions {
            executable: "mpv".into(),
            kind: PlayerKind::Mpv,
            no_detach: true,
            exit_after_play: false,
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
    fn iina_arguments_put_stream_before_mpv_prefixed_options() {
        let player = Player::new(PlayerOptions {
            executable: "iina".into(),
            kind: PlayerKind::Iina,
            no_detach: false,
            exit_after_play: false,
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
                "--mpv-tls-verify=no",
                "--mpv-force-media-title=Anime Episode 1",
                "--mpv-referrer=https://ref.example",
                "--mpv-http-header-fields=Origin: https://origin.example",
                "--mpv-sub-file=https://media/subtitles.vtt",
                "--mpv-slang=English",
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

    #[test]
    fn android_mpv_arguments_use_an_explicit_view_intent() {
        let player = Player::new(PlayerOptions {
            executable: "termux-am-starter".into(),
            kind: PlayerKind::AndroidMpv,
            no_detach: true,
            exit_after_play: false,
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
