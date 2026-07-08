use crate::error::{AppError, Result};
use crate::state::{DownloadProgress, VideoFormatOption, VideoMetadata};
use serde_json::Value;
use std::collections::BTreeSet;
use tauri::{AppHandle, Emitter};

pub const PROGRESS_PREFIX: &str = "yt-dlp-tauri-progress:";
pub const OUTPUT_PATH_PREFIX: &str = "yt-dlp-tauri-output:";
pub const BEST_MP4_FORMAT: &str = "bv*[vcodec^=avc][ext=mp4]+ba[ext=m4a]/bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]/bv*+ba/b";

pub fn parse_metadata_json(json: &str, fallback_url: &str) -> Result<VideoMetadata> {
    if json.trim().is_empty() {
        return Err(AppError::Custom("yt-dlp returned empty metadata.".to_string()));
    }

    let root: Value = serde_json::from_str(json)?;
    let title = root
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Untitled video")
        .to_string();
    let id = root.get("id").and_then(Value::as_str).map(str::to_string);
    let webpage_url = root
        .get("webpage_url")
        .or_else(|| root.get("original_url"))
        .and_then(Value::as_str)
        .unwrap_or(fallback_url)
        .to_string();
    let thumbnail_urls = read_thumbnail_urls(&root);
    let thumbnail_url = thumbnail_urls.first().cloned();
    let duration_seconds = root.get("duration").and_then(Value::as_f64);
    let description = root
        .get("description")
        .and_then(Value::as_str)
        .map(str::to_string);
    let format_options = build_format_options(&root);

    Ok(VideoMetadata {
        title,
        id,
        webpage_url,
        thumbnail_url,
        thumbnail_urls,
        duration_seconds,
        description,
        format_options,
    })
}

fn build_format_options(root: &Value) -> Vec<VideoFormatOption> {
    let mut options = vec![VideoFormatOption {
        label: "Best MP4".to_string(),
        format_selector: BEST_MP4_FORMAT.to_string(),
        height: None,
        extension: "mp4".to_string(),
        is_best: true,
    }];

    for height in read_available_heights(root).into_iter().rev() {
        options.push(VideoFormatOption {
            label: format!("{height}p MP4"),
            format_selector: format!(
                "bv*[height<={height}][vcodec^=avc][ext=mp4]+ba[ext=m4a]/bv*[height<={height}][ext=mp4]+ba[ext=m4a]/b[height<={height}][ext=mp4]/bv*[height<={height}]+ba/b[height<={height}]"
            ),
            height: Some(height),
            extension: "mp4".to_string(),
            is_best: false,
        });
    }

    options
}

fn read_available_heights(root: &Value) -> Vec<u32> {
    let mut heights = BTreeSet::new();
    if let Some(formats) = root.get("formats").and_then(Value::as_array) {
        for format in formats {
            let height = format.get("height").and_then(Value::as_u64);
            let video_codec = format.get("vcodec").and_then(Value::as_str);
            if let Some(height) = height {
                if height > 0 && video_codec.map(|codec| codec != "none").unwrap_or(true) {
                    heights.insert(height as u32);
                }
            }
        }
    }
    heights.into_iter().collect()
}

fn read_thumbnail_urls(root: &Value) -> Vec<String> {
    let mut urls = Vec::new();
    let mut seen = BTreeSet::new();

    if let Some(url) = root.get("thumbnail").and_then(Value::as_str) {
        push_thumbnail_url(&mut urls, &mut seen, url);
    }

    if let Some(items) = root.get("thumbnails").and_then(Value::as_array) {
        for url in items
            .iter()
            .rev()
            .filter_map(|item| item.get("url").and_then(Value::as_str))
        {
            push_thumbnail_url(&mut urls, &mut seen, url);
        }
    }

    urls
}

fn push_thumbnail_url(urls: &mut Vec<String>, seen: &mut BTreeSet<String>, raw_url: &str) {
    let trimmed = raw_url.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("null")
        || trimmed.eq_ignore_ascii_case("none")
    {
        return;
    }

    let normalized = if trimmed.starts_with("//") {
        format!("https:{trimmed}")
    } else {
        trimmed.to_string()
    };

    if let Some(rest) = normalized.strip_prefix("http://") {
        push_unique_thumbnail_url(urls, seen, format!("https://{rest}"));
    }
    push_unique_thumbnail_url(urls, seen, normalized);
}

fn push_unique_thumbnail_url(urls: &mut Vec<String>, seen: &mut BTreeSet<String>, url: String) {
    if seen.insert(url.clone()) {
        urls.push(url);
    }
}

pub fn parse_progress_line(line: &str) -> Option<DownloadProgress> {
    let payload = line.strip_prefix(PROGRESS_PREFIX)?;
    let parts = payload.split('|').collect::<Vec<_>>();
    Some(DownloadProgress {
        status: normalize_status(parts.first().copied().unwrap_or_default()),
        percent: parse_percent(parts.get(1).copied().unwrap_or_default()),
        speed: normalize_optional(parts.get(2).copied()),
        eta: normalize_optional(parts.get(3).copied()),
        raw: Some(line.to_string()),
    })
}

fn parse_percent(value: &str) -> Option<f64> {
    let number = value
        .chars()
        .filter(|character| character.is_ascii_digit() || *character == '.')
        .collect::<String>();
    number
        .parse::<f64>()
        .ok()
        .map(|percent| percent.clamp(0.0, 100.0))
}

fn normalize_status(value: &str) -> String {
    match value.trim() {
        "downloading" => "Downloading".to_string(),
        "finished" => "Merging".to_string(),
        "error" => "Failed".to_string(),
        "" => "Processing".to_string(),
        other => other.to_string(),
    }
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() || value == "N/A" {
        None
    } else {
        Some(value.to_string())
    }
}

pub fn emit_progress(app: &AppHandle, progress: DownloadProgress) {
    let _ = app.emit("download-progress", progress);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_metadata_upgrades_http_thumbnail_candidates() {
        let metadata = parse_metadata_json(
            r#"
            {
              "title": "Bilibili test",
              "webpage_url": "https://www.bilibili.com/video/BV1test",
              "thumbnail": "http://i0.hdslb.com/bfs/archive/cover.jpg",
              "formats": []
            }
            "#,
            "https://www.bilibili.com/video/BV1test",
        )
        .expect("metadata should parse");

        assert_eq!(
            metadata.thumbnail_url.as_deref(),
            Some("https://i0.hdslb.com/bfs/archive/cover.jpg")
        );
        assert_eq!(
            metadata.thumbnail_urls,
            vec![
                "https://i0.hdslb.com/bfs/archive/cover.jpg".to_string(),
                "http://i0.hdslb.com/bfs/archive/cover.jpg".to_string()
            ]
        );
    }
}
