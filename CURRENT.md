# Current Workspace State

**Last reviewed:** 2026-08-28
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

- **TreeFrog Content Manager — Phase 2E Desktop UX foundation (native dialogs, Windows theme, branding, navigation):** Replace `window.prompt()` with native Windows folder/file pickers via `src/services/dialog.ts` (`pickFolder()`, `pickFile()`) using `@tauri-apps/plugin-dialog` `open({directory:true})`; reusable abstraction for Games/Music/Video/BIOS/LGPT Samples/Projects/future SD target; Windows Light/Dark follows `prefers-color-scheme` dynamically via `src/services/theme.ts` + centralized CSS variables (`--bg`, `--surface`, `--surface-elevated`, `--text`, `--text-muted`, `--border`, `--accent`, `--success`, `--warning`, `--danger`, `--input`, `--focus`) in `src/styles.css`; TreeFrogUI frog branding from `xgame-logo.bmp` (480×854, TreeFrogUI CC BY-NC-SA 4.0) — frog ONLY as primary mark (`src/assets/branding/frog-only.png` 87×99 transparent, `frog-square.png` 99×99) via `scripts/generate_branding.py` (NEAREST, gap split 349–358), icons `src-tauri/icons/*` (32, 128, 128@2x, 256, 512, ico, icns) for window/installer/favicon/header; identity restrained `TreeFrog Content Manager` + frog; navigation `Overview | Games | Music | Videos | BIOS | LGPT | SD Card | Settings | About` (8 tabs) with placeholders "Coming in a future release" for not-yet-implemented; consistent `SourcePicker` (path visible + [Browse] native) + `EmptyState` (empty/loading/success/warning/error/not_implemented); LGPT + BIOS preserved functional.
- **TreeFrog Content Manager — LGPT & BIOS preserved:** LGPT Samples/Projects + BIOS Manager remain functional (see previous objective).
- **Release golden preserved:** `Bacon-1.5` TreeFrog Apps `RELEASE_GOLDEN=PASS` — `TREEFROGUI_REQUIRED=v1.0.15_a`, `TREEFROG_APPS_ENTRY_LGPT=1 / GAMES=0`, `POST_INSTALL_MANUAL_FIXES=0`, `DOWNLOAD-BACK PASS` (no runtime/sd_root mutation in this phase; verify `git diff -- sd_root` = NO).
- **Idle baseline:** No active runtime beyond manager — await next milestone.

## Last Relevant Validation

- `RELEASE_AUDIO_BOOTSTRAP PASS` + `FROG_UI_APPS_LGPT PASS` (Apps-only, hide) + `TREEFROG_APPS_RELEASE PASS` (57 files, frogui 76034b)
- `TRUE_PHYSICAL_CLEAN_INSTALL PASS` (`Stock OS + TreeFrogUI v1.0.15_a + ZIP` → `POST_INSTALL_MANUAL_FIXES=0`)
- `DOWNLOAD-BACK PASS` (`/tmp/download_back/LGPT_R36SX_Bacon-1.5_SD_ROOT.zip` `faf7a230` 7295274 57, `unzip -t PASS`, `test_treefrog_apps_lgpt_release PASS`)
- `ELFs`: shipped `b07bbb` vs vanilla `f10caa` vs apps-only `76034b` — MIPS32r2 O32 hard-float 7 PHDR GLIBC 2.0/2.2/2.3/2.15, no generic drift
- `MANAGER 2E`: `native dialogs` (dialog.ts + plugin) + `Windows theme` (prefers-color-scheme + CSS vars + dynamic watch) + `frog branding` (xgame-logo.bmp → frog-only 87×99 + icons 32/128/256/ico/icns, provenance `src/assets/branding/README.md` CC BY-NC-SA 4.0) + `navigation 8 tabs` (Overview/Games/Music/Videos/BIOS/LGPT/SD Card/Settings/About, placeholders) + `SourcePicker` consistent + `EmptyState` + `151 tests PASS` (131+20) + `Windows exe` launch PASS + native dialog PASS + light/dark PASS + icons PASS + `MANUAL_QA_2E.md` + zero SD writes
- `MANAGER LGPT/BIOS` preserved: `131 tests` + `Windows Desktop copy` + `self-check PASS`

## Known Issues / Risks

- `scripts/install.sh`/`verify.sh` legacy U2523 — not canonical.
- Dirty exFAT false failures — SD health check before runtime blame.
- Generic GCC 12.4 black-screen (029584…); official SDK 6.3.0 required.

## Pending Validation

- Content Manager 2E: awaiting review — then Phase 3 Music/Images/Ebooks or Phase 2D SD writes (not in this task). No SD writes.

## Next Exact Action

- Run `python3 tests/test_agent_context_contract.py`, `python3 tests/test_release_audio_bootstrap.py`, `python3 -m pytest treefrog-manager/tests -v` (151 tests), `bash scripts/agent_preflight.sh --allow-dirty`, verify `git diff -- sd_root`, verify Windows build `scripts/build_windows.ps1` + Desktop `TreeFrog-Content-Manager-0.1.0-Windows-x64-Setup.exe` + `treefrog-manager.exe --self-check` + native dialog + light/dark + icons + BIOS + LGPT per `docs/MANUAL_QA_2E.md`.

## Stop Conditions

- Any protected runtime drift (lgpt wrapper/core, OTG, audio, H38) → STOP
- Any inferred PHYSICAL PASS without device → STOP
- Generic GCC FrogUI → STOP
- Machine-specific path as authority → STOP
