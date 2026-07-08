# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`yt-dlp-tauri` is a Windows-first desktop video downloader: a Tauri 2 (Rust) shell around `yt-dlp`, with a vanilla TypeScript/Vite frontend. It manages its own local toolchain (`yt-dlp`, `ffmpeg`, `ffprobe`, `deno`) rather than requiring the user to install them. It is derived from `Chlience/yt-dlp-tauri`.

## Commands

Install dependencies:
```powershell
npm ci
```

Run the desktop app in dev mode (starts Vite + Tauri):
```powershell
npm run tauri dev
```

Build the Windows NSIS installer (output in `src-tauri\target\release\bundle\nsis\`):
```powershell
npm run tauri build
```

Frontend tests (Node's built-in test runner over `tests/**/*.test.ts`):
```powershell
npm test
```
Run a single frontend test file:
```powershell
node --test --experimental-strip-types tests/thumbnail.test.ts
```

Frontend build/typecheck only:
```powershell
npm run build
```

Rust backend tests:
```powershell
cargo test --manifest-path .\src-tauri\Cargo.toml --lib
```
Run a single Rust test:
```powershell
cargo test --manifest-path .\src-tauri\Cargo.toml --lib <test_name>
```

Rust compile check (fast, no test run):
```powershell
cargo check --manifest-path .\src-tauri\Cargo.toml
```

Restore the bundled tool binaries (yt-dlp/ffmpeg/ffprobe/deno) for local dev, matching CI:
```powershell
.\scripts\download-tools.ps1
```
There is no in-app "Install Tools" button despite what the README/CHANGELOG say — `utils::tools::install_manifest_target` implements the download/verify/extract flow but is not wired to any `#[tauri::command]` or UI control. Tools must currently be restored via the script above or placed manually.

CI (`.github/workflows/ci.yml`) runs, in order: `npm test` → `npm run build` → `cargo test --lib` → `cargo check`. Match this before considering a change done.

## Architecture

### Two halves, one IPC boundary

- **Frontend** (`src/main.ts`, `src/thumbnail.ts`, `src/styles.css`, loaded via `index.html`): single-file vanilla TS app (no framework) that renders the whole UI, handles i18n (en/zh) inline, and talks to Rust exclusively through `@tauri-apps/api`'s `invoke()` and `listen()`.
- **Backend** (`src-tauri/src/`): Rust/Tauri command handlers. All frontend-callable operations are registered as `#[tauri::command]` functions and wired into `invoke_handler!` in `src-tauri/src/lib.rs` — that list is the authoritative surface of what the frontend can call.

When adding a new frontend↔backend capability, add the `#[tauri::command]` fn in the appropriate `commands/*.rs` module, re-export it via `commands/mod.rs`, and register it in the `tauri::generate_handler!` list in `lib.rs`.

### Backend module layout (`src-tauri/src/`)

- `commands/config.rs` — app state getters/setters: tools directory, download directory, cookies file path. Persists small state files under the app's state directory (not Rust globals).
- `commands/tools.rs` — `check_tools`: probes yt-dlp/ffmpeg/ffprobe/deno and reports version/availability.
- `commands/video.rs` — the core flow: `parse_metadata` (runs `yt-dlp --dump-single-json`) and `download_video` (spawns `yt-dlp` as a child process, streams stdout for `--progress-template`/`--print` markers back to the frontend via Tauri events, tracks the active PID for `cancel_download`).
- `commands/login.rs` — generic webview-based login flow: `open_login_window(app, target_url)` opens a `WebviewWindowBuilder` window at the exact URL passed in (Bilibili keeps a hardcoded dedicated login page as a special case), and on window close extracts cookies from the shared webview cookie store via `cookies_for_url` and writes them out as a Netscape-format `cookies.txt` for yt-dlp to consume. Cookie domain is read per-cookie from `cookie.domain()`, not hardcoded. The window is a singleton reused across calls; `LoginState` (managed state) tracks which site it currently points at so a reused window's close handler syncs cookies for the right site, and `window.navigate()` re-targets it. There's no in-window address bar (Tauri's multi-webview/`add_child` API needed for that requires the `unstable` crate feature, deliberately not enabled) — to open a specific login page, paste the exact URL into the main URL field before clicking "Web Login".
- `utils/` — everything else, split by concern (each submodule is re-exported through `utils/mod.rs`, so existing `crate::utils::X` imports don't care which submodule `X` lives in):
  - `utils/tools.rs` — tool discovery/installation: `locate_tools`, `install_manifest_target`, manifest parsing, SHA-256 verification, zip extraction dispatch, `probe_tool`.
  - `utils/cookies.rs` — cookies.txt handling, including converting a raw `Cookie:` header into Netscape format.
  - `utils/net.rs` — `validate_http_url`, `http_url_host`, and the proxy setting (`proxy_url`, `yt_dlp_proxy_args` — appends `--proxy` to yt-dlp when configured).
  - `utils/paths.rs` — app directory resolution (download dir, state dir, log dir) and `append_log`.
  - `utils/process.rs` — process management (`background_command` uses `CREATE_NO_WINDOW` on Windows, `kill_process_tree`, `process_failure_message`) and `cleanup_incomplete_downloads` (best-effort removal of yt-dlp's `.part`/`.ytdl`/`.part-Frag*` artifacts, only called after a user-initiated cancel, not on generic failure, so automatic resume-on-retry still works).
  - `utils/parsing.rs` — yt-dlp JSON metadata parsing and `--progress-template` line parsing.
  - `utils/app_state.rs` — `build_app_state`, assembling the `AppState` DTO from the other submodules.
  - Each submodule has its own inline `#[cfg(test)]` block — check there before duplicating logic.
- `zip_utils.rs` — native zip extraction (replaces the previous PowerShell-based approach; avoids shelling out and command-injection surface).
- `state.rs` — all shared `struct`/`enum` types (serialized to the frontend via `serde`): `AppState`, `ToolPaths`, `ToolsManifest`/`ManifestTarget`/`ManifestTool` (mirrors `tools-manifest.json`), `DownloadProgress`, `CachedToolPaths` (a `Mutex`-guarded cache of resolved tool paths, invalidated whenever tool/download settings change), `DownloadProcessState` (tracks the active child PID + cancel flag for `cancel_download`).
- `error.rs` — single `AppError` enum (via `thiserror`) covering IO/JSON/Tauri/URL/Zip/lock/join errors plus a `Custom(String)` catch-all; serializes to a plain string for the frontend. All commands return `crate::error::Result<T>`.

### Tool management model

`src-tauri/tools-manifest.json` is the source of truth for which external binaries (yt-dlp, ffmpeg, ffprobe, deno) get downloaded, from where, and their pinned SHA-256 hashes, per platform target (`win-x64`, `macos-x64`, etc. — Windows NSIS is the only shipped bundle target today). `locate_tools`/`install_manifest_target` in `utils/tools.rs` read this manifest to resolve or install tools into `%LOCALAPPDATA%\yt-dlp-tauri\Tools\win-x64\`. If a tool version changes, update **both** `tools-manifest.json` and `scripts/download-tools.ps1` together, using fixed release URLs (never `latest`) with refreshed hashes — this is enforced by convention, not by CI.

Resolved tool paths are cached in the `CachedToolPaths` managed state to avoid re-scanning disk on every metadata parse/download call; any command that changes tool or download settings must invalidate this cache (see `set_tools_directory` in `commands/config.rs` for the pattern).

### Download/progress flow

`download_video` spawns `yt-dlp` with `--progress-template` and `--print after_move:...` markers prefixed by sentinel strings (`PROGRESS_PREFIX`, `OUTPUT_PATH_PREFIX` in `utils/parsing.rs`). Stdout is read line-by-line on a background thread; lines are parsed (`parse_progress_line`) and re-emitted to the frontend as Tauri events, which `src/main.ts` listens for to update the progress UI. Cancellation works by killing the tracked child PID tree, not by an in-process abort signal. Event names are kebab-case on both sides (`download-progress`, `cookies-synced`, `tool-install-progress`) — keep it that way; a prior refactor let the frontend listener names drift to snake_case and silently broke the progress bar and cookie-sync toast until it was caught in review.

### Data/storage locations (Windows)

- Downloads: `%USERPROFILE%\Downloads\yt-dlp-tauri\` (user-configurable)
- App state (small config files, cookies.txt): `%LOCALAPPDATA%\yt-dlp-tauri\state\`
- Logs: `%LOCALAPPDATA%\yt-dlp-tauri\logs\app.log` (write via `utils::append_log`, not `println!`)
- Installed tools: `%LOCALAPPDATA%\yt-dlp-tauri\Tools\win-x64\`

### Security-relevant constraints

- Tauri CSP (`src-tauri/tauri.conf.json`) whitelists specific image/script/connect origins (YouTube, Bilibili CDNs, `api.github.com`) — extend this deliberately, not by loosening to `*`. Adding a new site's cookie/login support (via `open_login_window`) does not require a CSP change since it opens a separate `WebviewWindow`, not content inside the main window.
- The WeChat-article-URL (`mp.weixin.qq.com`) special-case in `src/main.ts` (`onWeChatVideoIntercepted`, `wechat_video_intercepted` listener) is unfinished: nothing in the Rust backend emits that event, so pasting a WeChat article link just opens a generic login window for that domain. Extracting the real video URL would require reverse-engineering WeChat's private, token-signed `get_mp_video_play_url` endpoint — deliberately not implemented; treat this as a known gap, not a bug to silently "fix" by deleting.
- Tool installation always verifies SHA-256 (`verify_sha256`) against the pinned manifest hash before use.
- Zip extraction uses the native `zip` crate (`zip_utils.rs`), not shelling out to `tar`/`Expand-Archive`/PowerShell — keep it that way to avoid reintroducing a command-injection surface.

## Releasing

Tag pushes matching `v*` trigger `.github/workflows/release.yml`, which builds the Windows NSIS installer via `tauri-apps/tauri-action` and drafts a GitHub release (using `.github/scripts/extract-release-notes.mjs` against `CHANGELOG.md`). Before tagging: ensure `tools-manifest.json` uses fixed (non-`latest`) URLs, and that both test suites pass.

## Tool binaries: do not commit

`src-tauri/Tools/win-x64/`, `src-tauri/Tools/.tmp/`, `src-tauri/target/`, `dist/`, and `node_modules/` are gitignored restored/build artifacts — never add them back.
