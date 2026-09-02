# TreeFrog Content Manager

**Global SD card content manager for TreeFrogUI handhelds** — R36SX (v2.6 & v2.7), R36 HD, SF3000, SF3000 HD, SF3100, SF3500 and GB350.

Desktop app (Windows / Linux / macOS) that scans your ROM, music, video, BIOS and LGPT folders, shows exactly what will be copied, and writes it to the correct TreeFrogUI folders on your SD card — with one canonical plan and hardened path safety.

> Works with any [TreeFrogUI](https://github.com/tzubertowski/TreeFrogUI) SD card (TreeFrogUI itself is a separate project by tzubertowski).

## Downloads

Get the latest release for your platform from the
[**Releases**](https://github.com/ozkaoz/treefrog-content-manager/releases/latest) page:

| Platform | File | Notes |
|----------|------|-------|
| **Windows x64** | `TreeFrog-Content-Manager-*-Windows-x64.exe` | Portable — just run it, no install |
| **Windows x64** | `TreeFrog-Content-Manager-*-Windows-x64-Setup.exe` | NSIS installer |
| **Linux x64** | `TreeFrog-Content-Manager-*-Linux-x64.AppImage` | AppImage |
| **macOS x64** | `TreeFrog-Content-Manager-*-macOS-x64.dmg` | Disk image |

> **Windows SmartScreen note:** the executable is not code-signed, so the first run may show *"Windows protected your PC"*. Click **More info → Run anyway** — the app is safe. This warning fades as the release builds download reputation; code-signing can be added later (issue pending a certificate).

## What it does

Each tab maps your content to the folder TreeFrogUI expects:

| Tab | Destination on SD | Notes |
|-----|-------------------|-------|
| **Games** | `roms/<SYSTEM>/` | 75 systems, profile-driven (GBA, FC, PS, MD, …). Archive (ZIP) inspection, arcade `cps1/neogeo/m2k` kept as payload, CUE/BIN groups preserved |
| **Music** | `roms/music/` | Each subfolder = a playlist (TreeFrogUI music player semantics), hierarchy preserved |
| **Videos** | `roms/videos/` | ffprobe inspection; incompatible videos are converted with FFmpeg (staged + validated) — original never modified |
| **BIOS** | `cubegm/bios/` | Guided selection with filename / size / SHA-256 validation; stock BIOS on the SD are never silently overwritten |
| **LGPT** | `lgpt/samples/` + `lgpt/projects/` | Little Piggy Tracker samples and projects |

### The flow

```
Overview (SD detection)
   → Games → Music → Videos → BIOS → LGPT
   → SD Card (danger zone: review + delete) → Sync
```
- **One canonical plan**: what you preview is exactly what gets written. No re-scan, no drift between preview and deployment.
- **Duplicates by SHA-256**, not by name. Same content = skipped; same name + different content = conflict you resolve (`skip / replace / keep_both / keep_destination / keep_source`). `keep_both` renames collision-safely (`_1`, `_2`, …) against both the SD and the rest of the plan.
- **Space check** from the effective (resolved) actions — a conflict you resolve to `replace` counts, a duplicate you keep counts, nothing is double-counted.
- **Sync to SD** is available from every tab; after every sync the real SD state (TreeFrogUI detected, free space, per-type counts) refreshes in all tabs.

### Safety

- Canonical destination validation everywhere: `../` traversal, absolute paths, UNC paths, drive letters, ADS colons, reserved Windows names and illegal characters are rejected — nothing can escape the SD root.
- Archives: ZIP only, with entry limits, expansion limits, compression-ratio limits, symlink rejection and traversal checks. 7z/RAR are explicitly unsupported (reported, never extracted).
- Every file write goes through one safe writer (staged temp file + verified size + atomic rename).
- `UNKNOWN` content is never written; `UNSUPPORTED` content is never silently copied.

### Under the hood

- Tauri 2 + Rust backend, React + TypeScript frontend, SQLite deployment history (job/entry/fingerprint with hashes, versions and target identity), declarative JSON profiles (`profiles/treefrogui/`) as the single source of truth, stable SD identity via Windows volume GUID + serial.

## Requirements

- Windows 10/11 (WebView2 included), Linux (WebKit2GTK 4.1), or macOS 10.15+
- A TreeFrogUI SD card (stock OS + TreeFrogUI)
- **Optional**: `ffmpeg`/`ffprobe` in PATH for video compatibility inspection and conversion (not bundled — get them from [ffmpeg.org](https://ffmpeg.org))

## Build from source

```bash
cd treefrog-manager
npm install
npx tauri build   # or: npm run dev for development
```

Rust toolchain required (`cargo`). Frontend checks: `npx tsc --noEmit`. Backend checks: `cargo check` / `cargo test` (47 tests incl. path-escape, BIOS security and portable-embed fixtures). Python mirror tests: `python -m pytest tests` (224 tests).

Every shipped executable passes `--self-check` (profile, systems, ffmpeg detection **and the embedded BIOS catalog** — the BIOS section can never be empty in a portable binary).

CI runs the full matrix on every push (`.github/workflows/validate.yml`): frontend typecheck+build, Rust fmt/check/test, pytest with FFmpeg (including real conversion-deploy tests), version consistency, and the Tauri packaging build.

## Project layout

```
treefrog-manager/
├── src/                 React frontend (panels: Games, Music, Videos, BIOS, LGPT, SD Card)
├── src-tauri/src/       Rust backend (scanner, classify, planner, deploy, paths, video, archive, bios, db)
├── python/treefrog/     Python mirror of the planning logic (test parity)
└── tests/               224 pytest tests
profiles/treefrogui/     Declarative TreeFrogUI profiles (systems, bios, media, archive policy, video presets)
```

## Credits & licenses

- TreeFrog Content Manager (this repo): GPL-3.0-or-later
- [TreeFrogUI](https://github.com/tzubertowski/TreeFrogUI) by tzubertowski (CC BY-NC-SA 4.0) — the frontend this manager deploys content for
- Frog artwork © TreeFrogUI project (CC BY-NC-SA 4.0)

> TreeFrog Content Manager is an independent companion tool. It does not bundle or distribute TreeFrogUI, firmware, BIOS files or copyrighted ROMs — BIOS files are user-supplied and validated locally.
