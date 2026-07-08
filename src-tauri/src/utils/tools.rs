use super::paths::{app_data_root, state_directory, unix_timestamp};
use super::process::{background_command, first_line, process_failure_message};
use crate::error::{AppError, Result};
use crate::state::{
    CachedToolPaths, ManifestTarget, ManifestTool, ManifestToolKind, ToolInstallProgress,
    ToolNames, ToolPaths, ToolStatus, ToolsConfig, ToolsManifest,
};
use crate::zip_utils::extract_zip_archive;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

pub const TOOLS_DIRECTORY: &str = "Tools";
pub const TOOLS_MANIFEST_FILE: &str = "tools-manifest.json";

pub fn read_tools_config() -> Result<ToolsConfig> {
    let state_dir = state_directory()?;
    let config_path = state_dir.join("tools-config.json");
    if config_path.exists() {
        let content = fs::read_to_string(&config_path).unwrap_or_default();
        if let Ok(config) = serde_json::from_str::<ToolsConfig>(&content) {
            return Ok(config);
        }
    }
    Ok(ToolsConfig::default())
}

pub fn save_tools_config(config: &ToolsConfig) -> Result<()> {
    let state_dir = state_directory()?;
    fs::create_dir_all(&state_dir)?;
    let config_path = state_dir.join("tools-config.json");
    let content = serde_json::to_string_pretty(config)?;
    fs::write(&config_path, content)?;
    Ok(())
}

pub fn locate_tools(app: &AppHandle, cache: &CachedToolPaths) -> Result<ToolPaths> {
    if let Ok(guard) = cache.0.lock() {
        if let Some(ref paths) = *guard {
            return Ok(paths.clone());
        }
    }

    let paths = locate_tools_internal(app)?;

    if let Ok(mut guard) = cache.0.lock() {
        *guard = Some(paths.clone());
    }

    Ok(paths)
}

fn resolve_relative_path(path: &Path, base: &Path) -> PathBuf {
    if path.is_relative() {
        base.join(path)
    } else {
        path.to_path_buf()
    }
}

fn locate_tools_internal(app: &AppHandle) -> Result<ToolPaths> {
    let target = current_tool_target()?;
    let names = tool_names_for_target(&target)
        .ok_or_else(|| AppError::Custom(format!("Unsupported tool target: {target}.")))?;

    let config = read_tools_config().unwrap_or_default();

    let mut roots = Vec::new();
    if let Ok(root) = writable_tools_root(&target) {
        roots.push(root);
    }
    if let Ok(resource_dir) = app.path().resource_dir() {
        roots.push(resource_dir.join(TOOLS_DIRECTORY).join(&target));
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            roots.push(parent.join(TOOLS_DIRECTORY).join(&target));
        }
    }
    roots.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(TOOLS_DIRECTORY)
            .join(&target),
    );
    if let Ok(current_dir) = env::current_dir() {
        roots.push(
            current_dir
                .join("src-tauri")
                .join(TOOLS_DIRECTORY)
                .join(&target),
        );
        roots.push(current_dir.join(TOOLS_DIRECTORY).join(&target));
    }

    let default_root = roots
        .into_iter()
        .find(|root| root.join("yt-dlp").join(names.yt_dlp).exists())
        .unwrap_or_else(|| {
            writable_tools_root(&target).unwrap_or_else(|_| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join(TOOLS_DIRECTORY)
                    .join(&target)
            })
        });

    let default_yt_dlp = default_root.join("yt-dlp").join(names.yt_dlp);
    let default_ffmpeg = default_root.join("ffmpeg").join("bin").join(names.ffmpeg);
    let default_ffprobe = default_root.join("ffmpeg").join("bin").join(names.ffprobe);
    let default_deno = default_root.join("deno").join(names.deno);

    let exe_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|parent| parent.to_path_buf()));

    let get_path = |custom: &str, default: PathBuf, rel: String| -> (PathBuf, String) {
        if custom.is_empty() {
            (default, rel)
        } else {
            let path = PathBuf::from(custom);
            let resolved = if let Some(ref exe_dir) = exe_dir {
                resolve_relative_path(&path, exe_dir)
            } else {
                path
            };
            (resolved, custom.to_string())
        }
    };

    let (yt_dlp, yt_dlp_relative_path) = get_path(&config.yt_dlp_path, default_yt_dlp, tool_relative_path(&target, &["yt-dlp", names.yt_dlp]));
    let (ffmpeg, ffmpeg_relative_path) = get_path(&config.ffmpeg_path, default_ffmpeg.clone(), tool_relative_path(&target, &["ffmpeg", "bin", names.ffmpeg]));
    let (ffprobe, ffprobe_relative_path) = get_path(&config.ffprobe_path, default_ffprobe, tool_relative_path(&target, &["ffmpeg", "bin", names.ffprobe]));
    let (deno, deno_relative_path) = get_path(&config.deno_path, default_deno, tool_relative_path(&target, &["deno", names.deno]));

    let ffmpeg_dir = ffmpeg.parent().unwrap_or(default_ffmpeg.parent().unwrap()).to_path_buf();

    Ok(ToolPaths {
        root: default_root,
        yt_dlp,
        yt_dlp_relative_path,
        ffmpeg,
        ffmpeg_relative_path,
        ffmpeg_dir,
        ffprobe,
        ffprobe_relative_path,
        deno,
        deno_relative_path,
    })
}

