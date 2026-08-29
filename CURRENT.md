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

- **TreeFrog Content Manager — Phase 2C (video conversion + Windows desktop build):** Robust video inspection via `ffprobe` (authoritative), compatibility evaluation (`compatible`/`conversion_required`/`unsupported`/`inspection_error`) data-driven from `profiles/treefrogui/video_presets.json` (`PROVISIONAL_UNVALIDATED`), dedicated conversion service (never modifies source, temp workspace only, deterministic `*.converted.mp4`, overwrite protection, capture FFmpeg stderr, re-probe validation, clean temp, no SD writes), planner integration (`convert_then_copy`/`conversion_error` etc. as planned artifacts, single source of truth), dry-run UI shows video conversion (source/status/target/preset/reason), first real Windows x64 desktop build (`treefrog-manager/src-tauri/target/release/treefrog-manager.exe` + MSI/NSIS) with `--self-check` smoke test (profile load 1.1.0/75 systems, ffmpeg/ffprobe availability reporting, deterministic dataset: normal ROM, ZIP container, duplicate, compatible video, conversion-required video, zero SD writes).
- **Release golden preserved:** `Bacon-1.5` TreeFrog Apps `RELEASE_GOLDEN=PASS` — `TREEFROGUI_REQUIRED=v1.0.15_a`, `TREEFROG_APPS_ENTRY_LGPT=1 / GAMES=0`, `POST_INSTALL_MANUAL_FIXES=0`, `DOWNLOAD-BACK PASS` (no runtime/sd_root mutation in this phase; verify `git diff -- sd_root` = NO).
- **Idle baseline:** No active LGPT runtime task beyond content-manager Phase 2C — Await explicit user-approved objective for next milestone — No active implementation/runtime task beyond manager — Await explicit user-approved objective for next runtime change.

## Last Relevant Validation

- `RELEASE_AUDIO_BOOTSTRAP PASS` + `FROG_UI_APPS_LGPT PASS` (Apps-only, hide) + `TREEFROG_APPS_RELEASE PASS` (57 files, frogui 76034b)
- `TRUE_PHYSICAL_CLEAN_INSTALL PASS` (`Stock OS + TreeFrogUI v1.0.15_a + ZIP` → `POST_INSTALL_MANUAL_FIXES=0`)
- `DOWNLOAD-BACK PASS` (`/tmp/download_back/LGPT_R36SX_Bacon-1.5_SD_ROOT.zip` `faf7a230` 7295274 57, `unzip -t PASS`, `test_treefrog_apps_lgpt_release PASS`)
- `ELFs`: shipped `b07bbb` vs vanilla `f10caa` vs apps-only `76034b` — MIPS32r2 O32 hard-float 7 PHDR GLIBC 2.0/2.2/2.3/2.15, no generic drift
- `MANAGER Phase 2C`: `profile 1.1.0 PASS` + `video presets PROVISIONAL_UNVALIDATED PASS` + `ffprobe/ffmpeg service PASS` + `planner 2C PASS` + `77 tests PASS` (66+11 Phase 2C) + `Windows x64 exe 12.21 MB PASS` + `MSI 4.18 MB + NSIS 2.92 MB PASS` + `self-check PASS` (profile 1.1.0/75, video provisional, ffmpeg/ffprobe) + `smoke dataset PASS` (ROM/ZIP/duplicate/compatible+conversion videos, zero SD writes) + `context-contract PASS` + `preflight PASS` + `sd_root clean`

## Known Issues / Risks

- `scripts/install.sh`/`verify.sh` legacy U2523 — not canonical.
- Dirty exFAT false failures — SD health check before runtime blame.
- Generic GCC 12.4 black-screen (029584…); official SDK 6.3.0 required.

## Pending Validation

- Content Manager Phase 2C: awaiting review — then Phase 3 SD writes (not in this task). Video preset remains `PROVISIONAL_UNVALIDATED` until R36SX hardware test.

## Next Exact Action

- Run `python3 tests/test_agent_context_contract.py`, `python3 tests/test_release_audio_bootstrap.py`, `python3 -m pytest treefrog-manager/tests -v`, `bash scripts/agent_preflight.sh --allow-dirty`, verify `git diff -- sd_root`, verify Windows build `scripts/build_windows.ps1` and `treefrog-manager/src-tauri/target/release/treefrog-manager.exe --self-check`.

## Stop Conditions

- Any protected runtime drift (lgpt wrapper/core, OTG, audio, H38) → STOP
- Any inferred PHYSICAL PASS without device → STOP
- Generic GCC FrogUI → STOP
- Machine-specific path as authority → STOP
