use std::error::Error as StdError;

use super::{AniError, ErrorCode};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ErrorVerbosity {
    Normal,
    Verbose,
    Debug,
}

pub struct ErrorReport<'a> {
    pub code: ErrorCode,
    pub title: &'static str,
    pub message: String,
    pub help: Option<String>,
    pub docs: Option<&'static str>,
    pub source: Option<&'a (dyn StdError + 'static)>,
    pub context: Option<ErrorContext>,
}

#[derive(Clone)]
pub struct ErrorContext {
    pub provider: Option<String>,
    pub anime: Option<String>,
    pub episode: Option<String>,
    pub mode: Option<String>,
    pub additional_fields: Vec<(String, String)>,
}

impl<'a> ErrorReport<'a> {
    pub fn from_error(error: &'a AniError) -> Self {
        let code = error.code();
        let title = match code {
            ErrorCode::ProviderUnavailable => "Provider unavailable",
            ErrorCode::ProviderRequestFailed => "Provider request failed",
            ErrorCode::ProviderInvalidResponse => "Provider returned invalid response",
            ErrorCode::ProviderRateLimited => "Provider rate limited",
            ErrorCode::ProviderCatalogError => "Provider catalog error",
            ErrorCode::ProviderUrlValidationFailed => "Provider URL validation failed",
            ErrorCode::ProviderResponseSizeExceeded => "Provider response size exceeded",

            ErrorCode::NoSearchResults => "No search results found",
            ErrorCode::EmptySearchQuery => "Empty search query",
            ErrorCode::NoEpisodesAvailable => "No episodes available",
            ErrorCode::InvalidEpisodeSelection => "Invalid episode selection",
            ErrorCode::SelectionOutOfRange => "Selection out of range",
            ErrorCode::CommandRequiresQuery => "Command requires query",

            ErrorCode::NoPlayableSources => "No playable sources found",
            ErrorCode::SourceResolutionFailed => "Source resolution failed",
            ErrorCode::StreamUnavailable => "Stream unavailable",
            ErrorCode::UnsupportedEmbedHost => "Unsupported embed host",
            ErrorCode::NoNativeStreams => "No native streams available",

            ErrorCode::DownloadFailed => "Download failed",
            ErrorCode::NoDownloadTool => "No download tool available",
            ErrorCode::HlsDownloadFailed => "HLS download failed",
            ErrorCode::SubtitleDownloadFailed => "Subtitle download failed",
            ErrorCode::SubtitleSizeExceeded => "Subtitle size exceeded",
            ErrorCode::DownloadOutputError => "Download output error",

            ErrorCode::PlayerNotFound => "Player not found",
            ErrorCode::PlayerLaunchFailed => "Player launch failed",
            ErrorCode::PlayerExitedWithError => "Player exited with error",
            ErrorCode::AndroidTerminalRequired => "Android terminal required",
            ErrorCode::GeneralPlayerError => "General player error",

            ErrorCode::IoError => "I/O error",
            ErrorCode::HistoryStateDirectoryError => "History state directory error",
            ErrorCode::HistoryOperationFailed => "History operation failed",
            ErrorCode::InvalidHistoryEntry => "Invalid history entry",

            ErrorCode::UpdateCheckFailed => "Update check failed",
            ErrorCode::InvalidReleaseTag => "Invalid release tag",
            ErrorCode::InstallerExecutionFailed => "Installer execution failed",
            ErrorCode::PlatformNotSupported => "Platform not supported",

            ErrorCode::InternalError => "Internal error",
            ErrorCode::JsonParsingError => "JSON parsing error",
            ErrorCode::UrlParsingError => "URL parsing error",
            ErrorCode::InvalidInput => "Invalid input",
        };

        let message = Self::message(error);
        let help = match code {
            ErrorCode::ProviderUnavailable => Some("Check your internet connection, try again later, or try another provider.".to_string()),
            ErrorCode::ProviderRateLimited => Some("Wait a few minutes and try again, or try another provider.".to_string()),
            ErrorCode::NoSearchResults => Some("Check your search query spelling, try a different search term, or try another provider.".to_string()),
            ErrorCode::EmptySearchQuery => Some("Provide a search query or use interactive mode if available.".to_string()),
            ErrorCode::NoEpisodesAvailable => Some("Try another provider, check if the anime is released, or try a different anime.".to_string()),
            ErrorCode::NoPlayableSources => Some("Try another provider, try subtitle/dub mode toggle, or try a different episode.".to_string()),
            ErrorCode::NoDownloadTool => Some("Install yt-dlp or FFmpeg and ensure they are in your PATH.".to_string()),
            ErrorCode::PlayerNotFound => Some("Install the player, check player installation, or update player path in configuration.".to_string()),
            ErrorCode::AndroidTerminalRequired => Some("Use interactive terminal on Android or use download mode instead.".to_string()),
            ErrorCode::InternalError => Some("Run again with --verbose flag and report the issue with full error details.".to_string()),
            _ => None,
        };

        let docs = Some("https://vorlie.github.io/ani-cli-rs/errors");
        let context = Self::extract_context(error);

        Self {
            code,
            title,
            message,
            help,
            docs,
            source: error.source(),
            context,
        }
    }