pub fn tool_names_for_target(target: &str) -> Option<ToolNames> {
    match target {
        "win-x64" | "win-arm64" => Some(ToolNames {
            yt_dlp: "yt-dlp.exe",
            ffmpeg: "ffmpeg.exe",
            ffprobe: "ffprobe.exe",
            deno: "deno.exe",
        }),
        "macos-x64" | "macos-arm64" => Some(ToolNames {
            yt_dlp: "yt-dlp",
            ffmpeg: "ffmpeg",
            ffprobe: "ffprobe",
            deno: "deno",
        }),
        _ => None,
    }
}

pub fn tool_target_from(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("windows", "x86_64") => Some("win-x64"),
        ("windows", "aarch64") => Some("win-arm64"),
        ("macos", "x86_64") => Some("macos-x64"),
        ("macos", "aarch64") => Some("macos-arm64"),
        _ => None,
    }
}

pub fn current_tool_target() -> Result<String> {
    env::var("YT_DLP_TOOL_TARGET")
        .or_else(|_| env::var("YT_DLP_WINDOWS_TOOL_TARGET"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(Ok)
        .unwrap_or_else(|| {
            tool_target_from(env::consts::OS, env::consts::ARCH)
                .map(|target| target.to_string())
                .ok_or_else(|| {
                    AppError::Custom(format!(
                        "Unsupported tool target for {}-{}. Supported targets: win-x64, win-arm64, macos-x64, macos-arm64.",
                        env::consts::OS,
                        env::consts::ARCH
                    ))
                })
        })
}

pub fn tool_relative_path(target: &str, segments: &[&str]) -> String {
    let mut path_segments = vec![TOOLS_DIRECTORY, target];
    path_segments.extend_from_slice(segments);
    path_segments.join("/")
}

pub fn writable_tools_root(_target: &str) -> Result<PathBuf> {
    tools_directory()
}

pub fn tools_directory() -> Result<PathBuf> {
    let state_dir = state_directory()?;
    let path_file = state_dir.join("tools-directory.txt");
    if let Ok(content) = fs::read_to_string(&path_file) {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    let target = current_tool_target()?;
    Ok(app_data_root()?.join(TOOLS_DIRECTORY).join(target))
}

pub fn read_manifest_target(app: &AppHandle, target: &str) -> Result<ManifestTarget> {
    let manifest_path = manifest_path(app)?;
    let json = fs::read_to_string(&manifest_path)?;
    manifest_target_from_json(&json, target)
}

pub fn manifest_target_from_json(json: &str, target: &str) -> Result<ManifestTarget> {
    let manifest: ToolsManifest = serde_json::from_str(json)?;
    if manifest.schema_version < 2 {
        return Err(AppError::Custom("tools-manifest.json schemaVersion must be 2 or newer.".to_string()));
    }

    manifest
        .targets
        .into_iter()
        .find(|item| item.target == target)
        .ok_or_else(|| AppError::Custom(format!("No tool manifest target found for {target}.")))
}

pub fn manifest_path(app: &AppHandle) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join(TOOLS_MANIFEST_FILE));
    }

    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join(TOOLS_MANIFEST_FILE));
        }
    }

    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(TOOLS_MANIFEST_FILE));

    if let Ok(current_dir) = env::current_dir() {
        candidates.push(current_dir.join("src-tauri").join(TOOLS_MANIFEST_FILE));
        candidates.push(current_dir.join(TOOLS_MANIFEST_FILE));
    }

    candidates
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| AppError::Custom("Unable to locate tools-manifest.json.".to_string()))
}

