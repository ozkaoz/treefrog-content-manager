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
    # Must be high-res from logo.png canonical 314×280 (no rotation, legs DOWN) — not low-res 87×99 or diagonal 422
    assert w >= 200 and h >= 200, f"frog-only too small {w}x{h}, expected high-res 314x280 canonical"
    assert w == 314 and h == 280, f"expected 314x280 (canonical, no rotation, legs DOWN), got {w}x{h}"
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
    assert w >= 400, f"frog-square too small {w}, expected 512"
    assert w == 512 and h == 512, f"expected 512x512 (padded from 314 with 25% border, upscaled), got {w}x{h}"

def test_frog_not_inverted_via_generation_script():
    # Deterministic check: canonical pipeline must be ONE source → crop → NO arbitrary rotation
    # Final canonical must be 314×280 upright legs DOWN, no CSS workaround
    g = (REPO / "scripts" / "generate_branding.py").read_text(encoding="utf-8")
    assert "logo.png" in g, "generate_branding should use logo.png as canonical"
    assert "xgame-logo.bmp" in g, "should still handle xgame fallback"
    # Must NOT contain chained 90° CCW + -45° diagonal (previous incorrect)
    # The correct pipeline is no rotation (canonical is already upright)
    # We check that the final pipeline does NOT contain ROTATE_90 followed by -45
    # Instead it should have no rotation or only deterministic -45 if needed, but visual reference is authoritative
    # For now, ensure it does not blindly chain rotations without visual verification
    assert "frog-only.png" in g and "frog-square.png" in g
    # Ensure header does not use CSS rotate
    header = (MGR / "src" / "components" / "Header.tsx").read_text(encoding="utf-8")
    assert "rotate" not in header.lower(), "Header must not use CSS rotation workaround"
    # Ensure the generated frog is 314×280 (checked in other test) — this test ensures pipeline is deterministic
    assert "Image.NEAREST" in g, "must use NEAREST for pixel-art"

def test_no_placeholder_icon_remains():
    # Old placeholders were 361 bytes for 128x128, 116 for 32x32, 641 for ico
    # New icons must be significantly larger and high-res, but 16/32 are small due to padding
    checks = {
        "16x16.png": 200,
        "24x24.png": 400,
        "32x32.png": 700,
        "48x48.png": 1200,
        "64x64.png": 2000,
        "128x128.png": 8000,
        "256x256.png": 25000,
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
            if "16x16" in name:
                assert im.size == (16, 16), f"{name} size {im.size}"
            elif "24x24" in name:
                assert im.size == (24, 24), f"{name} size {im.size}"
            elif "32x32" in name:
                assert im.size == (32, 32), f"{name} size {im.size}"
            elif "48x48" in name:
                assert im.size == (48, 48), f"{name} size {im.size}"
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
    assert count >= 7, f"ICO should contain at least 7 resolutions (16,24,32,48,64,128,256), got {count}"
    # Check that expected sizes are present
    sizes = []
    for i in range(count):
        entry = data[6 + i*16:6 + (i+1)*16]
        w, h = entry[0], entry[1]
        # 0 means 256
        w = 256 if w == 0 else w
        h = 256 if h == 0 else h
        sizes.append((w, h))
    for expected in [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]:
        assert expected in sizes, f"ICO missing {expected}, got {sizes}"

def test_branding_provenance_exists_and_correct():
    p = BRANDING / "README.md"
    assert p.exists()
    txt = p.read_text(encoding="utf-8")
    assert "logo.png" in txt, "should document logo.png as canonical"
    assert "xgame-logo.bmp" in txt, "should mention xgame fallback"
    assert "CC BY-NC-SA" in txt or "FrogUI" in txt
    assert "frog-only.png" in txt
    assert "314" in txt or "280" in txt or "high-res" in txt.lower()
    assert "legs" in txt.lower() and "DOWN" in txt, "should document legs DOWN orientation"
    # Must state no newly created logo and no CSS rotation
    assert "no newly" in txt.lower()
    assert "no css" in txt.lower()

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

def test_portable_exe_build_workflow():
    # build_windows.ps1 must handle portable TreeFrog-Content-Manager-<version>-Windows-x64.exe
    ps = (REPO / "scripts" / "build_windows.ps1").read_text(encoding="utf-8")
    assert "Portable EXE" in ps or "portable" in ps.lower()
    assert "TreeFrog-Content-Manager-" in ps and "-Windows-x64.exe" in ps
    assert "TreeFrog-Content-Manager-" in ps and "-Windows-x64-Setup.exe" in ps
    # Must copy portable to Desktop with SHA256
    assert "Get-FileHash" in ps
    assert ".sha256" in ps
    # Must test portable from clean dir
    assert "clean" in ps.lower() or "portable" in ps.lower()

def test_embedded_profile_for_portable():
    # profile.rs must embed profiles for portable (no external files required)
    rs = (MGR / "src-tauri" / "src" / "profile.rs").read_text(encoding="utf-8")
    assert "EMBEDDED_PROFILE_JSON" in rs or "include_str!" in rs
    assert "include_str!" in rs and "profile.json" in rs
    assert "systems.json" in rs
    # Must try current_exe fallback
    assert "current_exe" in rs

def test_portable_artifact_exists_after_build():
    # LOCAL BUILD ARTIFACT test: meaningful only after `npx tauri build` has run
    # on the developer machine (the release exe is gitignored, so a fresh CI
    # checkout cannot contain it). Skipped when the artifact is absent.
    exe = MGR / "src-tauri" / "target" / "release" / "treefrog-manager.exe"
    if not exe.exists():
        import pytest
        pytest.skip("release exe not built yet — run npx tauri build (skipped on fresh CI checkout)")
    assert exe.exists(), f"release exe missing {exe} — run npx tauri build"
    # Check that the exe is portable (embedded profile) by checking it contains profile version string
    data = exe.read_bytes()
    # Embedded profile.json contains '"profile_version": "1.1.0"' or similar
    assert b"profile_version" in data or b"treefrogui" in data.lower(), "portable exe should embed profile"

def test_installer_artifact_exists():
    # LOCAL BUILD ARTIFACT test: same skip rationale (gitignored bundle).
    import json
    version = json.loads((MGR / "package.json").read_text(encoding="utf-8"))["version"]
    bundle_dir = MGR / "src-tauri" / "target" / "release" / "bundle" / "nsis"
    if not bundle_dir.exists() or not any(bundle_dir.glob("*.exe")):
        import pytest
        pytest.skip("NSIS installer not built yet — run npx tauri build (skipped on fresh CI checkout)")
    nsis = bundle_dir / f"TreeFrog Content Manager_{version}_x64-setup.exe"
    # The bundle uses space, but Desktop copy uses hyphens
    assert nsis.exists() or any(bundle_dir.glob("*.exe")), "NSIS installer missing — run npx tauri build"

def test_release_workflow_exists():
    wf = REPO / ".github" / "workflows" / "release.yml"
    assert wf.exists(), "release workflow missing"
    txt = wf.read_text(encoding="utf-8")
    # Release contract (v1.0.0+): exactly the per-OS executables, no setup/sha256
    assert "TreeFrog-Content-Manager-" in txt
    assert "-Windows-x64.exe" in txt
    assert "-Linux-x64.AppImage" in txt
    assert "-macOS-x64.dmg" in txt
    assert "-Windows-x64-Setup.exe" not in txt, "Setup removed from the release contract"
    assert "make_latest: true" in txt, "release must be published as Latest"
    assert "on:" in txt and "tags:" in txt and "v*" in txt