    fn extract_context(error: &AniError) -> Option<ErrorContext> {
        let mut ctx = ErrorContext {
            provider: None,
            anime: None,
            episode: None,
            mode: None,
            additional_fields: Vec::new(),
        };

        match error {
            AniError::ProviderUnavailable { provider, .. }
            | AniError::ProviderRequestFailed { provider, .. }
            | AniError::ProviderInvalidResponse { provider, .. }
            | AniError::ProviderCatalogError { provider, .. } => {
                ctx.provider = Some(provider.clone());
            }
            AniError::ProviderRateLimited {
                provider,
                retry_after_seconds,
            } => {
                ctx.provider = Some(provider.clone());
                ctx.additional_fields
                    .push(("retry_after".to_string(), format!("{retry_after_seconds}s")));
            }
            AniError::NoPlayableSources {
                anime,
                episode,
                mode,
            } => {
                ctx.anime = anime.clone();
                ctx.episode = episode.clone();
                ctx.mode = mode.clone();
            }
            AniError::PlayerNotFound { executable }
            | AniError::PlayerLaunchFailed { executable, .. } => {
                ctx.additional_fields
                    .push(("executable".to_string(), executable.clone()));
            }
            AniError::PlayerExitedWithError {
                executable,
                exit_code,
            } => {
                ctx.additional_fields
                    .push(("executable".to_string(), executable.clone()));
                if let Some(code) = exit_code {
                    ctx.additional_fields
                        .push(("exit_code".to_string(), code.to_string()));
                }
            }
            AniError::SubtitleSizeExceeded { size, limit } => {
                ctx.additional_fields
                    .push(("size".to_string(), format!("{} bytes", size)));
                ctx.additional_fields
                    .push(("limit".to_string(), format!("{} bytes", limit)));
            }
            AniError::ProviderResponseSizeExceeded {
                provider,
                size,
                limit,
            } => {
                ctx.provider = Some(provider.clone());
                ctx.additional_fields
                    .push(("size".to_string(), format!("{} bytes", size)));
                ctx.additional_fields
                    .push(("limit".to_string(), format!("{} bytes", limit)));
            }
            _ => {}
        }

        if ctx.provider.is_some() || ctx.anime.is_some() || !ctx.additional_fields.is_empty() {
            Some(ctx)
        } else {
            None
        }
    }

