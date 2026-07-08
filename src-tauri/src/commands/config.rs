use crate::error::{AppError, Result};
use crate::state::{AppState, CachedToolPaths, ToolsConfig};
use crate::utils::{
    build_app_state, cookies_file_state_path, download_directory,
    open_path, save_tools_config, state_directory, tools_directory,
    validate_cookies_file_path,
};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[tauri::command]
pub async fn get_app_state() -> Result<AppState> {
    build_app_state(String::new())
}

#[tauri::command]
pub async fn set_tools_directory(
    app: tauri::AppHandle,
    _directory: String,
    yt_dlp_path: String,
    ffmpeg_path: String,
    ffprobe_path: String,
    deno_path: String,
) -> Result<AppState> {
    let config = ToolsConfig {
        yt_dlp_path,
        ffmpeg_path,
        ffprobe_path,
        deno_path,
    };
    save_tools_config(&config)?;

    // Invalidate CachedToolPaths
    let cache = app.state::<CachedToolPaths>();
    if let Ok(mut guard) = cache.0.lock() {
        *guard = None;
    }

    build_app_state(String::new())
}

#[tauri::command]
pub async fn open_tools_directory() -> Result<()> {
    let path = tools_directory()?;
    let _ = fs::create_dir_all(&path);
    open_path(&path)?;
    Ok(())
}

#[tauri::command]
pub async fn set_download_directory(directory: String) -> Result<AppState> {
    tauri::async_runtime::spawn_blocking(move || {
        let trimmed = directory.trim();
        if trimmed.is_empty() {
            return Err(AppError::Custom("Download directory cannot be empty.".to_string()));
        }

        let path = PathBuf::from(trimmed);
        fs::create_dir_all(&path)?;
        let state_dir = state_directory()?;
        fs::create_dir_all(&state_dir)?;
        fs::write(
            state_dir.join("download-directory.txt"),
            path.display().to_string(),
        )?;

        build_app_state(String::new())
    })
    .await?
}

#[tauri::command]
pub async fn reset_download_directory() -> Result<AppState> {
    tauri::async_runtime::spawn_blocking(move || {
        let state_file = state_directory()?.join("download-directory.txt");
        if state_file.exists() {
            fs::remove_file(state_file)?;
        }

        let directory = download_directory()?;
        fs::create_dir_all(&directory)?;
        build_app_state(String::new())
    })
    .await?
}

#[tauri::command]
pub async fn set_cookies_file(path: String) -> Result<AppState> {
    tauri::async_runtime::spawn_blocking(move || {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err(AppError::Custom("Cookie file cannot be empty.".to_string()));
        }

        let path = PathBuf::from(trimmed);
        validate_cookies_file_path(&path)?;
        let state_dir = state_directory()?;
        fs::create_dir_all(&state_dir)?;
        fs::write(
            state_dir.join("cookies-file.txt"),
            path.display().to_string(),
        )?;

        build_app_state(String::new())
    })
    .await?
}

#[tauri::command]
pub async fn clear_cookies_file() -> Result<AppState> {
    tauri::async_runtime::spawn_blocking(move || {
        let state_file = cookies_file_state_path()?;
        if state_file.exists() {
            fs::remove_file(state_file)?;
        }

        build_app_state(String::new())
    })
    .await?
}

#[tauri::command]
pub async fn set_proxy_url(url: String) -> Result<AppState> {
    tauri::async_runtime::spawn_blocking(move || {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return Err(AppError::Custom("Proxy address cannot be empty.".to_string()));
        }

        let state_dir = state_directory()?;
        fs::create_dir_all(&state_dir)?;
        fs::write(state_dir.join("proxy-url.txt"), trimmed)?;

        build_app_state(String::new())
    })
    .await?
}

#[tauri::command]
pub async fn clear_proxy_url() -> Result<AppState> {
    tauri::async_runtime::spawn_blocking(move || {
        let state_file = state_directory()?.join("proxy-url.txt");
        if state_file.exists() {
            fs::remove_file(state_file)?;
        }

        build_app_state(String::new())
    })
    .await?
}

#[tauri::command]
pub async fn open_download_directory() -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let directory = download_directory()?;
        fs::create_dir_all(&directory)?;
        open_path(&directory)
    })
    .await?
}
