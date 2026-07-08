use super::paths::state_directory;
use crate::error::{AppError, Result};
use std::fs;
use std::path::PathBuf;

pub fn validate_http_url(url: &str) -> Result<()> {
    let trimmed = url.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        Ok(())
    } else {
        Err(AppError::Custom("Enter a valid http or https video URL.".to_string()))
    }
}

pub fn http_url_host(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("://")?;
    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('@')
        .next()
        .unwrap_or_default();
    if authority.starts_with('[') {
        return authority
            .split_once(']')
            .map(|(host, _)| host.trim_start_matches('[').to_ascii_lowercase());
    }

    authority
        .split(':')
        .next()
        .filter(|host| !host.trim().is_empty())
        .map(|host| host.trim().to_ascii_lowercase())
}

pub fn proxy_url_state_path() -> Result<PathBuf> {
    Ok(state_directory()?.join("proxy-url.txt"))
}

pub fn proxy_url() -> Result<Option<String>> {
    let configured = proxy_url_state_path()?;
    if configured.exists() {
        let value = fs::read_to_string(configured)?;
        let value = value.trim();
        if !value.is_empty() {
            return Ok(Some(value.to_string()));
        }
    }

    Ok(None)
}

pub fn yt_dlp_proxy_args(proxy_url: Option<&str>) -> Vec<String> {
    proxy_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| vec!["--proxy".to_string(), value.to_string()])
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omits_proxy_args_without_configured_proxy() {
        assert!(yt_dlp_proxy_args(None).is_empty());
        assert!(yt_dlp_proxy_args(Some("  ")).is_empty());
    }

    #[test]
    fn passes_configured_proxy_to_yt_dlp() {
        let args = yt_dlp_proxy_args(Some(" http://127.0.0.1:7890 "));

        assert_eq!(
            args,
            vec!["--proxy".to_string(), "http://127.0.0.1:7890".to_string()]
        );
    }
}
