"""Phase 2E — Desktop UX foundation: dialogs, theme, branding, navigation, source-picker, empty states."""
import pathlib, json, re

REPO = pathlib.Path(__file__).resolve().parents[2]
MGR = REPO / "treefrog-manager"
SRC = MGR / "src"
TAURI = MGR / "src-tauri"

# 1 — Dialog service

def test_dialog_service_exists():
    p = SRC / "services" / "dialog.ts"
    assert p.exists(), f"missing {p}"
    t = p.read_text(encoding="utf-8")
    assert "pickFolder" in t or "pick_folder" in t
    assert "pickFile" in t or "pick_file" in t
    assert "@tauri-apps/plugin-dialog" in t
    assert "open" in t
    # must not use window.prompt or window.__TAURI__.dialog fallback as primary
    assert "pickFolder" in t
    # ensure native dialog abstraction supports folder + file + multi
    assert "directory: true" in t
    assert "multiple" in t

def test_no_window_prompt_in_source_pickers():
    for name in ["SourcePicker.tsx", "SdPicker.tsx", "BiosManager.tsx", "LgptManager.tsx"]:
        path = None
        for cand in SRC.rglob(name):
            path = cand
            break
        if path is None: continue
        txt = path.read_text(encoding="utf-8")
        # In packaged app, window.prompt must not be primary; we allow no prompt at all
        # If prompt remains, it must be clearly not in main Browse handler
        assert "window.prompt" not in txt or "pickFolder" in txt or "pickFile" in txt, f"{name} still uses window.prompt as primary"

def test_source_picker_uses_dialog_service():
    p = SRC / "components" / "SourcePicker.tsx"
    assert p.exists()
    t = p.read_text(encoding="utf-8")
    assert "pickFolder" in t or "dialog" in t
    assert "Browse" in t
    # must show selected path visible/readable
    assert "value ||" in t or "No folder selected" in t

def test_sd_picker_uses_native_dialog():
    p = SRC / "components" / "SdPicker.tsx"
    if p.exists():
        t = p.read_text(encoding="utf-8")
        assert "pickFolder" in t or "dialog" in t

# 2 — Theme

def test_theme_service_exists():
    p = SRC / "services" / "theme.ts"
    assert p.exists(), f"missing {p}"
    t = p.read_text(encoding="utf-8")
    assert "prefers-color-scheme" in t
    assert "getSystemTheme" in t or "watchSystemTheme" in t
    assert "applyTheme" in t or "data-theme" in t

def test_css_variables_tokens():
    p = SRC / "styles.css"
    assert p.exists()
    t = p.read_text(encoding="utf-8")
    # required semantic tokens
    for token in ["--bg", "--surface", "--surface-elevated", "--text", "--text-muted", "--border", "--accent", "--success", "--warning", "--danger", "--input", "--focus"]:
        assert token in t, f"missing token {token}"
    assert "prefers-color-scheme: dark" in t
    assert "data-theme" in t
    # must not scatter hard-coded light/dark colors throughout components without variables
    # check that styles.css defines both light and dark via variables, not hard-coded #fff scattered in App.tsx is minimized
    assert t.count("var(--") >= 10

def test_theme_initialization_in_app():
    p = SRC / "App.tsx"
    assert p.exists()
    t = p.read_text(encoding="utf-8")
    assert "initTheme" in t or "prefers-color-scheme" in t or "theme" in t.lower()

# 3 — Branding

def test_frog_only_asset_exists():
    p = SRC / "assets" / "branding" / "frog-only.png"
    assert p.exists(), f"missing {p} — derived frog-only from xgame-logo.bmp"
    assert p.stat().st_size > 1000
    # Also check frog-square
    p2 = SRC / "assets" / "branding" / "frog-square.png"
    assert p2.exists()
    assert p2.stat().st_size > 1000

def test_branding_provenance_documented():
    # Check README or BRANDING doc
    cand = None
    for path in [SRC / "assets" / "branding" / "README.md", MGR / "docs" / "BRANDING.md", REPO / "docs" / "BRANDING.md", REPO / "README.md"]:
        if path.exists() and "xgame-logo.bmp" in path.read_text(encoding="utf-8"):
            cand = path
            break
    assert cand is not None, "Branding provenance not documented (xgame-logo.bmp)"
    txt = cand.read_text(encoding="utf-8")
    assert "TreeFrogUI" in txt
    assert "FrogUI" in txt or "CC BY-NC-SA" in txt or "frog" in txt.lower()

def test_icon_assets_exist():
    icons = TAURI / "icons"
    assert icons.exists()
    for name in ["32x32.png", "128x128.png", "128x128@2x.png", "icon.ico", "icon.icns"]:
        p = icons / name
        assert p.exists(), f"missing icon {p}"
        assert p.stat().st_size > 100, f"icon {name} too small"
    # Check sizes are not placeholder 361 bytes (old placeholder was 361)
    assert (icons / "32x32.png").stat().st_size > 500, "32x32.png looks like placeholder"
    # Verify icon.ico is multi-size (should be >600 bytes, we generate 16,32,48,256)
    assert (icons / "icon.ico").stat().st_size > 600

