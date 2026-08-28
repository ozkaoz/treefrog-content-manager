# TreeFrog Content Manager

Global TreeFrogUI SD-card content manager — one schema for all handhelds (R36SX/SF3000/GB350 etc).

**Stack:** Tauri 2 + Rust backend + React + TypeScript frontend + SQLite + serde versioned declarative JSON profiles

Windows first; filesystem portable for later macOS/Linux.

> This is Phase 0/1 bootstrap. First milestone: Select source folder + select TreeFrogUI SD + scan + preview exactly what would be copied/extracted/skipped/conflicted, without writing anything.

## Structure

```
treefrog-manager/
  src-tauri/          # Rust backend (Tauri 2)
    src/
      main.rs         # Tauri entry
      lib.rs          # profile/scanner/classify/archive/hash/planner/db/sd modules
      profile.rs      # serde profile loader (versioned)
      scanner.rs      # recursive scan + classification entry
      classify.rs     # profile+extension/content-hint classification
      archive.rs      # ZIP/7z/RAR inspection + safety (traversal/absolute/symlink/limits)
      hash.rs         # SHA-256 + duplicate engine
      planner.rs      # dry-run planner (unchanged/new/changed/duplicate/conflict/deletions)
      db.rs           # SQLite persistent index
      sd.rs           # SD detection via markers + health probe
      video.rs        # ffprobe adapter + conservative preset PROVISIONAL_UNVALIDATED
    Cargo.toml
    tauri.conf.json
  src/                # React TS frontend
    App.tsx           # source picker + SD picker + preview table (read-only)
    components/
      SourcePicker.tsx, SdPicker.tsx, DryRunPreview.tsx, BiosView.tsx, LgptView.tsx
    domain/           # TypeScript domain mirrors (profile types)
  python/treefrog/    # Python mirror of domain for pytest without Rust toolchain
    profile.py, scanner.py, classify.py, archive.py, hash.py, planner.py, sd.py
  tests/
    fixtures/         # archives, duplicates, media, BIOS, LGPT samples
  package.json
  vite.config.ts
```

## Profiles

Declarative, versioned JSON under `profiles/treefrogui/` — never hardcoded in UI:

- `profile.json` — global invariants, archive safety, duplicate, sync, artwork
- `systems.json` — 100+ folder aliases (case-sensitive) from `tzubertowski/treefrog-ui` cores.md + README
- `media.json` — music/videos/images/ebooks/rockbox/pico286 destinations
- `bios.json` — system/core, destination, patterns, size/hash, required/recommended, region variants (user-supplied only)
- `lgpt.json` — `lgpt/samples` + `lgpt/projects` (verified against Bacon-1.5 payload `sd_root/lgpt/*`)
- `video_presets.json` — conservative default `PROVISIONAL_UNVALIDATED` (not claim hardware compat)
- `sd_markers.json` — `cubegm/` + `roms/` heuristic, per-device `install_first/<device>/` differences

## Safety

- Prevent `../` traversal, absolute extraction paths, symlink/reparse hazards (skip+warn), collisions, enforce extraction-count/expansion limits, never silent overwrite.
- Duplicate: same content not same filename — SHA-256 exact, cheap metadata first, never delete source.
- Sync dry-run required before writes; normal sync never deletes; staging + atomic rename; resume/consistent on interrupt.

## Run (when toolchain available)

```bash
# frontend
npm install
npm run dev

# backend (requires Rust)
cargo test
cargo tauri dev

# python domain + pytest (no Rust needed)
python -m pytest treefrog-manager/tests -v
python -m pytest tests/test_content_manager_*.py -v
```

## Quality Gates

- `python tests/test_agent_context_contract.py` — PASS
- `python tests/test_release_audio_bootstrap.py` — PASS
- `python -m pytest treefrog-manager/tests` — PASS (scanner/archive/duplicate/planner fixtures)
- `git diff -- sd_root` must be NO for manager-only changes

See `docs/PLAN.md` phases 0-7 and `DECISIONS.md:DEC-2026-08-28-01`.
