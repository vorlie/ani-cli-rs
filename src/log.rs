use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
};

use tracing_subscriber::{
    EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt,
};

fn log_file_path() -> PathBuf {
    if let Ok(path) = std::env::var("ANI_CLI_LOG_PATH") {
        return PathBuf::from(path);
    }

    std::env::current_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("ani-cli-rs.log")
}

pub fn init() {
    let filter = EnvFilter::from_default_env();
    let log_path = log_file_path();

    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&log_path);

    match file {
        Ok(mut file) => {
            let _ = file.write_all(b"\n=== ani-cli-rs log started ===\n");
            let _ = file.flush();

            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_target(false)
                        .compact()
                        .with_writer(std::io::stderr)
                        .with_ansi(true)
                        .with_filter(filter.clone()),
                )
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_target(false)
                        .compact()
                        .with_writer(file)
                        .with_ansi(false)
                        .with_filter(filter),
                )
                .init();
        }
        Err(error) => {
            eprintln!("Failed to open log file {}: {error}", log_path.display());
            tracing_subscriber::fmt()
                .with_target(false)
                .compact()
                .with_env_filter(filter)
                .init();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use super::log_file_path;

    #[test]
    fn log_file_path_respects_env_override() {
        let override_path = PathBuf::from("/tmp/ani-cli-rs-custom.log");
        unsafe {
            env::set_var("ANI_CLI_LOG_PATH", &override_path);
        }

        let resolved = log_file_path();

        unsafe {
            env::remove_var("ANI_CLI_LOG_PATH");
        }
        assert_eq!(resolved, override_path);
    }
}