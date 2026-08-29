"""Phase 2E.1 — Branding and Windows icon correction: orientation, high-res, ICO multi-size."""
import pathlib, struct
from PIL import Image

REPO = pathlib.Path(__file__).resolve().parents[2]
MGR = REPO / "treefrog-manager"
BRANDING = MGR / "src" / "assets" / "branding"
ICONS = MGR / "src-tauri" / "icons"

def test_frog_only_high_res_and_alpha():
    p = BRANDING / "frog-only.png"
    assert p.exists(), f"missing {p}"
    im = Image.open(p)
    w, h = im.size
    # Must be high-res from logo.png (314x280), not low-res 87x99 placeholder
    assert w >= 200 and h >= 200, f"frog-only too small {w}x{h}, expected high-res 314x280 from logo.png"
    assert w == 314 and h == 280, f"expected 314x280, got {w}x{h}"
    # Alpha/transparency must exist
    assert im.mode == "RGBA"
    has_transparent = any(im.getpixel((x, y))[3] == 0 for y in range(h) for x in range(w))
    assert has_transparent, "frog-only should have transparent background"
    # Should not be solid green square (check not all opaque pixels are same green)
    opaque_pixels = [(r, g, b) for x in range(w) for y in range(h) for r, g, b, a in [im.getpixel((x, y))] if a != 0]
    unique = set(opaque_pixels)
    assert len(unique) > 10, "frog-only appears solid color, expected pixel-art with multiple colors"

def test_frog_square_high_res():
    p = BRANDING / "frog-square.png"
    assert p.exists()
    im = Image.open(p)
    w, h = im.size
    assert w == h, f"frog-square must be square, got {w}x{h}"
    assert w >= 300, f"frog-square too small {w}, expected 314"
    assert w == 314 and h == 314, f"expected 314x314, got {w}x{h}"

def test_frog_not_inverted_via_generation_script():
    # Deterministic check: generation script must use logo.png as canonical (desktop upright)
    # and flip only for xgame fallback (handheld inverted)
    g = (REPO / "scripts" / "generate_branding.py").read_text(encoding="utf-8")
    assert "logo.png" in g, "generate_branding should use logo.png as canonical"
    assert "xgame-logo.bmp" in g, "should still handle xgame fallback"
    # Verify orientation handling: logo extraction should NOT contain FLIP (upright)
    # and xgame fallback should contain FLIP_TOP_BOTTOM
    # Find the logo extraction function
    assert "extract_frog_logo" in g
    # Ensure logo path does not have flip, xgame does
    # Simple heuristic: count FLIP occurrences — should be at least 1 for xgame
    assert "FLIP_TOP_BOTTOM" in g, "should contain FLIP for xgame correction"
    # Ensure the flip is in xgame function, not logo
    logo_section = g.split("extract_frog_logo")[0] + g.split("extract_frog_logo")[1].split("def ")[0] if "extract_frog_logo" in g else ""
    # More robust: ensure the first FLIP appears after xgame function definition
    xgame_idx = g.find("extract_frog_xgame")
    flip_idx = g.find("FLIP_TOP_BOTTOM")
    assert xgame_idx != -1 and flip_idx != -1 and flip_idx > xgame_idx, "FLIP should be in xgame fallback, not logo canonical"

def test_no_placeholder_icon_remains():
    # Old placeholders were 361 bytes for 128x128, 116 for 32x32, 641 for ico
    # New icons must be significantly larger and high-res
    checks = {
        "32x32.png": 1500,
        "64x64.png": 4000,
        "128x128.png": 10000,
        "256x256.png": 50000,
        "512x512.png": 100000,
        "icon.ico": 5000,
        "icon.icns": 50000,
    }
    for name, min_size in checks.items():
        p = ICONS / name
        assert p.exists(), f"missing {name}"
        size = p.stat().st_size
        assert size >= min_size, f"{name} too small {size} < {min_size}, likely placeholder remains"
        # Also check dimensions via PIL where applicable
        if name.endswith(".png"):
            im = Image.open(p)
            # Extract expected size from filename
            if "32x32" in name:
                assert im.size == (32, 32), f"{name} size {im.size}"
            elif "64x64" in name:
                assert im.size == (64, 64)
            elif "128x128.png" == name:
                assert im.size == (128, 128)
            elif "128x128@2x" in name:
                assert im.size == (256, 256)
            elif "256x256" in name:
                assert im.size == (256, 256)
            elif "512x512" in name:
                assert im.size == (512, 512)

def test_ico_contains_multiple_resolutions():
    p = ICONS / "icon.ico"
    assert p.exists()
    data = p.read_bytes()
    # ICO header: 6 bytes, then 16 bytes per directory entry
    assert len(data) >= 6
    reserved, type_, count = struct.unpack("<HHH", data[:6])
    assert reserved == 0 and type_ == 1, "not a valid ICO"
    assert count >= 4, f"ICO should contain at least 4 resolutions (16,32,48,256), got {count}"
    # Check that expected sizes are present
    sizes = []
    for i in range(count):
        entry = data[6 + i*16:6 + (i+1)*16]
        w, h = entry[0], entry[1]
        # 0 means 256
        w = 256 if w == 0 else w
        h = 256 if h == 0 else h
        sizes.append((w, h))
    for expected in [(16, 16), (32, 32), (48, 48), (256, 256)]:
        assert expected in sizes, f"ICO missing {expected}, got {sizes}"

def test_branding_provenance_exists_and_correct():
    p = BRANDING / "README.md"
    assert p.exists()
    txt = p.read_text(encoding="utf-8")
    assert "logo.png" in txt, "should document logo.png as canonical"
    assert "xgame-logo.bmp" in txt, "should mention xgame fallback"
    assert "CC BY-NC-SA" in txt or "FrogUI" in txt
    assert "frog-only.png" in txt
    assert "314" in txt or "high-res" in txt.lower()
    # Must state no newly created logo
    assert "No newly" in txt or "no newly" in txt.lower()

def test_generation_script_deterministic():
    g = (REPO / "scripts" / "generate_branding.py").read_text(encoding="utf-8")
    # Must be deterministic: uses NEAREST, not BILINEAR
    assert "Image.NEAREST" in g, "must use NEAREST for pixel-art"
    assert "BILINEAR" not in g and "LANCZOS" not in g

def test_header_uses_frog_correctly():
    p = MGR / "src" / "components" / "Header.tsx"
    assert p.exists()
    txt = p.read_text(encoding="utf-8")
    assert "frog" in txt.lower()
    assert "TreeFrog Content Manager" in txt
    # Must not use CSS rotate as workaround
    assert "rotate(180" not in txt and "rotate(90" not in txt
    # Also check App.tsx header not using transform
    app = (MGR / "src" / "App.tsx").read_text(encoding="utf-8")
    assert "rotate" not in app.lower() or "rotate" in txt.lower() and "180" not in txt

def test_version_consistency_still():
    import json
    pkg = json.loads((MGR / "package.json").read_text(encoding="utf-8"))
    tauri = json.loads((MGR / "src-tauri" / "tauri.conf.json").read_text(encoding="utf-8"))
    import re
    cargo = (MGR / "src-tauri" / "Cargo.toml").read_text(encoding="utf-8")
    assert pkg["version"] == tauri["version"]
    assert pkg["version"] in cargo
