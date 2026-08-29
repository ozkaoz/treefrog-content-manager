# TreeFrog Content Manager — Development Plan

**Version:** 1.1.0  
**Date:** 2026-08-29  
**Scope:** Global TreeFrogUI SD-card content manager (desktop) — not per-device fork  
**Stack:** Tauri 2 + Rust backend + React + TypeScript frontend + SQLite + serde + versioned declarative JSON profiles 1.1.0 + SHA-256 + FFmpeg/ffprobe adapter + maintained archive libs (ZIP implemented, 7z/RAR stubs) + native Windows dialogs + system theme + TreeFrogUI branding

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

### Phase 2C — Video conversion + Windows desktop build (done)

- Video inspection via `ffprobe` authoritative, compatibility evaluation (`compatible`/`conversion_required`/`unsupported`/`inspection_error`) data-driven from `profiles/treefrogui/video_presets.json` (`PROVISIONAL_UNVALIDATED`, container/video/audio codec, resolution, pix_fmt, fps, sample rate, max size, streams)
- Dedicated conversion service: never modifies source, temp workspace only, deterministic `*.converted.mp4`, overwrite protection, capture FFmpeg stderr, re-probe validation, clean temp, no SD writes, planner single source (`convert_then_copy`)
- Planner integration: video branch in `planner.rs`/`planner.py` with `convert_then_copy`/`conversion_error`, dry-run UI shows `status`/`preset`/`converted_name`, temp validated
- Windows x64 desktop build: Tauri 2, Rust 1.98, Node 20.19, Tauri CLI 2.11, MSVC 14.51, WebView2 151, FFmpeg 7, reproducible via `scripts/build_windows.ps1` (PowerShell) and `build_windows.sh` (WSL wrapper), artifacts `treefrog-manager.exe` 12.21 MB + MSI/NSIS, `--self-check` smoke test
- **Tests:** `test_phase2c_video_conversion.py` 11 tests covering ffprobe parsing, compatible, incompatible, unsupported, inspection failure, command generation, deterministic, temp output, validation success/failure, FFmpeg failure, source preservation, planner action, metadata, missing diagnostics — **pytest 77 PASS (66+11)**
- **Gate:** `pytest 77 PASS`, `context-contract PASS`, `preflight PASS`, `Windows x64 exe` + `self-check PASS`, `git diff -- sd_root` empty

### Phase BIOS-A — Formal BIOS profile and validation model (done)

- Extend `profiles/treefrogui/bios.json` to 1.1.0 formal with `bios_definitions` (system/Bios identity, human-readable name, required/conditional semantics, `mandatory_when`, accepted filenames/patterns/aliases, destinations profile-driven, expected size, SHA-256 authoritative only, variants any one satisfies, archive payload/container via existing archive/hash, verification states `missing`/`found_valid`/`found_invalid`/`found_unknown`/`duplicate`/`conflict`/`not_required`, planner-compatible)
- Validation states explicit, matching order: exact filename+hash, alias+hash, size fallback, wrong content→`found_invalid`, unknown→`found_unknown`; multiple variants; conditional `psx_content_present` etc.; destinations profile-driven; reuse `archive`/`hash` infrastructure; `get_valid_destinations()`; no invented hashes (only GBA `a860a8...` and O2ROM MD5)
- **Tests:** `test_bios_validation.py` 17 tests covering valid by filename+hash, alias+hash, invalid wrong hash, size-only, unknown, missing, duplicate identical, conflict same filename diff content, multiple variants, conditional triggered/not required, archive payload/container/unsupported, deterministic, schema, no invented hashes — **pytest 94 PASS (77+17)**
- **Gate:** `pytest 94 PASS`, `cargo check` PASS (with `bios.rs` + `regex`), `context-contract PASS`, `preflight PASS`, `git diff -- sd_root` empty, no SD writes, desktop still buildable

### Phase BIOS-B — BIOS Manager UI + planner integration (current)

