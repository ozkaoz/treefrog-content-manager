# TreeFrog Content Manager — Branding Provenance

## Asset source

- **Primary source:** `xgame-logo.bmp` (480×854, black background, pixel-art frog + "treefrogui" wordmark) from upstream **TreeFrogUI** repository `https://github.com/tzubertowski/TreeFrogUI` (`main` branch, file `xgame-logo.bmp`).
- **Secondary sources inspected:** `logo.png` (1536×1024) and `logo-readme.png` (1303×341) from the same repository.
- **Secondary About asset:** Full frog + wordmark (`xgame-logo.bmp` original) retained for About/Credits only; not used as primary icon.

## Derived assets

- **frog-only.png** (87×99, transparent) — cropped frog pixel art only, extracted from `xgame-logo.bmp` by detecting near-black background (`r<20 && g<20 && b<20` → transparent) and splitting at the 10-pixel zero gap between frog and wordmark (original y ≈ 349–358). No redraw or stylistic reinterpretation; original pixel colors preserved, NEAREST scaling for icons to keep pixel-art crisp.
- **frog-square.png** (99×99, transparent) — square-padded version of `frog-only.png` for icon generation (centered, transparent padding).

Generation script: `scripts/generate_branding.py` (deterministic, PIL, NEAREST).

## Icon set (Tauri/Windows)

Derived from `frog-square.png` via NEAREST:

- `src-tauri/icons/32x32.png` (32×32)
- `src-tauri/icons/128x128.png` (128×128)
- `src-tauri/icons/128x128@2x.png` (256×256)
- `src-tauri/icons/256x256.png`
- `src-tauri/icons/512x512.png`
- `src-tauri/icons/icon.ico` (16, 32, 48, 256)
- `src-tauri/icons/icon.icns` (512)

Header/window uses `frog-only.png` (or `frog-square.png`) at 32×32 via CSS.

## License / Attribution

- **TreeFrogUI** is **CC BY-NC-SA 4.0** (Creative Commons Attribution-NonCommercial-ShareAlike 4.0). The frog pixel-art logo and original UI font were taken from upstream **FrogUI** (`https://github.com/tzubertowski/frogui`) and reused under their respective open-source/CC terms; default system backgrounds were sourced from **Art Book Next** (Anthony Caccese, ES-DE theme). Per `https://github.com/tzubertowski/TreeFrogUI/blob/main/LICENSE.md` and `README.md` License & Attribution section.
- **Our use:** Frog cropped and made transparent for application/window/installer/fav icon; foreword/Attribution retained in `docs/BRANDING.md` and this file. The full frog+wordmark is shown only in About/Credits as secondary. No newly generated logo is added; no reinterpretation.
- **TreeFrog Content Manager** itself is licensed as per `treefrog-manager/src-tauri/Cargo.toml` `GPL-3.0-or-later` for code, but the upstream frog asset remains CC BY-NC-SA 4.0 non-commercial — do not sell or bundle with commercial devices.

## Verification

- `python scripts/generate_branding.py` reproduces all icons deterministically from `xgame-logo.bmp` fetched from `https://raw.githubusercontent.com/tzubertowski/TreeFrogUI/main/xgame-logo.bmp`.
- Checks: frog-only must be 87×99, square 99×99, icons crisp (NEAREST), transparent background.
