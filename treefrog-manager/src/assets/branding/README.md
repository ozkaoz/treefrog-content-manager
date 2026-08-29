# TreeFrog Content Manager — Branding Provenance

## Asset source

- **Canonical primary source (Phase 2E.1):** `logo.png` (1536×1024, black background, pixel-art frog + "treefrogui" wordmark, horizontal for desktop README) from upstream **TreeFrogUI** `https://github.com/tzubertowski/TreeFrogUI` (`main` branch). Frog extracted from left side of the horizontal logo (left of 33-pixel zero gap at x≈517–549), which is desktop-upright and high-res.
- **Previously used source (Phase 2E):** `xgame-logo.bmp` (480×854, vertical for handheld boot, same frog+wordmark stacked vertically, gap at y≈349–358). That file is top-down BMP (`height=-854`) but is stored for handheld's rotated boot display; on desktop it appears vertically inverted (head at bottom) if used without flipping. The 87×99 frog derived from it was low-res and when downscaled to 32×32 via NEAREST became a near-solid green block (generic icon).
- **Secondary:** `logo-readme.png` (1303×341) inspected; `xgame-logo.bmp` retained only as fallback/reference, not committed as large duplicate.
- **About/Credits secondary:** Full frog + wordmark (from `logo.png` or `xgame-logo.bmp` original) shown only in About/Credits; not used as primary icon.

## Derived assets (Phase 2E.1, corrected orientation & high-res)

- **frog-only.png** (314×280, RGBA, transparent) — frog pixel-art only, extracted from `logo.png` left part (203,355,533,635) → transparent where `r<20&&g<20&&b<20`, trimmed to 314×280. No redraw, no `rotate(180deg)` CSS workaround; source pipeline is fixed to use desktop-upright high-res frog. No interpolation blur.
- **frog-square.png** (314×314, RGBA) — square-padded (centered) version of `frog-only.png` for icon generation. Previous square was 99×99 from low-res xgame frog; new square is 314×314 (upscaled base 512 for icons).
- **Why high-res:** 314×280 retains eye/belly detail at 32×32; 87×99 became solid green when NEAREST-scaled to 32.

Generation script: `scripts/generate_branding.py` (deterministic, PIL, NEAREST, `TEMP/opencode_branding` cache). For xgame fallback it would `FLIP_TOP_BOTTOM` to correct handheld inversion, but canonical is now `logo.png` without flip.

## Icon set (Tauri/Windows, Phase 2E.1 corrected)

Derived from `frog-square.png` (314×314, upscaled to 512 for generation) via **NEAREST** (pixel-art crisp, no blur):

- `src-tauri/icons/32x32.png` (32×32) — 1686 bytes (was 1791, now from high-res)
- `src-tauri/icons/64x64.png` (64×64) — new, for Windows
- `src-tauri/icons/128x128.png` (128×128)
- `src-tauri/icons/128x128@2x.png` (256×256)
- `src-tauri/icons/256x256.png`
- `src-tauri/icons/512x512.png`
- `src-tauri/icons/icon.ico` (16,32,48,64,128,256) — **103442 bytes** (was 641 bytes placeholder with single 16×16); now PNG-compressed multi-resolution via `icon_256.save(sizes=[(16,16)...(256,256)])` using 256 source (not 16), verified Windows picks correct size for shortcut/taskbar/installer
- `src-tauri/icons/icon.icns` (512) — 927052 bytes

Header/window uses `frog-square.png` at 32×32 via CSS (`image-rendering: pixelated` not needed, NEAREST source keeps crisp). No CSS `rotate(180deg)` workaround — asset itself is correct.

## Root cause (upside-down frog)

- **Cause:** Phase 2E derived frog from `xgame-logo.bmp` vertical boot asset (87×99) without accounting that the boot logo is stored for handheld's rotated display. On desktop the frog's head (eyes) ended up at image bottom (≈y80 of 99), belly yellow at bottom already, so flipping made head appear at bottom → user saw vertically inverted frog in header. Additionally, downscaling 87×99 → 32 via NEAREST lost detail → solid green.
- **Fix:** Switch canonical source to `logo.png` horizontal desktop logo (frog left, 314×280, desktop upright, high-res). Generation now uses `logo.png` without flip, plus correctly generates ICO from 256 source with 6 sizes. Documented in `scripts/generate_branding.py` and this file.

## License / Attribution

- **TreeFrogUI** is **CC BY-NC-SA 4.0** (Creative Commons Attribution-NonCommercial-ShareAlike 4.0). The frog pixel-art logo and original UI font were taken from upstream **FrogUI** (`https://github.com/tzubertowski/frogui`) and reused under their respective open-source/CC terms; default system backgrounds were sourced from **Art Book Next** (Anthony Caccese, ES-DE theme). Per `https://github.com/tzubertowski/TreeFrogUI/blob/main/LICENSE.md` and `README.md` License & Attribution section.
- **Our use:** Frog cropped and made transparent for application/window/installer/fav icon; full frog+wordmark shown only in About/Credits as secondary. No newly generated logo; no reinterpretation. The high-res frog preserves original pixel colors.
- **TreeFrog Content Manager** code: `GPL-3.0-or-later` per `Cargo.toml`; frog asset remains CC BY-NC-SA 4.0 non-commercial — do not sell or bundle with commercial devices.

## Verification

- `python scripts/generate_branding.py` reproduces deterministically from `https://raw.githubusercontent.com/tzubertowski/TreeFrogUI/main/logo.png` (primary) and `xgame-logo.bmp` fallback. Checks: `frog-only.png` 314×280, `frog-square.png` 314×314, `32x32.png` 1686 bytes, `icon.ico` 103442 bytes (6 sizes), `icon.icns` 927052 bytes, transparent alpha exists, no placeholder <500 bytes, `tauri.conf.json` bundle points to `icons/32x32.png` etc.
- Visual: header shows frog with eyes at top, belly yellow at bottom, not mirrored or stretched; works in both Light (`#ffffff`) and Dark (`#0f172a`) via transparent asset.

## Windows icon cache note

- Windows caches ICO for shortcuts/taskbar. After fixing ICO, a clean validation requires uninstall, remove residual shortcuts, rebuild, fresh install (see `docs/MANUAL_QA_2E.md` and `docs/BUILD_WINDOWS.md`). Do not require end user to manually rebuild cache.
