#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod error;
pub mod state;
pub mod utils;
pub mod zip_utils;
pub mod commands;

use state::{CachedToolPaths, DownloadProcessState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(DownloadProcessState::default())
        .manage(CachedToolPaths::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::set_tools_directory,
            commands::open_tools_directory,
            commands::set_download_directory,
            commands::reset_download_directory,
            commands::set_cookies_file,
            commands::clear_cookies_file,
            commands::open_download_directory,
            commands::check_tools,
            commands::parse_metadata,
            commands::download_video,
            commands::cancel_download,
            commands::open_login_window,
            commands::sync_webview_cookies
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