- BIOS tab in main navigation (`Overview` ↔ `BIOS`) with TreeFrogUI-global profile-driven BIOS, user-friendly labels (`Verified`/`Missing`/`Needs verification`/`Invalid`/`Duplicate`/`Conflict`)
- `BiosManager.tsx`: BIOS source directory picker (`C:\BIOS`), runs existing `scanner` → `archive inspector` → `hash` → `bios validator` recursively, safely inspects archives in temp workspace, hashes where needed, matches filenames/patterns/aliases, validates hashes/size, identifies duplicates/conflicts/unknown, preserves multiple variants (any one valid variant satisfies, UI shows `Variant: scph5501.bin`), conditional `psx_content_present` → “Required because PlayStation content was detected”, detail panel (system, logical BIOS name, requirement status, accepted filenames, selected variant, source path, expected destination `cubegm/bios`, expected size, SHA-256 known/actual, reason), read-only actions (`import/copy`, `skip`, `replace`, `conflict`, `duplicate`, `manual review` via existing `apply_resolutions`)
- Global `DeploymentPlan` integration: BIOS appears in `DryRunPreview` as `BIOS: source C:\BIOS\scph5501.bin → cubegm/bios/scph5501.bin action copy reason required PS1 BIOS is valid` (or `manual_review` if missing/invalid), `DryRunPreview` BIOS filtering (`all`/`bios`/`video`/`rom`), `App.tsx` health summary (`TreeFrogUI Health: ... BIOS ⚠ 2 required BIOS missing`), no BIOS downloads
- **Tests:** `test_bios_manager.py` 13 tests covering BIOS source scan, valid shown, missing conditional, invalid, duplicate, conflict, multiple variants, requirement activation/inactive, deployment plan entry, DryRunPreview filtering, zero SD writes, smoke dataset (valid, wrong-hash, duplicate, unknown, missing, multi-variant) — **pytest 107 PASS (94+13)**
- **Gate:** `pytest 107 PASS`, `cargo check` PASS, `npm run build` PASS, `context-contract PASS`, `preflight PASS`, `git diff -- sd_root` empty, `Windows exe 12.21 MB` + `self-check PASS`, no SD writes

### Phase LGPT — Samples + Projects (current, LGPT Manager)

- LGPT profile `lgpt.json` with `lgpt/samples` + `lgpt/projects` (verified `sd_root/lgpt/*`), profile-driven, R36SX is target, not manager identity; reuse scanner/logical-unit/archive/SHA-256/conflict resolver/deployment planner/dry-run UI; Samples: recursive scan, WAV baseline, SHA-256 duplicate, alias/conflict/unchanged, archive via Phase 2A, deterministic, dry-run, no SD writes; Projects: directory logical units (e.g., `lgptsav.dat` + `project.lgpt`), not flattened, duplicate/conflict/unchanged via deterministic project hash, nested content, archive/container handling, deterministic planning, dry-run, no SD writes; UI: `LGPT` tab with `Samples`/`Projects` subtabs, source folder pickers, scanning, counts, dry-run actions (`New`/`Unchanged`/`Duplicate`/`Conflict`/`Manual review`/`Unsupported`), filtering, inspecting conflicts/duplicates; Global DryRun shows `LGPT Sample source → lgpt/samples/...` and `LGPT Project source → lgpt/projects/...` alongside ROM/Music/Video/BIOS; Health summary includes LGPT; fixtures synthetic `tests/fixtures/lgpt/samples` + `projects/ProjectA`/`ProjectB`; Windows installer copied to Desktop as `TreeFrog-Content-Manager-Setup.exe` + `.sha256`, build remains reproducible.
- **Tests:** `test_lgpt_manager.py` 24 tests covering samples (normal, recursive, duplicate, alias, conflict, unchanged, archive, unsafe, deterministic) + projects (logical unit, duplicate, conflict, unchanged, deterministic identity, nested, container) + planner (deployment entries, correct destinations, duplicate/conflict actions, dry-run, zero SD writes) + build script (installer discovery, Desktop resolution) — **pytest 131 PASS (107+24)**
- **Gate:** `pytest 131 PASS`, `context-contract PASS`, `preflight PASS`, `Windows exe` + `Desktop installer` + `self-check PASS`, `git diff -- sd_root` empty

### Phase 2E — Desktop UX foundation: native dialogs, Windows theme, branding, navigation (current)

