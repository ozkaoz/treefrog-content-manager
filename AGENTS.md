# AGENTS.md — Project Constitution

**Version:** 2.2
**Date:** 2026-09-01
**Repo:** https://github.com/ozkaoz/treefrog-content-manager
**Scope:** Multi-agent operating contract for the TreeFrog Content Manager (global TreeFrogUI SD manager)

> Permanent constitution. Not history, not current state. Provider-neutral.
> Mutable values (branch, HEAD, SHA, SD hash) belong in CURRENT.md or Git — never here.
> This file defines startup, evidence, safety, validation, and context routing for all agents.

Architecture:
```
AGENTS.md (constitution) ─┬─ CURRENT.md (snapshot)
                          ├─ CONTEXT_MAP.md (router)
                          ├─ DECISIONS.md (durable)
                          └─ docs/ai/{VALIDATION,RELEASE_CONTRACT}.md
```

---

## 1. Startup Protocol

Every session MUST:

1. Read `AGENTS.md` then `CURRENT.md` then `CONTEXT_MAP.md`.
2. Run preflight: `bash scripts/agent_preflight.sh` (or `python3 tests/test_agent_context_contract.py`).
3. Resolve `REPO_ROOT`, `ACTIVE_BRANCH`, `HEAD`, `UPSTREAM`, `AHEAD_BEHIND`, `WORKTREE_STATE` from **Git directly** — never trust hardcoded docs.
4. Confirm objective and change class (Section 4) before editing.

### 1b. Local Environment (2026-09-01 reorganization)

| Qué | Dónde |
|-----|-------|
| **Worktree activo de build** | `C:\Users\DaFunkNoise\Documents\Default Project\lgpt-r36sx-port` (remote = este repo; node_modules + cargo target listos para compilar) |
| **Clon de referencia** | `D:\GitHub\treefrog-content-manager` (sincronizado con origin/main) |
| Node portable (builds) | `C:\Users\DAFUNK~1\AppData\Local\Temp\opencode\node-portable\node-v22.14.0-win-x64` |
| Validation local mínima | `scripts/quick_validate.ps1` (DEC-2026-09-01-02: checks proporcionales por clase de cambio; CI es el gate completo) |
| Releases | GitHub Releases del repo — v1.0.1 = Latest (3 ejecutables por SO + source). El flujo: tag `v*` → build 3 OS → `--self-check` (BIOS catalog verificado) → publish Latest. |

If unexplained local modifications exist: STOP, report `git status --short --branch` before editing.

## 2. Source-of-Truth Hierarchy

```
1. Explicit current user requirement
2. Permanent AGENTS.md invariant
3. ACTIVE durable decision (DECISIONS.md)
4. Direct current evidence (Git, build, filesystem, device, release asset)
5. CURRENT.md snapshot  ← CACHE, may be stale
6. Structural docs (CONTEXT_MAP, docs/ai/*)
7. Historical docs (CHANGELOG, old docs)
```

Rule: `CURRENT.md IS A CACHE`. If CURRENT contradicts direct evidence, direct evidence wins — repair CURRENT first. DO NOT TRUST HARDCODED HISTORICAL STATE.

## 3. Permanent Invariants

```
TARGET        = R36SX (TreeFrogUI, kernel 4.4.186-release)
AUDIO_RATE    = 48000 Hz (TreeFrogAudio.cpp / TreeFrogLibretro.cpp)
AUDIO_CHANNELS= 2 Stereo (SubmitStereo48000 / MixUsbCaptureMonitorStereo48000)
CURRENT_CORE_IDENTITY     = resolve from current Source/Physical/Release Golden evidence (CURRENT.md, docs/BACON_1_5_RELEASE_MANIFEST.md, LGPT_R36SX_Bacon-1.5_SHA256SUMS.txt, Git)
CURRENT_RELEASE_IDENTITY  = resolve from authoritative release manifests (ZIP name = authoritative SHA; see docs/ai/RELEASE_CONTRACT.md)
```

No MONO fallback, no 44100 resampling, no unapproved H36 APK without explicit DECISION.

## 4. Change Classification

| Class | Scope | Gate |
|-------|-------|------|
| **A** Context/Docs | AGENTS, CURRENT, CONTEXT_MAP, DECISIONS, README, agent defs | static review + `test_agent_context_contract.py` |
| **B** Host tooling/tests | audit scripts, host-only tests | static + relevant host tests |
| **C** Runtime | C/C++, audio, input, DSP, TreeFrog, Mixer/EQ/Analyzer | static + build + host tests + **physical R36SX** |
| **D** Deployment/bootstrap | launcher, OTG setup, `sd_root` persistent baseline | package checks + **physical clean-install** |
| **E** Release | public ZIP, SHA256SUMS, install contract | deterministic build + `CLEAN-INSTALL PHYSICAL PASS` + **download-back SHA equality** |

