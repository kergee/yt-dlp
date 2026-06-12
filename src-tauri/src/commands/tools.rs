use crate::error::Result;
use crate::state::{CachedToolPaths, ToolStatus};
use crate::utils::{locate_tools, probe_tool};
use tauri::Manager;

#[tauri::command]
pub async fn check_tools(
    app: tauri::AppHandle,
) -> Result<Vec<ToolStatus>> {
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let cache = app_clone.state::<CachedToolPaths>();
        let tools = locate_tools(&app_clone, &cache)?;
        Ok(vec![
            probe_tool(
                "yt-dlp",
                &tools.yt_dlp_relative_path,
                &tools.yt_dlp,
                &["--version"],
            ),
            probe_tool(
                "ffmpeg",
                &tools.ffmpeg_relative_path,
                &tools.ffmpeg,
                &["-version"],
            ),
            probe_tool(
                "ffprobe",
                &tools.ffprobe_relative_path,
                &tools.ffprobe,
                &["-version"],
            ),
            probe_tool(
                "deno",
                &tools.deno_relative_path,
                &tools.deno,
                &["--version"],
            ),
        ])
    })
    .await?
}
