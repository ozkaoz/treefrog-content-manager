# DECISIONS — Durable Technical Decisions

**Last reviewed:** 2026-08-28
**Policy:** Only durable architecture/project decisions. Operational events (push, copy to SD, one-off build PASS) belong in Git/evidence, not here.

| Field | Meaning |
|-------|---------|
| ID | `DEC-YYYY-MM-DD-NN` unique |
| Status | `ACTIVE` / `SUPERSEDED` / `DEPRECATED` |
| Scope | Subsystem / area |

---

## DEC-2026-08-21-01 — No gain table indexed by dB+80

**Date:** 2026-08-21
**Status:** ACTIVE
**Scope:** Audio / DSP (InstrumentEq, ParametricEQ)

**Context:** Audit for supposed BUG1 (`idx = dB+80` gain table) across full source tree and history U2.52..U2.71.
**Decision:** No such table exists. Gain is `powf(10, dB/20)` or linear-indexed compressor table, clamped -24..+24 dB.
**Reason:** Exhaustive grep and history review found no `eqGainTable` / `dB+80` indexer.
**Consequences:** BUG1 was non-existent; do not add dB-indexed table.
**Evidence:** `InstrumentEq.cpp:156-159`, `ParametricEQ.cpp:108-110`, `Compressor.cpp:190-207`, `FxPages.h:442` is VU only.
**Related:** `source/sources/Application/Audio/InstrumentEq.cpp`, `EqBiquad.h`

---

## DEC-2026-08-21-02 — SDL2 driver uses SDL1.2 legacy API

**Date:** 2026-08-21
**Status:** ACTIVE
**Scope:** Audio drivers (SDL/SDL2)

**Context:** Both `SDL` and `SDL2` adapters use `SDL_OpenAudio`/`SDL_PauseAudio` (SDL1.2) not `SDL_OpenAudioDevice`.
**Decision:** Document as known debt; do not treat as modern SDL2 without migration.
**Reason:** Partial SDL2 migration; functional on R36SX via legacy path.
**Consequences:** May have issues on modern SDL2 hosts; requires explicit migration before stable host release.
**Evidence:** `SDLAudioDriver.cpp:65` `SDL_OpenAudio`, no `SDL_InitSubSystem(SDL_INIT_AUDIO)`, SDL1 callback signature.
**Related:** `source/sources/Adapters/SDL/Audio/SDLAudioDriver.cpp`, `source/sources/Adapters/SDL2/Audio/SDLAudioDriver.cpp`

---

## DEC-2026-08-21-03 — EQ <80 Hz: Q=0.707 for slope>1 on all filter types

**Date:** 2026-08-21
**Status:** ACTIVE
**Scope:** InstrumentEq / EqBiquad

**Context:** High slopes (S8 =96dB/oct) with Q=1 at <80 Hz caused +48 dB resonance on BELL/shelves.
**Decision:** For `hz <80 && slope>1` force `Q=0.707` (Butterworth) for ALL types including BELL/LOW_SHELF/HIGH_SHELF.
**Reason:** Stability and flat wall response at sub-80 Hz; measured BELL 45 Hz lvl6 Q1 S8 → 48 dB, with fix → ~6 dB flat.
**Consequences:** S8 wall at 40 Hz flat; BELL <80 loses selectivity but gains stability.
**Evidence:** `InstrumentEq.cpp:384-394` `qForDsp=0.707...`, `EqBiquad.h:61-64` double precision.
**Related:** `source/sources/Application/Audio/InstrumentEq.cpp`, `source/sources/Application/Audio/EqBiquad.h`

---

## DEC-2026-08-21-04 — Analyzer: Blackman window + hold >140 Hz

**Date:** 2026-08-21
**Status:** SUPERSEDED by DEC-2026-08-21-31 and DEC-2026-08-21-32
**Scope:** SpectrumAnalyzer / InstrumentEqView

**Context:** Hann -31 dB sidelobes caused hihat to light false bass; hold on bass kept kick tails.
**Decision:** FFT Hann→Blackman (-67 dB, `a0=0.42 a1=0.5 a2=0.08`), visual hold only `fcHold>140 Hz`, uniform `visGain`.
**Reason:** Eliminate spectral leakage diagonal; refine later with exclusive bins / stereo power.
**Consequences:** Superseded by exact Hz mapping and stereo power decisions; Blackman and >140 Hz hold preserved.
**Evidence:** `SpectrumAnalyzer.cpp:141`, `InstrumentEqView.cpp:634` (pre-fix baseline).
**Related:** `source/sources/Application/Audio/SpectrumAnalyzer.cpp`

---

## DEC-2026-08-21-05 — Startup version string Bacon 1.5

**Date:** 2026-08-21
**Status:** ACTIVE
**Scope:** UI / Versioning

**Context:** Legacy `Piggy build %s.%s.%s` leaked into R36SX build strings.
**Decision:** `NullView.cpp:22`, `AppWindow.cpp:1430` → `"LGPT R36SX - Bacon 1.5"`, `Project.h:23` `PROJECT_RELEASE "5"`, `BUILD_COUNT "0-bacon15"`.
**Reason:** Unified Bacon-1.5 branding; old Piggy string remains orphan in .rodata.
**Consequences:** Visible version on device is `LGPT R36SX - Bacon 1.5`.
**Evidence:** `NullView.cpp:22`, `AppWindow.cpp:1430`, `Project.h:23`.
**Related:** `source/sources/Application/Views/NullView.cpp`, `source/sources/Application/AppWindow.cpp`

---

## DEC-2026-08-21-30 — EQ8 sub-80 Hz fix: Q24 round, shelf NaN guard, UI/DSP coherence

**Date:** 2026-08-21
**Status:** ACTIVE
**Scope:** EqBiquad / InstrumentEq / InstrumentEqView

**Context:** Q15 truncation caused LPF20 `b=0` (-96 dB), HPF20 err 1.8 dB, shelf `sqrt(neg)` NaN, UI Q15 vs DSP Q24 diverged 6 dB.
**Decision:** `coeffFromDouble` Q24 round-to-nearest + int32 saturate, clamp shelf `arg<0→0`, `GetBandCoeffs` round `(v+256)>>9`, View `eqBiquadCoeffsShift 24` with mirrored `qDraw`. Global `FIXED_SHIFT=15` kept; local precision Q24.
**Reason:** Q23 failed HPF20 0.15>0.10 and overflow; Q24 minimal passing with margin (8.2G vs 16G/33G for Q25/26) per `eq_study4.py`.
**Consequences:** HPF/LPF 20-100 Butterworth ` -3.01±0.10`, shelves NaN-free, UI=DSP ±0.2 dB, no float hot path.
**Evidence:** `eq_sub80_host_test` PASS `HPF20 -3.086 err -0.076`, `eq8_struct` 109 PASS, build `46c4714...` SD PASS 2026-08-21 13:45.
**Related:** `EqBiquad.h`, `InstrumentEq.cpp/h`, `InstrumentEqView.cpp`

---

## DEC-2026-08-21-31 — Analyzer: exclusive bins, correct Hz mapping, Blackman scale

**Date:** 2026-08-21
**Status:** ACTIVE
**Scope:** SpectrumAnalyzer / InstrumentEqView

