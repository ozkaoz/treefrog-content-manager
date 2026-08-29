# CONTEXT_MAP — Stable Subsystem Router

**Purpose:** IF I AM WORKING ON X, WHAT SHOULD I READ / BUILD / TEST.
Stable navigation only — no mutable branch/HEAD/SHA/objective here (those live in CURRENT.md / Git).

> Verify existence with `ls` / `git ls-files` if a path seems missing — evidence wins over map.

---

## Router

| Subsystem / Task | Read first | Source paths | Tests / Checks | Decisions |
|---|---|---|---|---|
| **Runtime core / build** | AGENTS.md, docs/ai/VALIDATION.md | `source/`, `source/projects/Makefile*`, `source/BUILD_*`, `VERSION` | `scripts/audit.sh`, `tests/host_syntax_check.sh`, `tests/run_host_*` | DEC-2026-08-21-01..05 (where applicable) |
| **TreeFrog audio (48k stereo)** | docs/ai/RELEASE_CONTRACT.md, docs/AUDIO_OTG.md | `source/sources/Adapters/TREEFROG/Audio/TreeFrogAudio.cpp`, `TreeFrogUac2Bridge.*`, `source/sources/Adapters/TREEFROG/` | `test_bacon14_48k_*`, `tests/run_host_audio_*`, `tests/test_release_audio_bootstrap.py` | — (permanent invariants AGENTS.md §3) |
| **OTG / MUSB / USB** | device/AGENTS.md | `device/otg_*.sh`, `device/r36s_*_usb_audio_io.c`, `device/r36s_*_host*.c` | `tests/run_host_sp404*`, `tests/run_host_usb*` | DEC-2026-08-23-02, DEC-2026-08-23-05, DEC-2026-08-23-06 |
| **Windows (UAC2)** | docs/BACON_1_5_GOLDEN_BOOTSTRAP_PHYSICAL_PASS.md | `device/r36s_u241_usb_audio_io*`, `source/sources/Adapters/TREEFROG/Audio/` | `tests/test_release_audio_bootstrap.py` + physical matrix | DEC-2026-08-23-02, DEC-2026-08-23-05 |
| **SP404 / Sampler** | device/AGENTS.md | `device/r36s_sp404_host_audio_io.c`, `device/otg_h37_*` | `tests/run_host_sp404_*`, `tests/test_bacon14_*` | DEC-2026-08-23-02, DEC-2026-08-23-05, DEC-2026-08-23-06 |
| **Android (H38 bridge)** | docs/ai/RELEASE_CONTRACT.md | `device/r36s_aoa_*`, `sd_root/ANDROID/LGPTUsbAudioBridge-H38-debug.apk` (H38-only, no H36) | `tests/run_host_android_aoa.sh`, physical bridge test | DEC-2026-08-23-02, DEC-2026-08-23-03 |
| **Input / Navigation** | docs/CONTROLS_EN.md | `source/sources/Adapters/TREEFROG/Input/`, `source/sources/Services/Controllers/` | `tests/run_host_input_policy.sh`, `tests/run_host_navigation.sh` | — |
| **Mixer / FX / Compressor** | docs/PLAN_FX_REDESIGN_ES.md | `source/sources/Application/Audio/FxEngine/`, `source/sources/Application/Audio/AudioMixer.cpp` | `tests/run_host_mixer*.sh`, `tests/run_host_compressor*.sh`, `test_fx_phase*.py` | DEC-2026-08-21-03 |
| **Analyzer / EQ8 / Spectrum** | — | `source/sources/Application/Audio/SpectrumAnalyzer.*`, `InstrumentEq.*`, `EqBiquad.h` | `tests/run_host_spectrum_analyzer.sh`, `run_host_eq*.sh`, `run_host_analyzer_target.sh` | DEC-2026-08-21-03, DEC-2026-08-21-30, DEC-2026-08-21-31, DEC-2026-08-21-32 |
| **Pitch / Chopper** | — | `source/sources/Application/Instruments/`, `source/sources/Application/UI/Views/*Chopper*` | `tests/run_host_pitch_tool.sh`, `run_host_chopper*.sh`, `test_bacon14_pitch*` | — |
| **Project / Filesystem / SD** | docs/ai/RELEASE_CONTRACT.md | `sd_root/`, `source/sources/Application/Persistency/`, `device/lgpt_launcher_u241.sh` | `tests/test_release_audio_bootstrap.py`, `tests/test_copy_root_launcher.sh` | DEC-2026-08-23-02, DEC-2026-08-23-03, DEC-2026-08-23-05 |
| **Build** | docs/BUILD_EN.md, docs/BUILD_ES.md | `source/BUILD_TREEFROG_*.sh`, `source/projects/Makefile.TREEFROG` | `scripts/audit.sh`, `tests/host_syntax_check.sh` | — |
| **Release packaging** | docs/ai/RELEASE_CONTRACT.md, docs/ai/VALIDATION.md | `scripts/build_copy_root_release.py`, `sd_root/`, `LGPT_R36SX_Bacon-1.5_SHA256SUMS.txt`, `docs/BACON_1_5_RELEASE_MANIFEST.md` | `tests/test_release_audio_bootstrap.py`, `scripts/verify_copy_root_layout.sh` | DEC-2026-08-23-02, DEC-2026-08-23-03, DEC-2026-08-23-04 |
| **Agent infrastructure** | AGENTS.md, docs/ai/VALIDATION.md | `.opencode/agents/`, `scripts/agent_preflight.sh`, `tests/test_agent_context_contract.py` | `tests/test_agent_context_contract.py` | DEC-2026-08-23-01 |
| **TreeFrog Content Manager** | AGENTS.md, docs/PLAN.md, `profiles/treefrogui/`, `docs/ARCHITECTURE.md` | `treefrog-manager/`, `profiles/treefrogui/`, `treefrog-manager/src-tauri/src/`, `treefrog-manager/src/` | `treefrog-manager/tests/*`, `tests/test_content_manager_*.py`, `tests/test_profile_*.py`, `cargo test`, `pytest` | DEC-2026-08-28-01, DEC-2026-08-28-02, DEC-2026-08-28-03, DEC-2026-08-28-04, DEC-LGPT-01 |
| **Content Manager profiles** | docs/PLAN.md, `profiles/treefrogui/archive_policy.json`, `profiles/treefrogui/video_presets.json`, `profiles/treefrogui/bios.json` | `profiles/treefrogui/manifest.json`, `profiles/treefrogui/profile.json`, `profiles/treefrogui/systems.json`, `profiles/treefrogui/media.json`, `profiles/treefrogui/bios.json` (1.1.0 formal), `profiles/treefrogui/lgpt.json`, `profiles/treefrogui/video_presets.json`, `profiles/treefrogui/sd_markers.json`, `profiles/treefrogui/archive_policy.json` | `tests/test_profile_loader.py`, `tests/test_bios_validation.py` | DEC-2026-08-28-01, DEC-2026-08-28-02, DEC-2026-08-28-03, DEC-2026-08-28-04, DEC-LGPT-01 |
| **Content Manager scanner/planner** | profiles/treefrogui/systems.json, profiles/treefrogui/archive_policy.json, profiles/treefrogui/video_presets.json, profiles/treefrogui/bios.json, profiles/treefrogui/lgpt.json | `treefrog-manager/src-tauri/src/scanner.rs`, `treefrog-manager/src-tauri/src/classify.rs`, `treefrog-manager/src-tauri/src/archive.rs` (ArchiveHandler), `treefrog-manager/src-tauri/src/hash.rs`, `treefrog-manager/src-tauri/src/planner.rs` (logical units, resolution, video, bios, lgpt), `treefrog-manager/src-tauri/src/bios.rs`, `treefrog-manager/src-tauri/src/video.rs` | `tests/test_scanner_classification.py`, `tests/test_archive_inspection.py`, `tests/test_duplicate_engine.py`, `tests/test_dry_run_planner.py`, `tests/test_phase2a_archive_ingestion.py`, `tests/test_phase2b_duplicate_resolution.py`, `tests/test_phase2c_video_conversion.py`, `tests/test_bios_validation.py`, `tests/test_lgpt_manager.py`, `tests/test_bios_manager.py` | DEC-2026-08-28-01, DEC-2026-08-28-02, DEC-2026-08-28-03, DEC-2026-08-28-04, DEC-LGPT-01 |
| **Content Manager archive ingestion (2A)** | `profiles/treefrogui/archive_policy.json`, docs/ARCHITECTURE.md | `treefrog-manager/python/treefrog/archive.py`, `treefrog-manager/python/treefrog/planner.py`, `treefrog-manager/src-tauri/src/archive.rs`, `treefrog-manager/src-tauri/src/planner.rs` | `tests/test_phase2a_archive_ingestion.py` (22 tests: valid ZIP, nested, traversal, absolute, drive-letter, symlink, hardlink, collision, limits, payload/container, CUE/BIN groups, duplicates, deterministic, temp workspace) | DEC-2026-08-28-02 |
| **Content Manager duplicate/conflict (2B)** | `profiles/treefrogui/archive_policy.json`, docs/ARCHITECTURE.md | `treefrog-manager/python/treefrog/planner.py` (`apply_resolutions`, content_type, source_hash/destination_hash, grouped duplicate), `treefrog-manager/src-tauri/src/planner.rs` (same), `treefrog-manager/src/App.tsx`, `treefrog-manager/src/components/DryRunPreview.tsx` | `tests/test_phase2b_duplicate_resolution.py` (13 tests: identical, alias, conflict, grouped, archive vs extracted, unchanged, explicit replace/keep_destination/keep_both/skip, deterministic, metadata, zero-write) | DEC-2026-08-28-03 |
| **Content Manager desktop UX (2E)** | docs/PLAN.md (Phase 2E), docs/ARCHITECTURE.md §7, docs/MANUAL_QA_2E.md, `src/assets/branding/README.md` | `treefrog-manager/src/services/dialog.ts` (native pickFolder/pickFile), `treefrog-manager/src/services/theme.ts` (prefers-color-scheme + data-theme), `treefrog-manager/src/styles.css` (CSS vars), `treefrog-manager/src/components/SourcePicker.tsx` (consistent), `treefrog-manager/src/components/EmptyState.tsx`, `treefrog-manager/src/components/Header.tsx` (frog), `treefrog-manager/src/components/About.tsx`, `treefrog-manager/src/App.tsx` (8 tabs Overview/Games/Music/Videos/BIOS/LGPT/SD Card/Settings/About), `treefrog-manager/src/assets/branding/frog-only.png`, `treefrog-manager/src-tauri/icons/*`, `scripts/generate_branding.py`, `treefrog-manager/src-tauri/capabilities/default.json` (dialog permissions) | `treefrog-manager/tests/test_phase2e_desktop_ux.py` (20 tests: dialog, no prompt, source-picker, theme tokens, branding, icons, navigation, placeholders, empty states, version, header/about), `docs/MANUAL_QA_2E.md` (16-step native dialog + light/dark + icon QA), `cargo check`, `npm run build` | DEC-2026-08-29-01 |
| **Content Manager SD target (3A)** | docs/PLAN.md (Phase 3A), docs/ARCHITECTURE.md §7 (SD target), `profiles/treefrogui/sd_markers.json` | `treefrog-manager/src-tauri/src/sd_target.rs` (VolumeInfo, TargetAnalysis, list_volumes, analyze_target, calculate_space, validate_destination_path, check_case_collision), `treefrog-manager/python/treefrog/sd_target.py` (mirror), `treefrog-manager/src-tauri/src/sd.rs` (legacy detect), `treefrog-manager/src-tauri/src/planner.rs` (single source), `treefrog-manager/src/components/SdCardPanel.tsx` (native pickFolder, volume info, analysis, dry-run), `treefrog-manager/src/App.tsx` (SD Card tab) | `treefrog-manager/tests/test_sd_target.py` (17 tests: valid/incomplete/unknown/inaccessible, removable-volume, ROM/media/BIOS/LGPT detection, logical units, hashing via planner, space ok/insufficient, destination validation, case-collision, integration, deterministic, zero writes, video/BIOS/LGPT space), `cargo check`, `npm run build` | DEC-2026-08-30-01 |

