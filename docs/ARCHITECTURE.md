# TreeFrog Content Manager — Architecture

**Version:** 1.1.0  
**Date:** 2026-08-28  
**Scope:** Phase BIOS-B BIOS Manager UI + planner integration (global TreeFrogUI, no SD writes) + Phase 2C video/desktop

---

## 1. Overview

TreeFrog Content Manager is a **global TreeFrogUI content manager**, not a per-device fork. One declarative profile schema covers all handhelds (R36SX/SF3000/GB350 etc). Device-specific logic is limited to SD detection/markers and optional capability checks, per `AGENTS.md:3` and `DEC-2026-08-28-01`.

Stack: Tauri 2 + Rust backend + React TS frontend + SQLite + serde versioned JSON profiles 1.1.0 + SHA-256 + FFmpeg/ffprobe adapter + archive handlers.

Filesystem layer is portable; Windows first.

Invariant: **no SD writes in Phase 0-BIOS-A**. All archive/BIOS/video work and duplicate checks happen in memory/temp (`tempfile::TempDir` / `SHA-256`), never to SD, never overwriting source, never silently replacing conflicting content. Desktop build is part of Definition of Done from Phase 2C onward.

---

## 2. Profiles — declarative, versioned, serde + Python mirror

Location: `profiles/treefrogui/`

- `manifest.json` 1.1.0 — lists 8 profile files, now includes `archive_policy.json`
- `profile.json` 1.1.0 — global invariants, `archive_policy` points to `archive_policy.json` with `abstraction: ArchiveHandler trait`
- `systems.json` 1.1.0 — 75 systems, `archive_payload_valid` preserved for backward compat
- `archive_policy.json` 1.1.0 — **new Phase 2A**: handlers, safety limits, modes, per_system, grouping

`archive_policy.json` structure:

```json
{
  "handlers": { ".zip": {"implemented": true}, ".7z": {"implemented": false, "stub_action": "unsupported_archive"} },
  "safety": { "limits": {"max_entries":1024, "max_expansion_bytes":1073741824, "max_total_files_per_job":10000, "max_depth":1} },
  "modes": { "payload": {"action":"copy"}, "extract_and_classify": {"action":"extract"}, "grouped": {...}, "manual": {"action":"manual_review"}, "unsupported": {"action":"unsupported_archive"} },
  "per_system": { "cps1": {".zip":"payload"}, "ps_psx": {".zip":"grouped"} },
  "grouping_rules": { "cue_bin": {...}, "collision": {...}, "nested_archive": {"max_depth":1} }
}
```

Loaded in Rust `profile.rs:LoadedProfile { archive_policy_full: Value }` and Python `profile.py:load_archive_policy()`. `serde` in Rust, `json` in Python — same files, no hardcoding in UI.

---

## 3. Archive abstraction

### 3.1 Trait / registry

**Rust:** `archive.rs:ArchiveHandler` concept via `inspect_archive` dispatcher and `get_handler_for_ext`. `ZipHandler` implemented with `zip` crate. `SevenZ`/`Rar` are stubs: `inspect_archive` returns `ArchiveError::Unsupported` → planner maps to `unsupported_archive`. When `sevenz-rust` becomes available, only `handlers[".7z"].implemented = true` and the match arm changes; planner does not need rewrite.

**Python:** `archive.py:HANDLERS = {".zip": {"implemented": True}, ".7z": {"implemented": False}}`, `inspect_archive` dispatches, `inspect_zip` is the real implementation.

This satisfies the requirement: *Supported archive formats should be determined by the available runtime/tooling. Design so additional formats can be added without rewriting the planner.*

### 3.2 Safety

All extraction must defend against:

