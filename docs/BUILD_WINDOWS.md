# Windows x64 Build — TreeFrog Content Manager

**Target:** Windows x64 (first supported desktop, per `docs/PLAN.md`)

**Stack:** Tauri 2 + Rust stable + Node 18+/20 + Vite + WebView2 + MSVC

**Artifacts:** `treefrog-manager/src-tauri/target/release/treefrog-manager.exe` (12 MB) + `bundle/msi/*.msi` + `bundle/nsis/*-setup.exe`

**Prerequisites (Windows):**

- **Rust** stable 1.98+ — `https://rustup.rs` → `cargo --version`, `rustc --version`
- **Node.js** 20.19.0 (or 18+) + `npm` 10.8+ — `https://nodejs.org` → `node --version`, `npm --version`
- **Tauri CLI** `2.11+` — installed as devDependency `npm install @tauri-apps/cli` → `npx tauri --version`
- **MSVC Build Tools 2026** (VS Community 18) with `MSVC v14.51` + `Windows 10/11 SDK` — `link.exe`, `cl.exe` via `vcvarsall.bat x64`
- **WebView2** 151+ (preinstalled on Windows 10/11) — `npx tauri info` checks
- **FFmpeg/ffprobe** 7+ (for video pipeline runtime, not required for build) — `https://ffmpeg.org` or `winget install Gyan.FFmpeg` → `ffmpeg -version`, `ffprobe -version`
- **WiX Toolset** and **NSIS** are auto-downloaded by Tauri during first bundle (requires internet)

**Exact reproducible build (Windows PowerShell, from repo root):**

```powershell
# 1. Ensure prerequisites (see above) and add to PATH if needed:
#    $env:PATH = "C:\Users\<you>\.cargo\bin;C:\path\to\node-portable\node-v20.19.0-win-x64;$env:PATH"

# 2. Install frontend deps (once):
cd treefrog-manager
npm install

# 3. Build frontend + Tauri Windows x64 (release, bundle):
# Option A — via script:
powershell -ExecutionPolicy Bypass -File ..\scripts\build_windows.ps1

# Option B — manual:
npm run build
npx tauri build
# or with MSVC env explicitly:
cmd /c "call `"C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvarsall.bat`" x64 >nul && npx tauri build"

# 4. Artifacts:
#   treefrog-manager/src-tauri/target/release/treefrog-manager.exe
#   treefrog-manager/src-tauri/target/release/bundle/msi/TreeFrog Content Manager_0.1.0_x64_en-US.msi
#   treefrog-manager/src-tauri/target/release/bundle/nsis/TreeFrog Content Manager_0.1.0_x64-setup.exe
```

**WSL note:** `scripts/build_windows.sh` documents that `npx tauri build` for Windows x64 **must** run on Windows native (MSVC + WebView2). WSL cross-compilation is not reliable for the full Tauri app; WSL can only do `cargo check` and `npm run build` for smoke, not the final `.exe`. Use the PowerShell command above.

**Smoke test (no SD required):**

```powershell
# Self-check (profile, video preset, ffmpeg/ffprobe availability):
& "treefrog-manager/src-tauri/target/release/treefrog-manager.exe" --self-check
# Expected: profile loaded: 1.1.0, systems: 75, video preset status: PROVISIONAL_UNVALIDATED, ffprobe/ffmpeg available: true/false, self-check PASS

# Deterministic dataset (used in CI):
#   src/game.gba (normal ROM) -> roms/GBA/game.gba copy
#   src/pack.zip (a.gba + b.sfc) -> roms/GBA/a.gba + roms/SFC/b.sfc extract
#   src/dup.gba (duplicate of game.gba) -> skip_duplicate
#   src/good.mp4 (h264 640x480 yuv420p 30fps aac 48000) -> roms/videos/good.mp4 copy (compatible)
#   src/bad.mkv (hevc 1920x1080 60fps) -> roms/videos/bad.converted.mp4 convert_then_copy (provisional)
# Run via Python planner (zero SD writes):
#   python treefrog-manager/tests/test_phase2c_video_conversion.py  (or smoke_test_dataset.py)
# All conversions in temp workspace, original never modified, validated with ffprobe, deterministic naming.

