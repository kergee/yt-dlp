<h1 align="center">yt-dlp-tauri</h1>

<p align="center">
  <strong>A modern, premium desktop video downloader powered by yt-dlp, Tauri 2, and Rust.</strong>
</p>

<p align="center">
  <a href="./README_zh.md">中文说明</a> ·
  <a href="#quick-start">Quick Start</a> ·
  <a href="#key-enhancements">Key Enhancements</a> ·
  <a href="#configuration">Configuration</a> ·
  <a href="#verification">Verification</a> ·
  <a href="#documentation">Documentation</a>
</p>

<p align="center">
  <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri" />
  <img alt="Rust" src="https://img.shields.io/badge/Rust-backend-B7410E?logo=rust" />
  <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-typed-3178C6?logo=typescript" />
  <img alt="Vite" src="https://img.shields.io/badge/Vite-build-646CFF?logo=vite" />
  <img alt="Windows" src="https://img.shields.io/badge/Windows-desktop-0078D4?logo=windows" />
</p>

<p align="center">
  <img alt="yt-dlp-tauri English interface" src="./docs/assets/readme-en.png" width="920" />
</p>

> [!NOTE]
> This project is derived from the open-source repository [Chlience/yt-dlp-tauri](https://github.com/Chlience/yt-dlp-tauri).

---

## What is yt-dlp-tauri?

`yt-dlp-tauri` is a lightweight, local-first desktop application designed for downloading videos without the hassle of typing command-line arguments. Simply paste a video URL, preview the rich metadata (title, description, duration, and high-quality thumbnails), select your desired format, and download high-quality files through a clean, aesthetic interface.

The project is designed to be self-managed, securing a local-only toolchain and offering deep customizability.

---

## Key Enhancements

We have recently implemented major architectural, performance, and UI/UX upgrades to elevate the application:

### 🎨 Premium UI & Interactive Experience
* **Native Dark Mode**: Full dark theme support adapting automatically to your system preference (`prefers-color-scheme: dark`) with a custom OKLCH color space.
* **Micro-Animations**: Butter-smooth sliding transitions for the Settings drawer, active button click scaling, Notice color fades, and fluid progress loading bars.
* **Responsive Window Resizing**: Minimum window size constraint reduced to `820`x`600` with full maximize/resize capabilities, allowing the responsive layouts to fit any display.
* **Thumbnail Fade-In**: Smooth opacity fade-in effects for video thumbnail candidates when loading and swapping links.

### ⚡ Robust Rust Backend Architecture
* **Modular Codebase**: Split a 2,330+ lines monolithic backend into clean, single-responsibility modules (`commands/`, `error.rs`, `state.rs`, `zip_utils.rs`, `utils.rs`).
* **Structured Error Handling**: Implemented a custom `AppError` type powered by `thiserror`, providing transparent Rust error context while maintaining serialization compatibility with the frontend.
* **Tool Location Cache**: Introduced a thread-safe `CachedToolPaths` managed state to cache path discovery. Disk scans are now avoided on every metadata parse and download request, with automatic cache invalidation on settings changes.
* **Native ZIP Extraction**: Replaced external Windows PowerShell execution with a native `zip` library, enforcing file execution permissions natively and removing command-line injection surface area.

---

## Tech Stack

| Layer | Technology |
| --- | --- |
| **Desktop Runtime** | Tauri 2 |
| **Backend** | Rust |
| **Frontend** | TypeScript, Vanilla JS, CSS3, Vite |
| **Toolchain** | App-managed `yt-dlp`, `ffmpeg`, `ffprobe`, `deno` |
| **Installer** | Windows NSIS installer |

---

## Quick Start

### 1. Install Prerequisites
* Windows 10/11 with WebView2 Runtime.
* Node.js 20+ or 22+.
* Rust stable toolchain.

### 2. Set Up and Run

Install dependencies:
```powershell
npm ci
```

*(Optional)* Restore development tools:
```powershell
.\scripts\download-tools.ps1
```
*Note: If tools are missing during run, you can simply click "Install Tools" in the app's Settings drawer.*

Run the desktop app in development:
```powershell
npm run tauri dev
```

Build the Windows NSIS installer locally:
```powershell
npm run tauri build
```
The build installer will be written to:
`src-tauri\target\release\bundle\nsis\`

---

## Configuration

| Path / Target | Purpose |
| --- | --- |
| `src-tauri/tools-manifest.json` | Pinned versions, download URLs, and SHA-256 hashes for all platform binaries. |
| `src-tauri/tauri.conf.json` | Tauri configuration containing window specifications, CSP rules, and compiler targets. |
| **Settings: Output Folder** | Saves download destinations, reset actions, and shortcuts. |
| **Settings: GitHub Site** | Configures update checking to go `Direct` or route through a `gh-proxy` cache. |

---

## Data, Storage, and Output

* **Default Downloads**: `%USERPROFILE%\Downloads\yt-dlp-tauri\`
* **App State**: `%LOCALAPPDATA%\yt-dlp-tauri\state\`
* **Operational Logs**: `%LOCALAPPDATA%\yt-dlp-tauri\logs\app.log`
* **Installed Toolchain**: `%LOCALAPPDATA%\yt-dlp-tauri\Tools\win-x64\`

---

## Verification

### Run Frontend Tests
```powershell
npm test
```

### Run Rust Backend Tests
```powershell
cargo test --manifest-path .\src-tauri\Cargo.toml --lib
```

### Compile-Check Rust Backend
```powershell
cargo check --manifest-path .\src-tauri\Cargo.toml
```

---

## Documentation

* [Changelog](./CHANGELOG.md)
* [Contributing Guidelines](./CONTRIBUTING.md)
* [Security Policy](./SECURITY.md)
* [Third-Party Notices](./THIRD-PARTY-NOTICES.md)

---

## Star History

<a href="https://www.star-history.com/?repos=Chlience%2Fyt-dlp-tauri&type=date&legend=top-left">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=Chlience/yt-dlp-tauri&type=date&theme=dark&legend=top-left" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=Chlience/yt-dlp-tauri&type=date&legend=top-left" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=Chlience/yt-dlp-tauri&type=date&legend=top-left" />
 </picture>
</a>

---

## Release Checklist (Windows)

Before drafting a release:
1. Run all frontend and backend verification tests.
2. Push a version tag matching `v*` (e.g., `v0.1.5`).
3. The GitHub Actions release workflow will automatically compile the Windows NSIS setup package and upload it as a draft release.
4. Ensure `src-tauri/tools-manifest.json` refers to fixed URL sources instead of floating `latest` versions.

---

## Legal

This project is licensed under the GPL-3.0 License. The application fetches and runs third-party command-line binaries, each adhering to their respective licenses. See [THIRD-PARTY-NOTICES.md](./THIRD-PARTY-NOTICES.md) for details.

This project is not affiliated with, authorized, or maintained by `yt-dlp`, FFmpeg, Deno, or Tauri.
