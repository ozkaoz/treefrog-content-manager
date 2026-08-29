# LGPT for R36SX

Little Piggy Tracker port for the R36SX V2.6 handheld, integrated with TreeFrogUI and bidirectional USB-C OTG audio.

**Current release: Bacon 1.5 — Latest**

Physically validated on R36SX. Single LGPT TreeFrogUI integration, Local / Windows / Android audio, SP404 USB Audio Host, Synths, EQ8 / Analyzer, Pitch / Chopper, Startup project actions via SELECT (Rename / Duplicate / Export / Delete), New: A random / START confirm, Save Song As under projects/
See [CHANGELOG](CHANGELOG.md).

## Main features

- Three selectable USB audio driver modes: Local, Windows (UAC2) and Android (H38 bridge, LGPTUsbAudioBridge-H38-debug.apk).
- LGPT audio output to Windows through USB Audio at 48 kHz.
- Windows audio capture from the LGPT Record screen.
- Android bidirectional audio bridge with the LGPTUsbAudioBridge APK.
- Input monitoring only while Record is open.
- Mixer view with live VU meters and per-instrument FX menu (`R2 + A`, `R1 + A` toggles Solo).
- Sidebar pattern table at bottom-left.
- Phrase FX commands reduced to beatmaking essentials.
- Transactional recording with Preview, Save and Discard.
- Safe sample rename and deletion from the sample browser.
- Chopper global Undo/Redo with `L1+X` and `R1+X`.
- Chord-aware input handling for the R36SX controls.
- Contextual help overlay (`SELECT+R1`) with per-view controls.
- TreeFrogUI integration for kernel `4.4.186-release`.

## How it works — navigation

| Combination | Action |
|---|---|
| `R1 + LEFT/RIGHT` | Switch main view (Song / Chain / Phrase / Instrument / Table / Groove / Mixer) |
| `SELECT + R1` | Contextual help overlay (latched; release to close) |
| `SELECT + R2` | Audio Driver dialog (USB mode) |
| `START` | Play / Stop |

In the **Mixer**, `SELECT` cycles the pages MIX → DELAY → REVERB → EQ → COMP.
On the FX pages, `UP/DN` moves the row, `L/R` edits, `A` is coarse, and
`BYPASS` is always the first row (`ON = effect disabled`). See
[docs/CONTROLS_EN.md](docs/CONTROLS_EN.md) (English) /
[docs/CONTROLS_ES.md](docs/CONTROLS_ES.md) (Español) for the full key map,
sample browser and chopper shortcuts.

## Download

### LGPT for R36SX (Bacon-1.5) — Runtime

Use the latest GitHub Release **Bacon-1.5** to install the precompiled port. The repository contains source code; release assets contain the compiled core and SD installer.

**Download:**
- `LGPT_R36SX_Bacon-1.5_SD_ROOT.zip` (direct-copy SD root, 7-8M) — extract and copy CONTENTS to SD root
- `LGPT_R36SX_Bacon-1.5_Android.apk` (H38, 298118, validated) — separate asset and also inside ZIP at root

- [Installation guide — English](docs/INSTALL_EN.md)
- [Guía de instalación — Español](docs/INSTALL_ES.md)
- [Build guide — English](docs/BUILD_EN.md)
- [Guía de compilación — Español](docs/BUILD_ES.md)
- [Controls — English](docs/CONTROLS_EN.md)
- [Controles — Español](docs/CONTROLS_ES.md)
- [USB audio architecture](docs/AUDIO_OTG.md)
- [Troubleshooting — English](docs/TROUBLESHOOTING_EN.md)
- [Solución de problemas — Español](docs/TROUBLESHOOTING_ES.md)

### TreeFrog Content Manager (Desktop) — Manager

**End-user — Portable (primary, no installer):**

