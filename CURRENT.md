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

- **TreeFrog Content Manager — Phase BIOS-B (BIOS Manager UI + planner integration):** BIOS tab in main navigation (Overview/Scan/Requirements/Details) with TreeFrogUI-global profile-driven BIOS, user-friendly labels (`Verified`/`Missing`/`Needs verification`/`Invalid`/`Duplicate`/`Conflict`), scanner via existing `scanner→archive→hash→validator` (recursive, temp workspace, hash where needed, alias/pattern/size/hash validation, duplicate/conflict/unknown detection), multiple variants (any one `scph5501.bin` satisfies `ps1_bios`), conditional `psx_content_present` → “Required because PlayStation content was detected”, detail panel (system, logical name, requirement, accepted filenames, selected variant, source path, expected destination `cubegm/bios`, expected size, SHA-256 known/actual, reason), read-only actions (`import/copy`, `skip`, `replace`, `conflict`, `duplicate`, `manual_review` via existing `apply_resolutions`), global `DeploymentPlan` integration (`BIOS: source C:\BIOS\scph5501.bin → cubegm/bios/scph5501.bin action copy`), `DryRunPreview` BIOS filtering, system-health summary (`BIOS ⚠ 2 required BIOS missing`), no BIOS downloads, zero SD writes, `cargo check` and `npm run build` still pass, Windows exe remains buildable.
- **TreeFrog Content Manager — Phase BIOS-A preserved:** Formal `bios.json` 1.1.0 with `bios_definitions`, 7 states, no invented hashes.
- **Release golden preserved:** `Bacon-1.5` TreeFrog Apps `RELEASE_GOLDEN=PASS` — `TREEFROGUI_REQUIRED=v1.0.15_a`, `TREEFROG_APPS_ENTRY_LGPT=1 / GAMES=0`, `POST_INSTALL_MANUAL_FIXES=0`, `DOWNLOAD-BACK PASS` (no runtime/sd_root mutation in this phase; verify `git diff -- sd_root` = NO).
- **Idle baseline:** No active LGPT runtime task beyond content-manager BIOS-B — Await explicit user-approved objective for next milestone — No active implementation/runtime task beyond manager — Await explicit user-approved objective for next runtime change.

## Last Relevant Validation

- `RELEASE_AUDIO_BOOTSTRAP PASS` + `FROG_UI_APPS_LGPT PASS` (Apps-only, hide) + `TREEFROG_APPS_RELEASE PASS` (57 files, frogui 76034b)
- `TRUE_PHYSICAL_CLEAN_INSTALL PASS` (`Stock OS + TreeFrogUI v1.0.15_a + ZIP` → `POST_INSTALL_MANUAL_FIXES=0`)
- `DOWNLOAD-BACK PASS` (`/tmp/download_back/LGPT_R36SX_Bacon-1.5_SD_ROOT.zip` `faf7a230` 7295274 57, `unzip -t PASS`, `test_treefrog_apps_lgpt_release PASS`)
- `ELFs`: shipped `b07bbb` vs vanilla `f10caa` vs apps-only `76034b` — MIPS32r2 O32 hard-float 7 PHDR GLIBC 2.0/2.2/2.3/2.15, no generic drift
- `MANAGER Phase BIOS-B`: `bios.json 1.1.0 PASS` + `bios validation + UI PASS` (17+13 tests) + `107 tests PASS` (94+13) + `Windows exe 12.21 MB still buildable PASS` ( `cargo check` + `npm run build` PASS) + `BiosManager` + `DryRunPreview` BIOS filtering + `DeploymentPlan` BIOS integration + `context-contract PASS` + `preflight PASS` + `sd_root clean` + `no invented hashes` + `smoke dataset` (valid, wrong-hash, duplicate, unknown, missing, multi-variant) `PASS` + `zero SD writes`
- `MANAGER Phase BIOS-A/2C` preserved: `video presets PROVISIONAL_UNVALIDATED` + `Windows exe` + `self-check`

## Known Issues / Risks

- `scripts/install.sh`/`verify.sh` legacy U2523 — not canonical.
- Dirty exFAT false failures — SD health check before runtime blame.
- Generic GCC 12.4 black-screen (029584…); official SDK 6.3.0 required.

## Pending Validation

- Content Manager BIOS-B: awaiting review — then Phase BIOS-C or Phase 2D SD writes (not in this task). No SD writes, no BIOS UI beyond read-only planning.

## Next Exact Action

- Run `python3 tests/test_agent_context_contract.py`, `python3 tests/test_release_audio_bootstrap.py`, `python3 -m pytest treefrog-manager/tests -v` (107 tests), `bash scripts/agent_preflight.sh --allow-dirty`, verify `git diff -- sd_root`, verify `cargo check` + `npm run build` + `npx tauri build` still produce Windows exe, verify `treefrog-manager.exe --self-check` and BIOS UI smoke test.

## Stop Conditions

- Any protected runtime drift (lgpt wrapper/core, OTG, audio, H38) → STOP
- Any inferred PHYSICAL PASS without device → STOP
- Generic GCC FrogUI → STOP
- Machine-specific path as authority → STOP