Physical test never inferred. Labels: `STATIC PASS | HOST PASS | PACKAGING PASS | PHYSICAL PASS | CLEAN-INSTALL PHYSICAL PASS | DOWNLOAD-BACK PASS`.

## 5. Golden State Model

- **SOURCE GOLDEN:** repository baseline that builds the validated core (see `docs/BACON_1_5_GOLDEN_BOOTSTRAP_PHYSICAL_PASS.md`).
- **PHYSICAL GOLDEN:** exact payload installed on R36SX and physically validated (LOCAL/WINDOWS/SP404/ANDROID matrix PASS).
- **RELEASE GOLDEN:** artifact that was clean-installed, physically passed, published, downloaded back, and `REMOTE_SHA == LOCAL_SHA` (`BACON_1_5_RELEASE_MANIFEST.md`).

`WORKS ON DEV SD != RELEASE PACKAGE COMPLETE` — never infer one from the other.

## 6. Release & Filesystem Safety

- **Install contract:** `Stock OS + TreeFrogUI + ZIP contents to SD root = fully functional PORT`, `POST_INSTALL_MANUAL_FIXES=0`. See `docs/ai/RELEASE_CONTRACT.md`.
- **Persistent baseline** (may be packaged): `enable_lgpt_uac2_bridge` (empty), `audio_usb_profile=STEREO_48K`, `audio_driver_mode/policy=LOCAL_CONSOLE`, `active_audio_branch=audio_driver_local_console`, `branches/.../MODE=LOCAL_CONSOLE`.
- **Volatile** (MUST NOT be packaged): FIFO, PID, daemon_pid, daemon_version, capture_abi, setup_result, sp404_card, aoa state, /tmp, runtime logs.
- **SD health:** before blaming runtime, verify mount healthy/writable/not read-only. Dirty exFAT caused false USB/bootstrap failures. `Detection != Runtime READY != PCM flow != physical PASS`. Never auto-run fs repair — requires explicit user authorization.
- **Kernel lifecycle:** `CONFIG_MODULE_UNLOAD=n` observed — do not assume hot-swap of ALSA families without evidence.

## 7. Golden Protection (Permanent)

- Current PHYSICAL/RELEASE GOLDEN must not be modified incidentally. CLASS A/B tasks must produce `SD_ROOT_CHANGED=NO` / `CORE_CHANGED=NO` / `RUNTIME_CHANGED=NO` (verify `git diff -- sd_root`).
- A CLASS C/D/E task may intentionally modify runtime/deployment/release only when explicitly approved, scoped, and its validation gate (Section 4 + docs/ai/VALIDATION.md) is followed.
- No rebuild/replace of release ZIP, SD deploy, or core/audio/daemons/APK/MUSB/TreeFrogUI mutation without explicit approval and evidence.

## 8. Git & Checkpoint Rules

- One canonical checkout per worktree; `git status --short --branch` is authority.
- Checkpoint eligibility is class-aware:
  - **A (docs):** context/static validation.
  - **C (runtime):** `PHYSICAL PASS` required.
  - **D (deployment):** `physical deployment PASS`.
  - **E (release):** `CLEAN-INSTALL + DOWNLOAD-BACK PASS`.
- Never `NO CHECKPOINT BEFORE SD PASS` as blanket — apply per class. Never describe inferred results as physical.
- No `rm -rf / reset --hard / clean -fd` without non-destructive audit.

## 9. Context Maintenance

- `CURRENT.md` = concise operational snapshot (60-120 lines), not changelog. History belongs in Git/CHANGELOG/evidence docs.
- `CONTEXT_MAP.md` = stable router (what to read/build/test for subsystem X). No mutable branch/HEAD/SHA.
- `DECISIONS.md` = durable decisions only (`ACTIVE|SUPERSEDED|DEPRECATED`), with ID/Date/Status/Scope/Context/Decision/Reason/Consequences/Evidence/Related files. No `pushed commit X / copied to G:` events.
- `scripts/install.sh` and `scripts/verify.sh` = **LEGACY U2523, NOT CANONICAL for Bacon-1.5** — do not use for current deployment without separate audit.
- Keep docs DRY; do not duplicate invariants everywhere — reference AGENTS.md.

## 10. Stop Conditions

