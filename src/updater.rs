use std::{path::PathBuf, time::SystemTime};

use ani_cli::{AniError, Result};
use serde::Deserialize;
use tokio::process::Command;

const REPOSITORY: &str = "vorlie/ani-cli-rs";
const LATEST_RELEASE_API: &str = "https://api.github.com/repos/vorlie/ani-cli-rs/releases/latest";

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

pub async fn run(check_only: bool) -> Result<()> {
    let release = fetch_latest_release().await?;
    let current = env!("CARGO_PKG_VERSION");
    if !is_newer_release(current, &release.tag_name)? {
        println!(
            "ani-cli-rs {current} is up to date (latest {}).",
            release.tag_name
        );
        return Ok(());
    }

    println!(
        "Update available: ani-cli-rs {current} → {}\n{}",
        release.tag_name, release.html_url
    );
    if check_only {
        return Ok(());
    }

    let script = download_installer(&release.tag_name).await?;
    launch_installer(&script).await
}

async fn fetch_latest_release() -> Result<GitHubRelease> {
    Ok(reqwest::Client::builder()
        .user_agent(concat!("ani-cli-rs/", env!("CARGO_PKG_VERSION")))
        .build()?
        .get(LATEST_RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

async fn download_installer(tag: &str) -> Result<PathBuf> {
    if !valid_tag(tag) {
        return Err(AniError::Update(format!(
            "GitHub returned an unsafe release tag: {tag}"
        )));
    }
    let extension = if cfg!(windows) { "ps1" } else { "sh" };
    let response = reqwest::Client::builder()
        .user_agent(concat!("ani-cli-rs/", env!("CARGO_PKG_VERSION")))
        .build()?
        .get(installer_url(tag, extension))
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|error| AniError::Update(format!("system clock error: {error}")))?
        .as_millis();
    let path = std::env::temp_dir().join(format!(
        "ani-cli-rs-update-{}-{timestamp}.{extension}",
        std::process::id()
    ));
    tokio::fs::write(&path, response).await?;
    Ok(path)
}

#[cfg(windows)]
async fn launch_installer(script: &std::path::Path) -> Result<()> {
    Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script)
        .env("ANI_CLI_RS_WAIT_FOR_PID", std::process::id().to_string())
        .env("ANI_CLI_RS_DELETE_INSTALLER", "1")
        .spawn()
        .map_err(|error| AniError::Update(format!("could not start PowerShell: {error}")))?;
    println!("The installer will continue after ani-cli-rs exits.");
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
async fn launch_installer(script: &std::path::Path) -> Result<()> {
    let status = Command::new("sh")
        .arg(script)
        .status()
        .await
        .map_err(|error| AniError::Update(format!("could not start installer: {error}")))?;
    let _ = tokio::fs::remove_file(script).await;
    if status.success() {
        Ok(())
    } else {
        Err(AniError::Update(format!(
            "installer exited with {}",
            status.code().unwrap_or(1)
        )))
    }
}

#[cfg(target_os = "macos")]
async fn launch_installer(script: &std::path::Path) -> Result<()> {
    let _ = tokio::fs::remove_file(script).await;
    Err(AniError::Update(
        "official macOS releases are not published; update by rebuilding from source".into(),
    ))
}

fn installer_url(tag: &str, extension: &str) -> String {
    format!("https://raw.githubusercontent.com/{REPOSITORY}/{tag}/scripts/install.{extension}")
}

fn valid_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn is_newer_release(current: &str, latest: &str) -> Result<bool> {
    let current = version_components(current)
        .ok_or_else(|| AniError::Update(format!("invalid current version: {current}")))?;
    let latest = version_components(latest)
        .ok_or_else(|| AniError::Update(format!("invalid release version: {latest}")))?;
    Ok(latest > current)
}

fn version_components(value: &str) -> Option<[u64; 3]> {
    let core = value
        .trim()
        .strip_prefix('v')
        .unwrap_or(value.trim())
        .split(['-', '+'])
        .next()?;
    let mut components = core.split('.');
    let version = [
        components.next()?.parse().ok()?,
        components.next()?.parse().ok()?,
        components.next()?.parse().ok()?,
    ];
    components.next().is_none().then_some(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_release_versions_without_lexical_ordering() {
        assert!(is_newer_release("0.9.0", "v0.10.0").unwrap());
        assert!(!is_newer_release("0.10.0", "0.9.9").unwrap());
        assert!(!is_newer_release("0.4.0", "0.4.0").unwrap());
    }

    #[test]
    fn rejects_unsafe_release_tags() {
        assert!(valid_tag("v0.5.0"));
        assert!(!valid_tag("../../main"));
        assert!(!valid_tag("v0.5.0;command"));
    }

    #[test]
    fn pins_installers_to_the_selected_release_tag() {
        assert_eq!(
            installer_url("0.5.0", "ps1"),
            "https://raw.githubusercontent.com/vorlie/ani-cli-rs/0.5.0/scripts/install.ps1"
        );
    }
}