pub fn install_manifest_target(
    app: &AppHandle,
    target: &ManifestTarget,
    root: &Path,
) -> Result<()> {
    let total_steps = target.tools.len().max(1) as f64;
    let mut zip_groups = BTreeMap::<String, Vec<ManifestTool>>::new();

    for tool in &target.tools {
        match tool.kind {
            ManifestToolKind::File => {
                let step = installed_tool_count(root, &target.tools) as f64;
                install_file_tool(
                    app,
                    root,
                    tool,
                    step / total_steps * 100.0,
                    100.0 / total_steps,
                )
                .map_err(|error| AppError::Custom(format!("Failed to install {}. {error}", tool.name)))?;
            }
            ManifestToolKind::Zip => {
                zip_groups
                    .entry(tool.source_url.clone())
                    .or_default()
                    .push(tool.clone());
            }
        }
    }

    for tools in zip_groups.values() {
        let step = installed_tool_count(root, &target.tools) as f64;
        let group_label = zip_group_label(tools);
        install_zip_tools(
            app,
            root,
            tools,
            step / total_steps * 100.0,
            tools.len() as f64 / total_steps * 100.0,
        )
        .map_err(|error| AppError::Custom(format!("Failed to install {group_label}. {error}")))?;
    }

    for tool in &target.tools {
        let path = root.join(relative_manifest_tool_path(tool)?);
        if !path.exists() {
            return Err(AppError::Custom(format!(
                "Installed tool is missing after install: {}",
                path.display()
            )));
        }
        verify_sha256(&path, &tool.sha256)?;
    }

    Ok(())
}

fn installed_tool_count(root: &Path, tools: &[ManifestTool]) -> usize {
    tools
        .iter()
        .filter(|tool| {
            relative_manifest_tool_path(tool)
                .map(|path| root.join(path).exists())
                .unwrap_or(false)
        })
        .count()
}

fn install_file_tool(
    app: &AppHandle,
    root: &Path,
    tool: &ManifestTool,
    base_percent: f64,
    span_percent: f64,
) -> Result<()> {
    let destination = root.join(relative_manifest_tool_path(tool)?);
    download_to_destination(app, tool, &destination, base_percent, span_percent * 0.82)?;
    emit_tool_install_progress(
        app,
        ToolInstallProgress {
            percent: Some((base_percent + span_percent * 0.9).min(99.0)),
            status: format!("Verifying {}", tool.name),
            tool: Some(tool.name.clone()),
        },
    );
    verify_sha256(&destination, &tool.sha256)?;
    mark_executable(&destination)?;
    Ok(())
}

fn install_zip_tools(
    app: &AppHandle,
    root: &Path,
    tools: &[ManifestTool],
    base_percent: f64,
    span_percent: f64,
) -> Result<()> {
    let first = tools
        .first()
        .ok_or_else(|| AppError::Custom("Zip tool group cannot be empty.".to_string()))?;
    let temp_root = app_data_root()?.join("tool-downloads");
    fs::create_dir_all(&temp_root)?;
    let zip_path = temp_root.join(format!(
        "{}-{}.zip",
        sanitize_file_name(&first.name),
        unix_timestamp()
    ));

    download_source_to_file(
        app,
        &first.source_url,
        &zip_path,
        &format!("Downloading {}", zip_group_label(tools)),
        first.name.as_str(),
        base_percent,
        span_percent * 0.55,
    )?;

    let extract_root = temp_root.join(format!(
        "extract-{}-{}",
        sanitize_file_name(&first.name),
        unix_timestamp()
    ));
    fs::create_dir_all(&extract_root)?;
    extract_zip_archive(&zip_path, &extract_root)?;

    for (index, tool) in tools.iter().enumerate() {
        let offset = index as f64 / tools.len() as f64;
        emit_tool_install_progress(
            app,
            ToolInstallProgress {
                percent: Some((base_percent + span_percent * (0.58 + offset * 0.34)).min(99.0)),
                status: format!("Extracting {}", tool.name),
                tool: Some(tool.name.clone()),
            },
        );
        extract_tool_from_directory(&extract_root, root, tool)?;
        let destination = root.join(relative_manifest_tool_path(tool)?);
        verify_sha256(&destination, &tool.sha256)?;
        mark_executable(&destination)?;
    }

    let _ = fs::remove_file(zip_path);
    let _ = fs::remove_dir_all(extract_root);
    Ok(())
}