    fn message(error: &AniError) -> String {
        match error {
            AniError::ProviderUnavailable { provider, .. } => {
                format!("{provider} could not be reached.")
            }
            AniError::ProviderRequestFailed { provider, .. } => {
                format!("A request to {provider} failed.")
            }
            AniError::ProviderInvalidResponse {
                provider, message, ..
            } => format!("{provider} returned data ani-cli-rs could not understand: {message}"),
            AniError::ProviderRateLimited {
                provider,
                retry_after_seconds,
            } => format!("{provider} asked us to retry after {retry_after_seconds} seconds."),
            AniError::ProviderCatalogError {
                provider, message, ..
            } => format!("{provider}'s catalog request failed: {message}"),
            AniError::ProviderUrlValidationFailed { reason } => {
                format!("A provider URL failed validation: {reason}")
            }
            AniError::ProviderResponseSizeExceeded {
                provider,
                size,
                limit,
            } => format!("{provider} returned {size} bytes, exceeding the {limit}-byte limit."),
            AniError::NoEpisodesAvailable { anime: Some(anime) } => {
                format!("No episodes are available for {anime}.")
            }
            AniError::InvalidEpisodeSelection { episode, reason } => {
                format!("Episode selection {episode:?} is invalid: {reason}")
            }
            AniError::SelectionOutOfRange { max, selected } => match selected {
                Some(selected) => {
                    format!("Selection {selected} is outside the available range (1–{max}).")
                }
                None => format!("The selection is outside the available range (1–{max})."),
            },
            AniError::SourceResolutionFailed { provider, reason } => {
                format!("Could not resolve a playable source from {provider}: {reason}")
            }
            AniError::StreamUnavailable { reason } => {
                format!("The selected stream is unavailable: {reason}")
            }
            AniError::UnsupportedEmbedHost { host } => {
                format!("The provider uses unsupported embed host {host}.")
            }
            AniError::NoNativeStreams { provider } => {
                format!("{provider} did not provide a supported native stream.")
            }
            AniError::NoPlayableSources {
                anime,
                episode,
                mode,
            } => {
                let context = match (anime, episode, mode) {
                    (Some(anim), Some(epi), Some(m)) => {
                        format!(" for {anim} — Episode {epi} ({m})")
                    }
                    (Some(anim), Some(epi), None) => {
                        format!(" for {anim} — Episode {epi}")
                    }
                    (Some(anim), None, Some(m)) => {
                        format!(" for {anim} ({m})")
                    }
                    (None, Some(epi), Some(m)) => {
                        format!(" for episode {epi} ({m})")
                    }
                    (Some(anim), None, None) => {
                        format!(" for {anim}")
                    }
                    (None, Some(epi), None) => {
                        format!(" for episode {epi}")
                    }
                    (None, None, Some(m)) => {
                        format!(" ({m})")
                    }
                    (None, None, None) => String::new(),
                };
                format!("No playable sources were found{context}.")
            }
            AniError::DownloadFailed { reason, .. } | AniError::HlsDownloadFailed { reason } => {
                format!("Download failed: {reason}")
            }
            AniError::SubtitleDownloadFailed { track, reason } => {
                format!("Could not download subtitle track {track:?}: {reason}")
            }
            AniError::SubtitleSizeExceeded { size, limit } => {
                format!("The subtitle track is {size} bytes, exceeding the {limit}-byte limit.")
            }
            AniError::DownloadOutputError { path, reason } => {
                format!("Could not write download output to {path}: {reason}")
            }
            AniError::PlayerNotFound { executable } => {
                format!("Player executable {executable:?} was not found.")
            }
            AniError::PlayerLaunchFailed { executable, reason } => {
                format!("Could not launch {executable:?}: {reason}")
            }
            AniError::PlayerExitedWithError {
                executable,
                exit_code,
            } => match exit_code {
                Some(code) => format!("{executable:?} exited with status {code}."),
                None => format!("{executable:?} exited unsuccessfully."),
            },
            AniError::GeneralPlayerError { reason } => format!("Player failed: {reason}"),
            AniError::IoError { source, context } => format!("Could not {context}: {source}"),
            AniError::HistoryOperationFailed { operation, reason } => {
                format!("Could not {operation} history: {reason}")
            }
            AniError::InvalidHistoryEntry { reason } => format!("Invalid history entry: {reason}"),
            AniError::UpdateCheckFailed { reason } => {
                format!("Could not check for updates: {reason}")
            }
            AniError::InvalidReleaseTag { tag } => {
                format!("GitHub returned an invalid release tag: {tag}")
            }
            AniError::InstallerExecutionFailed { exit_code } => match exit_code {
                Some(code) => format!("The installer exited with status {code}."),
                None => "Could not start the installer.".to_string(),
            },
            AniError::PlatformNotSupported { platform } => {
                format!("Automatic updates are not supported on {platform}.")
            }
            AniError::InternalError { message } => message.clone(),
            AniError::JsonParsingError { context, .. } => {
                format!("Could not parse JSON while {context}.")
            }
            AniError::UrlParsingError { context, .. } => {
                format!("Could not parse a URL while {context}.")
            }
            AniError::InvalidInput { input, reason } => {
                format!("Invalid input {input:?}: {reason}")
            }
            _ => error.to_string(),
        }
    }

    pub fn render(&self, verbosity: ErrorVerbosity) -> String {
        match verbosity {
            ErrorVerbosity::Normal => self.render_normal(),
            ErrorVerbosity::Verbose => self.render_verbose(),
            ErrorVerbosity::Debug => self.render_debug(),
        }
    }

    fn render_normal(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("error[{}]: {}\n", self.code.id(), self.title));
        output.push('\n');
        output.push_str(&self.message);
        output.push('\n');

        if let Some(help) = &self.help {
            output.push('\n');
            output.push_str(&format!("help: {}", help));
        }

        if let Some(docs) = self.docs {
            output.push_str(&format!("\ndocs: {docs}#{}", self.code.id()));
        }

