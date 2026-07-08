use super::cookies::cookies_file;
use super::net::proxy_url;
use super::paths::download_directory;
use super::tools::read_tools_config;
use crate::error::Result;
use crate::state::AppState;
use std::path::PathBuf;

pub fn build_app_state(tools_root: String) -> Result<AppState> {
    let config = read_tools_config().unwrap_or_default();

    let cookies = cookies_file()?.map(|p| p.display().to_string());
    let cookies_status = if let Some(ref path) = cookies {
        if PathBuf::from(path).exists() {
            "valid".to_string()
        } else {
            "warning".to_string()
        }
    } else {
        "none".to_string()
    };

    Ok(AppState {
        download_directory: download_directory()?.display().to_string(),
        tools_root,
        yt_dlp_path: config.yt_dlp_path,
        ffmpeg_path: config.ffmpeg_path,
        ffprobe_path: config.ffprobe_path,
        deno_path: config.deno_path,
        cookies_file: cookies,
        cookies_status,
        proxy_url: proxy_url()?,
    })
}
