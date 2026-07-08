use crate::error::{AppError, Result};
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn download_directory() -> Result<PathBuf> {
    let configured = state_directory()?.join("download-directory.txt");
    if configured.exists() {
        let value = fs::read_to_string(configured)?;
        let value = value.trim();
        if !value.is_empty() {
            return Ok(PathBuf::from(value));
        }
    }

    Ok(default_download_directory())
}

fn default_download_directory() -> PathBuf {
    home_directory()
        .map(|home| home.join("Downloads").join("yt-dlp-tauri"))
        .unwrap_or_else(|| {
            env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("downloads")
        })
}

pub fn app_data_root() -> Result<PathBuf> {
    if cfg!(target_os = "windows") {
        if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
            return Ok(PathBuf::from(local_app_data).join("yt-dlp-tauri"));
        }
    }

    if let Ok(xdg_data_home) = env::var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(xdg_data_home).join("yt-dlp-tauri"));
    }

    home_directory()
        .map(|home| home.join(".local").join("share").join("yt-dlp-tauri"))
        .ok_or_else(|| AppError::Custom("Unable to determine app data directory.".to_string()))
}

pub fn state_directory() -> Result<PathBuf> {
    Ok(app_data_root()?.join("state"))
}

pub fn log_directory() -> Result<PathBuf> {
    Ok(app_data_root()?.join("logs"))
}

pub fn ensure_writable_directories() -> Result<()> {
    fs::create_dir_all(app_data_root()?)?;
    fs::create_dir_all(state_directory()?)?;
    fs::create_dir_all(log_directory()?)?;
    fs::create_dir_all(download_directory()?)?;
    Ok(())
}

pub fn append_log(phase: &str, message: &str) {
    let Ok(directory) = log_directory() else {
        return;
    };
    if fs::create_dir_all(&directory).is_err() {
        return;
    }
    let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("app.log"))
    else {
        return;
    };
    let sanitized = message.replace('\r', " ").replace('\n', " ");
    let _ = writeln!(file, "{} [{phase}] {sanitized}", unix_timestamp());
}

pub(crate) fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn home_directory() -> Option<PathBuf> {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
}
