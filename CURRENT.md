# Current Workspace State

**Last reviewed:** 2026-08-29
**Repo:** https://github.com/ozkaoz/treefrog-content-manager
> This is a last-known snapshot and must be verified against direct evidence (Git, build, device, release asset). If it contradicts direct evidence, direct evidence wins.

---

## Authority

Constitution: `AGENTS.md v2.1` > ACTIVE `DECISIONS.md` > direct evidence > this snapshot.
Do not trust hardcoded historical state.

## Repository

- Branch: RESOLVE FROM GIT AT SESSION START — `git branch --show-current` at session start
- HEAD: RESOLVE FROM GIT AT SESSION START — `git rev-parse HEAD` at session start (do not hardcode SHA here; see Source/Physical/Release Golden for authoritative SHAs)
- Upstream: RESOLVE FROM GIT AT SESSION START — `git rev-parse --abbrev-ref --symbolic-full-name @{u}` + `git status --short --branch`
- Worktree: environment-specific — `git worktree list`
- Stash: verify `git stash list`

## Current Product Baseline

- Version: Bacon-1.5 TreeFrog Apps Migration (2026-08-24) — Apps-only
- Core: `46bd84ebb0d1b1be8caec7c76fecbe6fb4baa8e9bbd603b44488bcc929dedec6` (1559548) — unchanged, `cubegm/cores/lgpt_core.so`
- FrogUI: `76034bd3c142a9fe24df8729a1ef0dee6f1d8c6b4e5e046db05ebc890b54a0ef` (326700, `cubegm/cores/frogui_libretro.so`) — Apps LGPT, r36sx 028b011, CC BY-NC-SA 4.0, patch `patches/frogui_apps_lgpt.patch`
- TreeFrogUI required: `v1.0.15_a` (Apps-capable)
- ZIP: `LGPT_R36SX_Bacon-1.5_SD_ROOT.zip` `7295274` `faf7a230c06660b2299664f819f8d517c139311d5bbe8e8a0cbc421623ba0dec` (57 files, Apps→LGPT, Games absent) — `docs/BACON_1_5_RELEASE_MANIFEST.md`, `LGPT_R36SX_Bacon-1.5_SHA256SUMS.txt`
- Install: `Stock OS + TreeFrogUI v1.0.15_a + ZIP → Apps→LGPT` `POST_INSTALL_MANUAL_FIXES=0` `VISIBLE_LGPT_ENTRIES_TOTAL=1`

## Source Golden

- `49a640b` (main) — Apps migration Apps-only, deterministic from `sd_root` (57 files), FrogUI 76034b, wrapper/core unchanged.

## Physical Golden

- Payload `faf7a230c06660b2299664f819f8d517c139311d5bbe8e8a0cbc421623ba0dec` (57 files, Apps→LGPT) **TRUE clean-install PASS** via `Stock OS + TreeFrogUI v1.0.15_a + ZIP` `POST_INSTALL_MANUAL_FIXES=0` — `TREEFROGUI_BOOT PASS`, `TREEFROG_APPS_ENTRY_LGPT=1 / GAMES=0`, `LOCAL/WINDOWS/SP404/ANDROID/SWITCHING PASS`, `LGPT functional regression PASS`.
- Evidence: `docs/BACON_1_5_TREEFROG_APPS_PHYSICAL_PASS.md` (2026-08-24), `build/release_candidate/LGPT_R36SX_Bacon-1.5_SD_ROOT.zip` `faf7a230` physically validated.
- Previous `C5C77A...` (56 files, Games→LGPT) remains historical.

## Release Golden

- New ZIP `faf7a230c06660b2299664f819f8d517c139311d5bbe8e8a0cbc421623ba0dec` `7295274` `57` built deterministically, `unzip -t PASS`, `bootstrap PASS`, `test_treefrog_apps_lgpt_release PASS`, `TRUE_PHYSICAL_CLEAN_INSTALL PASS`, **published**, **download-back `REMOTE_SHA==LOCAL_SHA` `faf7a230`**, `REMOTE_IDENTICAL=YES`, `LATEST=YES` `PRERELEASE=NO` `DRAFT=NO`.
- Previous `C5C77A0212e4784a9d0e6d0eddc4de1a8bbe0943b9ebef8b13a18a82a6b9cb1e` `7138546` `56` remains historical (Games→LGPT, v1.0.14_a).
- Tag `Bacon-1.5` moved from `d404091`/`86e071` to `ba43a71` (`27edc78` annotated) — `RELEASE_GOLDEN_COMMIT=ba43a71`.

## Current Objective

