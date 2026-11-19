use std::{
    io::{IsTerminal, Write},
    time::{Duration, Instant},
};

pub(super) struct DownloadProgress {
    initial: u64,
    downloaded: u64,
    total: Option<u64>,
    started: Instant,
    last_draw: Instant,
    last_width: usize,
    terminal: bool,
    finished: bool,
}

impl DownloadProgress {
    pub(super) fn new(initial: u64, total: Option<u64>) -> Self {
        let now = Instant::now();
        let mut progress = Self {
            initial,
            downloaded: initial,
            total,
            started: now,
            last_draw: now.checked_sub(Duration::from_secs(1)).unwrap_or(now),
            last_width: 0,
            terminal: std::io::stderr().is_terminal(),
            finished: false,
        };
        progress.draw(false);
        progress
    }

    pub(super) fn advance(&mut self, bytes: u64) {
        self.downloaded += bytes;
        let interval = if self.terminal {
            Duration::from_millis(125)
        } else {
            Duration::from_secs(5)
        };
        if self.last_draw.elapsed() >= interval {
            self.draw(false);
        }
    }

    pub(super) fn finish(&mut self) {
        self.finished = true;
        self.draw(true);
    }

    fn draw(&mut self, finished: bool) {
        let elapsed = self.started.elapsed();
        let transferred = self.downloaded.saturating_sub(self.initial);
        let speed = if elapsed.as_secs_f64() > 0.0 {
            transferred as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };
        let line = progress_line(self.downloaded, self.total, speed, finished);
        let padding = " ".repeat(self.last_width.saturating_sub(line.len()));
        if self.terminal {
            eprint!("\r{line}{padding}");
            if finished {
                eprintln!();
            }
            let _ = std::io::stderr().flush();
        } else {
            eprintln!("{line}");
        }
        self.last_width = line.len();
        self.last_draw = Instant::now();
    }
}

impl Drop for DownloadProgress {
    fn drop(&mut self) {
        if self.terminal && !self.finished {
            eprintln!();
        }
    }
}

fn progress_line(downloaded: u64, total: Option<u64>, speed: f64, finished: bool) -> String {
    let state = if finished {
        "Downloaded"
    } else {
        "Downloading"
    };
    let speed_text = format!("{}/s", format_bytes(speed));
    match total {
        Some(total) if total > 0 => {
            let percent = (downloaded as f64 / total as f64 * 100.0).min(100.0);
            let remaining = total.saturating_sub(downloaded);
            let eta = if speed > 0.0 {
                format_duration(Duration::from_secs_f64(remaining as f64 / speed))
            } else {
                "--:--".into()
            };
            format!(
                "{state}: {percent:5.1}%  {} / {}  {speed_text}  ETA {eta}",
                format_bytes(downloaded as f64),
                format_bytes(total as f64)
            )
        }
        _ => format!("{state}: {}  {speed_text}", format_bytes(downloaded as f64)),
    }
}

fn format_bytes(bytes: f64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes.max(0.0);
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let hours = seconds / 3600;
    let minutes = seconds % 3600 / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_progress_with_speed_and_eta() {
        assert_eq!(
            progress_line(512 * 1024, Some(1024 * 1024), 128.0 * 1024.0, false),
            "Downloading:  50.0%  512.0 KiB / 1.0 MiB  128.0 KiB/s  ETA 00:04"
        );
    }

    #[test]
    fn formats_long_download_eta() {
        assert_eq!(format_duration(Duration::from_secs(3723)), "01:02:03");
    }
}
