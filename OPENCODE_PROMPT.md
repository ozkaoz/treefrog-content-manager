# OpenCode implementation prompt

You are implementing **TreeFrog Content Manager**, a desktop application that manages TreeFrogUI SD-card content globally across all TreeFrogUI-supported handhelds. The repository contains `AGENTS.md`, `CURRENT.md`, `CONTEXT_MAP.md`, `DECISIONS.md`, profiles under `profiles/treefrogui/`, scoped agents under `.opencode/agents/`, and the development plan in `docs/PLAN.md`.

## Before changing code

1. Read `AGENTS.md`.
2. Read `CURRENT.md`.
3. Read `CONTEXT_MAP.md`.
4. Read only the decision records relevant to the current task.
5. Inspect the live upstream TreeFrogUI sources/releases and LGPT R36SX repository before finalizing assumptions. Use direct evidence over stale notes.
6. Run the repository preflight/tests when available.

## Product requirements

Build a **global TreeFrogUI content manager**, not a different manager per console/device.

The app must manage:

- ROMs / game files
- music
- videos
- images
- ebooks
- BIOS / firmware files
- LGPT R36SX samples
- LGPT R36SX projects
- incremental SD synchronization

### Artwork

Do NOT implement another scraper or artwork service.

TreeFrogUI already recommends Mini Scraper for box art. Keep Mini Scraper as the external artwork solution. The app may provide an integration button/open action and optionally verify `.res` artwork after the scrape, but it must not build a second artwork backend.

### TreeFrogUI device independence

Treat TreeFrogUI content as one global schema. Do not fork the application into R36SX/SF3000/R36HD-specific ROM managers.

Device-specific logic is limited to:

- SD detection/markers
- optional capability/validation checks
- installation or release differences that are actually evidenced

Folder mappings, media destinations and BIOS rules must live in declarative profiles.

### ROM classification

Scan arbitrary source libraries recursively.

Classify by profile + extension/content hints. Do not rely solely on filenames.

Support multi-file game sets where applicable (for example CUE/BIN) and preserve them as groups.

Use the current TreeFrogUI mapping as the authoritative source of folder aliases and case-sensitive names. The manager must be able to support the full profile, not just the bootstrap examples.

### Archives

Recognize common archives such as ZIP, 7z and RAR.

Before copying an archive:

1. Inspect entries.
2. Determine whether the archive itself is an expected runtime payload for the target system.
3. If the profile says the archive is a valid runtime asset, copy it intact.
4. Otherwise extract supported contents into the canonical TreeFrogUI destination.
5. Handle nested archives only within a bounded, explicit policy.

Safety requirements:

- prevent `../` traversal
- prevent absolute extraction paths
- handle symlink/reparse-point hazards safely
- detect name collisions
- enforce extraction-count and expansion-size limits
- never overwrite a different file silently

### Duplicate handling

A duplicate means the same content, not merely the same filename.

Preferred algorithm:

1. compare obvious cheap metadata first;
2. use SHA-256 when an exact identity decision is required;
3. classify:
   - same path + same hash -> unchanged
   - different path + same hash -> duplicate content, default skip
   - same path + different hash -> conflict
   - new path + new hash -> copy

Never delete source-library files because a duplicate was detected.

### BIOS menu

Add a dedicated BIOS management view.

The profile must define:

- system/core
- destination path
- accepted filename patterns
- expected size/hash when known
- required/recommended status
- region variants where applicable

UI should support:

- discover
- verify
- import
- replace
- backup current
- reveal destination

BIOS files are user-supplied only. Do not download or bundle copyrighted BIOS files.

### Video handling

The app must not assume that a `.mp4` or `.mkv` is compatible merely because the extension appears in TreeFrogUI docs.

Inspect videos using `ffprobe` (or a robust equivalent) for:

- container
- video codec
- codec profile/level when available
- pixel format / bit depth
- dimensions
- frame rate
- audio codec / sample properties
- stream count where relevant

When a video is compatible with the active profile, copy it directly.

When it is incompatible, automatically convert it with FFmpeg.

