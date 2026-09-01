# Working Audit Note — TreeFrog Content Manager Full Audit + Hardening

**Date:** 2026-08-31 (completed)
**Repo:** https://github.com/ozkaoz/treefrog-content-manager (local: `treefrog-manager/` inside lgpt-r36sx-port worktree, branch `main`)
**Status:** ALL FIXES IMPLEMENTED + HOST-VALIDATED. See "Resolution status" below.

## Resolution status (final)

| ID | Status | Evidence |
|----|--------|----------|
| P0-SEC-1 BIOS parallel write | FIXED | `bios_entries_to_plan_entries` + canonical deploy; tests `bios_destination_escape_rejected`, `bios_full_lifecycle_through_canonical_pipeline`, `bios_conflict_resolved_via_canonical_model` PASS |
| P0-SEC-2 canonical resolver | FIXED | new `paths.rs` (`resolve_validated_destination` + `DestinationError`), used by deploy writer, planner validation, BIOS, user overrides, archive members; 8 paths tests + writer-direct security tests PASS |
| P1-1 sd::detect tri-state | FIXED | Rust `sd.rs` + Python `sd.py`: `accessible`/`writable`(Option)/`healthy` never inferred; `detect_with_probe` proves; tests PASS |
| P1-2 real video conversion | FIXED | `deploy_converted_video`: probe→stage→ffmpeg→ffprobe-validate→deploy; staged cleanup on failure/cancel; phases in progress events; BONUS: found+fixed real ffmpeg `-vf scale=min(640\,iw)` filtergraph escaping bug (Rust+Python) — conversion was silently failing before; 5 Rust + 4 Python real-ffmpeg tests PASS |
| P1-3 effective_action | FIXED | `lib::effective_action` used in deploy/progress/summary/space/collisions/`write_dests`; test PASS |
| P1-4 space from effective | FIXED | Rust `sd_target::calculate_space` + Python mirror: single per-entry effective decision; per-resolution-type tests PASS |
| P1-5 keep_both safety | FIXED | `next_available_destination` (`_1.._N` vs disk + plan + case) in Rust `planner::apply_resolutions_ctx` + Python mirror; frontend reimplementación deleted; tests PASS |
| P1-6 override validation | FIXED | deploy-time user overrides must pass canonical validator (observable error, never silent) |
| P1-7 archive stubs | FIXED | ZIP only; 7z/RAR explicit precise `unsupported_archive` reason; stub strings removed (Rust+Python); fixtures PASS |
| P1-8 BIOS single model | FIXED | `bios_catalog` + `validate_bios_file` command derive from `bios.json`; stock guard from profile; delete-guard from profile; catalog test PASS |
| P2-1 Overview counts | FIXED | backend `get_content_counts` semantic counters; UI displays directly; fake `*100` multipliers removed |
| P2-2 version SSoT | FIXED | `app_version` command (CARGO_PKG_VERSION); UpdateChecker/SettingsPanel use it; CI version-consistency gate enforces Cargo=package=tauri.conf |
| P2-3 stable SD id | FIXED | Windows volume GUID (`GetVolumeNameForVolumeMountPointW`) + serial; mount path excluded; fallback explicit `fallback:` prefix; test PASS |
| P2-4 SQLite | FIXED (scoped) | `db.rs` migrations + job/job_entry/deployment/content_fingerprint with hash/size/versions/target identity; deploy records jobs; scope+remainder documented in module doc; tests PASS |
| P2-5 repo metadata | FIXED | Cargo.toml repository → treefrog-content-manager; misleading tool-adapter comments removed |
| CI | FIXED | `validate.yml` (frontend/tsc/build, rust fmt/check/test, pytest+ffmpeg, version gate) + release `needs: validate` |
| Pre-existing test failure | FIXED | BiosManager + MusicPanel now use dialogService; 190→213 tests pass |

## Validation matrix (executed)

