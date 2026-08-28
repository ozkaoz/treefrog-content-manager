# TreeFrog Content Manager — Development Plan

**Version:** 1.1.0  
**Date:** 2026-08-28  
**Scope:** Global TreeFrogUI SD-card content manager (desktop) — not per-device fork  
**Stack:** Tauri 2 + Rust backend + React + TypeScript frontend + SQLite + serde + versioned declarative JSON profiles 1.1.0 + SHA-256 + FFmpeg/ffprobe adapter + maintained archive libs (ZIP implemented, 7z/RAR stubs)

Windows is first supported desktop platform; core filesystem layer portable for later macOS/Linux.

> Evidence hierarchy per AGENTS.md: explicit user requirement > AGENTS invariants > ACTIVE DECISIONS > direct evidence (Git/filesystem/device/release) > CURRENT snapshot > CONTEXT_MAP/docs/ai > history.

---

## Product Invariants

- Global TreeFrogUI schema — do NOT fork into R36SX/SF3000/R36HD-specific ROM managers. Device-specific limited to SD detection/markers, optional capability/validation, and evidenced release differences.
- Folder mappings, media destinations, BIOS rules live in declarative profiles (`profiles/treefrogui/`), never hardcoded in UI code.
- Artwork: Mini Scraper remains external solution (`https://github.com/tzubertowski/mini-scraper-cfw/releases`). App provides integration button/open action and optional `.res` verification; no second artwork backend.
- BIOS files user-supplied only; never download/bundle copyrighted BIOS.
- Video compatibility: ffprobe inspection required; auto-convert via FFmpeg when incompatible; provisional conservative preset marked `PROVISIONAL_UNVALIDATED` until physical R36SX validation.
- Music: preserve subfolders under `roms/music` — each folder is a TreeFrogUI playlist.
- LGPT samples → `lgpt/samples`, projects → `lgpt/projects` (verified against Bacon-1.5 payload `sd_root/lgpt/*`).
- Sync: dry-run plan before destructive writes (`2,331 unchanged / 34 new / 12 changed / 7 duplicate / 3 conflicts / 0 deletions`); normal Sync never deletes; deletion explicit; staging + atomic rename; resume/consistent on interrupt.
- Persistent index: SQLite (or robust local) for source libraries, known SD targets, fingerprints, deployments, profile version, tool version, job history. Never commit user paths.
- Safety: prevent `../` traversal, absolute paths, symlink/reparse hazards, name collisions, expansion limits, never overwrite different file silently.

---

## Phases

### Phase 0 — Bootstrap (done)

