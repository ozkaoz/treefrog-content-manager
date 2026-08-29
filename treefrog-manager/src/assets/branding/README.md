# TreeFrog Content Manager — Branding Provenance

## Asset source

- **Canonical primary source (Final, 2E.3):** `logo.png` (1536×1024, black background, pixel-art frog + "treefrogui" wordmark, horizontal for desktop README) from upstream **TreeFrogUI** `https://github.com/tzubertowski/TreeFrogUI` (`main` branch). Frog extracted from left side (left of 33-pixel zero gap at x≈517–549). Visual inspection (`build/branding-preview.png` LEFT canonical vs RIGHT generated) shows this frog is already **upright with head top, body below, legs DOWN** at `314×280` wide — no rotation required. Previous chaining `90° CCW` (280×314 tall, sideways) + `-45°` (422×422 diagonal) was incorrect; root-cause investigation shows the canonical without rotation matches the reference.
- **Previously misused:** `xgame-logo.bmp` `480×854` vertical boot asset (gap y≈349–358, 87×99) — low-res, inverted for handheld, became solid green at 32×32.
- **Intermediate incorrect:** `logo.png` 314×280 → 90° CCW (280×314 tall, sideways) and → 90° CCW + -45° (422×422 diagonal, legs diagonal) — both visually verified as not matching canonical (see `build/branding-variants.png` 6 variants, canonical 314×280 is correct).
- **Final (2E.3):** `logo.png` 314×280 **no rotation** — correct upright, legs DOWN. About/Credits secondary: full frog+wordmark from `logo.png` original.

## Derived assets (Final, no rotation + high-res)

- **frog-canonical.png** (also `frog-only.png` for backward compat, 314×280, RGBA, transparent) — frog pixel-art only, extracted from `logo.png` left part (203,355,533,635) → transparent where `r<20&&g<20&&b<20`, trimmed to 314×280, **no rotation**, no CSS `rotate()`, pipeline fixed to canonical visual reference (`build/branding-preview.png` LEFT vs RIGHT identical). Previous 422×422 diagonal was over-rotated.
- **frog-square.png** (512×512, RGBA) — square-padded (centered, 25% border) version of `frog-canonical.png` for icon generation. 314×280 with 25% border → 512×512 (padded from max 314). Previous square was 527×527 diagonal.
- **Why high-res + padding:** 314×280 retains eye/belly detail at 16×16/32×32; 87×99 became solid green. 25% transparent border ensures low-res icons (16×16 89 unique colors, transparent corners) don't become solid green square — preserves silhouette.

Generation script: `scripts/generate_branding.py` (deterministic, PIL, NEAREST, `TEMP/opencode_branding` cache). No `ROTATE_90`/`-45`/`FLIP` for canonical — only for xgame fallback if needed. The `frog-canonical.png` is the single source: `frog-canonical.png → frog-only.png (alias) → frog-square.png → ALL icons`.

## Icon set (Tauri/Windows, Final corrected)

Derived from `frog-square.png` (512×512) via **NEAREST** (pixel-art crisp, no blur):

- `src-tauri/icons/16x16.png` (16×16) — 272B transparent
- `src-tauri/icons/24x24.png` (24×24) — 569B
- `src-tauri/icons/32x32.png` (32×32) — 839B (was solid green 116B placeholder)
- `src-tauri/icons/48x48.png` (48×48) — 1624B
- `src-tauri/icons/64x64.png` (64×64) — 2579B
- `src-tauri/icons/128x128.png` (128×128) — 8755B
- `src-tauri/icons/128x128@2x.png` (256×256) — 31093B
- `src-tauri/icons/256x256.png` — 31093B
- `src-tauri/icons/512x512.png` — 105840B
- `src-tauri/icons/icon.ico` (16,24,32,48,64,128,256) — **52759B** (7 sizes PNG-compressed, was 641B single) via `icon_256.save(sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])`, verified `16x16` 89 unique not solid, `32x32` 177 unique
- `src-tauri/icons/icon.icns` (512) — 655978B

Header `Header.tsx:1` uses `frog-canonical.png` / `frog-square.png` without transform: `[frog 32×32 upright legs DOWN] TreeFrog Content Manager`. Same frog for window/taskbar/Desktop/StartMenu/installer.

## Root cause

- **Phase 2E upside-down:** `xgame-logo.bmp` 87×99 without flip → head at bottom → header inverted, low-res → solid green.
- **Phase 2E.1 sideways:** `logo.png` 314×280 wide correctly upright, but pipeline added `90° CCW` (280×314 tall, head at left, sideways) → header sideways.
- **Phase 2E.2 diagonal:** `90° CCW + -45°` (422×422 diagonal, legs diagonal down-right) → still not legs directly DOWN, narrow silhouette.
- **Final fix (2E.3):** Remove all chained rotations; canonical `314×280` without rotation is already upright with legs DOWN as verified visually (`build/branding-preview.png` LEFT vs RIGHT identical, `build/header-preview.png` shows [frog][TreeFrog Content Manager] correctly). Also added 25% padding and 7-size ICO to fix green square.

## License / Attribution

- **TreeFrogUI** is **CC BY-NC-SA 4.0**. Frog pixel-art from upstream **FrogUI** (`https://github.com/tzubertowski/frogui`). Per `LICENSE.md`.
- **Our use:** Frog cropped, made transparent, no rotation (legs DOWN) for window/installer/favicon/header; full frog+wordmark only in About/Credits as secondary. No newly generated logo; no CSS rotation; no reinterpretation.
- **Code:** `GPL-3.0-or-later` per `Cargo.toml`; frog asset remains CC BY-NC-SA 4.0 non-commercial.

## Verification

- `python scripts/generate_branding.py` reproduces deterministically from `https://raw.githubusercontent.com/tzubertowski/TreeFrogUI/main/logo.png`. Checks: `frog-only.png` 314×280, `frog-square.png` 512×512, `16x16.png` 272B 40 unique, `32x32.png` 839B 177 unique, `icon.ico` 52759B 7 sizes, transparent, `Header.tsx` uses `frog-canonical.png` without `rotate()`, `build/branding-preview.png` LEFT vs RIGHT identical, `build/header-preview.png` shows correct header.
- Visual: header shows frog with legs DOWN (314×280), not sideways/diagonal, crisp via NEAREST, works in Light (`#ffffff`) and Dark (`#0f172a`).

## Windows icon cache note

- Windows caches ICO for shortcuts/taskbar. After fixing ICO, clean validation requires uninstall, remove residual shortcuts, rebuild, fresh install (see `docs/MANUAL_QA_2E.md`). Do not require end user to manually rebuild cache.