- `cargo fmt --check` PASS
- `cargo check` PASS (0 warnings)
- `cargo test` 43/43 PASS (unit + security + BIOS integration + real conversion)
- `pytest treefrog-manager/tests` 213/213 PASS (incl. security fixtures, archive fixtures, real-ffmpeg deploy pipeline)
- `npx tsc --noEmit` PASS
- `npm run build` PASS
- `npx tauri build` PASS → `treefrog-manager.exe` 19,652,096 B
- `--self-check` PASS (profile 1.1.0, 75 systems, ffprobe/ffmpeg available)
- GUI smoke: process alive, clean stop — PASS

## Remaining (genuinely deferred, documented — not hidden)

- Physical R36SX validation of a real deploy (CLASS C gate) — not executable in this host session.
- `video_presets.json` still declares `status: PROVISIONAL_UNVALIDATED` meaning "not hardware-validated on device" — accurate and surfaced; conversions themselves are executed+ffprobe-validated now.
- SQLite read-back for incremental scans (fingerprints written; nothing reads them yet — documented in `db.rs`).

## Discovered issues (original audit, kept for traceability)

### P0-SEC-1: BIOS parallel write logic in `deploy_to_sd` (lib.rs:368-534)
- Two separate BIOS copy loops join paths with `Path::join(entry.destination)` with NO canonical destination validation; only a weak `starts_with("cubegm/bios")` prefix check on a lowercased copy in the second loop. First loop has NO check at all.
- `STOCK_BIOS` hardcoded filename list duplicated 3× in lib.rs (lines 395, 495, 904) + separate `bios_catalog.rs` hardcoded list — 4 BIOS rule sources (bios.json, bios.rs, bios_catalog.rs, lib.rs).
- Affected: `src-tauri/src/lib.rs`
- Reproduction: `deploy_to_sd(bios_entries=[{destination: "../evil.bin"}])` writes outside SD.
- Fix: BIOS becomes normal PlanEntry flow through planner → resolve → validate → space → deploy.
- Regression: Rust test `bios_destination_escape_rejected` with the 6 malicious paths.

### P0-SEC-2: No canonical `resolve_validated_destination`
- `sd_target::validate_destination_path` validates a STRING only; `safe_copy_file` in deploy.rs reconstructs a "relative" by string-splitting on "roms/" — a destination like `roms/../../../evil` or archive member crafted paths can produce mismatch between validated string and written path. No final containment check against resolved SD root; no reserved-name ADS checks on backslash paths (backslash rejected only AFTER other checks — order OK but the function is string-only and never resolves).
- Affected: `sd_target.rs`, `deploy.rs` (safe_copy_file), user overrides in lib.rs (destination override joins `format!("{}/{}", new_folder, file_name)` with no validation).
- Fix: new `paths.rs` module with `resolve_validated_destination(sd_root, rel) -> Result<PathBuf, DestinationError>` + `validate_relative_destination(&str)`; used by deploy writer, planner final pass, user overrides, BIOS, archive extraction.
- Regression: security fixture tests (traversal/UNC/drive/ADS/reserved/empty/case).

### P1-1: `sd::detect()` claims writable=healthy=true (sd.rs:28)
- `writable = Some(true)` inferred from markers. Python `sd.py` same.
- Fix: tri-state `Option<bool>` (true/false/None=unknown) via non-destructive write probe; `healthy` only when accessible AND writable==Some(true).
- Regression: writable dir, read-only dir (Windows ACL), inaccessible, unknown.

### P1-2: `convert_then_copy` copies the ORIGINAL (deploy.rs:392-427)
- Warning says "PROVISIONAL_UNVALIDATED... would convert" while actually copying source unchanged. Silent fallback, violates state machine.
- Fix: real conversion pipeline in deploy: temp staging → ffmpeg → ffprobe validation → deploy converted output; failure/cancellation removes staging. Progress events: probing/converting/deploying. Planner reason no longer says "provisional".
- Regression: Rust + Python tests for compatible/incompatible codec/container/invalid output/ffmpeg missing.