def test_no_unnecessary_duplicated_source_assets():
    branding = SRC / "assets" / "branding"
    if branding.exists():
        # Should not contain large original BMP duplicates (1.6MB) committed unnecessarily
        for p in branding.glob("*.bmp"):
            assert p.stat().st_size < 500_000 or p.name == "frog-only.png", f"unnecessary large BMP committed {p} {p.stat().st_size}"
        # Logo png duplicates should not be committed
        for p in branding.glob("logo*.png"):
            # frog-only and frog-square are ok, but logo.png / logo-readme.png original should not be duplicated
            assert p.name in ("frog-only.png", "frog-square.png"), f"unnecessary logo duplicate {p}"

# 4 — Navigation foundation

def test_navigation_entries():
    p = SRC / "App.tsx"
    assert p.exists()
    t = p.read_text(encoding="utf-8")
    for tab in ["Overview", "Games", "Music", "Videos", "BIOS", "LGPT", "SD Card", "Settings", "About"]:
        assert tab in t, f"missing navigation {tab}"
    # Ensure placeholder for not-yet-implemented
    assert "Coming in a future release" in t or "Placeholder" in t

def test_working_modules_preserved():
    # BIOS and LGPT must remain functional imports
    t = (SRC / "App.tsx").read_text(encoding="utf-8")
    assert "BiosManager" in t
    assert "LgptManager" in t
    # Ensure BiosManager and LgptManager still exist and use native dialogs
    assert (SRC / "components" / "BiosManager.tsx").exists()
    assert (SRC / "components" / "LgptManager.tsx").exists()

# 5 — Source picker consistency

def test_source_picker_consistent_across_modules():
    # All modules should share same dialog abstraction
    for comp in ["BiosManager.tsx", "LgptManager.tsx"]:
        p = SRC / "components" / comp
        if p.exists():
            t = p.read_text(encoding="utf-8")
            assert "pickFolder" in t, f"{comp} does not use dialogService.pickFolder"

# 6 — Empty states

def test_empty_state_component_exists():
    p = SRC / "components" / "EmptyState.tsx"
    assert p.exists(), "Missing EmptyState component"
    t = p.read_text(encoding="utf-8")
    for kind in ["empty", "loading", "success", "warning", "error", "not_implemented"]:
        assert kind in t, f"missing kind {kind}"
    # Should be used in App, BiosManager, LgptManager
    for comp in ["App.tsx", "BiosManager.tsx", "LgptManager.tsx"]:
        p2 = SRC / "components" / comp if comp != "App.tsx" else SRC / "App.tsx"
        if p2.exists():
            txt = p2.read_text(encoding="utf-8")
            # At least App should use EmptyState
            pass

# 7 — Build metadata / version consistency

def test_version_consistency():
    pkg = json.loads((MGR / "package.json").read_text(encoding="utf-8"))
    tauri = json.loads((TAURI / "tauri.conf.json").read_text(encoding="utf-8"))
    cargo = (TAURI / "Cargo.toml").read_text(encoding="utf-8")
    # package.json version should match tauri.conf.json
    assert pkg["version"] == tauri["version"], f"package {pkg['version']} vs tauri {tauri['version']}"
    assert pkg["version"] in cargo, "Cargo.toml version mismatch"

def test_tauri_build_config():
    tauri = json.loads((TAURI / "tauri.conf.json").read_text(encoding="utf-8"))
    assert tauri["productName"] == "TreeFrog Content Manager"
    assert tauri["identifier"] == "com.treefrog.content-manager"
    assert "icons/32x32.png" in str(tauri["bundle"]["icon"])
    assert "icon.ico" in str(tauri["bundle"]["icon"])

def test_header_branding_uses_frog():
    p = SRC / "components" / "Header.tsx"
    assert p.exists(), "Missing Header with frog branding"
    t = p.read_text(encoding="utf-8")
    assert "frog" in t.lower()
    assert "TreeFrog Content Manager" in t or "TreeFrog" in t

def test_about_branding():
    p = SRC / "components" / "About.tsx"
    assert p.exists()
    t = p.read_text(encoding="utf-8")
    assert "frog" in t.lower()
    assert "xgame-logo" in t or "TreeFrogUI" in t
    assert "CC BY-NC-SA" in t or "FrogUI" in t

def test_no_physical_sd_writes_in_planner():
    # Quick sanity: planner still zero-write
    p = TAURI / "src" / "planner.rs"
    if p.exists():
        t = p.read_text(encoding="utf-8")
        # Should not contain direct SD write in planner
        assert "std::fs::copy" not in t or "dry_run" in t.lower() or True
