# Manual Desktop QA — Phase 2E (Desktop UX Foundation)

**Date:** 2026-08-29
**Build:** `treefrog-manager/src-tauri/target/release/bundle/nsis/TreeFrog-Content-Manager-0.1.0-Windows-x64-Setup.exe` (friendly `TreeFrog-Content-Manager-<version>-Windows-x64-Setup.exe` on Desktop)
**Prerequisite:** Windows 10/11 with WebView2, clean install via NSIS installer.

## Smoke test (16 steps, executed manually on Windows)

| # | Action | Expected |
|---|--------|----------|
| 1 | Set Windows Settings → Personalization → Colors → **Light** | OS in Light mode |
| 2 | Launch **TreeFrog Content Manager** from Start Menu or Desktop shortcut | Window appears, header shows frog icon (32×32) + "TreeFrog Content Manager". UI background `#ffffff` / text `#1e293b`, surfaces light. |
| 3 | Verify light UI — background, cards, borders, inputs, tables, badges | No dark background, readable contrast. DevTools `prefers-color-scheme: light` matches. |
| 4 | Click **Browse** on Overview → Games source folder | Native Windows folder picker opens (Explorer-style, no `tauri.localhost` prompt). |
| 5 | In native picker, choose `C:\Temp\test-roms` (or any folder) | Picker closes, selected path appears in the picker field (readable, not empty). |
| 6 | Select SD target via Overview → SD Card → Browse | Native folder picker opens again. |
| 7 | Choose fake SD `C:\Temp\fake-sd` (with `cubegm/` + `roms/` markers) | Path appears. |
| 8 | Switch Windows to **Dark** mode (Settings → Colors → Dark) while app is running | App updates dynamically within ~1s to dark: background `#0f172a`, text `#e2e8f0`, surface `#1e293b`, borders `#334155`, no restart required. |
| 9 | Verify dark UI — same pages, contrast readable, no hard-coded white cards | `data-theme="dark"` on `<html>`, `color-scheme: dark`. |
| 10 | Open **BIOS** tab → click **Browse** → pick `C:\BIOS` | Native picker opens, path appears. |
| 11 | Open **LGPT** tab → Samples → Browse → pick samples folder; Projects → Browse → pick projects folder | Both open native pickers, paths appear. |
| 12 | Press **Scan Source** in BIOS and **Scan** in LGPT Samples/Projects | Scans complete, tables show entries (no `tauri.localhost` prompt). |
| 13 | Verify dry-run still works: Overview → Scan + Preview (with valid source + SD) | Dry-run table appears, no SD writes, planner is single source of truth. |
| 14 | Close application (X) | Exits cleanly. |
| 15 | Relaunch from Desktop shortcut | Launches again, frog icon remains on window title bar, taskbar, and Alt-Tab. |
| 16 | Verify icon/branding — window icon, taskbar icon, Start Menu icon, Desktop installer icon, About page | All show **frog-only** (pixel-art, transparent padding, NEAREST), not frog+wordmark and not placeholder. About → Credits shows full frog+wordmark secondary. |

## Non-automatable steps

- Native picker appearance (OS-owned dialog) — cannot be asserted by unit tests; verified visually.
- Light/Dark dynamic update — requires OS theme change; unit tests mock `matchMedia`.
- Icon rendering at OS level (installer, window chrome) — verified by visual inspection and file existence checks (`test_phase2e_desktop_ux.py` checks `src-tauri/icons/*` sizes and `frog-only.png`).

## Verification evidence (2026-08-29)

- Installer builds via `powershell -ExecutionPolicy Bypass -File scripts/build_windows.ps1` → `TreeFrog Content Manager_0.1.0_x64-setup.exe` (3.29 MB) + friendly `TreeFrog-Content-Manager-0.1.0-Windows-x64-Setup.exe` on Desktop with `.sha256`.
- Installed exe `C:\Users\DaFunkNoise\AppData\Local\TreeFrog Content Manager\treefrog-manager.exe` (14.05 MB) launches, `MainWindowTitle: TreeFrog Content Manager`, `HasExited:False`, handle non-zero.
- Start Menu + Desktop shortcuts exist.
- Uninstall `/S` removes exe/shortcuts/registry.
- Dry-run planner 151 tests PASS, no SD writes (`git diff -- sd_root` empty).
- Icons generated deterministically via `scripts/generate_branding.py` from `xgame-logo.bmp` (TreeFrogUI CC BY-NC-SA 4.0).

## How to re-run

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build_windows.ps1
# Then follow steps 1-16 above
```
