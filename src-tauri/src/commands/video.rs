use crate::error::{AppError, Result};
use crate::state::{
    CachedToolPaths, DownloadProcessState, DownloadProgress, DownloadRequest,
};
use crate::utils::{
    append_log, background_command, cleanup_incomplete_downloads, clear_active_process,
    download_directory, emit_progress, ensure_writable_directories, kill_process_tree,
    locate_tools, parse_metadata_json, parse_progress_line, prepared_cookies_file_for_url,
    process_failure_message, proxy_url, require_tools, set_active_process, validate_http_url,
    was_cancel_requested, BEST_MP4_FORMAT, OUTPUT_PATH_PREFIX, PROGRESS_PREFIX,
    yt_dlp_cookie_args, yt_dlp_proxy_args,
};
use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::Manager;

#[tauri::command]
pub async fn parse_metadata(
    app: tauri::AppHandle,
    url: String,
) -> Result<crate::state::VideoMetadata> {
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        validate_http_url(&url)?;
        let cache = app_clone.state::<CachedToolPaths>();
        let tools = locate_tools(&app_clone, &cache)?;
        require_tools(&tools)?;
        let cookies_file = prepared_cookies_file_for_url(&url)?;
        let proxy = proxy_url()?;
        append_log("metadata", &format!("Parsing {url}"));

        let mut command = background_command(&tools.yt_dlp);
        let output = command
            .args([
                "--ignore-config",
                "--no-playlist",
                "--dump-single-json",
                "--ffmpeg-location",
            ])
            .arg(&tools.ffmpeg_dir)
            .args(["--js-runtimes"])
            .arg(format!("deno:{}", tools.deno.display()))
            .args(yt_dlp_cookie_args(
                cookies_file.as_ref().map(|f| f.path()),
            ))
            .args(yt_dlp_proxy_args(proxy.as_deref()))
            .arg(&url)
            .output()?;

        if !output.status.success() {
            append_log("metadata", "Failed to parse metadata.");
            return Err(AppError::Custom(process_failure_message(
                "Failed to parse video metadata.",
                output.status.code(),
                &output.stderr,
                &output.stdout,
            )));
        }

        parse_metadata_json(&String::from_utf8_lossy(&output.stdout), &url)
    })
    .await?
}

#[tauri::command]
pub async fn download_video(
    app: tauri::AppHandle,
    process_state: tauri::State<'_, DownloadProcessState>,
    request: DownloadRequest,
) -> Result<Option<String>> {
    let process_state = process_state.inner().clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn_blocking(move || {
        validate_http_url(&request.url)?;
        let cache = app_clone.state::<CachedToolPaths>();
        let tools = locate_tools(&app_clone, &cache)?;
        require_tools(&tools)?;
        ensure_writable_directories()?;
        let output_dir = download_directory()?;
        let cookies_file = prepared_cookies_file_for_url(&request.url)?;
        let proxy = proxy_url()?;
        append_log("download", &format!("Starting {} {}", request.label, request.url));

        let mut command = background_command(&tools.yt_dlp);
        command
            .args([
                "--ignore-config",
                "--no-playlist",
                "--newline",
                "--paths",
            ])
            .arg(format!("home:{}", output_dir.display()))
            .args(["--output", "%(title).200B [%(id)s].%(ext)s", "--format"])
            .arg(if request.format_selector.trim().is_empty() {
                BEST_MP4_FORMAT.to_string()
            } else {
                request.format_selector.clone()
            })
            .args(["--merge-output-format", "mp4", "--ffmpeg-location"])
            .arg(&tools.ffmpeg_dir)
            .args(["--js-runtimes"])
            .arg(format!("deno:{}", tools.deno.display()))
            .args(yt_dlp_cookie_args(
                cookies_file.as_ref().map(|f| f.path()),
            ))
            .args(yt_dlp_proxy_args(proxy.as_deref()))
            .args([
                "--progress-template",
                &format!(
                    "{}%(progress.status)s|%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s",
                    PROGRESS_PREFIX
                ),
                "--print",
                &format!("after_move:{}%(filepath)s", OUTPUT_PATH_PREFIX),
                "--progress",
            ])
            .arg(&request.url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let download_started_at = std::time::SystemTime::now();
        let mut child = command
            .spawn()
            .map_err(|error| AppError::Custom(format!("Failed to start yt-dlp at {}: {error}", tools.yt_dlp.display())))?;
        
        let pid = child.id();
        set_active_process(&process_state, pid)?;

        emit_progress(
            &app,
            DownloadProgress {
                percent: None,
                status: format!("Starting {}", request.label),
                speed: None,
                eta: None,
                raw: None,
            },
        );

        let output_path = Arc::new(Mutex::new(None::<String>));
        let stderr_lines = Arc::new(Mutex::new(Vec::<String>::new()));

        let stdout_handle = child.stdout.take().map(|stdout| {
            let app = app.clone();
            let output_path = Arc::clone(&output_path);
            thread::spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(std::result::Result::ok) {
                    if let Some(progress) = parse_progress_line(&line) {
                        emit_progress(&app, progress);
                    }

                    if let Some(path) = line.strip_prefix(OUTPUT_PATH_PREFIX) {
                        if let Ok(mut guard) = output_path.lock() {
                            *guard = Some(path.trim().to_string());
                        }
                    }
                }
            })
        });

        let stderr_handle = child.stderr.take().map(|stderr| {
            let stderr_lines = Arc::clone(&stderr_lines);
            thread::spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(std::result::Result::ok) {
                    if let Ok(mut guard) = stderr_lines.lock() {
                        guard.push(line);
                    }
                }
            })
        });

        let status = child.wait()?;
        if let Some(handle) = stdout_handle {
            let _ = handle.join();
        }
        if let Some(handle) = stderr_handle {
            let _ = handle.join();
        }

        if !status.success() {
            let details = stderr_lines.lock().map(|lines| lines.join("\n")).unwrap_or_default();
            let cancelled = was_cancel_requested(&process_state);
            clear_active_process(&process_state, pid);
            if cancelled {
                cleanup_incomplete_downloads(&output_dir, download_started_at);
                append_log("download", "Cancelled by user.");
                return Err(AppError::Custom("Download cancelled.".to_string()));
            }
            append_log("download", &format!("Failed. {details}"));
            return Err(AppError::Custom(process_failure_message(
                "Download failed.",
                status.code(),
                details.as_bytes(),
                &[],
            )));
        }

        clear_active_process(&process_state, pid);

        emit_progress(
            &app,
            DownloadProgress {
                percent: Some(100.0),
                status: "Completed".to_string(),
                speed: None,
                eta: None,
                raw: None,
            },
        );

        let saved_path = output_path.lock().ok().and_then(|guard| guard.clone());
        append_log("download", &format!("Completed. Output={}", saved_path.as_deref().unwrap_or("unknown")));
        Ok(saved_path)
    })
    .await?
}

#[tauri::command]
pub async fn cancel_download(
    process_state: tauri::State<'_, DownloadProcessState>,
) -> Result<()> {
    let pid = {
        let guard = process_state.active_pid.lock().map_err(|e| AppError::Lock(e.to_string()))?;
        *guard
    };

    let Some(pid) = pid else {
        return Ok(());
    };

    {
        let mut guard = process_state.cancel_requested.lock().map_err(|e| AppError::Lock(e.to_string()))?;
        *guard = true;
    }

    tauri::async_runtime::spawn_blocking(move || kill_process_tree(pid))
        .await?
}
