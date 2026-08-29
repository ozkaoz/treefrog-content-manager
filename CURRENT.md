# Current Workspace State

**Last reviewed:** 2026-08-30
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

- **TreeFrog Content Manager — Phase 3A SD target detection + target validation + deployment-plan integration (READ-ONLY, no SD writes) — DONE:** Platform abstraction for removable volumes (Windows: `GetLogicalDrives`/`GetDriveTypeW`/`GetVolumeInformationW`/`GetDiskFreeSpaceExW` via `windows` crate, no admin, no modify; `VolumeInfo` with path/label/filesystem/total/free/removable/accessible); TreeFrogUI validation via `sd_markers.json` (cubegm/ + roms/ → valid, cubegm xor roms → incomplete, none → unknown, not accessible → inaccessible, TreeFrogUI global); read-only target analysis (`analyze_target` never creates files, shows `Volume TREEFROG` `exFAT` `64 GB` `42.8 GB` `TreeFrogUI ✓` `LGPT ✓` `READY`, `existing_count`/`total_size`, `rom_dirs`/`media_dirs`/`bios_dirs`/`lgpt_dirs`); target indexing reusing `scanner`/`hash` (`walkdir` read-only, `sha256` for duplicate, logical units preserved); planner single source (`dry_run_with_target` = `SOURCE SCAN → TARGET SCAN → plan → validate_destination_path → check_case_collision → calculate_space`); space calc (`bytes_to_copy`/`extract`/`generate`/`skip`, `required` vs `available`, `insufficient_space`); safe path handling (absolute/traversal/drive/UNC/ADS/reserved/illegal `<>:\"|?*`/backslash/case-collision, profile as source); SD Card UI `SdCardPanel.tsx` native `pickFolder` → `Selected E:\` `Volume` `Filesystem` `Capacity` `Free` `TreeFrogUI` `LGPT` `Status` → `[Analyze]` → `[Dry-run with target]` → `New/Changed/Duplicate/Conflict/Conversion/BIOS warnings/Insufficient space` (`Space: Required 8.42 GB / Available 7.91 GB / Not enough space`), Sync disabled.
- **TreeFrog Content Manager — Completed:** Bootstrap/scanner, Archive ingestion, Duplicate/conflict, Video pipeline, BIOS (A+B+Manager), Desktop UX (native dialogs, Windows theme, branding `frog-canonical.png` 314×280, portable `14.29 MB` + installer `3.49 MB`), LGPT Samples/Projects, **SD target (3A) 17 new tests** — all `182 tests PASS`.
- **Release golden preserved:** `Bacon-1.5` `RELEASE_GOLDEN=PASS` — `TREEFROGUI_REQUIRED=v1.0.15_a`, `POST_INSTALL_MANUAL_FIXES=0`, `DOWNLOAD-BACK PASS` (`git diff -- sd_root` = NO).
- **Idle baseline:** No active SD writes — await Phase 3B deployment engine (staging, atomic rename, resume).

## Last Relevant Validation

- `RELEASE_AUDIO_BOOTSTRAP PASS` + `FROG_UI_APPS_LGPT PASS` (Apps-only, hide) + `TREEFROG_APPS_RELEASE PASS` (57 files, frogui 76034b)
- `TRUE_PHYSICAL_CLEAN_INSTALL PASS` (`Stock OS + TreeFrogUI v1.0.15_a + ZIP` → `POST_INSTALL_MANUAL_FIXES=0`)
- `DOWNLOAD-BACK PASS` (`/tmp/download_back/LGPT_R36SX_Bacon-1.5_SD_ROOT.zip` `faf7a230` 7295274 57, `unzip -t PASS`, `test_treefrog_apps_lgpt_release PASS`)
- `ELFs`: shipped `b07bbb` vs vanilla `f10caa` vs apps-only `76034b` — MIPS32r2 O32 hard-float 7 PHDR GLIBC 2.0/2.2/2.3/2.15, no generic drift
- `MANAGER 2E.3`: `frog-canonical.png` 314×280 `frog-square.png` 512×512 `icon.ico` 48487B 7 sizes (16 307B 40 unique, 32 876B 177 unique) header `[frog upright legs DOWN]` + `window/taskbar/Desktop/StartMenu/installer` all frog via fresh install after `tauri.conf.json` `icon.ico` first → PE 7 PNG-compressed icons + `portable` 14.29 MB + `installer` 3.49 MB both `7` sizes not solid, `165 tests PASS` + `MANUAL_QA_2E.md` + zero SD writes
- `MANAGER 2E.2/2E.1/2E` preserved: `header-preview.png` `build/branding-preview.png` + `clean_test` `14.33 MB` + `explorer cache` handling

## Known Issues / Risks

- `scripts/install.sh`/`verify.sh` legacy U2523 — not canonical.
- Dirty exFAT false failures — SD health check before runtime blame.
- Generic GCC 12.4 black-screen (029584…); official SDK 6.3.0 required.

## Pending Validation

- Content Manager 3A: SD target detection + validation + analysis + indexing + planner integration + space calculation + safe path + SD Card UI + zero-write — awaiting implementation.

## Next Exact Action

- Implement `treefrog-manager/src-tauri/src/sd_target.rs` / `python/treefrog/sd_target.py` (Windows removable volumes via `windows` crate, `GetVolumeInformationW`/`GetDiskFreeSpaceExW`, markers from `sd_markers.json`), `target_scan` read-only, `target_index` reuse `scanner`/`hash`, `planner` integration, `space` calc, `safe_path` validation, `SdCard.tsx` UI with native `pickFolder`, `tauri` commands `list_volumes`/`analyze_target`/`dry_run_with_target`, tests with temp fixture, `npm run build` + `cargo check` + `pytest` + `git diff -- sd_root` empty + Windows build portable+installer + SD Card tab + manual SD (if available).

## Stop Conditions

- Any protected runtime drift (lgpt wrapper/core, OTG, audio, H38) → STOP
- Any inferred PHYSICAL PASS without device → STOP
- Generic GCC FrogUI → STOP
- Machine-specific path as authority → STOP