        output
    }

    fn render_verbose(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("error[{}]: {}\n", self.code.id(), self.title));
        output.push('\n');
        output.push_str(&self.message);
        output.push('\n');

        if let Some(ctx) = &self.context {
            let mut context_lines = Vec::new();
            if let Some(provider) = &ctx.provider {
                context_lines.push(format!("Provider: {}", provider));
            }
            if let Some(anime) = &ctx.anime {
                context_lines.push(format!("Anime: {}", anime));
            }
            if let Some(episode) = &ctx.episode {
                context_lines.push(format!("Episode: {}", episode));
            }
            if let Some(mode) = &ctx.mode {
                context_lines.push(format!("Mode: {}", mode));
            }
            for (key, value) in &ctx.additional_fields {
                context_lines.push(format!("{}: {}", key, value));
            }

            if !context_lines.is_empty() {
                output.push('\n');
                for line in context_lines {
                    output.push_str(&format!("{}\n", line));
                }
            }
        }

        if let Some(help) = &self.help {
            output.push('\n');
            output.push_str(&format!("help: {}", help));
        }

        if let Some(docs) = self.docs {
            output.push_str(&format!("\ndocs: {docs}#{}", self.code.id()));
        }

        if let Some(source) = self.source {
            output.push_str("\n\nCaused by:\n");
            let mut current = Some(source);
            let mut indent = 0;
            while let Some(err) = current {
                output.push_str(&format!("  {}: {}\n", "  ".repeat(indent), err));
                current = err.source();
                indent += 1;
                if indent > 5 {
                    output.push_str("  ...\n");
                    break;
                }
            }
        }

        output
    }

    fn render_debug(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("error[{}]\n", self.code.id()));
        output.push('\n');
        output.push_str("Debug information:\n");

        if let Some(ctx) = &self.context {
            if let Some(provider) = &ctx.provider {
                output.push_str(&format!("  provider = {}\n", provider));
            }
            if let Some(anime) = &ctx.anime {
                output.push_str(&format!("  anime = {}\n", anime));
            }
            if let Some(episode) = &ctx.episode {
                output.push_str(&format!("  episode = {}\n", episode));
            }
            if let Some(mode) = &ctx.mode {
                output.push_str(&format!("  mode = {}\n", mode));
            }
            for (key, value) in &ctx.additional_fields {
                output.push_str(&format!("  {} = {}\n", key, value));
            }
        }

        output.push_str(&format!("  error_code = {}\n", self.code.id()));
        output.push_str(&format!("  error_slug = {}\n", self.code.slug()));

        if let Some(source) = self.source {
            output.push_str("\nError chain:\n");
            let mut current = Some(source);
            let mut indent = 0;
            while let Some(err) = current {
                output.push_str(&format!("  [{}]: {}\n", indent, err));
                current = err.source();
                indent += 1;
                if indent > 10 {
                    output.push_str("  [...]\n");
                    break;
                }
            }
        }

        output.push_str("\nUser message:\n");
        output.push_str(&format!("  {}\n", self.title));
        output.push_str(&format!("  {}\n", self.message));

        if let Some(help) = &self.help {
            output.push_str("\nHelp:\n");
            output.push_str(&format!("  {}\n", help));
        }

        if let Some(docs) = self.docs {
            output.push_str(&format!("\nDocumentation: {docs}#{}\n", self.code.id()));
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::{AniError, ErrorReport, ErrorVerbosity};

    #[test]
    fn verbose_rate_limit_report_includes_provider_and_retry_after() {
        let error = AniError::ProviderRateLimited {
            provider: "Anikoto.cz".to_string(),
            retry_after_seconds: 120,
        };

        let output = ErrorReport::from_error(&error).render(ErrorVerbosity::Verbose);

        assert!(output.contains("error[ACL-1004]: Provider rate limited"));
        assert!(output.contains("Provider: Anikoto.cz"));
        assert!(output.contains("retry_after: 120s"));
        assert!(output.contains("docs: https://vorlie.github.io/ani-cli-rs/errors#ACL-1004"));
    }

    #[test]
    fn normal_report_is_compact_and_actionable() {
        let error = AniError::NoPlayableSources {
            anime: Some("One Piece".to_string()),
            episode: Some("1132".to_string()),
            mode: Some("sub".to_string()),
        };

        let output = ErrorReport::from_error(&error).render(ErrorVerbosity::Normal);

        assert!(output.contains("error[ACL-3001]: No playable sources found"));
        assert!(output.contains("help: Try another provider"));
        assert!(!output.contains("Provider:"));
    }
}