# GUI smoke (manual, where practical):
#   Launch: & "treefrog-manager/src-tauri/target/release/treefrog-manager.exe"
#   Verify: window appears (TreeFrog Content Manager), profile loads (75 systems), source picker works, Scan + Preview generates dry-run with video conversion row (bad.mkv -> conversion_required -> roms/videos/bad.converted.mp4, provisional), no SD writes.
```

**Clean:**

```powershell
cd treefrog-manager
cargo clean
Remove-Item -Recurse -Force dist, node_modules, src-tauri/target
```

**Not committed:** `dist/`, `target/`, `node_modules/`, `*.msi`, `*.exe` (in `target/`) are in `.gitignore`.

**Desktop UX verification (Phase 2E):**

- Native dialogs: `src/services/dialog.ts` (`pickFolder`/`pickFile`) via `@tauri-apps/plugin-dialog` → native Explorer picker, not `window.prompt`. Manual QA steps in `docs/MANUAL_QA_2E.md` verify Browse → native picker in packaged app. Capabilities `dialog:allow-open/save/message` in `src-tauri/capabilities/default.json`.
- Windows theme: App follows `prefers-color-scheme` → Light (`--bg:#fff`) / Dark (`--bg:#0f172a`) via `src/styles.css` tokens + `src/services/theme.ts` `watchSystemTheme`. Switch OS dark while running → app updates dynamically. Verified via manual steps 8-9 in `MANUAL_QA_2E.md`.
- Branding (Phase 2E.1 corrected): Frog-only pixel-art from `logo.png` 1536×1024 high-res desktop upright (was `xgame-logo.bmp` 480×854 vertical boot asset stored inverted for handheld rotated display → appeared upside-down and low-res 87×99 → solid green at 32×32). Now `logo.png` left part 314×280 (no flip) → `frog-only.png` 314×280 + `frog-square.png` 314×314 via `scripts/generate_branding.py` (NEAREST, `r<20` transparent, x-gap 517–549). Icons `src-tauri/icons/32x32.png` 1686B, `64x64.png`, `128x128.png`, `256x256.png`, `512x512.png`, `icon.ico` 103442B (6 sizes 16/32/48/64/128/256, was 641B placeholder), `icon.icns` 927k. Header `[frog 32×32 upright, not mirrored/stretched/blurred] TreeFrog Content Manager`; same transparent frog works in Light/Dark. See `src/assets/branding/README.md` for root cause & pipeline.
- Navigation: `Overview | Games | Music | Videos | BIOS | LGPT | SD Card | Settings | About` — `Games/Music/Videos/Settings` are placeholders ("Coming in a future release"), no fake functionality; BIOS/LGPT/Overview remain functional.
- Installer identity: `TreeFrog Content Manager` with frog icon; `TreeFrog-Content-Manager-<version>-Windows-x64-Setup.exe` on Desktop (existing workflow) with `.sha256`. **Windows icon cache:** after fixing ICO, clean validation requires uninstall → remove residual shortcuts → rebuild → fresh install (see `MANUAL_QA_2E.md` § Windows icon cache QA); do not require end user to rebuild cache.

**Verified on:** Windows 10.0.26200 x64, Rust 1.98.0, Node 20.19.0, Tauri CLI 2.11.4, WebView2 151, MSVC 14.51, FFmpeg 7 (WinGet). Phase 2E icons + dialogs + theme verified 2026-08-29; Phase 2E.1 corrected orientation & ICO multi-size verified 2026-08-29 (fresh install Desktop/StartMenu/taskbar/window/header all frog, Light/Dark, Browse/BIOS/LGPT).

**Reproducibility:** `Cargo.lock` and `package-lock.json` are committed; `npm install` and `cargo build` fetch exact versions.