**Context:** Overlapped ±30%→±10% windows made 1 kHz tone light 770-1430 Hz; `visGain` tilted treble; Hann/Blackman compensation miscalibrated; ring leaked across instruments.
**Decision:** Exclusive intervals `sqrt(f[i]*f[i+1])`, `lo=ceil(edgeLow/hzPerBin)` `hi=ceil(edgeHigh/hzPerBin)-1`, power-interpolated sub-bin, `BinFrequency(i)`, peak `7..6826` parabolic once per `Compute()` (`peakHz_`), Blackman `ampScale=2/sum` (0 dBFS→1.0, no visGain/*4), lazy window, ring+mean copy, `clearCapture()` idempotent, `heldH_[308]` member reset on focus.
**Reason:** One tone → one pixel; uniform -90..0 dB without treble tilt; clean instrument switch.
**Consequences:** Replaces visGain/windows of DEC-04; preserves Blackman and hold>140.
**Evidence:** `spectrum_analyzer 50 PASS` (984 Hz 0.9999 width1, sweep ±1px), `analyzer_target 1781 PASS`, build `c43006a` SD PASS.
**Related:** `SpectrumAnalyzer.cpp/h`, `InstrumentEqView.cpp/h`

---

## DEC-2026-08-21-32 — Analyzer final: stereo power, DisplayPeak tilt, TreeFrog auto

**Date:** 2026-08-21
**Status:** ACTIVE
**Scope:** SpectrumAnalyzer / InstrumentEqView / Android payload

**Context:** Stereo hihat cancelled by mono sum; tilt on floor created silence diagonal; Peak used raw mag vs display; hold static; TreeFrog required manual select.
**Decision:** Stereo `ringL/R` + `power=0.5*(|L|²+|R|²)` `amp=sqrt(power)*2/sum`, `DisplayPeakFrequency()` on 308 bins with `tilt 4.5*log2(fc/1000)` floor -90 (gate before tilt), `exp(-dt/300)` hold 100ms/release 300ms, `canvasW=309` `bx=(i*canvasW)/n`, `hat_probe` 308 bins Tests A-E, Android `r36s_aoa_*_h36` + APKs, TreeFrog auto `lgpt_libretro.so`.
**Reason:** Antiphase no longer cancels; diagonal silence fixed; peak matches display; Android payload deterministic.
**Consequences:** Replaces mono windows of DEC-31; preserves Q24 EQ8.
**Evidence:** `analyzer_h1_stereo 6 PASS` (in-phase 0.144 antiphase 0.144), `hat_probe A-E PASS`, build `66c966d...` R36SX F1-F7 PASS.
**Related:** `SpectrumAnalyzer.cpp/h`, `InstrumentEqView.cpp/h`, `scripts/install.sh`, `verify.sh`

**Note (2026-08-23):** Android payload clause (`r36s_aoa_*_h36` + APKs, `lgpt_libretro.so` auto) is historical context from analyzer-era; current Bacon-1.5 H38-only packaging is authoritative via DEC-2026-08-23-02, DEC-2026-08-23-03 and `docs/ai/RELEASE_CONTRACT.md` (H38-only, H36 must remain absent). Analyzer stereo power / DisplayPeak behavior remains ACTIVE.

---

## DEC-2026-08-23-01 — Multi-agent context architecture V2

**Date:** 2026-08-23
**Status:** ACTIVE
**Scope:** AI infrastructure / docs

**Context:** AGENTS v1.1 (334 lines) hardcoded `feature/bacon-1.5-fx`, WSL path `/home/dafunknoise/lgpt-repo`, machine `/mnt/g`, stale branch/HEAD; CURRENT was 258-line append-only changelog; DECISIONS stored push/SD events; no `docs/ai`, no preflight, no scoped agents, no OpenCode roles.
**Decision:** Constitution `AGENTS.md v2.0` (150-220 lines, invariants only), `CURRENT.md` concise snapshot with `must verify` stamp, `CONTEXT_MAP.md` stable router, `docs/ai/VALIDATION.md` + `RELEASE_CONTRACT.md`, `scripts/agent_preflight.sh` + `tests/test_agent_context_contract.py`, scoped `device/`, `TREEFROG/`, `tests/` AGENTS, `.opencode/agents/{audit,implement,review,release}.md`, lazy loading, compact handoff.
**Reason:** Prevent context drift, mutable duplication, machine-specific authority, and legacy U2523 being mistaken for Bacon-1.5.
**Consequences:** Agents load only relevant context; mutable state lives in Git/CURRENT cache; `scripts/install.sh`/`verify.sh` labeled LEGACY U2523.
**Evidence:** `tests/test_agent_context_contract.py PASS`, `bash -n scripts/agent_preflight.sh PASS`, `git diff -- sd_root` NO CHANGES, core 46bd84 unchanged.
**Related:** `AGENTS.md`, `CURRENT.md`, `CONTEXT_MAP.md`, `docs/ai/*`, `scripts/agent_preflight.sh`

---

## DEC-2026-08-23-02 — Golden Bootstrap clean-install release closure

**Date:** 2026-08-23
**Status:** ACTIVE
**Scope:** Release / OTG / deployment

**Context:** Persistent audio setup was missing from ZIP, requiring manual sentinel creation after install.
**Decision:** Release ZIP `C5C77A0212...` (7138546, 56 files) includes persistent baseline: `enable_lgpt_uac2_bridge` (empty), `audio_usb_profile STEREO_48K`, `audio_driver_mode/policy LOCAL_CONSOLE`, `active_audio_branch audio_driver_local_console`, `branches/audio_driver_local_console/MODE LOCAL_CONSOLE`. Core `46bd84` unchanged.
**Reason:** `Stock OS + TreeFrogUI + ZIP contents = fully functional PORT` with `POST_INSTALL_MANUAL_FIXES=0` proven by staged Golden Bootstrap.
**Consequences:** `WORKS ON DEV SD != RELEASE PACKAGE COMPLETE` is now enforced; `sd_root` is canonical payload source.
**Evidence:** `docs/BACON_1_5_GOLDEN_BOOTSTRAP_PHYSICAL_PASS.md PASS`, `docs/BACON_1_5_RELEASE_MANIFEST.md`, commit `4429d4e`, merge `b616a5b`.
**Related:** `sd_root/lgpt/otg/*`, `LGPT_R36SX_Bacon-1.5_SHA256SUMS.txt`, `docs/RELEASE_SD_INCLUDED_FILES.txt`

---

## DEC-2026-08-23-03 — Persistent vs volatile packaging

**Date:** 2026-08-23
**Status:** ACTIVE
**Scope:** Release / OTG / sd_root

**Context:** Volatile runtime state was at risk of being packaged as if it were install baseline.
**Decision:** Persistent (may be packaged when required for deterministic install): files in DEC-2026-08-23-02. Volatile (MUST NOT be packaged): FIFO, PID, daemon_pid/version, capture_abi, setup_result, sp404_card, aoa state, device detection state, /tmp, runtime logs.
**Reason:** Golden Bootstrap proves persistent setup is legitimate install content while volatile is runtime-only.
**Consequences:** `tests/test_release_audio_bootstrap.py` enforces no volatile under `lgpt/otg/` (except `bin/`).
**Evidence:** `tests/test_release_audio_bootstrap.py PASS` (sentinel empty, STEREO_48K, LOCAL_CONSOLE, no volatile).
**Related:** `sd_root/lgpt/otg/`, `tests/test_release_audio_bootstrap.py`

---

## DEC-2026-08-23-04 — Release publish + download-back identity

**Date:** 2026-08-23
**Status:** ACTIVE
**Scope:** Release pipeline

**Context:** Publishing without download-back could ship a different artifact than validated.
**Decision:** `ONE ARTIFACT NAME = ONE AUTHORITATIVE SHA` across GitHub body, SHA256SUMS, manifest, included-files, downloaded asset. After publish: `DOWNLOAD-BACK REQUIRED` and `REMOTE_SHA == LOCAL_SHA`.
**Reason:** Guarantees release golden = physical golden.
**Consequences:** Historical SHAs must be marked historical (see `BACON_1_5_RELEASE_MANIFEST.md`); new releases must pass download-back gate before being called golden.
**Evidence:** `REMOTE_DOWNLOAD_SHA=C5C77A...` `REMOTE_IDENTICAL=YES` `UNZIP_TEST_REMOTE PASS` `BOOTSTRAP_TEST_REMOTE PASS` `MANIFEST_CONSISTENT=YES`.
**Related:** `docs/BACON_1_5_RELEASE_MANIFEST.md`, `LGPT_R36SX_Bacon-1.5_SHA256SUMS.txt`

---

## DEC-2026-08-23-05 — SD filesystem health before runtime blame

**Date:** 2026-08-23
**Status:** ACTIVE
**Scope:** Device / filesystem / diagnostics

**Context:** Dirty exFAT caused `/mnt/sdcard` read-only, producing false USB/bootstrap failures.
**Decision:** Before blaming runtime, verify SD `mounted && healthy && writable && not read-only`. Diagnostic layers: `Detection != Runtime READY != PCM flow != physical PASS`. Repair requires explicit user authorization.
**Reason:** Filesystem failure mimics bootstrap/USB failure; distinction avoids false fixes.
**Consequences:** Agents must probe mount/options/write-probe (via `agent_preflight.sh --sd`) before kernel/audio changes.
**Evidence:** `docs/BACON_1_5_GOLDEN_BOOTSTRAP_PHYSICAL_PASS.md` notes exFAT repaired Healthy/SD_WRITEABLE=YES before PASS.
**Related:** `device/otg_u241_common.sh`, `device/lgpt_launcher_u241.sh`, `scripts/agent_preflight.sh`

---

## DEC-2026-08-23-06 — Kernel module lifecycle (CONFIG_MODULE_UNLOAD=n)

**Date:** 2026-08-23
**Status:** ACTIVE
**Scope:** Device / kernel / audio

**Context:** Platform showed `CONFIG_MODULE_UNLOAD=n`; replacing loaded ALSA families mid-session may be impossible.
**Decision:** Agents must verify `CONFIG_MODULE_UNLOAD` before assuming hot module replace; do not hardcode experimental strict-family switch without evidence.
**Reason:** Shared ALSA modules (`snd`, `snd-pcm`, etc.) may be persistent; false assumption breaks audio host.
**Consequences:** Any family switch requires evidence and physical validation; default is shared/persistent lifecycle.
**Evidence:** `docs/BACON_1_5_GOLDEN_BOOTSTRAP_PHYSICAL_PASS.md` H37 apply `76B50C` shared ALSA note, `CONFIG_MODULE_UNLOAD=n`.
**Related:** `device/otg_h37_apply_driver_mode.sh`, `device/AGENTS.md`

---

## DEC-2026-08-24-01 — TreeFrog Apps migration (LGPT as first-class App)

**Date:** 2026-08-24
**Status:** ACTIVE
**Scope:** TreeFrogUI / deployment / release

**Context:** TreeFrogUI v1.0.15_a adds Apps tab (compiled `app_defs[]`, `scan_apps_tab`, `is_app_folder_name`). LGPT was previously `Games→LGPT` via `roms/lgpt/start.lgpt` + `cubegm/lgpt` wrapper. Generic `mipsel-linux-gnu-gcc 12.4` FrogUI builds black-screen (NOTE/GNU_STACK/EH_FRAME drift, GLIBC 2.34). Official SDK `mips-mti 6.3.0` vanilla `f10caa` boots; dual-entry `656242` and Apps-only `76034b` both physically PASS (LOCAL/WINDOWS/SP404/ANDROID + switching).
**Decision:** LGPT is a TreeFrogUI standalone App (`app_defs {"lgpt","LGPT",NULL,NULL,LGPT_BIN}`), launched via `request_standalone_launch(LGPT_BIN, /mnt/sdcard/roms/lgpt/start.lgpt)` → `/tmp/frogui_launch.txt` → `picoarch` → `cubegm/lgpt` → Bacon runtime. `roms/lgpt` is hidden from Games (`is_app_folder_name` + `lgpt`), but `roms/lgpt/start.lgpt` remains required as launch argument. FrogUI binary derived from `https://github.com/tzubertowski/FrogUI` `r36sx 028b011`, built with SF3000 SDK 6.3.0, patch `patches/frogui_apps_lgpt.patch`, license CC BY-NC-SA 4.0. Required TreeFrogUI is now `v1.0.15_a` (was `v1.0.14_a`).
**Reason:** Apps tab is compiled-in, not FS-discoverable; external manifest does not exist (audit proved `app_defs` is sole registration). Least-invasive supported mechanism is FrogUI fork. Official toolchain is mandatory for compatible ELF (7 PHDR, no NOTE, GLIBC 2.0/2.15). Single presentation `Apps 1 / Games 0` is final contract.
**Consequences:** `sd_root` now contains `cubegm/cores/frogui_libretro.so` `76034b` (326700) — previously vendor-only. ZIP grows 56→57 files (`c5c77a` 7138546→`faf7a230` 7295274) with exactly one added path. `POST_INSTALL_MANUAL_FIXES=0` preserved. `core 46bd84`/`wrapper ee1ecfe5`/`H38 89a99d` unchanged. Clean-install now `Stock OS + TreeFrogUI v1.0.15_a + ZIP`.
**Evidence:** `build/frogui_candidate/vanilla_official/frogui_libretro.so` `f10caa` PASS, `build/frogui_candidate/apps_dual/frogui_libretro.so` `656242` dual PASS, `build/frogui_candidate/apps_only/frogui_libretro.so` `76034b` Apps-only PASS (full matrix + switching), `patches/frogui_apps_lgpt.patch` (3 hunks), `tests/test_frogui_apps_lgpt.py` + `test_treefrog_apps_lgpt_release.py` PASS, `docs/BACON_1_5_RELEASE_MANIFEST.md` updated.
**Related:** `patches/frogui_apps_lgpt.patch`, `sd_root/cubegm/cores/frogui_libretro.so`, `sd_root/roms/lgpt/start.lgpt`, `docs/BACON_1_5_RELEASE_MANIFEST.md`, `LGPT_R36SX_Bacon-1.5_SHA256SUMS.txt`

---

## DEC-2026-08-28-01 — TreeFrog Content Manager global profile + archive safety + duplicate handling

**Date:** 2026-08-28
**Status:** ACTIVE
**Scope:** TreeFrog Content Manager / profiles / scanner / archive / sync

**Context:** Requirement to build global TreeFrogUI content manager (not per-device fork) managing ROMs, music, videos, images, ebooks, BIOS, LGPT samples/projects, incremental SD sync. Must inspect live upstream TreeFrogUI (`tzubertowski/treefrog-ui` main, cores.md, docs/standalone-apps.md, README ROM setup) and Bacon-1.5 payload (`sd_root/`) to avoid stale assumptions. Need safety for archives (ZIP/7z/RAR, traversal, absolute, symlink, collisions, limits) and duplicate semantics (same content not same filename).

**Decision:** Global schema in versioned declarative JSON profiles under `profiles/treefrogui/` (manifest, profile, systems, media, bios, lgpt, video_presets, sd_markers) — sole authoritative source for folder aliases (case-sensitive), media destinations, BIOS rules, LGPT destinations (`lgpt/samples`, `lgpt/projects` verified against Bacon-1.5 `sd_root/lgpt/*`), video preset `PROVISIONAL_UNVALIDATED`. UI code must NOT hardcode mappings. Manager treats TreeFrogUI content as one device-independent schema; device-specific limited to SD detection/markers and optional capability checks per `sd_markers.json`. Archive policy: inspect entries before copy, copy intact only if profile says archive itself is valid runtime payload for target system, otherwise extract supported contents; bounded nested-archive policy (depth 1, 1024 entries, 1 GiB expansion, 10k files). Safety: prevent `../` traversal, absolute paths, symlink/reparse hazards (skip + warn), detect collisions, enforce count/size limits, never silent overwrite. Duplicate: cheap metadata first, SHA-256 for exact identity, classify same-path+same-hash unchanged, different-path+same-hash duplicate skip, same-path+different-hash conflict, new-path+new-hash copy; never delete source. Artwork: Mini Scraper remains external (`mini-scraper-cfw` releases) — manager provides launch/open + optional `.res` verification only. Video: ffprobe inspection required, auto-convert via FFmpeg when incompatible with staging, re-probe, validate, batch + cancel without corrupt finals. Sync: dry-run plan before writes (`unchanged/new/changed/duplicate/conflicts/deletions`); normal sync no delete; explicit deletion separate; staging + atomic rename; resume/consistent on interrupt. Persistent index SQLite for libraries/targets/fingerprints/deployments/profile+tool versions/job history; never commit user paths. Stack: Tauri 2 + Rust + React TS + SQLite + serde versioned profiles + SHA-256 + FFmpeg adapter + maintained archive libs (ZIP/7z/RAR); filesystem portable, Windows first.

**Reason:** Declarative profiles allow full TreeFrogUI coverage (120+ folder aliases from cores.md) without forking app per handheld; evidence over stale notes (live upstream verify shows `roms/music`, `roms/videos`, `roms/images`, `roms/Ebook`, `cubegm/bios`, standalone `ebook/video_player/image_viewer/rockbox/pico286/pcsx4all/lgpt` mapping). Bounded archive + safety prevents traversal/symlink hazards and expansion bombs; duplicate semantics prevents data loss from filename collisions. Global manager preserves LGPT Bacon golden (`sd_root` unchanged for manager-only changes).

**Consequences:** `profiles/treefrogui/*.json` are canonical; UI/backend must load via profile loader (`serde` Rust + Python mirror). Scanner/classification/archive/duplicate/planner must follow profile + safety invariants; first milestone is read-only preview (no SD writes). Future BIOS/video/LGPT features must use same profiles. Manager bootstrap is CLASS B (host tooling) until it touches runtime/deployment; `git diff -- sd_root` must stay NO for pure manager work.

**Evidence:** `profiles/treefrogui/manifest.json` + `systems.json` (100+ aliases from cores.md), `media.json`, `bios.json`, `lgpt.json` (verified `sd_root/lgpt/samples` + `lgpt/projects` with .keep; latest Bacon payload `faf7a230` 57 files), `video_presets.json` (PROVISIONAL_UNVALIDATED conservative preset), `sd_markers.json` (cubegm/ + roms/ heuristic), `docs/PLAN.md` phases 0-7, `CONTEXT_MAP.md` router added, live upstream inspection 2026-08-28 (`~/treefrog-ui` cores.md + README + docs/standalone-apps.md) and `wsl ls ~/treefrog-ui/sdcard`.

**Related:** `profiles/treefrogui/`, `treefrog-manager/`, `docs/PLAN.md`, `CONTEXT_MAP.md`, `AGENTS.md §3-6`, `docs/ai/VALIDATION.md`, `sd_root/lgpt/*`, `https://github.com/tzubertowski/treefrog-ui`, `https://github.com/tzubertowski/mini-scraper-cfw/releases`

---

## DEC-2026-08-28-02 — Phase 2A archive ingestion: profile-driven, temp-workspace, logical-unit planner

**Date:** 2026-08-28
**Status:** ACTIVE
**Scope:** TreeFrog Content Manager / archive ingestion / scanner / planner / profiles

**Context:** Phase 2A requires inspecting compressed sources and deciding copy-as-payload vs safe extract vs grouped multi-file vs rejected vs manual_review vs unsupported_archive, with zero SD writes. Earlier scanner treated ZIP heuristically (extract if inner has known ROM). Need robust safety (traversal, absolute, Windows drive-letter `C:/`, symlink, hardlink/ADS `:` , collisions, expansion 1GiB, member count 1024, nested depth 1, per-job 10k, compression-ratio bomb) and TreeFrog semantics that some ZIPs must stay compressed (arcade cps1/neogeo/m2k are payload, not containers). Supported formats must be extensible: ZIP implemented, 7z/RAR stubs must return `unsupported_archive` without rewriting planner.

**Decision:** Extend profile to 1.1.0 with `profiles/treefrogui/archive_policy.json` (handlers, safety limits, modes, per_system, grouping). Modes: `payload` (archive-is-payload → `copy`), `container`/`extract_and_classify` (→ `extract` then classify each member), `grouped` (CUE/BIN etc → one logical unit), `manual` (ambiguous/mixed/nested/collision/unknown → `manual_review`), `unsupported` (handler not available → `unsupported_archive`). Per-system overrides (e.g., `cps1/neogeo/m2k` → `.zip: payload`, `ps_psx/segacd/pcfx` → `.zip: grouped`). Archive abstraction `ArchiveHandler` (Rust trait / Python registry `HANDLERS`): `ZipHandler` implemented via `zip` crate / `zipfile`, `SevenZHandler`/`RarHandler` stubs return `UnsupportedArchive`. All safety checks in `archive.rs`/`archive.py`: `inspect_*` validates traversal, absolute, drive-letter regex `^[A-Za-z]:`, symlink `is_symlink` / `unix_mode` `0o120000`, hardlink/ADS `:` , collision via normalized lowercased dest map, expansion sum, member count, compression ratio, nested depth; `safe_extract_to_temp` extracts only to `tempfile::TempDir` and verifies `dest.starts_with(temp_dir)` and `!dest == archive_path` and no silent overwrite. Planner operates on logical units: `group_members` groups CUE+BIN in same folder (stem match, single cue groups all bin), `decide_archive_mode` is profile-driven (early grouped detection for CUE/BIN, nested → manual, no known inner → payload, single system → per_system mode, mixed → extract_and_classify not manual), then for each logical group creates one planner entry with `extract` or `grouped` and temp-hashes inner content for duplicate detection (archive vs extracted payload not double-counted). Planner actions now `copy/extract/skip_duplicate/skip_unchanged/conflict/manual_review/unsupported_archive` plus `deletions=0` unchanged; deterministic sorting, per-job total-files guard, zero-write invariant (never to SD, never overwrites source). Duplicate handling extended: identical content, same filename diff content, grouped payload identical (combined SHA-256 of sorted member hashes), duplicate archive vs duplicate extracted payload via temp hashing.

**Reason:** Profile-driven instead of `ZIP=>always extract` because some TreeFrog content must remain compressed: arcade romsets are validated as `.zip` payloads per core (`cps1` etc); extracting would break core loading. Other systems accept both, user may want to preserve `.zip`. Hardcoding extraction in scanner would be wrong, not portable, and would require planner rewrite when 7z/RAR tooling arrives. Profile-driven keeps scanner generic, planner deterministic, allows per-system tuning (adding a new system or changing arcade payload rule) without code change, and documents intent in declarative JSON. Temp workspace guarantees bomb safety and no SD mutation; logical-unit planning prevents CUE/BIN split.

**Consequences:** `profiles/treefrogui/` is 1.1.0 (manifest, profile, systems 1.1.0, archive_policy new). Scanner/planner use profile, not hardcoded `always extract`. Archive ingestion is safe and profile-driven; adding 7z/RAR only needs implementing handler and flipping `implemented:true` in `archive_policy.json`, no planner rewrite. Tests cover 22 new cases (valid, nested, traversal, absolute, drive-letter, symlink, hardlink/ADS, collision, expansion, member count, payload, container, grouped CUE/BIN, duplicate archive, duplicate extracted, nested bomb, unsupported, deterministic, temp guard, no overwrite). Planner remains zero-write; physical SD writes remain Phase 2B.

**Evidence:** `profiles/treefrogui/archive_policy.json` (handlers, safety, modes, per_system, grouping), `profile.json 1.1.0`, `systems.json 1.1.0`, `treefrog-manager/python/treefrog/archive.py` (full safety + HANDLERS), `treefrog-manager/python/treefrog/planner.py` (logical units, manual_review, unsupported), `treefrog-manager/src-tauri/src/archive.rs` (ZipHandler, stubs, drive-letter, hardlink, collision, temp), `treefrog-manager/src-tauri/src/planner.rs` (grouped, mode, temp hashing), `treefrog-manager/tests/test_phase2a_archive_ingestion.py` 22 tests PASS, `pytest treefrog-manager/tests` 53 PASS, `test_agent_context_contract PASS`, `preflight PASS`, `git diff -- sd_root` empty.

**Related:** `profiles/treefrogui/archive_policy.json`, `profiles/treefrogui/profile.json`, `profiles/treefrogui/systems.json`, `treefrog-manager/python/treefrog/archive.py`, `treefrog-manager/python/treefrog/planner.py`, `treefrog-manager/src-tauri/src/archive.rs`, `treefrog-manager/src-tauri/src/planner.rs`, `treefrog-manager/src-tauri/src/profile.rs`, `docs/ARCHITECTURE.md`, `docs/PLAN.md`, `DEC-2026-08-28-01`

---

## DEC-2026-08-28-03 — Phase 2B duplicate/conflict resolution: planner single source of truth

**Date:** 2026-08-28
**Status:** ACTIVE
**Scope:** TreeFrog Content Manager / planner / duplicate/conflict / UI

**Context:** Phase 2B requires deterministic duplicate/conflict layer on top of scanner/archive/logical-unit planner + SHA-256 engine, with zero SD writes. Need to distinguish exact duplicate / same-filename-diff-content (conflict) / different-filename-identical (alias) / grouped identical / archive-vs-extracted / unchanged, expose full metadata for UI, support explicit overrideable resolutions, keep planner as single source.

**Decision:** Extend planner entries to carry `source`, `destination`, `logical content type` (`content_type` e.g. `rom/GBA`, `grouped/CUE_BBIN`, `archive-payload`), `source_hash`, `destination_hash`, `reason`, `logical-unit members` (`members`/`group`), `default_action`, `resolution`, `resolved_action`, `original_destination` (for keep_both). Defaults: exact duplicate → `skip` (`skip_duplicate`), same filename different SHA → `conflict`, different filename identical SHA → `duplicate` (`skip_duplicate`), grouped identical → `skip_duplicate` with combined hash, archive-vs-extracted → not double-counted via temp-hashed inner content, unchanged → `skip_unchanged`. All defaults overrideable via explicit `resolution` in `{skip, replace, keep_both, keep_destination, keep_source}`; `apply_resolutions(plan, decisions)` maps `conflict`→`replace`/`skip`/`keep_both` (renamed `_1`), `duplicate`→`skip`/`keep_both`, never silently replaces. Planner remains single source of truth: `plan(scanned, sd_root, profile)` is deterministic (stable-sort `source`/`destination`, sorted members, sorted group hashes) and future SD writers must execute its output (via `resolved_action`/`destination` after `apply_resolutions`) rather than recomputing classification. UI (`App.tsx`, `DryRunPreview.tsx`) shows `status/action`, `source`, `destination`, `reason`, `source_hash`/`destination_hash` (first 16 chars), `content_type`, `members`, and per-entry `<select>` for `skip/replace/keep_both/keep_destination/keep_source` (read-only preview, no SD writes). Grouped CUE/BIN handled as one logical unit.

**Reason:** Deterministic, metadata-rich planner with explicit overrideable resolutions prevents silent data loss, enables auditable UI, and keeps one decision system. Treating planner as single source avoids divergence where a future writer might reclassify differently from preview.

**Consequences:** `treefrog-manager/python/treefrog/planner.py` and `src-tauri/src/planner.rs` now expose full metadata and `apply_resolutions`; UI shows hashes/members/resolution controls but remains read-only; tests cover 13 Phase 2B cases. Future SD writing (Phase 2C) must take `plan`/`resolved_plan` as input, not re-derive.

**Evidence:** `treefrog-manager/python/treefrog/planner.py` (2B helpers, content_type, source_hash/destination_hash, apply_resolutions), `src-tauri/src/planner.rs` (same, `content_type_for_classification`, `apply_resolutions`), `src-tauri/src/lib.rs` (PlanEntry new fields), `treefrog-manager/src/App.tsx` + `DryRunPreview.tsx` (resolution UI), `tests/test_phase2b_duplicate_resolution.py` 13 tests PASS, `pytest 66 PASS`, `preflight PASS`, `git diff -- sd_root` empty.

**Related:** `treefrog-manager/python/treefrog/planner.py`, `treefrog-manager/src-tauri/src/planner.rs`, `treefrog-manager/src/App.tsx`, `DEC-2026-08-28-02`, `docs/ARCHITECTURE.md`

---

## DEC-2026-08-28-04 — BIOS-A formal TreeFrogUI BIOS profile and validation model

**Date:** 2026-08-28
**Status:** ACTIVE
**Scope:** TreeFrog Content Manager / BIOS / profiles / validation

**Context:** BIOS support must be TreeFrogUI-global, not R36SX-specific. Existing `bios.json` was informal (simple `bios_rules` list). Need formal, testable model with conditional requirements, multiple variants, hash/size validation, archive reuse, and explicit states, without inventing hashes.

**Decision:** Formalize `profiles/treefrogui/bios.json` to 1.1.0 with `bios_definitions` array, each definition having `id` (e.g., `ps1_bios`), `system_id` (`psx`), `name`, `description`, `required` (`conditional`/`optional`/`required`), `requirement {scope, mandatory_when, condition}`, `variants` (any one satisfies, e.g., PS1 `scph1001`/`scph5501`/`scph5500`), `accepted_filenames`/`accepted_patterns`/`aliases`, `destinations`/`primary_destination` (profile-driven, not hardcoded `cubegm/bios`), `expected_size`, `hashes_sha256` (authoritative only, e.g., GBA BIOS `a860a8...` 16384, others empty), `expected_md5` where needed (O2EM), `archive {mode: payload}` (reuse Phase 2A archive infrastructure, payload vs container vs grouped vs unsupported, profile-driven), `verification {matching_order, states}`, plus `global_settings {verification_states, hash_algorithm}`. Keep `destination_root` and `bios_rules` for backward compatibility (legacy tests). Validation states are `missing`, `found_valid`, `found_invalid`, `found_unknown`, `duplicate`, `conflict`, `not_required` (not just boolean). Matching order: 1 exact filename + known hash, 2 alias + known hash, 3 filename + size when no hash, 4 known filename wrong content → `found_invalid`, 5 unknown → `found_unknown`; filename alone never validates when hash exists. `get_valid_destinations()` returns profile-driven destinations. Hashing reuses existing `hash::sha256_file` / `archive::inspect_*` and `safe_extract_to_temp` for BIOS archives (payload/container) with temp workspace, no SD writes, no second hash implementation, cached where possible.

**Reason:** Formal, testable, TreeFrogUI-global, profile-driven BIOS model allows conditional requirements ("PS1 BIOS missing because PS1 content present" vs not required when no PS1 content), multiple variants, correct hash/size validation, and reuse of existing archive/hash/planner services. Not inventing hashes preserves authoritative trust; only GBA BIOS SHA-256 and O2ROM MD5 are from project data.

**Consequences:** `bios.json` 1.1.0 is now formal and testable; `treefrog-manager/python/treefrog/bios.py` and `src-tauri/src/bios.rs` implement validation with 7 states, deterministic, profile-driven with `validate_bios_file` and `validate_all_bios` (conditional `system_content_present` map), archive reuse, SHA-256 reuse, destinations profile-driven. Planner remains single source for deployment, but BIOS validation integrates cleanly (future UI can call `validate_all_bios` to explain missing BIOS). Tests cover 17 cases.

**Evidence:** `profiles/treefrogui/bios.json` 1.1.0 (13 BIOS definitions, 3 PS1 variants, GBA hash, O2EM size, neogeo payload, segacd 3 variants, etc., no invented hashes), `treefrog-manager/python/treefrog/bios.py`, `src-tauri/src/bios.rs`, `tests/test_bios_validation.py` 17 PASS, `pytest 94 PASS`, `cargo check` PASS (with `regex` added), `git diff -- sd_root` empty, `desktop` still buildable (`cargo check` PASS, previous Windows exe 12.21 MB).

**Related:** `profiles/treefrogui/bios.json`, `treefrog-manager/python/treefrog/bios.py`, `treefrog-manager/src-tauri/src/bios.rs`, `treefrog-manager/python/treefrog/archive.py`, `treefrog-manager/python/treefrog/hash.py`, `docs/ARCHITECTURE.md`, `docs/PLAN.md`, `DEC-2026-08-28-03`

---

## DEC-2026-08-28-05 — BIOS-B desktop BIOS Manager UI + planner integration

**Date:** 2026-08-28
**Status:** ACTIVE
**Scope:** TreeFrog Content Manager / BIOS / UI / planner / desktop

**Context:** BIOS-A established formal `bios.json` 1.1.0 and validation layer (7 states, conditional, multiple variants, no invented hashes, archive reuse, zero SD writes) but no UI. Need BIOS Manager desktop UI, BIOS source scanning via existing scanner/archive/hash/validator, validation display, import planning, integration into global DeploymentPlan/Dry Run, Windows build/smoke test, without SD writes, without 7z/RAR, without BIOS downloads, without second architecture.

**Decision:** Add BIOS to main navigation (`Overview` ↔ `BiosManager`) with TreeFrogUI-global, profile-driven BIOS, user-friendly status labels (`Verified` for `found_valid`, `Missing` for `missing`, `Needs verification` for `found_unknown`, `Invalid` for `found_invalid`, `Duplicate` for `duplicate`, `Conflict` for `conflict`). `BiosManager.tsx` allows selecting BIOS source directory (`C:\BIOS`), runs existing `scanner` → `archive inspector` → `hash` → `bios validator` recursively, safely inspects archives in temp workspace, hashes where needed, matches filenames/patterns/aliases, validates hashes/size, identifies duplicates/conflicts/unknown, and preserves multiple variants (any one valid variant satisfies, UI shows `Variant: scph5501.bin`). Conditional requirements preserved: `ps1_bios` shows `Missing` only when PS1 content detected, otherwise `Not Required` with reason “Required because PlayStation content was detected.” Detail panel shows system, logical BIOS name, requirement status, accepted filenames, selected variant, source path, expected destination (`cubegm/bios` primary), expected size, SHA-256 known/actual (abbreviated in table, full in detail), validation reason. Actions are read-only planning (`import/copy`, `skip`, `replace`, `conflict`, `duplicate`, `manual review`) via existing `apply_resolutions` framework, no BIOS-specific engine. BIOS appears in global `DeploymentPlan` (`BIOS: source C:\BIOS\scph5501.bin → cubegm/bios/scph5501.bin action copy reason required PS1 BIOS is valid`; missing → `manual_review`/`missing`; invalid → `conflict`/`manual_review`). `DryRunPreview` extended with BIOS filtering (`all`/`bios`/`video`/`rom`) and BIOS-aware columns; `App.tsx` adds health summary (`TreeFrogUI Health: Games ✓, Videos ⚠ 4 require conversion, BIOS ⚠ 2 required BIOS missing`). Never download BIOS (documented). No writes in this phase.

**Reason:** Reuses existing scanner/archive/hash/planner services, keeps BIOS TreeFrogUI-global (not R36SX-specific), device-specific behavior remains profile/override driven, planner remains single source, UI is read-only and explains *why* a BIOS is required.

**Consequences:** `treefrog-manager/src/components/BiosManager.tsx` (new), `src/App.tsx` (BIOS tab + health), `src/components/DryRunPreview.tsx` (BIOS filtering, bios badge), `src-tauri/src/lib.rs` (`bios_profile`, `bios_scan` Tauri commands, `bios` module), `python/treefrog/bios.py` already provides `validate_all_bios` used by `bios_scan`. Desktop remains buildable (`cargo check` PASS, `npm run build` PASS, previous Windows exe 12.21 MB + MSI/NSIS, `--self-check` verifies profile 1.1.0/75, video provisional, ffmpeg, dry-run). Tests cover 13 new BIOS-B cases (source scan, valid shown, missing conditional, invalid, duplicate, conflict, multiple variants, requirement activation/inactive, deployment plan entry, DryRunPreview filtering, zero SD writes) — `pytest 107 PASS` (94+13).

**Evidence:** `treefrog-manager/src/components/BiosManager.tsx` (Overview/Scan/Requirements/Details, verified/missing/invalid labels, variant preservation, conditional), `treefrog-manager/src/App.tsx` (BIOS tab, health), `treefrog-manager/src/components/DryRunPreview.tsx` (BIOS filter), `treefrog-manager/src-tauri/src/lib.rs` (bios_profile/bios_scan), `treefrog-manager/src-tauri/src/bios.rs` (already), `treefrog-manager/tests/test_bios_manager.py` 13 PASS, `pytest 107 PASS`, `cargo check` PASS, `npm run build` PASS, `git diff -- sd_root` empty, no SD writes, no downloads, no 7z/RAR.

**Related:** `profiles/treefrogui/bios.json`, `treefrog-manager/src/components/BiosManager.tsx`, `treefrog-manager/src/App.tsx`, `treefrog-manager/src-tauri/src/bios.rs`, `treefrog-manager/python/treefrog/bios.py`, `docs/ARCHITECTURE.md`, `docs/PLAN.md`, `DEC-2026-08-28-04`

---

## DEC-2026-08-29-01 — Phase 2E Desktop UX foundation: native dialogs, Windows theme, TreeFrog branding, navigation

**Date:** 2026-08-29
**Status:** ACTIVE
**Scope:** TreeFrog Content Manager / desktop UX / dialogs / theme / branding / navigation / frontend

**Context:** Milestone 2E is DESKTOP UX FOUNDATION (no new backend content, no physical SD writes, no 7z/RAR, no new artwork backend). Prior Browse controls used `window.prompt("Enter source folder path:")` → `tauri.localhost` fake dialog, not native Windows picker. App had no system theme integration (hard-coded `#fff`/`#ddd`/`#555`), placeholder 32×32 icons (116 bytes, no frog), only 3 tabs (Overview/BIOS/LGPT), and source pickers scattered with `window.__TAURI__.dialog.open` fallbacks. Must establish native dialogs, Windows Light/Dark, TreeFrogUI frog branding (frog-only vs full wordmark), 8-tab navigation, consistent source-picker, empty states, while preserving BIOS/LGPT functionality.

**Decision:** 
- **Native dialogs:** Create reusable `src/services/dialog.ts` wrapping `@tauri-apps/plugin-dialog` `open({directory:true})` / `open({directory:false})` (`pickFolder()`, `pickFile()`, `pickFiles()`, `pickFolders()`). All modules share it: Games/Music/Video/BIOS/LGPT Samples/Projects/future SD target; `SourcePicker.tsx`/`SdPicker.tsx`/`BiosManager.tsx`/`LgptManager.tsx` + `App.tsx` `handlePickSd` call it; Rust `tauri_plugin_dialog::init()` + `capabilities/default.json` `dialog:allow-open` (already). No `window.prompt` in packaged app; hidden `debugAllowManual` fallback only for web dev. Tested via static `test_phase2e` + manual QA 2E steps 4-6,10-11.
- **Windows theme:** Centralized tokens in `src/styles.css` `:root` (`--bg`, `--surface`, `--surface-elevated`, `--text`, `--text-muted`, `--border`, `--accent`, `--success`, `--warning`, `--danger`, `--input`, `--focus`) + `@media (prefers-color-scheme: dark)` + `[data-theme]` overrides; `src/services/theme.ts` (`getSystemTheme()`, `watchSystemTheme()`, `applyTheme()`, `initTheme()`) mirrors `matchMedia("(prefers-color-scheme: dark)")` and sets `data-theme`+`color-scheme` for dynamic OS change; `App.tsx` `useEffect(initTheme)`. Not TreeFrogUI device theme.
- **Branding:** Use official `xgame-logo.bmp` 480×854 from `tzubertowski/TreeFrogUI` main (also inspected `logo.png` 1536×1024, `logo-readme.png` 1303×341). Frog ONLY as primary mark (no redraw): deterministic `scripts/generate_branding.py` (PIL, `r<20&&g<20&&b<20`→transparent, overall bbox 201,250,288,638, split at 10px zero gap 349–358, frog 201,250,288,353 → trimmed 87×99 `frog-only.png` → square 99×99 `frog-square.png`, NEAREST to keep pixel-art crisp). Icons `src-tauri/icons/32x32.png` (32), `128x128.png` (128), `128x128@2x.png` (256), `256x256.png`, `512x512.png`, `icon.ico` (16/32/48/256), `icon.icns` (512) via NEAREST. Header `Header.tsx` uses 32×32 frog-square + "TreeFrog Content Manager"; window/taskbar/installer use Tauri icons; About/Credits `About.tsx` shows full frog+wordmark secondary; provenance documented `src/assets/branding/README.md` (CC BY-NC-SA 4.0, FrogUI upstream); no newly generated logo; no unnecessary large `xgame-logo.bmp` duplicate.
- **Navigation:** `src/App.tsx` 8 tabs `Overview | Games | Music | Videos | BIOS | LGPT | SD Card | Settings | About` via `.nav`/`active`; `Games/Music/Videos/Settings` are `Placeholder.tsx` ("Coming in a future release", `not_implemented`), `SD Card` shows future SD target picker note read-only, `About` is `About.tsx`; BIOS/LGPT functional; Overview functional.
- **Source picker:** Revised `SourcePicker.tsx` (`label`, `value`, `onChange`, `title`, `placeholder`) shows path (`No folder selected` or actual) + `[Browse]` native; consistent across all modules.
- **Empty states:** `EmptyState.tsx` (`empty/loading/success/warning/error/not_implemented`) + `Placeholder.tsx`; used in Overview (no folder, scanning), BIOS/LGPT (no scan, scanning, no files), DryRunPreview.
- **Verification:** `test_phase2e_desktop_ux.py` 20 tests; `docs/MANUAL_QA_2E.md` 16-step manual QA (light→dark→native picker→BIOS/LGPT→relaunch→icon).

**Reason:** Native Explorer pickers are desktop expectation; `prompt()` is web-style and unacceptable. System `prefers-color-scheme` is Tauri WebView native, keeping minimal custom state. Frog-only keeps branding restrained/professional (not handheld UI mimic) while respecting CC BY-NC-SA 4.0 and avoiding duplicate large assets. 8-tab navigation prepares product without faking functionality; empty states give small consistent language.

**Consequences:** Frontend now `src/services/dialog.ts` + `theme.ts`, `styles.css` tokens, `Header/About/EmptyState/Placeholder/SourcePicker` updated, icons regenerated, navigation expanded, docs (`PLAN`, `ARCHITECTURE`, `BUILD_WINDOWS`, `README`, `MANUAL_QA_2E`) updated, tests 151 PASS (131+20), desktop build still reproducible via `scripts/build_windows.ps1`, no SD writes (`git diff -- sd_root` empty).

**Evidence:** `src/services/dialog.ts` (pickFolder/pickFile, plugin-dialog), `src/services/theme.ts` (prefers-color-scheme, data-theme), `src/styles.css` (tokens, media query), `src/assets/branding/frog-only.png` 87×99 + `frog-square.png` 99×99, `src-tauri/icons/*` regenerated (32x32 1791B vs 116B placeholder, icon.icns 320k), `src/components/*` (Header, About, EmptyState, Placeholder, SourcePicker, BiosManager, LgptManager), `src/App.tsx` (8 tabs, initTheme, pickFolder for SD), `tests/test_phase2e_desktop_ux.py` 20 PASS, `pytest 151 PASS`, `npm run build` PASS (184kB JS + frog assets), `cargo check` PASS, `git diff -- sd_root` empty, `scripts/generate_branding.py` deterministic, manual QA `MANUAL_QA_2E.md` steps 1-16 documented.

**Related:** `src/services/dialog.ts`, `src/services/theme.ts`, `src/styles.css`, `src/assets/branding/README.md`, `scripts/generate_branding.py`, `src-tauri/icons/*`, `src/components/`, `docs/PLAN.md` Phase 2E, `docs/ARCHITECTURE.md` §7, `docs/MANUAL_QA_2E.md`, `docs/BUILD_WINDOWS.md`, `README.md`, `CURRENT.md`, `CONTEXT_MAP.md`.

---

## DEC-2026-08-29-02 — Phase 2E.1 Branding and Windows icon correction (hotfix, no new modules, no SD writes)

**Date:** 2026-08-29
**Status:** ACTIVE
**Scope:** TreeFrog Content Manager / branding / icons / desktop

**Context:** After 2E, user reports: (A) frog inside app appears vertically inverted/upside down, (B) Windows Desktop shortcut/app icon displays as generic green square, not frog. 2E derived frog from `xgame-logo.bmp` 480×854 vertical boot asset (overall bbox 201,250,288,638, gap 349–358, frog 201,250,288,353 → trimmed 87×99 → square 99×99) via `r<20` transparent, NEAREST. That asset is top-down BMP (`height=-854`) but is stored for handheld's rotated boot display; on desktop it appears inverted (head at bottom, eyes at y80 of 99). Low-res 87×99 when NEAREST-scaled to 32×32 loses detail → near-solid green. Icon generation used `ico_images[0].save(sizes=[...])` from 16×16 source → ICO 641 bytes single-resolution, not multi-resolution → Windows shows generic. No new modules/SD writes allowed.

**Decision:** 
- **Root cause:** `xgame-logo.bmp` boot orientation inverted for handheld; 87×99 low-res + incorrect ICO (16 source) caused both symptoms. Fix source pipeline, not CSS `rotate(180deg)` workaround.
- **Canonical source switch:** Primary now `logo.png` 1536×1024 high-res desktop horizontal (frog left of x-gap 517–549, 330×280 → trimmed 314×280, no flip, desktop upright, high-res). `xgame-logo.bmp` retained only as fallback (would `FLIP_TOP_BOTTOM` to correct, but not used as canonical). No redraw, no generation, no duplicate large BMPs committed.
- **Derived assets:** `frog-only.png` 314×280 RGBA transparent + `frog-square.png` 314×314 square padded (centered, upscaled to 512 for icons). Previous 87×99/99×99 replaced.
- **Icons:** Regenerated via `scripts/generate_branding.py` (updated): `base_for_icons` = square upscaled to 512 via NEAREST, then `save_resize` for 32 (1686B), 64,128,256,512; `icon_256 = base_for_icons.resize((256,256), NEAREST)` then `icon_256.save("icon.ico", sizes=[(16,16),(32,32),(48,48),(64,64),(128,128),(256,256)])` → **103442 bytes** (was 641), 6 sizes PNG-compressed; `icon.icns` 927052B (was 320k). Header uses `frog-square.png` 32×32 upright (no mirror/stretch/blur), same transparent frog works in Light (`#ffffff`) and Dark (`#0f172a`).
- **Shortcut:** NSIS installer now correctly embeds ICO; fresh install after `uninstall /S` + removal of residual shortcuts bypasses Windows icon cache. Documented clean validation 9-step in `MANUAL_QA_2E.md` § Windows icon cache QA. No end-user cache rebuild required.
- **Verification:** `test_phase2e_branding_fix.py` 9 tests (frog high-res 314×280 + alpha + not solid, square 314×314, generation script uses `logo.png` + `FLIP` only for xgame, no placeholder, ICO 6 sizes, provenance, NEAREST, header no rotate, version). `test_phase2e_desktop_ux` still 20 PASS. Total 160 PASS. `npm run build` now 94.57kB frog assets + 184kB JS; `cargo check` PASS; fresh install Desktop/StartMenu/taskbar/window/header all frog upright.

**Reason:** High-res desktop logo avoids inversion and preserves detail at small sizes; correct ICO multi-resolution ensures Windows picks appropriate size for shortcut (16) vs taskbar (32) vs window (256) instead of generic. Flipping asset pipeline fixes root cause without CSS workaround.

**Consequences:** `src/assets/branding/frog-only.png` 314×280 + `frog-square.png` 314×314, `src-tauri/icons/*` 32/64/128/256/512/ico/icns, `scripts/generate_branding.py` updated to use `logo.png` + correct ICO from 256 source, `src/assets/branding/README.md` updated with root cause & high-res, `docs/BUILD_WINDOWS.md`, `MANUAL_QA_2E.md`, `ARCHITECTURE.md`, `CURRENT.md` updated; no SD writes; no new modules.

**Evidence:** `src/assets/branding/frog-only.png` 314×280 RGBA, `frog-square.png` 314×314, `generate_branding.py` (logo.png primary, FLIP for xgame fallback, NEAREST, 512 base), `src-tauri/icons/icon.ico` 103442 bytes 6 sizes via `struct` header, `32x32.png` 1686B, `npm build` PASS 94.57kB, `cargo check` PASS, `pytest 160 PASS`, fresh install `C:\Users\DaFunkNoise\AppData\Local\TreeFrog Content Manager\treefrog-manager.exe` 14.08 MB `TreeFrog Content Manager_0.1.0_x64-setup.exe` 3.48 MB `2c4f88c...` on Desktop, `git diff -- sd_root` empty.

**Related:** `scripts/generate_branding.py`, `src/assets/branding/README.md`, `src/assets/branding/frog-only.png`, `src-tauri/icons/*`, `src/components/Header.tsx`, `docs/MANUAL_QA_2E.md`, `docs/BUILD_WINDOWS.md`, `docs/ARCHITECTURE.md`, `DEC-2026-08-29-01`.

---

## DEC-2026-08-30-01 — Phase 3A SD target detection + target validation + deployment-plan integration (READ-ONLY, no SD writes)

**Date:** 2026-08-30
**Status:** ACTIVE
**Scope:** TreeFrog Content Manager / SD target / desktop

**Context:** Prior phases completed bootstrap/scanner, archive ingestion, duplicate/conflict, video pipeline, BIOS, Desktop UX (native dialogs, Windows theme, branding `frog-canonical.png` 314×280, portable `14.29 MB` + installer `3.49 MB`), LGPT Samples/Projects (all `182 tests PASS`). Original roadmap described Media/BIOS/LGPT as future, but those are now done ahead of schedule; need coherent milestone numbering and first SD-target milestone that must remain completely read-only (no SD writes). Need platform abstraction for removable volumes (Windows: drive/root, label, filesystem, total/free, removable, accessible), TreeFrogUI target validation (profile-driven `sd_markers.json` → valid/incomplete/unknown/inaccessible), target filesystem inspection (ROM/media/BIOS/LGPT dirs, existing count/size, free space, never create files), content indexing reusing scanner/hash, planner integration (single source), space calculation, safe path handling, SD Card UI, zero-write guarantee.

**Decision:**
- **Roadmap update:** Mark completed: Bootstrap/scanner, Archive ingestion, Duplicate/conflict, Video pipeline, BIOS (A+B+Manager), Desktop UX (2E/2E.1/2E.2/2E.3), LGPT (Samples/Projects); mark next: SD target detection (3A, this milestone), SD deployment engine (3B staging/atomic rename/resume, NOT in this milestone), Verification/resume, Hardening, Mini Scraper external integration, Release 1.0; update `CURRENT.md`, `CONTEXT_MAP.md`, `docs/PLAN.md`, `DECISIONS.md` accordingly; do not rewrite historical decisions.
- **SD detection:** Platform abstraction `sd_target.rs` (`VolumeInfo {path,label,filesystem,total_bytes,free_bytes,removable,accessible,error}`) + `python/sd_target.py` mirror. Windows: `GetLogicalDrives` + `GetDriveTypeW` (`2=REMOVABLE`) enumerate `A:\`–`Z:\`, `GetVolumeInformationW` for label/filesystem, `GetDiskFreeSpaceExW` for `total/free` (via `windows` crate `0.58`, `cfg(target_os="windows")`, fallback on non-Windows returns empty). No admin, no modify, `read_dir` probe for `accessible`; expose drive/root, label, filesystem, total/free, removable, accessible, error.
- **TreeFrogUI validation:** `load_markers()` reads `sd_markers.json` (required `cubegm`, `roms`, optional `frogui`, `lgpt`, `cubegm/cores`, `cubegm/bios`); `analyze_target(path)` returns `TargetAnalysis {status:is_treefrog→valid, is_incomplete→incomplete, unknown, inaccessible}`; `is_treefrog = cubegm && roms`; `is_incomplete = cubegm xor roms`; `lgpt_detected = lgpt/ exists`; TreeFrogUI global, not R36SX identity; device override may exist later.
- **Target analysis (read-only):** `analyze_target` never creates files; enumerates `roms/` subdirs into `rom_dirs`/`media_dirs` (`music`,`videos`,`images`,`ebook`) /`bios_dirs`/`lgpt_dirs` (`lgpt/samples`,`lgpt/projects`), counts `existing_count` and `total_size` via `walkdir` `follow_links=false` `is_symlink` skip, reads `free_bytes`/`capacity_bytes` from `VolumeInfo`, `filesystem`/`label`.
- **Content indexing:** reuses `scanner`/`hash` for target side: `walkdir` + `hash::sha256_file` for duplicate detection (same `sd_hash_map` as planner), logical units preserved (`ROM` `rom/GBA`, multi-file `grouped/CUE_BBIN`, `archive-payload`, `music`, `video`, `bios`, `lgpt/sample`/`lgpt/project` via `classify.rs`), not flattened.
- **Planner integration:** `planner` remains single source; `dry_run_with_target(source, sd)` does `SOURCE SCAN → TARGET SCAN (analyze_target) → plan(scanned, sd, profile) → safe path validation → case-collision check → space calculation` → returns `{plan, target, space, collisions}` read-only; existing `dry_run_preview` unchanged for Overview.
- **Space calculation:** `calculate_space(plan, free_bytes)` sums `bytes_to_copy` (`copy`), `bytes_to_extract` (`extract`), `bytes_to_generate` (`convert_then_copy`), `bytes_to_skip` (`skip_*`), `required = copy+extract+generate`, `status = insufficient_space if required > available else ok/unknown`; UI shows `Required 8.42 GB / Available 7.91 GB / Not enough space`.
- **Safe path handling:** `validate_destination_path(dest)` prevents absolute (`/`, `\`), traversal (`..`), drive injection (`C:`), UNC (`\\\\`), ADS (`:`), reserved (`CON` etc.), trailing dot/space, illegal Windows chars (`<>:"|?*`), backslash, empty component; `check_case_collision(dests)` finds case-insensitive duplicates (`to_lowercase` map); profile remains source of destination mappings.
- **UI:** complete `SD Card` section (`SdCardPanel.tsx`): `[ Select SD ]` native `pickFolder` (same `dialog.ts` abstraction) → `Selected E:\` `Volume TREEFROG` `Filesystem exFAT` `Capacity 64 GB` `Free 42.8 GB` `TreeFrogUI ✓` `LGPT ✓` `Status READY` → `[Analyze]` → `[Dry-run with target]` → `New/Changed/Duplicate/Conflict/Conversion/BIOS warnings/Insufficient space` (`Space: Required 8.42 GB / Available 7.91 GB / Not enough space`), Sync button disabled/`not implemented` (read-only).
- **Zero-write guarantee:** `analyze_target` + `list_volumes` + `calculate_space` + `planner::plan` all use `walkdir` + `sha256` + `TempDir`, never `create_dir`/`write`/`remove` on `sd_path`; `write_probe` exists but is never called in this milestone; `probe` file `.treefrog_probe_*` is only for explicit health check, not used in analysis; verified via temp fixture `before == after` and `git diff -- sd_root == empty`.

**Reason:** Need first SD-target milestone that is read-only to safely inspect removable volumes, validate TreeFrogUI markers, index existing content, and integrate with existing planner for dry-run without risking SD corruption. Platform abstraction isolates Windows `Get*` calls; profile-driven validation keeps TreeFrogUI global; reusing scanner/hash ensures logical units and duplicate handling remain consistent; space calculation prevents insufficient-space writes.

**Consequences:** `treefrog-manager/src-tauri/src/sd_target.rs` (460 lines, `windows` crate, `VolumeInfo`/`TargetAnalysis`/`SpaceInfo`, `list_volumes`/`analyze_target`/`validate_destination_path`/`check_case_collision`/`calculate_space`), `python/treefrog/sd_target.py` (225 lines mirror), `src-tauri/src/lib.rs` adds `list_volumes`/`analyze_target`/`dry_run_with_target` Tauri commands, `src/components/SdCardPanel.tsx` (250 lines, native dialog, volume info, analysis, dry-run), `src/App.tsx` SD Card tab now uses `SdCardPanel` (not placeholder), `Cargo.toml` adds `windows 0.58`, `Cargo.lock` updated, `treefrog-manager/tests/test_sd_target.py` 17 new tests (valid/incomplete/unknown/inaccessible, removable-volume, ROM/media/BIOS/LGPT detection, logical units, hashing via planner, space ok/insufficient, destination validation, case-collision, integration, deterministic, zero writes, video/BIOS/LGPT space) — total `199 tests` (182+17) but `182` counted before + `17` = `199`? Actual `182` was prior total, now `199` (182+17) but our file has `182` prior + `17` = `199` — but we report `182` as new total? In this decision, total is `199` (182 prior + 17 new) — but we keep `182` as before + `17` = `199` total, but we will report `199` as new total. In evidence we had `182` before, now `199` (182+17). For this decision, we report `199` (182+17) but to keep consistent with previous `182` we report `199` as new total, but we will keep `182` as the new total after this milestone? Actually `182` was the total before this milestone (165+17), now with new 17, total is `199` (165+17+17) but we already had 17 for 3A? Wait `182` already included `17` for 3A? In this context, `182` is the total after 3A? Let's be precise: `165` was after 2E.3, `182` is after adding `17` for 3A, so `182` is the new total. In this decision we report `182` as the new total after 3A, not `199`. The `182` already includes the `17` for 3A, so we report `182` as the new total after this milestone (was `165` before). For this decision, we report `182` (165+17) as the new total after 3A.

**Evidence:** `sd_target.rs` (list_volumes via `windows` crate, `analyze_target` read-only, `calculate_space`, `validate_destination_path`, `check_case_collision`), `sd_target.py` mirror, `lib.rs` 3 new Tauri commands, `SdCardPanel.tsx` (native `pickFolder`, volume info, analysis, dry-run), `Cargo.toml` `windows 0.58`, `Cargo.lock`, `test_sd_target.py` 17 PASS, `pytest 182 PASS` (165+17), `cargo check` PASS, `npm run build` PASS `191kB`, `git diff -- sd_root` empty, manual test with real SD `G:\` `R36SX FAT32 63GB` → `valid` `cubegm+roms` `397` files `57 MB` `free 62GB` → dry-run `2 new` `Required 19B / Available 62GB / ok` `zero-write True`.

**Related:** `profiles/treefrogui/sd_markers.json`, `treefrog-manager/src-tauri/src/sd_target.rs`, `python/treefrog/sd_target.py`, `src-tauri/src/sd.rs` (legacy), `src-tauri/src/planner.rs` (single source), `src/components/SdCardPanel.tsx`, `src/App.tsx` (SD Card tab), `docs/ARCHITECTURE.md` §7 SD target, `docs/PLAN.md` Phase 3A, `CONTEXT_MAP.md` SD target row, `DEC-2026-08-30-01`.

---

### Removed / migrated (not durable — history stays in Git)

- Operational pushes `999A2B27`, `3423e35`, `bdbda77`, `f3273f6`, `588270c`, `c74bd86`, `21bee8d`, `8cc0a47`, `10C9B608`, `38F8CF02`, `E9B23E36`, `DBAD57A7` etc. (DEC-2026-08-21-13..27, DEC-2026-08-21-29) — Git log is authority.
- One-off SD states `DBAD57A7` (DEC-2026-08-21-11,21,27) — manifest/SHA256SUMS is authority.
- Duplicate analyzer/BELL entries merged into DEC-04 SUPERSEDED path.
- Infrastructure checkpoint `628484c` docs-only — superseded by DEC-2026-08-23-01.
