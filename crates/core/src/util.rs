use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use jwalk::WalkDir;

/// Builds a `Command` that never flashes a console window on Windows
/// (the release build has no console of its own).
pub fn command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

pub fn run_command(program: &str, args: &[&str], timeout: Duration) -> Option<String> {
    let child = command(program).args(args).output().ok()?;
    if child.status.success() {
        Some(String::from_utf8_lossy(&child.stdout).trim().to_string())
    } else {
        let _ = timeout;
        None
    }
}

pub fn which(program: &str) -> bool {
    if cfg!(windows) {
        command("where")
            .arg(program)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        command("which")
            .arg(program)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

pub fn dir_size(path: &Path) -> u64 {
    if !path.is_dir() {
        return file_size(path);
    }
    WalkDir::new(path)
        .skip_hidden(false)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

pub fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

pub fn modified_time(path: &Path) -> Option<DateTime<Utc>> {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(system_time_to_datetime)
}

fn system_time_to_datetime(t: SystemTime) -> Option<DateTime<Utc>> {
    let dur = t.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    DateTime::from_timestamp(dur.as_secs() as i64, dur.subsec_nanos())
}

pub fn home_dir() -> Option<PathBuf> {
    directories::UserDirs::new().map(|u| u.home_dir().to_path_buf())
}

pub fn local_data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        home_dir().map(|h| h.join(".local").join("share"))
    }
}

pub fn cache_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|b| b.cache_dir().to_path_buf())
}

pub fn temp_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(tmp) = std::env::var("TEMP") {
        dirs.push(PathBuf::from(tmp));
    }
    if let Ok(tmp) = std::env::var("TMP") {
        dirs.push(PathBuf::from(tmp));
    }
    if let Ok(tmp) = std::env::var("TMPDIR") {
        dirs.push(PathBuf::from(tmp));
    }
    #[cfg(windows)]
    if let Some(local) = local_data_dir() {
        dirs.push(local.join("Temp"));
    }
    #[cfg(unix)]
    {
        dirs.push(PathBuf::from("/tmp"));
        if let Some(home) = home_dir() {
            dirs.push(home.join(".cache"));
        }
    }
    dirs.sort();
    dirs.dedup();
    dirs
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        let n = format!("{:.1}", size);
        // Portuguese and Spanish use a decimal comma.
        let n = match crate::i18n::lang() {
            crate::i18n::Lang::En => n,
            _ => n.replace('.', ","),
        };
        format!("{} {}", n, UNITS[unit])
    }
}

pub fn age_days(path: &Path) -> Option<u32> {
    let modified = modified_time(path)?;
    let now = Utc::now();
    let days = (now - modified).num_days();
    Some(days.max(0) as u32)
}