- Download the latest **TreeFrog Content Manager** GitHub Release asset: `TreeFrog-Content-Manager-<version>-Windows-x64.exe` (portable, with `.sha256`). No installation, no Rust/Node/Tauri required — just WebView2 (preinstalled on Windows 10/11).
- Copy to any folder (e.g., `C:\Tools\` or clean `C:\Temp\`) and double-click → **TreeFrog Content Manager** appears. Frog header upright, native Browse, BIOS/LGPT/dry-run all work, profile `1.1.0` embedded (no external `profiles/` needed). See `docs/MANUAL_QA_2E.md` § Portable EXE.

**End-user — Installer (optional):**

- Download `TreeFrog-Content-Manager-<version>-Windows-x64-Setup.exe` (NSIS installer, with `.sha256`). No manual `target/release` lookup — the build copies both artifacts to Desktop as `TreeFrog-Content-Manager-<version>-Windows-x64.exe` (portable) and `TreeFrog-Content-Manager-<version>-Windows-x64-Setup.exe` (installer).
- Run the installer → Start Menu / Desktop shortcuts → launch **TreeFrog Content Manager**. Window/taskbar/installer/shortcut icon is the TreeFrog frog pixel-art (upright, high-res, 6-size ICO). If Windows shows stale icon, see icon-cache clean validation in `docs/MANUAL_QA_2E.md`.

**Developer build (from source):**

```powershell
# Prerequisites: Rust stable, Node 20, MSVC, WebView2 — see docs/BUILD_WINDOWS.md
git clone https://github.com/ozkaoz/treefrog-content-manager.git
cd treefrog-content-manager
powershell -ExecutionPolicy Bypass -File scripts/build_windows.ps1
# Artifacts: treefrog-manager/src-tauri/target/release/treefrog-manager.exe (portable, 14 MB, profile embedded)
#            bundle/msi/*.msi + bundle/nsis/*-Setup.exe (installer)
#            Desktop: TreeFrog-Content-Manager-<version>-Windows-x64.exe + .sha256 (portable)
#                     TreeFrog-Content-Manager-<version>-Windows-x64-Setup.exe + .sha256 (installer)
# Manual QA: docs/MANUAL_QA_2E.md (portable + installed, frog orientation, icons, Light/Dark, BIOS/LGPT)
```

Branding: Frog pixel-art from `logo.png` 1536×1024 high-res desktop upright (primary, `logo.png` left 314×280 → `frog-canonical.png` 314×280, `frog-square.png` 512×512, no rotation, legs DOWN) — previous `xgame-logo.bmp` vertical boot asset was inverted/sideways and low-res (87×99 → solid green at 32). Pipeline `scripts/generate_branding.py` (NEAREST, `r<20` transparent, x-gap 517–549, 25% padding, 7-size ICO 16/24/32/48/64/128/256) + `icon.ico` 48k + `icon.icns` 577k. Full frog+wordmark retained only in About/Credits (`src/assets/branding/README.md`). No newly generated logo, no CSS rotation.

## Repository layout

- `source/`: current LGPT/TreeFrog source.
- `device/`: R36SX launcher, USB Audio daemon and OTG scripts.
- `deployment/`: files installed on the SD card.
- `recovery/`: validated UAC2 kernel module.
- `kernel_module_tools/`: module rebuild and verification tools.
- `scripts/`: build, audit, release, verification and legacy deployment utilities. `scripts/install.sh` and `scripts/verify.sh` are legacy U2523 and are **not** the canonical Bacon-1.5 installation path. Current installation: GitHub Bacon release ZIP → contents to SD root (see `docs/ai/RELEASE_CONTRACT.md`). `scripts/build_windows.ps1` + `scripts/generate_branding.py` for desktop.
- `tests/`: current regression tests.
- `docs/`: consolidated user and developer documentation. `docs/BUILD_WINDOWS.md` (desktop build), `docs/MANUAL_QA_2E.md` (manual QA).
- `profiles/treefrogui/`: versioned declarative TreeFrogUI profiles (systems, media, bios, lgpt, video, archive, sd markers).
 - `treefrog-manager/`: Tauri 2 desktop app (Rust + React + TypeScript, `src/services/dialog.ts` native dialogs, `src/services/theme.ts` Windows theme, `src/assets/branding/frog-canonical.png` 314×280 frog icon, `src/components/` navigation/source-picker/empty-states, `src-tauri/src/sd_target.rs` SD target detection + `src/components/SdCardPanel.tsx` SD analysis UI (read-only, zero-write, profile-driven `sd_markers.json`)).

## License

See [LICENSE](LICENSE). The repository may not contain the complete TreeFrogUI vendor base, but the Bacon-1.5 direct-copy release ZIP (`LGPT_R36SX_Bacon-1.5_SD_ROOT.zip`) contains the required integration/vendor runtime files deliberately packaged for this validated configuration, including `cubegm/picoarch` and `cubegm/lgpt.elf` as applicable.
