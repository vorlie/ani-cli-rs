use std::{path::PathBuf, process::Stdio};

use tokio::process::Command;

use crate::{AniError, Result, StreamLink, relay_stream, requires_hls_relay};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayerKind {
    Mpv,
    Vlc,
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
        let referer = stream.headers.referer.as_deref().unwrap_or("");
        match self.options.kind {
            PlayerKind::Mpv => {
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
                args.push(stream.url.clone());
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
        if requires_hls_relay(stream) {
            let (_relay, local) = relay_stream(stream).await?;
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
        let mut command = Command::new(&self.options.executable);
        command.args(self.command_args(stream, title));
        if self.options.no_detach || force_attached {
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
    use crate::RequestHeaders;
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
}