fn download_to_destination(
    app: &AppHandle,
    tool: &ManifestTool,
    destination: &Path,
    base_percent: f64,
    span_percent: f64,
) -> Result<()> {
    let temp = destination.with_extension("download");
    download_source_to_file(
        app,
        &tool.source_url,
        &temp,
        &format!("Downloading {}", tool.name),
        tool.name.as_str(),
        base_percent,
        span_percent,
    )?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::Custom(format!(
                "Failed to prepare directory for {} at {}: {error}",
                tool.name,
                parent.display()
            ))
        })?;
    }
    if destination.exists() {
        fs::remove_file(destination).map_err(|error| {
            AppError::Custom(format!(
                "Failed to replace existing {} at {}: {error}",
                tool.name,
                destination.display()
            ))
        })?;
    }
    fs::rename(&temp, destination).map_err(|error| {
        AppError::Custom(format!(
            "Failed to move downloaded {} from {} to {}: {error}",
            tool.name,
            temp.display(),
            destination.display()
        ))
    })?;
    Ok(())
}

fn download_source_to_file(
    app: &AppHandle,
    source_url: &str,
    destination: &Path,
    status: &str,
    tool_name: &str,
    base_percent: f64,
    span_percent: f64,
) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::Custom(format!(
                "Failed to prepare download directory {}: {error}",
                parent.display()
            ))
        })?;
    }
    emit_tool_install_progress(
        app,
        ToolInstallProgress {
            percent: Some(base_percent.min(99.0)),
            status: status.to_string(),
            tool: Some(tool_name.to_string()),
        },
    );

    let output = background_command("curl")
        .args([
            "-L",
            "--fail",
            "--show-error",
            "--silent",
            "--retry",
            "2",
            "--output",
        ])
        .arg(destination)
        .arg(source_url)
        .output()
        .map_err(|error| AppError::Custom(format!("Failed to start curl for {tool_name}: {error}")))?;

    if !output.status.success() {
        return Err(AppError::Custom(process_failure_message(
            &format!(
                "Failed to download {tool_name} from {source_url} to {}.",
                destination.display()
            ),
            output.status.code(),
            &output.stderr,
            &output.stdout,
        )));
    }

    emit_tool_install_progress(
        app,
        ToolInstallProgress {
            percent: Some((base_percent + span_percent).min(99.0)),
            status: format!("Downloaded {tool_name}"),
            tool: Some(tool_name.to_string()),
        },
    );
    Ok(())
}

fn extract_tool_from_directory(
    extract_root: &Path,
    tools_root: &Path,
    tool: &ManifestTool,
) -> Result<()> {
    let suffix = tool
        .archive_path_suffix
        .as_deref()
        .ok_or_else(|| AppError::Custom(format!("{} is missing archivePathSuffix.", tool.name)))?
        .replace('\\', "/");
    let source = find_file_by_normalized_suffix(extract_root, &suffix)?.ok_or_else(|| {
        AppError::Custom(format!(
            "Unable to find {} at {} in extracted archive {}.",
            tool.name,
            suffix,
            extract_root.display()
        ))
    })?;
    let destination = tools_root.join(relative_manifest_tool_path(tool)?);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::Custom(format!(
                "Failed to prepare directory for {} at {}: {error}",
                tool.name,
                parent.display()
            ))
        })?;
    }
    fs::copy(&source, &destination).map_err(|error| {
        AppError::Custom(format!(
            "Failed to copy {} from {} to {}: {error}",
            tool.name,
            source.display(),
            destination.display()
        ))
    })?;
    Ok(())
}

