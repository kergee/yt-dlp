# Tool cache

Large Windows tool binaries are not committed to Git because GitHub rejects files over 100 MB.

The app can install the current target's toolchain automatically from the Toolchain panel. For development or offline packaging, restore `win-x64` from the Tauri project root with:

```powershell
.\scripts\download-tools.ps1
```

The expected targets, layout, source URLs, and hashes are documented in `src-tauri/tools-manifest.json`.