Conversion requirements:

- keep original source untouched
- stage output in a temporary location
- re-probe output after conversion
- only deploy output after successful validation
- show progress and errors
- support batch jobs
- support cancellation without leaving corrupt final files
- make presets declarative

Create a conservative default TreeFrogUI video preset, but mark it `PROVISIONAL_UNVALIDATED` until it is physically validated on an R36SX. Do not claim hardware compatibility from source assumptions alone.

### Music / images / ebooks

Use current TreeFrogUI media locations and formats from the profile. Preserve music subfolders because TreeFrogUI treats each folder under `roms/music` as a playlist.

### LGPT

Provide a global LGPT integration profile for the user's R36SX port.

Initial destinations:

- samples -> `lgpt/samples`
- projects -> `lgpt/projects`

Do not hardcode these in UI code. Verify exact paths against the latest Bacon release payload before finalizing.

Projects should be copied as groups/directories, not flattened.

Samples should support audio metadata/preview where practical and exact duplicate detection.

### SD synchronization

The user must see a dry-run plan before destructive writes.

Example:

```text
2,331 unchanged
34 new
12 changed
7 duplicate content
3 conflicts
0 deletions
```

Normal Sync must not delete destination files. Deletion is an explicit separate action.

Use staging + atomic rename where supported.

Interrupted operations should resume or leave a consistent state.

### Persistent library index

Use SQLite (or a similarly robust local store) for:

- source libraries
- known SD targets
- content fingerprints
- previous deployments
- profile version
- tool version
- job history

Never commit user paths or personal library metadata.

## Recommended implementation stack

Use:

- Tauri 2
- Rust backend
- React + TypeScript frontend
- SQLite
- serde + versioned declarative JSON profiles
- SHA-256 hashing
- FFmpeg/ffprobe adapter
- maintained archive libraries/tool adapters for ZIP/7z/RAR

Windows is the first supported desktop platform. Keep the core filesystem layer portable for later macOS/Linux support.

## Required repository work

Before implementing features, create/maintain:

- `AGENTS.md` (already bootstrapped; preserve its constitution role)
- `CURRENT.md`
- `CONTEXT_MAP.md`
- `DECISIONS.md`
- `.opencode/agents/audit.md`
- `.opencode/agents/implement.md`
- `.opencode/agents/review.md`
- `.opencode/agents/release.md`
- profile files under `profiles/treefrogui/`
- tests/fixtures for archives, duplicates, media, BIOS and LGPT

`AGENTS.md` must remain similar in *function* to the user's LGPT port constitution: startup protocol, source-of-truth hierarchy, change classes, safety rules, context maintenance, stop conditions and handoff contract.

## Suggested phases

### Phase 0

Bootstrap app shell, profile loader, CI, fixtures, agent contract tests.

### Phase 1

Scanner + TreeFrogUI classification + archive inspection + duplicate engine + dry-run planner.

### Phase 2

SD detection + sync execution + progress + conflict handling + resume.

### Phase 3

Music/images/ebooks + video probe/conversion pipeline.

### Phase 4

BIOS manager.

### Phase 5

LGPT samples/projects.

### Phase 6

Mini Scraper launcher + artwork verification.

### Phase 7

Hardening, packaging, large-library performance and release QA.

## Quality gates

For every meaningful change:

- add or update tests
- run formatter/linter/build
- run targeted unit/integration tests
- run relevant fixture tests
- update `CURRENT.md` only for mutable state
- update `DECISIONS.md` only for durable decisions
- update `CONTEXT_MAP.md` when subsystem routing changes

Never claim:

- hardware compatibility without physical evidence
- video preset compatibility without target-device validation
- a clean release without running the release validation gates

## First implementation task

Do not jump straight into every feature.

Start by creating the Tauri/Rust/React application shell plus the domain/profile layer, then implement the scanner and dry-run planner. The first demonstrable milestone should be:

> Select a source folder + select a TreeFrogUI SD + scan + preview exactly what would be copied/extracted/skipped/conflicted, without writing anything.

Only after that is correct should SD mutation be implemented.