fn find_file_by_normalized_suffix(root: &Path, suffix: &str) -> Result<Option<PathBuf>> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let normalized = path.to_string_lossy().replace('\\', "/");
                if normalized.ends_with(suffix) {
                    return Ok(Some(path));
                }
            }
        }
    }
    Ok(None)
}

pub fn relative_manifest_tool_path(tool: &ManifestTool) -> Result<PathBuf> {
    let normalized = tool.path.replace('\\', "/");
    let prefix = format!("{TOOLS_DIRECTORY}/");
    let relative = normalized
        .strip_prefix(&prefix)
        .and_then(|value| value.split_once('/').map(|(_, rest)| rest))
        .ok_or_else(|| AppError::Custom(format!("Invalid tool path in manifest: {}", tool.path)))?;
    Ok(PathBuf::from(relative))
}

pub fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    let actual = sha256_file(path)?;
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(AppError::Custom(format!(
            "SHA-256 mismatch for {}. Expected {}, got {}.",
            path.display(),
            expected,
            actual
        )))
    }
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024]; // Used 1MB buffer for better efficiency instead of 64KB
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn sanitize_file_name(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn zip_group_label(tools: &[ManifestTool]) -> String {
    tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect::<Vec<_>>()
        .join(" / ")
}

#[cfg(unix)]
pub fn mark_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn mark_executable(_path: &Path) -> Result<()> {
    Ok(())
}

pub fn require_tools(tools: &ToolPaths) -> Result<()> {
    for path in [&tools.yt_dlp, &tools.ffmpeg, &tools.ffprobe, &tools.deno] {
        if !path.exists() {
            return Err(AppError::Custom(format!("Missing bundled tool: {}", path.display())));
        }
    }
    Ok(())
}

pub fn probe_tool(
    name: &str,
    relative_path: &str,
    full_path: &Path,
    version_args: &[&str],
) -> ToolStatus {
    if !full_path.exists() {
        return ToolStatus {
            name: name.to_string(),
            relative_path: relative_path.to_string(),
            full_path: full_path.display().to_string(),
            availability: "missing".to_string(),
            version: None,
            error: Some("Bundled tool file is missing.".to_string()),
        };
    }

    let mut command = background_command(full_path);
    match command.args(version_args).output() {
        Ok(output) if output.status.success() => ToolStatus {
            name: name.to_string(),
            relative_path: relative_path.to_string(),
            full_path: full_path.display().to_string(),
            availability: "available".to_string(),
            version: first_line(&output.stdout),
            error: None,
        },
        Ok(output) => ToolStatus {
            name: name.to_string(),
            relative_path: relative_path.to_string(),
            full_path: full_path.display().to_string(),
            availability: "cannot_execute".to_string(),
            version: None,
            error: Some(process_failure_message(
                &format!(
                    "{name} at {} failed to report a version.",
                    full_path.display()
                ),
                output.status.code(),
                &output.stderr,
                &output.stdout,
            )),
        },
        Err(error) => ToolStatus {
            name: name.to_string(),
            relative_path: relative_path.to_string(),
            full_path: full_path.display().to_string(),
            availability: "cannot_execute".to_string(),
            version: None,
            error: Some(format!(
                "Failed to run {name} at {}: {error}",
                full_path.display()
            )),
        },
    }
}

pub fn emit_tool_install_progress(app: &AppHandle, progress: ToolInstallProgress) {
    let _ = app.emit("tool-install-progress", progress);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn resolves_relative_path_correctly() {
        let base = Path::new("/app/dir");
        let rel = Path::new("tools/yt-dlp.exe");
        let abs = Path::new("/other/dir/yt-dlp.exe");

        assert_eq!(resolve_relative_path(rel, base), PathBuf::from("/app/dir/tools/yt-dlp.exe"));
        assert_eq!(resolve_relative_path(abs, base), PathBuf::from("/other/dir/yt-dlp.exe"));
    }

    #[test]
    fn maps_supported_platform_arch_pairs_to_tool_targets() {
        assert_eq!(tool_target_from("windows", "x86_64"), Some("win-x64"));
        assert_eq!(tool_target_from("windows", "aarch64"), Some("win-arm64"));
        assert_eq!(tool_target_from("macos", "x86_64"), Some("macos-x64"));
        assert_eq!(tool_target_from("macos", "aarch64"), Some("macos-arm64"));
        assert_eq!(tool_target_from("linux", "x86_64"), None);
    }

    #[test]
    fn uses_platform_specific_tool_names() {
        let windows_tools = tool_names_for_target("win-x64").expect("windows tool names");
        assert_eq!(windows_tools.yt_dlp, "yt-dlp.exe");
        assert_eq!(windows_tools.ffmpeg, "ffmpeg.exe");
        assert_eq!(windows_tools.ffprobe, "ffprobe.exe");
        assert_eq!(windows_tools.deno, "deno.exe");

        let macos_tools = tool_names_for_target("macos-arm64").expect("macos tool names");
        assert_eq!(macos_tools.yt_dlp, "yt-dlp");
        assert_eq!(macos_tools.ffmpeg, "ffmpeg");
        assert_eq!(macos_tools.ffprobe, "ffprobe");
        assert_eq!(macos_tools.deno, "deno");

        assert!(tool_names_for_target("linux-x64").is_none());
    }

    #[test]
    fn selects_tools_for_current_manifest_target() {
        let manifest = r#"
        {
          "schemaVersion": 2,
          "targets": [
            {
              "target": "win-x64",
              "tools": [
                {
                  "name": "yt-dlp",
                  "path": "Tools/win-x64/yt-dlp/yt-dlp.exe",
                  "sourceUrl": "https://example.test/yt-dlp.exe",
                  "sha256": "abc",
                  "kind": "file"
                }
              ]
            },
            {
              "target": "win-arm64",
              "tools": []
            }
          ]
        }
        "#;

        let target = manifest_target_from_json(manifest, "win-x64").expect("target should parse");
        assert_eq!(target.target, "win-x64");
        assert_eq!(target.tools.len(), 1);
        assert_eq!(target.tools[0].name, "yt-dlp");
    }

    #[test]
    fn production_manifest_uses_fixed_release_urls() {
        let manifest = include_str!("../../tools-manifest.json");
        let manifest: ToolsManifest =
            serde_json::from_str(manifest).expect("manifest should parse");

        for target in manifest.targets {
            for tool in target.tools {
                let source_url = tool.source_url.to_ascii_lowercase();
                assert!(
                    !source_url.contains("/latest/") && !source_url.contains("/latest/download/"),
                    "{} for {} uses a floating latest URL: {}",
                    tool.name,
                    target.target,
                    tool.source_url
                );
                assert!(
                    !source_url.contains("master-latest"),
                    "{} for {} uses a floating latest asset name: {}",
                    tool.name,
                    target.target,
                    tool.source_url
                );
            }
        }
    }

    #[test]
    fn production_manifest_contains_expected_tool_targets() {
        let manifest = include_str!("../../tools-manifest.json");
        let manifest: ToolsManifest =
            serde_json::from_str(manifest).expect("manifest should parse");
        let targets: BTreeMap<_, _> = manifest
            .targets
            .iter()
            .map(|target| (target.target.as_str(), target))
            .collect();
        let expected_tools = BTreeSet::from(["deno", "ffmpeg", "ffprobe", "yt-dlp"]);

        for target_name in ["win-x64", "macos-x64", "macos-arm64"] {
            let target = targets
                .get(target_name)
                .unwrap_or_else(|| panic!("missing manifest target {target_name}"));
            let tools = target
                .tools
                .iter()
                .map(|tool| tool.name.as_str())
                .collect::<BTreeSet<_>>();
            assert_eq!(tools, expected_tools, "unexpected tools for {target_name}");

            for tool in &target.tools {
                assert!(
                    tool.path
                        .starts_with(&format!("{TOOLS_DIRECTORY}/{target_name}/")),
                    "{} for {} has path outside its target: {}",
                    tool.name,
                    target_name,
                    tool.path
                );
            }
        }
    }
}
