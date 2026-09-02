# Current Workspace State

**Last reviewed:** 2026-09-01
**Repo:** https://github.com/ozkaoz/treefrog-content-manager
> This is a last-known snapshot and must be verified against direct evidence (Git, build, device, release asset). If it contradicts direct evidence, direct evidence wins.

---

## Authority

Constitution: `AGENTS.md v2.1` > ACTIVE `DECISIONS.md` > direct evidence > this snapshot.
Do not trust hardcoded historical state.

## Repository

- Branch: RESOLVE FROM GIT AT SESSION START — `git branch --show-current`
- HEAD: RESOLVE FROM GIT AT SESSION START — `git rev-parse HEAD` (see Release Golden for the shipped SHAs)
- Upstream: `git rev-parse --abbrev-ref --symbolic-full-name @{u}` + `git status --short --branch`
- Note: this repository hosts BOTH (a) the **TreeFrog Content Manager** (the product: `treefrog-manager/`) and (b) the historical **LGPT R36SX port** golden artifacts (`source/`, `sd_root/`, Bacon-* releases). The Content Manager is the active product; the LGPT port is preserved baseline/evidence.

## Current Product — TreeFrog Content Manager

- **Version: 0.1.1** (Cargo/package.json/tauri.conf, consistent — CI-enforced)
- **Release: v1.0.1 = LATEST** — https://github.com/ozkaoz/treefrog-content-manager/releases/tag/v1.0.1
  - Assets: `TreeFrog-Content-Manager-0.1.1-Windows-x64.exe` (portable), `TreeFrog-Content-Manager-0.1.1-Linux-x64.AppImage`, `TreeFrog-Content-Manager-0.1.1-macOS-x64.dmg` + auto source zip/tar.gz
  - **DOWNLOAD-BACK PASS**: exe downloaded from the release, `--self-check PASS` (profile 1.1.0, 75 systems, `bios catalog entries: 13`), GUI smoke PASS
  - Release flow: tag `v*` → build 1 executable per OS → `--self-check` (BIOS catalog verified) on each platform → publish Latest. No re-validation in release (same commit already passed validate.yml on push to main).
- **Architecture state (audit 2026-08-31 + UI 2026-09-01, all shipped in v1.0.1):**
  - One canonical plan model: preview == deployment (`planEntries` param — no re-scan drift)
  - `paths.rs`: single destination validator (absolute/UNC/drive/`..`/empty/ADS/reserved/illegal + containment); used by writer/planner/BIOS/overrides/archives
  - BIOS = normal PlanEntry flow (no parallel write path); `bios.json` **embedded** in the binary (portable contract: BIOS catalog never empty on any platform) + filesystem override for dev
  - Real video conversion (staged → ffmpeg → ffprobe-validate → deploy; ffmpeg filtergraph escaping bug fixed); `convert_then_copy` never copies the original
  - `effective_action()` everywhere; space from effective actions; `keep_both` collision-safe `_1.._N` (backend authority, `resolve_plan` command, thin frontend)
  - `sd::detect` tri-state (writable/healthy proven, never inferred); stable SD id = Windows volume GUID + serial
  - SQLite persistence (migrations; job/job_entry/deployment/content_fingerprint)
  - Archives: ZIP only (safe adapter); 7z/RAR explicitly `unsupported_archive`
  - Per-tab deploy to exact TreeFrogUI paths: Games→`roms/<SYSTEM>/`, Music→`roms/music/` (subfolder=playlist), Videos→`roms/videos/`, BIOS→`cubegm/bios/`, LGPT→`lgpt/samples|projects/`
  - UI: unified panel buttons (Scan/Clear/Back/Skip/Continue/Sync — actions above Browse); **Sync to SD always active** (observable outcome); Music search gated by scan; BIOS search removed; shared `SdStatusBar` (real SD state everywhere, refreshed after every sync via `sdRefreshSignal`); dynamic Overview status from live plans + real SD counts
- **Validation state:** CI green on push (validate.yml: frontend tsc+build, Rust fmt/check/test 47, pytest 224 incl. real-ffmpeg conversion + security fixtures, version gate, Tauri build). Local policy: minimal per-change checks (`scripts/quick_validate.ps1`, DEC-2026-09-01-02); CI is the full gate.

## Known Issues / Risks

- **SmartScreen (unsigned binaries)**: the Windows exe shows "Windows protected your PC" because it is not code-signed. Options documented: Certum Open Source (~€69/yr), OV/EV certs, or wait for organic reputation. Workflow ready to add signing when a cert is provided.
- `video_presets.json` declares `status: PROVISIONAL_UNVALIDATED` (= not hardware-validated on device; conversions themselves are executed + ffprobe-validated).
- Physical R36SX deploy validation of the Manager remains pending (no device in session); all SD-write paths covered by canonical-validation security tests on fixtures.

## Historical baseline preserved (LGPT R36SX port — NOT the active product)

- Bacon-1.5 golden: ZIP `faf7a230…` (57 files, Apps→LGPT), `Stock OS + TreeFrogUI v1.0.15_a + ZIP`, `POST_INSTALL_MANUAL_FIXES=0`, download-back PASS. See `docs/BACON_1_5_*` and Git history.

## Pending Validation

- Physical SD deploy with the Manager on a real device (user-accepted host validation so far).

## Next Exact Action

- None blocked. Optional next steps: code-signing certificate for SmartScreen; physical-device deploy validation round.

## Stop Conditions

- Any protected runtime drift (lgpt wrapper/core, OTG, audio, H38) → STOP
- Any inferred PHYSICAL PASS without device → STOP
- Machine-specific path as authority → STOP
