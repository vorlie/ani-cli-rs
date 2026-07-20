use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let Some(profile_path) = env::var_os("USERPROFILE") else {
        eprintln!("USERPROFILE is unavailable; cannot hide the local build path.");
        return ExitCode::FAILURE;
    };

    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("release helper must remain under scripts/release-helper")
        .to_owned();
    let manifest_path = project_root.join("Cargo.toml");
    let profile_path = profile_path.to_string_lossy();
    let remap_flag = format!("--remap-path-prefix={profile_path}=/build");
    let mut rustflags: Vec<String> = match env::var("CARGO_ENCODED_RUSTFLAGS") {
        Ok(existing) if !existing.is_empty() => {
            existing.split('\u{1f}').map(str::to_owned).collect()
        }
        _ => env::var("RUSTFLAGS")
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_owned)
            .collect(),
    };
    rustflags.push(remap_flag);
    let encoded_rustflags = rustflags.join("\u{1f}");
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    let status = Command::new(cargo)
        .args([
            "build",
            "--locked",
            "--release",
            "--target",
            "x86_64-pc-windows-msvc",
            "--manifest-path",
        ])
        .arg(manifest_path)
        .env_remove("RUSTFLAGS")
        .env("CARGO_ENCODED_RUSTFLAGS", encoded_rustflags)
        .status();

    match status {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => {
            eprintln!("Cargo build failed with {status}.");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("Could not start Cargo: {error}");
            ExitCode::FAILURE
        }
    }
}
