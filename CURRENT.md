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

- **TreeFrog Content Manager — Phase 2E.3 Final frog orientation + Windows EXE/shortcut icon correction (hotfix, no SD writes):** Fix remaining sideways frog (user reports header still sideways, requires **-45° (45° clockwise)** relative to 2E.2 asset, legs DOWN). Root cause: `logo.png` 314×280 wide without rotation displayed as sideways header (280×314 tall after 90° CCW still sideways). Corrected `scripts/generate_branding.py` to `90° CCW` then `-45°` (net 45° CCW from original) → `frog-only.png` `422×422` upright (head top, body below head, legs DOWN, not sideways/upside-down/mirrored) + `frog-square.png` `527×527` (25% padding) via `NEAREST` + transparent. Icons regenerated from correctly oriented frog: `16×16.png` 272B, `24×24.png` 569B, `32×32.png` 839B, `48×48.png` 1624B, `64×64.png` 2579B, `128×128.png` 8755B, `256×256.png` 31093B, `512×512.png` 105840B, `icon.ico` 52759B (7 sizes 16/24/32/48/64/128/256, was 641B placeholder, previous 103k 6 sizes), `icon.icns` 655978B. Verified header `[frog 32×32 upright legs DOWN, not sideways]` + Desktop/StartMenu/taskbar/window/Alt-Tab/installer all frog via fresh install after cache bypass. Audit chain `frog-square.png → PNG variants → ICO (valid multi-res, transparent, not solid green at 16×16) → Tauri `bundle.icon` → PE exe (`treefrog-manager.exe` 14.31 MB) → NSIS shortcuts (`,0` exe icon)`. Portable `TreeFrog-Content-Manager-0.1.0-Windows-x64.exe` 14.31 MB (profile `1.1.0` embedded `include_str!`, clean dir `--self-check PASS`) + Installer `TreeFrog-Content-Manager-0.1.0-Windows-x64-Setup.exe` 3.51 MB both on Desktop with `.sha256`, WebView2 only. Build `scripts/build_windows.ps1` + `.github/workflows/release.yml` produce both for tags.
- **TreeFrog Content Manager — Phase 2E.2/2E.1/2E preserved:** native dialogs, Windows theme, navigation 8 tabs, SourcePicker, EmptyState, BIOS/LGPT functional; 165 tests (151+9+5).
- **Release golden preserved:** `Bacon-1.5` `RELEASE_GOLDEN=PASS` — `TREEFROGUI_REQUIRED=v1.0.15_a`, `POST_INSTALL_MANUAL_FIXES=0`, `DOWNLOAD-BACK PASS` (`git diff -- sd_root` = NO).
- **Idle baseline:** No active runtime beyond manager — await next milestone.

## Last Relevant Validation

- `RELEASE_AUDIO_BOOTSTRAP PASS` + `FROG_UI_APPS_LGPT PASS` (Apps-only, hide) + `TREEFROG_APPS_RELEASE PASS` (57 files, frogui 76034b)
- `TRUE_PHYSICAL_CLEAN_INSTALL PASS` (`Stock OS + TreeFrogUI v1.0.15_a + ZIP` → `POST_INSTALL_MANUAL_FIXES=0`)
- `DOWNLOAD-BACK PASS` (`/tmp/download_back/LGPT_R36SX_Bacon-1.5_SD_ROOT.zip` `faf7a230` 7295274 57, `unzip -t PASS`, `test_treefrog_apps_lgpt_release PASS`)
- `ELFs`: shipped `b07bbb` vs vanilla `f10caa` vs apps-only `76034b` — MIPS32r2 O32 hard-float 7 PHDR GLIBC 2.0/2.2/2.3/2.15, no generic drift
- `MANAGER 2E.2`: `frog corrected 90° CCW` (logo.png 314×280 → 280×314 upright, was 314×280 sideways, was xgame 87×99 inverted + solid green) + `icons corrected` (32 1717B, 64 5387B, 128 19186B, 256 65841B, 512 117415B, ico 103360B 6 sizes was 641, icns 911k) + `header [frog upright]` + `window/taskbar/Desktop (`,0` exe icon)/StartMenu/installer` all frog via fresh install + `portable` `TreeFrog-Content-Manager-0.1.0-Windows-x64.exe` 14.28 MB 3dc7e229... + `installer` 3.48 MB 89adb9a3... both on Desktop with `.sha256`, `profile embedded` (`include_str!` + `current_exe` fallback) → clean dir `--self-check PASS` (no external profiles needed) + `165 tests PASS` (160+5 portable/release) + `MANUAL_QA_2E.md` + zero SD writes
- `MANAGER 2E.1/2E` preserved: `160 tests` + `Windows Desktop copy` + `self-check PASS`

## Known Issues / Risks

- `scripts/install.sh`/`verify.sh` legacy U2523 — not canonical.
- Dirty exFAT false failures — SD health check before runtime blame.
- Generic GCC 12.4 black-screen (029584…); official SDK 6.3.0 required.

## Pending Validation

- Content Manager 2E.2: awaiting review — then Phase 3 Music/Images/Ebooks or Phase 2D SD writes (not in this task). No SD writes, no new modules.

## Next Exact Action

- Run `python3 tests/test_agent_context_contract.py`, `python3 tests/test_release_audio_bootstrap.py`, `python3 -m pytest treefrog-manager/tests -v` (165 tests), `bash scripts/agent_preflight.sh --allow-dirty`, verify `git diff -- sd_root`, verify Windows build `scripts/build_windows.ps1` + Desktop `TreeFrog-Content-Manager-0.1.0-Windows-x64.exe` (portable, clean dir --self-check) + `TreeFrog-Content-Manager-0.1.0-Windows-x64-Setup.exe` (installer, fresh install) + `MANUAL_QA_2E.md` (portable + installed, frog orientation, icons, Light/Dark, Browse, BIOS, LGPT).

## Stop Conditions

- Any protected runtime drift (lgpt wrapper/core, OTG, audio, H38) → STOP
- Any inferred PHYSICAL PASS without device → STOP
- Generic GCC FrogUI → STOP
- Machine-specific path as authority → STOP