- **TreeFrog Content Manager — Phase 2E.1 Branding & Windows icon correction (hotfix, no new modules, no SD writes):** Fix vertically inverted frog (root cause: `xgame-logo.bmp` 87×99 low-res boot asset stored for handheld rotated display, used without flip + NEAREST scaling → solid green square at 32×32 and upside-down header). Corrected canonical to `logo.png` 1536×1024 high-res desktop upright (frog left, 314×280, no flip) via updated `scripts/generate_branding.py` (is_bg `r<20`, x-gap 517–549, NEAREST, 314×314 square upscaled to 512 for icons). Regenerated icons `32x32.png` 1686B, `64x64.png`, `128x128.png`, `256x256.png`, `512x512.png`, `icon.ico` 103442B (6 sizes 16/32/48/64/128/256, was 641B placeholder), `icon.icns` 927052B; `frog-only.png` 314×280 + `frog-square.png` 314×314 transparent, provenance `src/assets/branding/README.md` updated (no redraw, no CSS rotate workaround). Header now `[frog 32×32 upright, not mirrored/stretched/blurred] TreeFrog Content Manager`; same transparent frog works in Light (`#ffffff`) and Dark (`#0f172a`). Installer `TreeFrog-Content-Manager-0.1.0-Windows-x64-Setup.exe` now carries correct ICO for Desktop/Start Menu/taskbar/window.
- **TreeFrog Content Manager — Phase 2E preserved:** native dialogs (`dialog.ts`), Windows theme (`prefers-color-scheme` + CSS vars), navigation 8 tabs, SourcePicker, EmptyState, BIOS/LGPT functional; 160 tests (151+9 branding fix).
- **Release golden preserved:** `Bacon-1.5` `RELEASE_GOLDEN=PASS` — `TREEFROGUI_REQUIRED=v1.0.15_a`, `TREEFROG_APPS_ENTRY_LGPT=1 / GAMES=0`, `POST_INSTALL_MANUAL_FIXES=0`, `DOWNLOAD-BACK PASS` (no runtime/sd_root mutation; `git diff -- sd_root` = NO).
- **Idle baseline:** No active runtime beyond manager — await next milestone.

## Last Relevant Validation

- `RELEASE_AUDIO_BOOTSTRAP PASS` + `FROG_UI_APPS_LGPT PASS` (Apps-only, hide) + `TREEFROG_APPS_RELEASE PASS` (57 files, frogui 76034b)
- `TRUE_PHYSICAL_CLEAN_INSTALL PASS` (`Stock OS + TreeFrogUI v1.0.15_a + ZIP` → `POST_INSTALL_MANUAL_FIXES=0`)
- `DOWNLOAD-BACK PASS` (`/tmp/download_back/LGPT_R36SX_Bacon-1.5_SD_ROOT.zip` `faf7a230` 7295274 57, `unzip -t PASS`, `test_treefrog_apps_lgpt_release PASS`)
- `ELFs`: shipped `b07bbb` vs vanilla `f10caa` vs apps-only `76034b` — MIPS32r2 O32 hard-float 7 PHDR GLIBC 2.0/2.2/2.3/2.15, no generic drift
- `MANAGER 2E.1`: `frog corrected` (logo.png 314×280 high-res upright, was xgame 87×99 inverted + solid green at 32) + `icons corrected` (32 1686B, 64,128,256,512, ico 103442 6 sizes was 641, icns 927k) + `header/taskbar/Desktop/StartMenu/installer` all frog via fresh install after `generate_branding.py` (NEAREST) + `native dialogs` + `theme Light/Dark` + `8 tabs` + `160 tests PASS` (151+9) + `Windows exe 14.08 MB` launch PASS + `MANUAL_QA_2E.md` updated + zero SD writes
- `MANAGER 2E` preserved: `151 tests` + `Windows Desktop copy` + `self-check PASS`

## Known Issues / Risks

- `scripts/install.sh`/`verify.sh` legacy U2523 — not canonical.
- Dirty exFAT false failures — SD health check before runtime blame.
- Generic GCC 12.4 black-screen (029584…); official SDK 6.3.0 required.

## Pending Validation

- Content Manager 2E.1: awaiting review — then Phase 3 Music/Images/Ebooks or Phase 2D SD writes (not in this task). No SD writes, no new modules.

## Next Exact Action

- Run `python3 tests/test_agent_context_contract.py`, `python3 tests/test_release_audio_bootstrap.py`, `python3 -m pytest treefrog-manager/tests -v` (160 tests), `bash scripts/agent_preflight.sh --allow-dirty`, verify `git diff -- sd_root`, verify Windows build `scripts/build_windows.ps1` + Desktop `TreeFrog-Content-Manager-0.1.0-Windows-x64-Setup.exe` + fresh install + `MANUAL_QA_2E.md` steps 1-9 (Desktop/StartMenu/taskbar/window/header) + Light/Dark + Browse + BIOS + LGPT.

## Stop Conditions

- Any protected runtime drift (lgpt wrapper/core, OTG, audio, H38) → STOP
- Any inferred PHYSICAL PASS without device → STOP
- Generic GCC FrogUI → STOP
- Machine-specific path as authority → STOP