---

## Legacy / Non-canonical Tooling

| Script | Status | Note |
|---|---|---|
| `scripts/install.sh` | **LEGACY U2523 — NOT CANONICAL FOR BACON-1.5** | Uses `/mnt/f` and `/mnt/d/R36S/PORT LPTRACKER/BUILD/U2523/lgpt_r36sx_u2523.so`; Bacon-1.5 payload is `sd_root/cubegm/cores/lgpt_core.so` + `LGPT_R36SX_Bacon-1.5_SD_ROOT.zip`. Do not use without separate audit. |
| `scripts/verify.sh` | **LEGACY U2523 — NOT CANONICAL FOR BACON-1.5** | Same legacy paths/artifact names; not the Bacon-1.5 install contract (`Stock OS + TreeFrogUI + ZIP contents`). |

Do not modify their functional behavior in CLASS A tasks — only label them.

---

## Canonical Payload Locations

- Build artifact: `sd_root/cubegm/cores/lgpt_core.so` (and `sd_root/cubegm/lgpt` launcher)
- Release ZIP: `LGPT_R36SX_Bacon-1.5_SD_ROOT.zip` (57 files Apps→LGPT, see `docs/RELEASE_SD_INCLUDED_FILES.txt`)
- SHA manifest: `LGPT_R36SX_Bacon-1.5_SHA256SUMS.txt`
- Golden evidence: `docs/BACON_1_5_GOLDEN_BOOTSTRAP_PHYSICAL_PASS.md`, `docs/BACON_1_5_RELEASE_MANIFEST.md`, `docs/BACON_1_5_TREEFROG_APPS_PHYSICAL_PASS.md`
- Content Manager profiles: `profiles/treefrogui/` (versioned declarative JSON 1.1.0, serde + Python mirror, now includes `archive_policy.json`, `video_presets.json` 1.1.0 provisional, `bios.json` 1.1.0 formal)
- Content Manager app: `treefrog-manager/` (Tauri 2 + Rust backend + React TS frontend + SQLite; Phase BIOS-A formal BIOS validation, planner single source, no SD writes)
- Content Manager tests: `treefrog-manager/tests/` (182 tests: 31 original + 22 Phase 2A + 13 Phase 2B + 11 Phase 2C + 17 BIOS-A + 13 BIOS-B + 24 LGPT + 20 Phase 2E + 14 Phase 2E.3 + 17 Phase 3A) + `tests/test_content_manager_*.py` (fixtures for archives/duplicates/media/BIOS/LGPT/video, now with `tests/fixtures/lgpt/samples` + `projects/ProjectA`/`ProjectB` synthetic WAVs)
- Content Manager docs: `docs/PLAN.md` (phases 0-3A, now 1.1.0 + 3A SD target), `docs/ARCHITECTURE.md` (archive abstraction, safety, profile-driven, planner single source, BIOS model, video pipeline, SD target detection + analysis + space + safe path)

`BUILD/`, `buildTREEFROG/`, `source/dist/`, `treefrog-manager/src-tauri/target/` are `.gitignore`'d — not source.

---

## Update Policy

- Update this map when subsystem locations change or new subsystems appear.
- Do not add mutable state (branch, HEAD, SHA, hashes, temporary objective).
- Verify listed paths exist after any move; fix map immediately if stale.