- **Native Windows dialogs:** replace `window.prompt()` with Tauri `@tauri-apps/plugin-dialog` `open({directory:true})` / `open({directory:false})` via reusable `src/services/dialog.ts` (`pickFolder()`, `pickFile()`, `pickFiles()`, `pickFolders()`). All source folders (Games, Music, Video, BIOS, LGPT Samples/Projects, future SD target) share same abstraction; path returned cleanly to Rust/frontend state; manual QA verifies native picker in packaged Windows app.
- **Windows Light/Dark:** follow `prefers-color-scheme`, dynamic `watchSystemTheme` via `window.matchMedia("(prefers-color-scheme: dark)")`; centralized CSS variable tokens (`--bg`, `--surface`, `--surface-elevated`, `--text`, `--text-muted`, `--border`, `--accent`, `--success`, `--warning`, `--danger`, `--input`, `--focus`) defined in `src/styles.css` for `light` and `dark` via `@media (prefers-color-scheme: dark)` and `[data-theme]`; readable contrast both themes; never make TreeFrogUI device theme.
- **Branding:** use official `xgame-logo.bmp` (480×854, frog+wordmark, black background) from `tzubertowski/TreeFrogUI` main; frog ONLY as primary mark (no redraw), transparent crop `src/assets/branding/frog-only.png` (87×99) + `frog-square.png` (99×99) via deterministic `scripts/generate_branding.py` (NEAREST, `r<20&&g<20&&b<20` → transparent, split at 10px zero gap y≈349–358); generated icons `src-tauri/icons/32x32.png`, `128x128.png`, `128x128@2x.png`, `256x256.png`, `512x512.png`, `icon.ico` (16/32/48/256), `icon.icns` (512); full frog+wordmark retained only in About/Credits secondary; provenance documented in `src/assets/branding/README.md` (CC BY-NC-SA 4.0, frog from FrogUI); no newly generated logo; no unnecessary duplicated source assets.
- **Identity:** `TreeFrog Content Manager` with frog icon as primary mark — window icon, desktop/Start Menu icon, installer icon, header branding (32×32 frog + wordmark), About section; restrained professional, not mimicking TreeFrogUI handheld frontend.
- **Navigation foundation:** `Overview | Games | Music | Videos | BIOS | LGPT | SD Card | Settings | About` (8 tabs) in `src/App.tsx`; not-yet-implemented modules show `Placeholder` with "Coming in a future release" — no fake functionality; BIOS + LGPT remain functional, Overview functional.
- **Source picker:** revised `src/components/SourcePicker.tsx` — `[Browse]` opens native picker, selected path visible/readable, no manual typing required except hidden debug fallback; consistent behavior across BIOS/LGPT/future Games/Music/Videos/SD.
- **Empty/standard states:** `src/components/EmptyState.tsx` — consistent visual for `empty/loading/success/warning/error/not_implemented` (e.g., "No folder selected", "Scanning…", "No files found", "Requires attention"); not over-designed.
- **Tests:** `test_phase2e_desktop_ux.py` 20 tests covering dialog service, prompt removal, source-picker state, theme tokens, theme init, branding assets, provenance, icons, navigation entries, working modules, source-picker consistency, placeholder, empty states, version consistency, header/about branding, no SD writes — **pytest 151 PASS (131+20)**.
- **Desktop build:** still via `scripts/build_windows.ps1`; verifies installer builds, installer icon is frog, app icon correct, installs, launches, native Browse opens real dialog, light/dark reflected, BIOS/LGPT functional; installer copied to Desktop as `TreeFrog-Content-Manager-<version>-Windows-x64-Setup.exe` + `.sha256`; manual QA documented in `docs/MANUAL_QA_2E.md`.
- **Gate:** `pytest 151 PASS` + `cargo check PASS` + `npm run build PASS` + `Windows exe` launch PASS + native dialog PASS + light/dark PASS + icons PASS + BIOS/LGPT PASS + `git diff -- sd_root` empty.

### Phase 3 — Music / Images / Ebooks (next)

- Music/images/ebooks: use profile media destinations/formats; preserve music subfolders; MuPDF ebook per-book `.positions` ignore
- **Gate:** fixture tests, no claim without evidence

### Phase 4 — BIOS Manager (done: BIOS-A + BIOS-B)

- BIOS Manager UI + planner integration already done (see above); profile defines system/core, destination, accepted patterns, expected size/hash when known, required/recommended, region variants (profile `bios.json` 1.1.0 formal)
- **Gate:** BIOS verification + UI tests done

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

## Next Exact Action (2026-08-29)

- Phase 2E complete: Desktop UX foundation (native dialogs, Windows theme, TreeFrog branding, navigation, source picker, empty states) + Windows build + manual QA — next is Phase 3 Music/Images/Ebooks or Phase 2D SD writes (not in this task). Run `python3 -m pytest treefrog-manager/tests -v` (151 tests) + `bash scripts/agent_preflight.sh --allow-dirty` + `cargo check` + `npm run build` + verify `TreeFrog-Content-Manager-0.1.0-Windows-x64-Setup.exe` on Desktop + manual QA `docs/MANUAL_QA_2E.md`.

## Unresolved for real-device validation (updated for LGPT)

- Arcade payload `.zip` handling (cps1/neogeo/m2k) is profile-driven as `payload` per `archive_policy.json` but has not been physically validated on R36SX/SF3000 hardware that those cores require the zip to stay compressed. The same for 7z payload when handler becomes available.
- Video preset remains `PROVISIONAL_UNVALIDATED`; no claim of hardware decoder support without device probe.
- Large-library performance (10k+ files, hash caching) not yet exercised on real SD; per-job limit 10k is conservative.
- LGPT hardware validation remaining: sample path behavior (`lgpt/samples` vs `lgpt/samples/records`), project path behavior (`lgpt/projects` with `lgptsav.dat` + `project.lgpt`), supported sample formats (WAV baseline, FLAC/OGG), project structure (directory + `lgptsav.dat` + `sample.wav`), filename/path constraints if any — requires real R36SX/LGPT installation.
