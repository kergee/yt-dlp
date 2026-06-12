# macOS GitHub Release Packaging Design

## Goal

Add GitHub Actions release packaging for the existing Tauri app so pushing a version tag creates a draft GitHub Release with Windows and macOS installers attached.

Version tags use the repository's existing convention: `v*`, such as `v0.1.3`.

## Scope

- Add a release workflow triggered by `v*` tags and manual dispatch.
- Build the existing Windows NSIS installer on `windows-latest`.
- Build macOS DMG artifacts for Intel and Apple Silicon on GitHub-hosted macOS runners.
- Keep release publishing as a draft so artifacts can be checked before publication.
- Add enough macOS tool-target support for packaged macOS apps to install and locate their runtime tools.

## Out of Scope

- Apple Developer ID signing and notarization.
- Changing the app's downloader behavior or UI.
- Bundling large tool binaries in Git.
- Linux packaging.

## Architecture

GitHub Actions will use `tauri-apps/tauri-action` with a small OS/target matrix. The workflow grants `contents: write` so the action can create a draft release and upload artifacts. Release metadata uses the pushed tag name as the release tag and release name.

Tauri bundle configuration will include both Windows and macOS bundle targets. Windows keeps the NSIS installer. macOS adds DMG output.

The Rust backend will stop assuming every platform uses `.exe` tools. Tool names and manifest targets will be derived from the current OS and architecture:

- `windows` + `x86_64` -> `win-x64`
- `windows` + `aarch64` -> `win-arm64`
- `macos` + `x86_64` -> `macos-x64`
- `macos` + `aarch64` -> `macos-arm64`

The tool manifest will gain macOS targets with pinned URLs and hashes for `yt-dlp`, `ffmpeg`, `ffprobe`, and `deno`. Runtime installation continues through the existing toolchain panel.

## Error Handling

If a runner cannot resolve a tool target, the app will report a clear unsupported-target error instead of silently falling back to Windows paths.

If a macOS tool cannot be downloaded, extracted, hashed, or executed, the existing install/probe errors will surface the specific tool and command failure.

## Testing

- Add or update Rust unit tests for platform target mapping and platform-specific tool paths.
- Run frontend tests and build.
- Run Rust unit tests and `cargo check`.
- The local environment cannot produce macOS DMG artifacts because macOS packaging requires macOS runners; the GitHub workflow is the integration verification point.
