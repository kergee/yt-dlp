use super::paths::append_log;
use crate::error::{AppError, Result};
use crate::state::DownloadProcessState;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn background_command(program: impl AsRef<OsStr>) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut command = Command::new(program);
        command.creation_flags(CREATE_NO_WINDOW);
        command
    }
    #[cfg(not(windows))]
    {
        Command::new(program)
    }
}

pub fn set_active_process(state: &DownloadProcessState, pid: u32) -> Result<()> {
    {
        let mut guard = state.active_pid.lock().map_err(|e| AppError::Lock(e.to_string()))?;
        *guard = Some(pid);
    }
    {
        let mut guard = state.cancel_requested.lock().map_err(|e| AppError::Lock(e.to_string()))?;
        *guard = false;
    }
    Ok(())
}

pub fn clear_active_process(state: &DownloadProcessState, pid: u32) {
    if let Ok(mut guard) = state.active_pid.lock() {
        if guard.is_some_and(|active_pid| active_pid == pid) {
            *guard = None;
        }
    }
    if let Ok(mut guard) = state.cancel_requested.lock() {
        *guard = false;
    }
}

pub fn was_cancel_requested(state: &DownloadProcessState) -> bool {
    state
        .cancel_requested
        .lock()
        .map(|guard| *guard)
        .unwrap_or(false)
}

pub fn kill_process_tree(pid: u32) -> Result<()> {
    let pid_text = pid.to_string();
    let mut command = if cfg!(target_os = "windows") {
        let mut command = background_command("taskkill");
        command.args(["/PID", &pid_text, "/T", "/F"]);
        command
    } else {
        let mut command = background_command("kill");
        command.args(["-TERM", &pid_text]);
        command
    };
    let output = command
        .output()
        .map_err(|error| AppError::Custom(format!("Failed to start cancel command for process {pid}: {error}")))?;

    if output.status.success() {
        append_log("download", &format!("Cancel requested for process {pid}."));
        Ok(())
    } else {
        Err(AppError::Custom(process_failure_message(
            &format!("Failed to cancel process {pid}."),
            output.status.code(),
            &output.stderr,
            &output.stdout,
        )))
    }
}

pub fn first_line(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
}

pub fn process_failure_message(
    action: &str,
    code: Option<i32>,
    stderr: &[u8],
    stdout: &[u8],
) -> String {
    let status = match code {
        Some(code) => format!("Exit code {code}."),
        None => "Process terminated without an exit code.".to_string(),
    };
    let details = first_line(stderr).or_else(|| first_line(stdout));

    match details {
        Some(details) => format!("{action} {status} {details}"),
        None => format!("{action} {status}"),
    }
}

/// True for yt-dlp's own incomplete-download artifact naming (resume files,
/// fragment parts, and the sidecar `.ytdl` resume-state file).
fn is_incomplete_download_artifact(file_name: &str) -> bool {
    file_name.ends_with(".part")
        || file_name.ends_with(".ytdl")
        || file_name.contains(".part-Frag")
}

/// Best-effort removal of yt-dlp's incomplete-download artifacts left behind by a
/// user-cancelled download. Only removes files matching the known artifact naming
/// that were modified at or after `started_after`, so unrelated or older resumable
/// downloads in the same directory are left alone. Never fails the caller.
pub fn cleanup_incomplete_downloads(output_dir: &Path, started_after: SystemTime) {
    // Windows' filesystem mtime clock and SystemTime::now() can drift by a few
    // milliseconds, so a file written right after `started_after` was captured can
    // appear slightly older than it. A small margin avoids skipping this download's
    // own artifacts over that drift while still leaving older resumable downloads alone.
    let cutoff = started_after
        .checked_sub(std::time::Duration::from_secs(2))
        .unwrap_or(started_after);

    let Ok(entries) = fs::read_dir(output_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !is_incomplete_download_artifact(name) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if modified >= cutoff {
            let _ = fs::remove_file(&path);
        }
    }
}

pub fn open_path(path: &Path) -> Result<()> {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer");
        command.arg(path);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(path);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };

    command.spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_failure_message_prefers_stderr() {
        let message = process_failure_message(
            "Failed to download yt-dlp.",
            Some(22),
            b"curl: (22) The requested URL returned error: 404\n",
            b"ignored stdout\n",
        );

        assert_eq!(
            message,
            "Failed to download yt-dlp. Exit code 22. curl: (22) The requested URL returned error: 404"
        );
    }

    #[test]
    fn process_failure_message_falls_back_to_stdout() {
        let message = process_failure_message(
            "Failed to extract archive.",
            None,
            b"",
            b"Archive is invalid\n",
        );

        assert_eq!(
            message,
            "Failed to extract archive. Process terminated without an exit code. Archive is invalid"
        );
    }

    #[test]
    fn recognizes_incomplete_download_artifact_names() {
        assert!(is_incomplete_download_artifact("My Video [abc123].mp4.part"));
        assert!(is_incomplete_download_artifact("My Video [abc123].ytdl"));
        assert!(is_incomplete_download_artifact("My Video [abc123].f137.part-Frag3"));
        assert!(!is_incomplete_download_artifact("My Video [abc123].mp4"));
    }

    #[test]
    fn cleanup_removes_only_recent_incomplete_artifacts() {
        let dir = std::env::temp_dir().join(format!(
            "yt-dlp-tauri-test-cleanup-{}-{}",
            std::process::id(),
            unix_timestamp_for_test()
        ));
        fs::create_dir_all(&dir).expect("create test dir");

        let started_after = SystemTime::now();

        let stale_part = dir.join("stale.mp4.part");
        fs::write(&stale_part, b"stale").expect("write stale part");
        filetime_set_past(&stale_part);

        let fresh_part = dir.join("fresh.mp4.part");
        fs::write(&fresh_part, b"fresh").expect("write fresh part");

        let fresh_finished = dir.join("fresh.mp4");
        fs::write(&fresh_finished, b"finished").expect("write finished file");

        cleanup_incomplete_downloads(&dir, started_after);

        assert!(stale_part.exists(), "older .part files should be left alone");
        assert!(!fresh_part.exists(), "in-progress .part file should be removed");
        assert!(fresh_finished.exists(), "completed files should never be removed");

        let _ = fs::remove_dir_all(&dir);
    }

    fn unix_timestamp_for_test() -> u64 {
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or_default()
    }

    fn filetime_set_past(path: &Path) {
        let past = SystemTime::now() - std::time::Duration::from_secs(3600);
        let file = fs::OpenOptions::new().write(true).open(path).expect("open for mtime backdate");
        file.set_modified(past).expect("backdate mtime");
    }
}