MUST stop and request human intervention when: regression appears, unexpected dependency, evidence contradicts hypothesis, scope creep, insufficient evidence, checkpoint needs human, or physical validation is required but unavailable. Do NOT auto-fix beyond scope.
Prohibited auto-continuation after `STOP`: do not chain fix B after fix A without re-entering `C0` context and re-validating.

## 11. Handoff Contract

Compact handoff (not append-only log):

```
CHANGE_CLASS=  FILES_CHANGED=  HEAD=  CHECKS_RUN=  PHYSICAL_EVIDENCE=  RELEASE_EVIDENCE=  BLOCKER=  NEXT_EXACT_ACTION=  STOP_CONDITION=
```

Raw logs go to dedicated evidence files, not CURRENT.md.
Handoff must be reproducible from Git + evidence files alone — no chat-context dependency.

## 12. Lazy Context Loading

```
Always: AGENTS.md, CURRENT.md
Then:   CONTEXT_MAP.md
Only when needed:
  DECISIONS relevant to subsystem
  docs/ai/VALIDATION.md  (validation gates)
  docs/ai/RELEASE_CONTRACT.md (packaging/release)
  hardware evidence (only for hardware/release work)
```

Do not force every session to read full history. Evidence before synthesis.

## 13. Release Identity & Validation Routing

- One artifact name = one authoritative SHA. `GitHub release body == SHA256SUMS == manifest == included-files == downloaded asset`. Historical SHAs must be marked historical.
- After publishing: `DOWNLOAD-BACK IS REQUIRED` and `REMOTE_SHA == LOCALLY_VALIDATED_SHA`.
- Release assets: exactly ONE executable per OS (Windows portable `.exe` / Linux `.AppImage` / macOS `.dmg`) + auto-generated source. No setups, no `.sha256` sidecars, no extra binaries — per v1.0.1 contract.
- Every shipped executable must pass `--self-check` (profile, systems, ffmpeg detection, **embedded BIOS catalog** — a portable binary with an empty BIOS section can never ship).
- Validation routing per Section 4; detailed procedures in `docs/ai/VALIDATION.md`. Never use ambiguous `DONE/VERIFIED` without class-appropriate gate.
- Provider neutrality: `AGENTS.md` is canonical. `CLAUDE.md / GEMINI.md / Copilot` etc. must be tiny routers pointing here if they exist — never duplicate full policy.

## 14. Permanent Rule

> Evidence has priority over plan. If new evidence contradicts written context, update context FIRST, adjust hypothesis, and continue only from the next valid checkpoint.
> Compiling is not validating. Static PASS + Host PASS never equals Physical PASS.
> Context files are part of the product — keeping them accurate is engineering work.

## 15. Desktop Application Definition of Done (TreeFrog Content Manager)

TreeFrog Content Manager is a **DESKTOP APPLICATION** for managing TreeFrogUI SD content globally. A milestone is **NOT** complete merely because unit tests pass.

Every future milestone MUST include:

1. implementation (Rust/Tauri + React + profiles)
2. automated tests (`pytest treefrog-manager/tests`, `test_agent_context_contract`, `test_release_audio_bootstrap`, plus new feature tests)
3. documentation/context updates (`CURRENT.md`, `CONTEXT_MAP.md`, `DECISIONS.md`, `docs/PLAN.md`, `docs/ARCHITECTURE.md`, `docs/BUILD_WINDOWS.md` where applicable)
4. **Windows x64 desktop build** (`scripts/build_windows.ps1` or `scripts/build_windows.sh` + `npx tauri build`) producing `treefrog-manager/src-tauri/target/release/treefrog-manager.exe` and bundles
5. **executable smoke test** (`--self-check` + GUI launch, profile load, source scan, dry-run with video inspection, no SD writes)
6. reproducible build instructions (`docs/BUILD_WINDOWS.md`, `scripts/build_windows.*`)
7. focused Git commit (`feat(manager): Phase ...`) and push to `origin/main`

**First supported desktop target:** Windows x64 (MSVC + WebView2). WSL cross-compilation is **not** equivalent to a verified Windows build — `scripts/build_windows.sh` documents the limitation and delegates to Windows-native `build_windows.ps1`.

No milestone may be marked complete without a tested Windows desktop build. `cargo check` or `npm run build` alone is insufficient.

**Global profile invariant (from DEC-2026-08-28-01):** `TreeFrog Content Manager → TreeFrogUI profile → optional device-specific overrides`. Do NOT create console-specific managers (R36SX manager, SF3000 manager). R36SX is a target/device, not the application identity. Keep `systems`, `media`, `archive`, `BIOS`, `video` data-driven in `profiles/treefrogui/`.
