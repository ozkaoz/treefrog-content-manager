# TreeFrog Content Manager — Architecture

**Version:** 1.1.0  
**Date:** 2026-08-29  
**Scope:** Phase 2A archive ingestion and safe temp extraction (zero SD writes) — read-only planner architecture

---

## 1. Overview

TreeFrog Content Manager is a **global TreeFrogUI content manager**, not a per-device fork. One declarative profile schema covers all handhelds (R36SX/SF3000/GB350 etc). Device-specific logic is limited to SD detection/markers and optional capability checks, per `AGENTS.md:3` and `DEC-2026-08-28-01`.

Stack: Tauri 2 + Rust backend + React TS frontend + SQLite + serde versioned JSON profiles 1.1.0 + SHA-256 + FFmpeg/ffprobe adapter + archive handlers.

Filesystem layer is portable; Windows first.

Invariant: **no SD writes in Phase 0-2A**. All archive work happens in a temporary workspace (`tempfile::TempDir` / `tempfile.TemporaryDirectory`), never to SD, never overwriting source.

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

## 5. Planner — logical units, zero-write

`planner.rs` / `planner.py`: read-only, zero-write to SD. Input: `Vec<ScannedFile>` + `sd_root` + `profile`. Output: `Plan { summary, entries, warnings }`.

**Summary:** `unchanged, new, changed, duplicate_content, conflicts, deletions=0, manual_review, unsupported_archive`

**Entry actions:** `copy, extract, skip_duplicate, skip_unchanged, conflict, manual_review, unsupported_archive`

**Flow for archive:**

1. Check handler availability → `unsupported_archive` if stub.
2. `inspect_archive` with limits → `SafetyViolation/Collision/NestedBomb` → `manual_review` (not `conflict`).
3. Per-job `max_total_files_per_job` guard → `manual_review`.
4. `decide_archive_mode` (profile-driven):
   - `payload` → copy intact (hash archive file itself)
   - `grouped` → `group_members` (CUE/BIN in same folder, stem match), one logical entry per group
   - `extract_and_classify` / `container` → each member individually, but opportunistic CUE/BIN grouping if both present
   - `manual` / `unsupported` → respective actions
   - Mixed systems: **not** manual; mixed `GBA+SFC` in same zip extracts each to its correct system folder. Only `nested` or `collision` or `unknown` triggers `manual`.
5. For `payload`: copy intact, duplicate check via `sha256(archive)` against `sd_hash_map` and `hash_to_dest`.
6. For `extract`/`grouped`: for each logical group, `safe_extract_to_temp` to temp, hash inner content (`sha256(file)` or combined `sha256(sorted member hashes)` for groups), check `sd_hash_map` and `hash_to_dest` for *duplicate extracted payload* vs `duplicate archive`, check `dest.exists()` for `skip_unchanged`/`conflict` via `inner_hash == dst_hash`, detect collisions among archive members via `detect_collisions`.

**Duplicate handling (SHA-256 exact):**

- Identical content (same bytes, different path) → `skip_duplicate`
- Same filename different content → `conflict` (if same path) or `copy` (if different path but different hash, it's still new)
- Grouped payload identical → combined hash of sorted member hashes compared
- Duplicate archive vs duplicate extracted payload → planner temp-hashes inner content, so a loose `game.gba` and a zip containing identical `inner.gba` are recognized as `skip_duplicate`, not two independent copies. *We do not silently classify container and its extracted file as two independent copies.*

All `hash_to_dest` and `sd_hash_map` are keyed by SHA-256, not filename. Cheap metadata (size) first, then SHA-256 only when needed.

**Temp workspace:** every `safe_extract_to_temp` call uses a fresh `TempDir` that is dropped after hashing. No file is ever written to `sd_path`.

---

## 6. Why profile-driven instead of globally extracting every archive

*Decision:* `DEC-2026-08-29-01`

Some TreeFrog content **must** remain compressed. Arcade cores (`cps1/cps2/cps3/neogeo/m2k`) are validated as `.zip` payloads; their cores open the zip directly and expect the romset's internal layout (often dozens of `.rom` blobs with specific CRCs). Extracting such a zip would break core loading and scatter opaque blobs into `roms/cps1/` where the core would not find them.

Other systems (NES/SNES/GBA) traditionally accept both loose and `.zip`, but a user may want to preserve a curated `.7z` set as-is.

If the scanner hardcoded `ZIP => always extract`, it would be wrong for arcade, not portable across devices, and would require a planner rewrite when 7z/RAR tooling arrives. Profile-driven keeps scanner generic (`Kind::Archive`), planner deterministic, and allows per-system tuning (adding a new system or changing `cps1` from `payload` to `extract_and_classify` for a future core) by editing `archive_policy.json` alone, without code change, as demonstrated by adding `sevenz-rust` later only needs flipping `implemented:true`.

See `archive_policy.json:rationale`.

---

## 7. Testing

`treefrog-manager/tests/` 53 tests:

- 31 original: profile_loader, scanner_classification, archive_inspection, duplicate_engine, dry_run_planner, sd_detection, bios_and_lgpt
- 22 new Phase 2A (`test_phase2a_archive_ingestion.py`): valid ZIP, nested dirs, traversal, absolute, drive-letter, symlink, hardlink/ADS colon, collision, expansion limit, member count limit, payload, container, grouped CUE/BIN, duplicate archive, duplicate extracted, nested bomb, unsupported (7z/rar), deterministic, temp workspace guard, no overwrite, profile-driven

All run without SD (`tempfile.TemporaryDirectory` for source and fake SD with `cubegm/`+`roms/` markers). No test writes to real SD.

`test_agent_context_contract.py` and `test_release_audio_bootstrap.py` remain PASS.

---

## 8. Git discipline and next

- `sd_root/` untouched (`git diff -- sd_root` empty)
- Content manager repo independent from LGPT runtime payload
- Phase 2B (SD detection, sync execution with staging, progress, conflict handling, resume, SQLite) is next and **not** in this task.

---

## 9. Unresolved requiring real-device validation

- Arcade `.zip` payload handling is profile-driven as `payload` but not physically validated on R36SX that those cores require the zip to stay compressed (plausible per upstream docs/cores/arcade.md, but needs device).
- 7z/RAR payload handling when handlers become available.
- Video preset still `PROVISIONAL_UNVALIDATED`.

