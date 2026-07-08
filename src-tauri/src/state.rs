use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Clone)]
pub struct AppState {
    pub download_directory: String,
    pub tools_root: String,
    pub yt_dlp_path: String,
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub deno_path: String,
    pub cookies_file: Option<String>,
    pub cookies_status: String,
    pub proxy_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ToolStatus {
    pub name: String,
    pub relative_path: String,
    pub full_path: String,
    pub availability: String,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VideoMetadata {
    pub title: String,
    pub id: Option<String>,
    pub webpage_url: String,
    pub thumbnail_url: Option<String>,
    pub thumbnail_urls: Vec<String>,
    pub duration_seconds: Option<f64>,
    pub description: Option<String>,
    pub format_options: Vec<VideoFormatOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFormatOption {
    pub label: String,
    pub format_selector: String,
    pub height: Option<u32>,
    pub extension: String,
    pub is_best: bool,
}

#[derive(Debug, Deserialize)]
pub struct DownloadRequest {
    pub url: String,
    pub format_selector: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub percent: Option<f64>,
    pub status: String,
    pub speed: Option<String>,
    pub eta: Option<String>,
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolInstallProgress {
    pub percent: Option<f64>,
    pub status: String,
    pub tool: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ToolPaths {
    pub root: PathBuf,
    pub yt_dlp: PathBuf,
    pub yt_dlp_relative_path: String,
    pub ffmpeg: PathBuf,
    pub ffmpeg_relative_path: String,
    pub ffmpeg_dir: PathBuf,
    pub ffprobe: PathBuf,
    pub ffprobe_relative_path: String,
    pub deno: PathBuf,
    pub deno_relative_path: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ToolNames {
    pub yt_dlp: &'static str,
    pub ffmpeg: &'static str,
    pub ffprobe: &'static str,
    pub deno: &'static str,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsManifest {
    pub schema_version: u32,
    pub targets: Vec<ManifestTarget>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestTarget {
    pub target: String,
    pub tools: Vec<ManifestTool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestTool {
    pub name: String,
    pub path: String,
    pub source_url: String,
    #[serde(rename = "version")]
    pub _version: Option<String>,
    pub sha256: String,
    pub kind: ManifestToolKind,
    pub archive_path_suffix: Option<String>,
    #[serde(rename = "licenseNotes")]
    pub _license_notes: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ManifestToolKind {
    File,
    Zip,
}

#[derive(Clone, Default)]
pub struct DownloadProcessState {
    pub active_pid: Arc<Mutex<Option<u32>>>,
    pub cancel_requested: Arc<Mutex<bool>>,
}

/// Tracks which site the (singleton, reused) login webview window is currently
/// pointed at, so the cookie-sync-on-close handler reads the *current* target
/// even when the window is reused across multiple `open_login_window` calls.
#[derive(Default)]
pub struct LoginState(pub Mutex<Option<String>>);

pub struct PreparedCookiesFile {
    pub path: PathBuf,
    pub temporary: bool,
}

impl PreparedCookiesFile {
    pub fn new(path: PathBuf, temporary: bool) -> Self {
        Self { path, temporary }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PreparedCookiesFile {
    fn drop(&mut self) {
        if self.temporary {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    pub yt_dlp_path: String,
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub deno_path: String,
}

#[derive(Default)]
pub struct CachedToolPaths(pub Mutex<Option<ToolPaths>>);
