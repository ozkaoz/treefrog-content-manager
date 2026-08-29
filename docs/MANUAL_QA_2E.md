# Manual Desktop QA — Phase 2E / 2E.1 (Desktop UX + Branding correction)

**Date:** 2026-08-29 (2E.1 hotfix)
**Build:** `treefrog-manager/src-tauri/target/release/bundle/nsis/TreeFrog-Content-Manager-0.1.0-Windows-x64-Setup.exe` (friendly `TreeFrog-Content-Manager-0.1.0-Windows-x64-Setup.exe` on Desktop, `TreeFrog-Content-Manager-Setup.exe` legacy)
**Prerequisite:** Windows 10/11 with WebView2, clean install via NSIS installer. For 2E.1 icon correctness, perform **clean validation** (uninstall → remove shortcuts → rebuild → fresh install) to bypass Windows icon cache — see § Windows icon cache QA.

## Smoke test (16 steps, executed manually on Windows)

| # | Action | Expected |
|---|--------|----------|
| 1 | Set Windows Settings → Personalization → Colors → **Light** | OS in Light mode |
| 2 | Launch **TreeFrog Content Manager** from Start Menu or Desktop shortcut | Window appears, header shows **correctly oriented frog** (eyes at top, belly yellow at bottom, 32×32, not upside-down/mirrored/stretched/blurred, `frog-only.png` 314×280 high-res) + "TreeFrog Content Manager". UI background `#ffffff` / text `#1e293b`, surfaces light. |
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
| 16 | Verify icon/branding — window icon, taskbar icon, Start Menu icon, Desktop installer icon, About page | All show **frog-only upright** (high-res 314×280, transparent, NEAREST, not inverted/mirrored, not generic green square), not frog+wordmark and not placeholder (32 1686B, ico 103k 6 sizes). About → Credits shows full frog+wordmark secondary. |

## Windows icon cache QA (2E.1)

Because Windows caches shortcut/application icons, perform clean validation:

1. `C:\Users\DaFunkNoise\AppData\Local\TreeFrog Content Manager\uninstall.exe /S` → wait 3s → verify `treefrog-manager.exe` removed
2. Delete residual `Desktop\TreeFrog Content Manager.lnk` and `AppData\Roaming\Microsoft\Windows\Start Menu\Programs\TreeFrog Content Manager.lnk` if they remain
3. Rebuild: `powershell -ExecutionPolicy Bypass -File scripts/build_windows.ps1` (or `npx tauri build` + copy to Desktop as `TreeFrog-Content-Manager-0.1.0-Windows-x64-Setup.exe`)
4. Install fresh: `TreeFrog-Content-Manager-0.1.0-Windows-x64-Setup.exe /S`
5. Verify Desktop shortcut exists, `WScript.Shell` `IconLocation` is `,0` (exe's first icon) and visually frog (not green square)
6. Verify Start Menu entry exists and shows frog
7. Launch → window title `TreeFrog Content Manager` with frog in title bar
8. Verify taskbar icon is frog while running
9. If stale cache persists, `taskkill /f /im explorer.exe & start explorer.exe` or `ie4uinit.exe -show` or reboot — document observed, but do not require end user to rebuild cache as part of normal use.

Result 2026-08-29: fresh install after `generate_branding.py` high-res (314) + ICO 103k (6 sizes) shows frog correctly in all four places; previous 87×99 low-res with 641B ICO showed green square and inverted header (fixed by switching canonical to `logo.png` and fixing ICO generation to use 256 source).

## Non-automatable steps

- Native picker appearance (OS-owned dialog) — cannot be asserted by unit tests; verified visually.
- Light/Dark dynamic update — requires OS theme change; unit tests mock `matchMedia`.
- Icon rendering at OS level (installer, window chrome) — verified by visual inspection and file existence checks (`test_phase2e_desktop_ux.py` + `test_phase2e_branding_fix.py` check `src-tauri/icons/*` sizes 32 1686B, ico 103k 6 sizes, `frog-only.png` 314×280, header no `rotate(180deg)`).

## Verification evidence (2026-08-29, 2E.1 fresh install)

- Installer builds via `powershell -ExecutionPolicy Bypass -File scripts/build_windows.ps1` → `TreeFrog Content Manager_0.1.0_x64-setup.exe` (3.48 MB, 3481860 bytes, SHA256 `2c4f88c730c40a1fb7b93d521b4cf8c862446f5cd43e64252c5e8ed9992858f3`) + friendly `TreeFrog-Content-Manager-0.1.0-Windows-x64-Setup.exe` on Desktop with `.sha256` (and legacy `TreeFrog-Content-Manager-Setup.exe`).
- Installed exe `C:\Users\DaFunkNoise\AppData\Local\TreeFrog Content Manager\treefrog-manager.exe` **14.08 MB** launches, `MainWindowTitle: TreeFrog Content Manager`, `HasExited:False`, handle non-zero; Desktop/StartMenu/taskbar/window all show **frog upright** (314 high-res, not inverted, not green square) — previous 87×99 inverted + 641B ICO showed green square.
- Start Menu + Desktop shortcuts exist (`IconLocation ,0` → exe's frog); after fresh install header shows frog 32×32 correctly oriented (eyes top, belly bottom).
- Uninstall `/S` removes exe/shortcuts/registry.
- Dry-run planner **160 tests PASS** (151+9 branding fix), no SD writes (`git diff -- sd_root` empty).
- Icons generated deterministically via `scripts/generate_branding.py` from `logo.png` 1536×1024 high-res desktop upright (primary) + `xgame-logo.bmp` fallback flipped (104078 bytes ico was placeholder, now 103442 with 6 sizes). `frog-only.png` 314×280, `frog-square.png` 314×314, `32x32.png` 1686B, `icon.ico` 103k, `icon.icns` 927k — all via NEAREST.

## How to re-run

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build_windows.ps1
# Then follow steps 1-16 above
```