- `../` traversal → check `Path.components()` for `ParentDir`
- absolute `/foo` → `name.startswith('/')` or `PurePath.is_absolute()`
- Windows drive-letter `C:/`, `C:\`, `C:`, `\\?\`, `\\server\share` → regex `^[A-Za-z]:` and UNC check
- symlink → `entry.is_symlink()` and `unix_mode() == 0o120000`
- hardlink-like / ADS → `mode not in (0o100000,0o040000,0o120000)` and `':' in name && !drive`
- collisions → normalized lowercased `dest.replace('\\','/').lower()` map; duplicate normalized path → `CollisionError` → `manual_review`
- excessive expansion / member count / compression ratio → sum `file_size` vs `max_expansion_bytes`, `len(infolist) > max_entries`, `file_size/compress_size > 100` for large files
- nested bombs → inner entry is `.zip/.7z/.rar` and `depth > max_depth` (1) → `manual_review`
- writing outside temp → `dest.starts_with(temp_dir)` and `dest.resolve().relative_to(temp_dir.resolve())`

`inspect_zip` validates before any extraction. `safe_extract_to_temp(archive_path, temp_dir)` extracts only to `temp_dir`, creates parents, checks `dest.exists()` collision, and never overwrites source (`dest != archive_path`). `safe_join` guards SD destination construction.

### 3.3 Determinism

Planner sorts `scanned` by `source_path` and archive inner entries are processed in archive order but final `entries` are sorted by `source` then `destination`. This ensures deterministic preview.

---

## 4. Scanner and classification

`scanner.rs` / `scanner.py`: recursive `WalkDir` / `Path.rglob`, `follow_links=false`, `is_symlink` skip, classify via `classify.rs`/`classify.py` (profile `ext_to_system` + media/bios heuristics, plus `.cue` grouping for loose files). Multi-file loose groups (CUE/BIN) are coalesced to one `ScannedFile` with `group_members`.

`classify.rs` recognizes `.zip/.7z/.rar` as `Kind::Archive` first, then media, then ROM via profile. No hardcoding of system payload vs container there; that decision is deferred to planner's `decide_archive_mode` which reads `archive_policy.json`.

---

## 5. Planner — logical units, duplicate/conflict resolution, zero-write, single source of truth

`planner.rs` / `planner.py`: read-only, zero-write to SD. Input: `Vec<ScannedFile>` + `sd_root` + `profile`. Output: `Plan { summary, entries, warnings }` where planner is **single source of truth**; future SD writers must execute `resolved_action`/`destination` from `apply_resolutions` output, not recompute.

**Summary:** `unchanged, new, changed, duplicate_content, conflicts, deletions=0, manual_review, unsupported_archive` plus `resolved_summary` after `apply_resolutions`.

**Entry actions (default):** `copy, extract, skip_duplicate, skip_unchanged, conflict, manual_review, unsupported_archive`
**Resolved actions after explicit decision:** `skip, copy, extract, replace, conflict, manual_review` (via `skip/replace/keep_both/keep_destination/keep_source`).

**Entry metadata for UI (Phase 2B):** `source`, `destination`, `action`, `reason`, `hash` (legacy), `source_hash`, `destination_hash`, `content_type` (`rom/GBA`, `grouped/CUE_BBIN`, `archive-payload`, `music`…), `size`, `group`/`members` (logical-unit members), `default_action`, `resolution`, `resolved_action`, `original_destination` (for `keep_both`).

**Flow for archive (as in 2A):** handler check → `unsupported_archive`; `inspect_archive` safety → `manual_review`; per-job limit → `manual_review`; `decide_archive_mode` profile-driven (payload/grouped/extract/manual); for `payload` hash archive; for `extract`/`grouped` temp-hash inner content, check `sd_hash_map`/`hash_to_dest` for duplicate extracted payload, `dest.exists()` for `skip_unchanged`/`conflict`, collisions via `detect_collisions`.

**Duplicate handling (SHA-256 exact, Phase 2B):**

- Identical content (same logical bytes, any path) → `skip_duplicate` (default `skip`)
- Same filename different SHA → `conflict` (default `conflict`, needs explicit `replace`/`keep_both`/`keep_destination`/`skip`)
- Different filename identical SHA → `duplicate`/`alias` → `skip_duplicate` (default `skip`, overrideable to `keep_both`)
- Grouped logical unit identical → combined `SHA-256(sorted member hashes)` compared → `skip_duplicate` (e.g., two identical CUE/BIN zips)
- Archive container vs extracted payload → planner temp-hashes inner `inner.gba` and compares to loose `game.gba`; identical logical content → `skip_duplicate`, not two independent copies.
- Unchanged target (source and destination already identical SHA) → `skip_unchanged`.

All `hash_to_dest` and `sd_hash_map` are keyed by SHA-256, not filename. Cheap metadata (size) first, then SHA-256 only when needed. Deterministic stable-sort `source` → `destination`, sorted members, sorted group hashes.

**Resolution model (explicit, overrideable):** `VALID_RESOLUTIONS = {skip, replace, keep_both, keep_destination, keep_source}`; `_default_resolution_for_action` maps `skip_duplicate→skip`, `conflict→conflict`, etc.; `apply_resolutions(plan, decisions)` (where `decisions: {index|source|destination|source->destination: resolution}`) returns new plan with `resolved_action`/`destination` (for `keep_both` adds `_1` suffix) and `reason` annotated `[resolved: …]`, without recomputing classification — planner remains single source.

**Temp workspace:** every `safe_extract_to_temp` call uses a fresh `TempDir` that is dropped after hashing. No file is ever written to `sd_path`.

---

## 5b. BIOS — formal TreeFrogUI-global profile and validation

`profiles/treefrogui/bios.json` 1.1.0 `bios_definitions` — each definition: `id` (`ps1_bios`), `system_id` (`psx`), `name`, `required` (`conditional`/`optional`/`required`), `requirement {scope, mandatory_when}` (e.g., `psx_content_present`), `variants` (any one satisfies, e.g., PS1 `scph1001`/`scph5501`/`scph5500`, GBA single `gba_bios.bin` with SHA-256 `a860a8...`, O2EM `o2rom.bin` 1024 + MD5, neogeo `neogeo.zip` payload, segacd 3 region variants), `accepted_filenames`/`accepted_patterns`/`aliases` (fnmatch, case-insensitive), `destinations`/`primary_destination` (profile-driven, not hardcoded `cubegm/bios`), `expected_size`, `hashes_sha256` (authoritative only, no invented; PS1 empty, GBA single, others empty), `expected_md5` where needed, `archive {mode: payload}` (reuse Phase 2A `ArchiveHandler`, payload for `neogeo.zip`, container via temp extract, unsupported for 7z), `verification {states, matching_order}`. Global `verification_states` = `missing`, `found_valid`, `found_invalid`, `found_unknown`, `duplicate`, `conflict`, `not_required`.

Matching order: 1 exact filename + known hash, 2 alias + known hash, 3 filename + size when no hash, 4 known filename wrong hash → `found_invalid`, 5 unknown → `found_unknown`; filename alone never validates when hash exists. `get_valid_destinations()` returns profile-driven destinations. Hashing reuses `hash::sha256_file` and `archive::inspect_*`/`safe_extract_to_temp` (temp workspace, no SD). `validate_bios_file` and `validate_all_bios(source_files, definitions, system_content_present)` are deterministic, profile-driven, handle multiple variants (any one `found_valid` satisfies), conditional `not_required` vs `missing`, duplicate (same hash different path) vs conflict (same filename different hash). No invented hashes; only GBA SHA-256 and O2ROM MD5 are authoritative.

---

## 6. Why profile-driven instead of globally extracting every archive

*Decision:* `DEC-2026-08-28-02`

Some TreeFrog content **must** remain compressed. Arcade cores (`cps1/cps2/cps3/neogeo/m2k`) are validated as `.zip` payloads; their cores open the zip directly and expect the romset's internal layout (often dozens of `.rom` blobs with specific CRCs). Extracting such a zip would break core loading and scatter opaque blobs into `roms/cps1/` where the core would not find them.

Other systems (NES/SNES/GBA) traditionally accept both loose and `.zip`, but a user may want to preserve a curated `.7z` set as-is.

If the scanner hardcoded `ZIP => always extract`, it would be wrong for arcade, not portable across devices, and would require a planner rewrite when 7z/RAR tooling arrives. Profile-driven keeps scanner generic (`Kind::Archive`), planner deterministic, and allows per-system tuning (adding a new system or changing `cps1` from `payload` to `extract_and_classify` for a future core) by editing `archive_policy.json` alone, without code change, as demonstrated by adding `sevenz-rust` later only needs flipping `implemented:true`.

See `archive_policy.json:rationale`.

---

## 7. Testing

`treefrog-manager/tests/` 94 tests:

- 31 original: profile_loader, scanner_classification, archive_inspection, duplicate_engine, dry_run_planner, sd_detection, bios_and_lgpt
- 22 Phase 2A (`test_phase2a_archive_ingestion.py`): valid ZIP, nested dirs, traversal, absolute, drive-letter, symlink, hardlink/ADS colon, collision, expansion limit, member count limit, payload, container, grouped CUE/BIN, duplicate archive, duplicate extracted, nested bomb, unsupported (7z/rar), deterministic, temp workspace guard, no overwrite, profile-driven
- 13 Phase 2B (`test_phase2b_duplicate_resolution.py`): identical loose files, identical different filenames, same filename diff content, grouped CUE/BIN duplicates, archive vs extracted duplicates, destination unchanged, explicit replace/keep_destination/keep_both/skip, deterministic, collision/resolution metadata, zero SD writes
- 17 Phase BIOS-A (`test_bios_validation.py`): valid by filename+hash, alias+hash, invalid wrong hash, size-only, unknown, missing, duplicate identical, conflict same filename diff content, multiple variants, conditional triggered/not required, archive payload/container/unsupported, deterministic, schema, no invented hashes

All run without SD (`tempfile.TemporaryDirectory` for source and fake SD with `cubegm/`+`roms/` markers). No test writes to real SD.

`test_agent_context_contract.py` and `test_release_audio_bootstrap.py` remain PASS.

---

## 8. Desktop build (Definition of Done) + Planner as single source

**Desktop (from Phase 2C):** Windows x64 via Tauri 2 + Rust stable + Node 20 + MSVC + WebView2 + FFmpeg/ffprobe. Reproducible via `scripts/build_windows.ps1` (PowerShell) and `scripts/build_windows.sh` (WSL wrapper documenting cross-compile limitation). Artifacts `treefrog-manager.exe` + MSI/NSIS, `--self-check` verifies profile 1.1.0/75, video provisional, ffmpeg, dry-run. No milestone complete without tested desktop build (AGENTS.md §15).

**Planner as single source of truth**

**Rule:** *The deployment planner is the single source of truth for content decisions; future SD writers must execute its output rather than independently reclassifying content.*

`plan()` is deterministic and metadata-rich; `apply_resolutions()` only maps explicit user decisions to `resolved_action`/`destination` without re-running classification. No second competing decision system exists. The dry-run preview and the future `sync` command share the same `Plan` type (`lib.rs:Plan`, `App.tsx:Plan`). This prevents divergence where preview says `skip_duplicate` but writer would `copy`.

---

## 9. Git discipline and next

- `sd_root/` untouched (`git diff -- sd_root` empty) — confirmed for BIOS-A
- Content manager repo independent from LGPT runtime payload
- Phase BIOS-A is backend/profile validation only (no SD writes, no BIOS UI, no 7z/RAR, no video conversion). Next is Phase BIOS-B UI or Phase 2C SD writes (not in this task).

## 10. BIOS is TreeFrogUI-global, not R36SX-specific

- `bios.json` is TreeFrogUI-global; R36SX may later provide device override, but BIOS model remains TreeFrogUI-global per `DEC-2026-08-28-04`.
- Destinations are profile-driven; no `cubegm/bios` hardcoding in `bios.rs`/`bios.py`.
- Reuses existing `archive`/`hash`/`planner` services; no second extraction/hash engine.
- Video preset remains `PROVISIONAL_UNVALIDATED`; conversion never modifies source; planner remains single source.

---

## 11. Unresolved requiring real-device validation

- Arcade `.zip` payload handling is profile-driven as `payload` but not physically validated on R36SX that those cores require the zip to stay compressed (plausible per upstream docs/cores/arcade.md, but needs device).
- 7z/RAR payload handling when handlers become available.
- Video preset still `PROVISIONAL_UNVALIDATED`.
- BIOS `PROVISIONAL` validation (no hardware test of actual BIOS on R36SX; hashes are authoritative only for GBA).