### P1-3: No `effective_action()` — mixed action/resolved_action
- Space calc uses `e.action` first then adds resolved on top (double counting when resolution changes action from copy→skip: entry counts in BOTH to_copy and to_skip). Summary uses `action` only. Progress totals count only copy|extract|convert (missing replace).
- Fix: `pub fn effective_action(e)->&str` in lib.rs; use in deploy, space, summary, collisions, dry-run.
- Regression: space tests per resolution type (conflict→replace counts, skip_duplicate→copy counts).

### P1-4: keep_both always `_1` (planner.rs:202-221 + DryRunPreview.tsx:33-47 duplicates it)
- No uniqueness loop against FS/plan/case. Frontend reimplements rename.
- Fix: `next_available_destination()` in planner (Rust) checking: existing SD files, plan destinations (case-insensitive), returns `file_N.ext`; `apply_resolutions` moved to backend command `resolve_plan`; frontend becomes thin (collect choice → invoke → display).
- Regression: existing _1/_2, case collisions, multiple keep_both in same plan.

### P1-5: User destination overrides unvalidated (lib.rs:542-551)
- `entry.destination = format!("{}/{}", new_folder, file_name)` — new_folder from frontend map, never validated.
- Fix: validate via canonical validator; reject invalid override with error.

### P1-6: Archive 7z/RAR stubs claim handler exists (`get_handler_for_ext` returns "sevenz-stub"/"rar-stub")
- planner checks `handler.is_none() || ext == ".7z" || ".rar"` — works but "stub" naming is misleading; Cargo.toml comment claims fallback to external tools that doesn't exist.
- Fix: minimum-acceptable path — explicit `unsupported_archive` with precise reason (format not supported, ZIP only), remove stub strings, no extraction path bypasses (inspect_archive already rejects; keep).
- Regression: existing + new fixtures asserting unsupported reason text.

### P2-1: Overview counts nonsense (App.tsx:470-499)
- `Games: gamesFromPlan + (rom_dirs.length>0 ? existing_count : 0)` mixes planned entries with total SD file count; LGPT samples `* 100` fake multiplier.
- Fix: backend `content_counts` semantic counters in `analyze_target` (rom_count, music_track_count, video_count, image_count, ebook_count, bios_count, lgpt_sample_count, lgpt_project_count); UI displays directly.
- Regression: Rust test + Python fixture.

### P2-2: Version hardcoded in UpdateChecker.tsx (`const currentVersion = "0.1.0"`)
- Fix: `app_version()` command returning `env!("CARGO_PKG_VERSION")`; ensure Cargo.toml/package.json/tauri.conf.json agree via CI check; UpdateChecker + About use backend version.

### P2-3: stable_id uses label+filesystem+capacity (sd_target.rs:639-672)
- Fix: Windows volume GUID via `GetVolumeNameForVolumeMountPointW` + serial; fallback chain documented. Mount path stored separately (session state).
- Regression: stable_id unchanged across path change (simulate via fn on temp dirs), changes across different volume.

### P2-4: SQLite `db.rs` unused (no callers)
- Fix: minimal persistence layer: init at app start in `%APPDATA%`, migrations table, record deployments (job, job_entry, deployment) with hashes/sizes/versions/target identity. Keep scoped; document remainder.

### P2-5: Repo metadata: Cargo.toml `repository = lgpt-r36sx-port` (wrong), docs/AGENTS scope
- Fix: point to treefrog-content-manager where non-historical.

### CI: release.yml builds without validation gates
- Fix: add `validate.yml` (frontend build+tsc, cargo fmt --check/check/test, pytest, security fixtures) + make release depend on validation job.

### Pre-existing test failure (not ours yet)
- `test_phase2e_desktop_ux.py::test_source_picker_consistent_across_modules`: BiosManager uses raw `open()` from plugin-dialog instead of `dialogService.pickFile`. Fix by migrating BiosManager to dialogService.

## State machine invariants to enforce (Section 19)
- PREVIEW must not write (dry_run_* commands are read-only — verify no side effects).
- UNKNOWN never written (deploy skip guard exists; keep + test).
- UNSUPPORTED never copied as supported.
- CONVERSION_REQUIRED must produce validated output (P1-2 fix).
- BIOS same rules as all content (P0-SEC-1 fix).