- [x] Evidence: inspect live upstream TreeFrogUI (`tzubertowski/treefrog-ui` main, cores.md, docs/cores/*, docs/standalone-apps.md, sdcard) and LGPT R36SX Bacon-1.5 payload (`sd_root/`, LGPT_R36SX_Bacon-1.5_SHA256SUMS.txt)
- [x] Profile loader: versioned declarative JSON 1.0.0 under `profiles/treefrogui/` (manifest, profile, systems, media, bios, lgpt, video_presets, sd_markers) with serde + Python mirror
- [x] App shell: Tauri 2 scaffold (`treefrog-manager/`), Rust backend, React+TS frontend, SQLite placeholder, CI stub, agent contract tests
- [x] Fixtures: archive samples, duplicate sets, media samples, BIOS patterns, LGPT sample/project samples
- [x] CURRENT/CONTEXT_MAP/DECISIONS preservation per AGENTS.md contracts
- **Gate:** `scripts/agent_preflight.sh --allow-dirty` PASS, `test_agent_context_contract.py PASS`, `test_release_audio_bootstrap.py PASS`, profile JSON validation PASS, `git diff -- sd_root` = NO

### Phase 1 — Scanner + Classification + Archive + Duplicate + Dry-Run Planner (done)

> **Milestone:** Select source folder + select TreeFrogUI SD + scan + preview exactly what would be copied/extracted/skipped/conflicted, without writing anything.

- [x] Scanner: recursive arbitrary source libraries; classify by profile + extension/content hints (not filenames alone); multi-file groups (CUE/BIN, CHD, m3u, baseq2, pico286 img sets) preserved as groups
- [x] Archive inspection (Phase 1 baseline): recognize ZIP, inspect entries, heuristic payload vs extract, bounded nested-archive policy, safety (traversal/absolute/symlink/collisions/limits/never silent overwrite) — extended in Phase 2A
- [x] Duplicate engine: SHA-256 exact identity; cheap metadata first; classify: same path+same hash unchanged, different path+same hash duplicate skip, same path+different hash conflict, new path+new hash copy; never delete source
- [x] Planner: dry-run preview table with counts (`unchanged / new / changed / duplicate_content / conflicts / deletions=0`) and per-item action (copy/extract/skip/conflict)
- [x] SD reader: read-only scan of destination for fingerprint comparison (no writes)
- **Tests:** `test_scanner_classification.py`, `test_archive_inspection.py`, `test_duplicate_engine.py`, `test_dry_run_planner.py` + fixtures under `treefrog-manager/tests/fixtures/` — **53 tests PASS (31+22 Phase 2A)**
- **Gate:** targeted unit/integration PASS, fixture tests PASS, CURRENT updated only for mutable state, no SD mutation

### Phase 2A — Archive ingestion and safe temp extraction (current, no SD writes)

> **Objective:** Inspect compressed sources and decide (copy as-is / safely extract in temp workspace / grouped multi-file game / rejected / manual_review / unsupported_archive) with profile-driven policy; planner operates on logical units, zero-write.

- [x] Extend profile to 1.1.0: `archive_policy.json` (handlers `.zip` implemented + `.7z`/`.rar` stubs, safety, modes `payload/container/extract_and_classify/grouped/manual/unsupported`, per_system overrides for arcade `cps1/neogeo/m2k` → payload, `ps/segacd` → grouped, grouping rules for CUE/BIN)
- [x] Archive abstraction `ArchiveHandler` (Rust trait / Python registry `HANDLERS`): `ZipHandler` via `zip` crate, `SevenZ`/`Rar` stubs return `unsupported_archive` without rewriting planner; deterministic dispatch via `archive_policy.handlers`
- [x] Safety: traversal `../`, absolute `/`, Windows drive-letter `C:/`, symlink `0o120000`, hardlink/ADS `:`, collisions normalized lowercased, expansion 1GiB, member count 1024, nested depth 1, per-job 10k, compression ratio, all extraction in `tempfile::TempDir` via `safe_extract_to_temp` (never to SD, never overwrites source)
- [x] Profile-driven decision `decide_archive_mode`: early grouped detection for CUE/BIN in same folder, nested bomb → manual, no known inner → payload, single system → per_system mode, mixed → extract_and_classify (not manual), grouped hint → grouped
- [x] Planner logical units: `group_members` for CUE/BIN (same folder, stem match), planner creates one entry per logical game with `group` members, temp-hashes inner content for duplicate detection (duplicate archive vs duplicate extracted payload not double-counted), collision detection among archive members, deterministic sorting, `manual_review`/`unsupported_archive` actions, per-job limits
- [x] Duplicate: SHA-256 exact with grouped payload combined hash (sorted member hashes), distinguish identical vs same filename diff content vs grouped identical vs duplicate container vs duplicate payload
- **Tests:** `test_phase2a_archive_ingestion.py` 22 tests covering valid ZIP, nested dirs, traversal, absolute, drive-letter, symlink, hardlink/ADS, collision, expansion, member count, payload, container, grouped CUE/BIN, duplicate archive, duplicate extracted, nested bomb, unsupported (7z/rar), deterministic, temp workspace guard, no overwrite, profile-driven — **all run without SD**
- **Gate:** `pytest treefrog-manager/tests` 53 PASS, `test_agent_context_contract PASS`, `preflight PASS`, `git diff -- sd_root` empty, zero-write

### Phase 2B — Duplicate & Conflict Resolution (done, zero SD writes)

> **Objective:** Deterministic duplicate/conflict layer on top of scanner/archive/logical-unit planner + SHA-256 engine; planner single source of truth; UI shows hashes/members/resolution.

- [x] Planner distinguishes: exact duplicate (same hash) → `skip_duplicate` (default `skip`), same filename different content → `conflict` (default `conflict`), different filename identical → `duplicate`/`alias` → `skip_duplicate`, grouped identical → `skip_duplicate` with combined hash, archive-vs-extracted → not double-counted via temp-hashed inner content, unchanged → `skip_unchanged`; all via SHA-256 exact, cheap metadata first.
- [x] Extend planner model: entries carry `source`, `destination`, `content_type`, `source_hash`, `destination_hash`, `reason`, `members`/`group`, `default_action`, `resolution`, `resolved_action`, `original_destination`; UI can explain.
- [x] Explicit resolutions: `skip`, `replace`, `keep_both` (renamed `_1`), `keep_destination`, `keep_source`; defaults overrideable via `apply_resolutions(plan, decisions)`; never silently replace; `replace`/`keep_source` → `replace`, `keep_both` → renamed `copy`/`extract`, `keep_destination`/`skip` → `skip`.
- [x] Determinism: stable-sort `source`/`destination`, sorted members, sorted group hashes; `plan(scanned, sd_root, profile)` deterministic.
- [x] Planner / execution boundary: `planner` remains single source; future SD writers must execute `resolved_action`/`destination` from `apply_resolutions` output, not recompute.
- [x] UI: `App.tsx` + `DryRunPreview.tsx` show `status/action`, `source`, `destination`, `reason`, `source_hash`/`destination_hash` (16 chars), `content_type`, `members`, per-entry `<select>` for 5 resolutions, read-only (no SD writes).
- **Tests:** `test_phase2b_duplicate_resolution.py` 13 tests covering identical, alias, conflict, grouped, archive vs extracted, unchanged, explicit replace/keep_destination/keep_both/skip, deterministic, metadata, zero SD writes — **all run without SD, zero writes**.
- **Gate:** `pytest 66 PASS` (53+13), `context-contract PASS`, `preflight PASS`, `git diff -- sd_root` empty

### Phase 2C — SD Detection + Sync Execution + Progress + Resume (next)

- SD detection via markers (`cubegm/` + `roms/` etc profile `sd_markers.json`), optional capability checks, mount health/writable probes, filesystem portable layer
- Sync execution: staging + atomic rename where supported, progress events, cancellable without corrupt finals, conflict handling via `resolved_action` from planner (skip/replace/keep_both), resume/consistent state on interrupt
- Normal Sync must not delete destination files; deletion explicit separate action
- SQLite deployments/history + tool/profile versioning
- **Gate:** integration tests with temp SD fixtures, interruption tests, `git diff -- sd_root` still NO for manager ops outside target

### Phase 3 — Music / Images / Ebooks + Video Probe/Conversion

- Music/images/ebooks: use profile media destinations/formats; preserve music subfolders; MuPDF ebook per-book `.positions` ignore
- Video: ffprobe adapter (container, video codec, profile/level, pix_fmt/bitdepth, dimensions, fps, audio codec, stream count), compatible → copy, incompatible → auto-convert via FFmpeg with temp staging, re-probe, validate, batch + cancel, declarative presets (conservative default `PROVISIONAL_UNVALIDATED`)
- **Gate:** ffprobe mock + fixture videos, conversion staging tests, no claim of hardware compatibility without physical evidence

### Phase 4 — BIOS Manager

- Profile defines system/core, destination, accepted patterns, expected size/hash when known, required/recommended, region variants (profile `bios.json`)
- UI: discover, verify, import, replace, backup current, reveal destination
- BIOS user-supplied only
- **Gate:** BIOS verification tests (size/hash/pattern), import/replace with backup tests

### Phase 5 — LGPT Samples/Projects

- Global LGPT profile (`lgpt.json`): samples → `lgpt/samples`, projects → `lgpt/projects` (verified against Bacon release); do not hardcode in UI; projects as groups/directories, samples with audio metadata/preview + exact duplicate detection
- **Gate:** LGPT group copy tests, sample preview/metadata tests

### Phase 6 — Mini Scraper Launcher + Artwork Verification

- External tool launcher button/open action via Tauri opener; optional verification of `.res` artwork after scrape (`.res/<rom>.png`, also `Imgs/`, `images/` compat, title/screen suffixes `-title/-titlescreen/-screenshot/-screen/-preview`)
- No second scraper/backend
- **Gate:** launcher opener test (mock), `.res` verification fixture tests

### Phase 7 — Hardening, Packaging, Performance, Release QA

- Large-library performance (10k+ files, hash caching, parallel scanning where safe)
- Packaging: Windows installer (Tauri bundler), portable fs layer, SQLite migrations
- Release QA: full dry-run + sync + BIOS + LGPT + video matrix with fixtures; never claim hardware/video compatibility without physical evidence; never claim clean release without release gates
- **Gate:** `BUILD PASS`, `HOST PASS`, `PACKAGING PASS`, performance fixture PASS, `CLEAN-INSTALL PHYSICAL PASS` not inferrable (requires device), so label `PHYSICAL PASS` only with evidence

---

## Quality Gates (per AGENTS.md §4 + docs/ai/VALIDATION.md)

For every meaningful change:

- add or update tests
- run formatter/linter/build
- run targeted unit/integration tests
- run relevant fixture tests
- update `CURRENT.md` only for mutable state
- update `DECISIONS.md` only for durable decisions
- update `CONTEXT_MAP.md` when subsystem routing changes
- never claim hardware compatibility without physical evidence; never claim video preset compatibility without target-device validation; never claim clean release without release validation gates

Derive validation class from `docs/ai/VALIDATION.md`: Content Manager bootstrap is CLASS B (host tooling) for Phase 0/1 until it touches runtime/deployment; LGPT-related deployment would be CLASS D etc. Keep `sd_root` untouched for manager-only changes (`SD_ROOT_CHANGED=NO`).

---

## Repository Work Required

- `AGENTS.md` (preserve constitution) — no hardcode of mutable branch/HEAD/SHA
- `CURRENT.md` (snapshot ≤150 lines, HEAD RESOLVE FROM GIT)
- `CONTEXT_MAP.md` (router, no mutable state; add TreeFrog Content Manager row)
- `DECISIONS.md` (durable only; add content-manager global-profile/archive-safety/duplicate decisions)
- `.opencode/agents/{audit,implement,review,release}.md` (existing)
- `profiles/treefrogui/` (versioned declarative JSON profiles)
- `tests/fixtures` + `treefrog-manager/tests` for archives/duplicates/media/BIOS/LGPT/video
- `treefrog-manager/` (Tauri 2 + Rust + React + TS + SQLite shell)

---

## Risk Log

- Dirty exFAT SD health mimics manager failures — always probe mount health before blaming logic (AGENTS §6, sd_markers.json)
- Archive symlink/reparse hazards — treat symlinks as unsafe entries; skip + warn
- Video preset must remain PROVISIONAL_UNVALIDATED until physical R36SX/SF3000 family probe (hardware decoder variance)
- Large libraries: hashing + scanning must be incremental + cached via SQLite fingerprints

---

## Next Exact Action (2026-08-28)

- Phase 2B complete: deterministic duplicate/conflict resolution with explicit overrideable decisions — next is Phase 2C SD detection + sync execution (staging, progress, resume) — not in this task. Run `python3 -m pytest treefrog-manager/tests -v` + `bash scripts/agent_preflight.sh --allow-dirty` to verify.

## Unresolved for real-device validation

- Arcade payload `.zip` handling (cps1/neogeo/m2k) is profile-driven as `payload` per `archive_policy.json` but has not been physically validated on R36SX/SF3000 hardware that those cores require the zip to stay compressed. The same for 7z payload when handler becomes available.
- Video preset remains `PROVISIONAL_UNVALIDATED`; no claim of hardware decoder support without device probe.
- Large-library performance (10k+ files, hash caching) not yet exercised on real SD; per-job limit 10k is conservative.
