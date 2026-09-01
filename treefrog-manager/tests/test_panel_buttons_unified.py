"""Unified panel action buttons (Scan/Clear/Skip/Continue) — parity fixture.

Every panel that includes these actions must use the SAME classes
(.panel-btn.scan/.clear/.skip/.continue) — identical look and behavior across
Games, Music, Videos, BIOS, LGPT. No ad-hoc inline styles for these buttons.
"""
import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
SRC = REPO / "treefrog-manager" / "src"

PANELS = [
    "GamesPanel.tsx",
    "MusicPanel.tsx",
    "VideosPanel.tsx",
    "BiosManager.tsx",
    "LgptManager.tsx",
]


def _has_action_button(text: str, kind: str) -> bool:
    """A panel uses the unified class for the given action kind."""
    if kind == "scan":
        return f'className="panel-btn scan"' in text
    if kind == "clear":
        return f'className="panel-btn clear"' in text
    if kind == "skip":
        return 'className="panel-btn skip"' in text
    if kind == "continue":
        return 'className="panel-btn continue"' in text
    if kind == "back":
        return 'className="panel-btn back"' in text
    if kind == "sync":
        return 'className="panel-btn sync"' in text
    return False


def test_all_panels_use_unified_button_classes():
    css = (SRC / "styles.css").read_text(encoding="utf-8")
    # The unified system exists in CSS
    for sel in (
        "button.panel-btn.scan",
        "button.panel-btn.clear",
        "button.panel-btn.skip",
        "button.panel-btn.continue",
        "button.panel-btn.back",
        "button.panel-btn.sync",
        ".panel-actions",
    ):
        assert sel in css, f"styles.css missing unified selector: {sel}"

    expectations = {
        "GamesPanel.tsx": ["scan", "clear", "skip", "continue", "back", "sync"],
        "MusicPanel.tsx": ["scan", "clear", "skip", "continue", "back", "sync"],
        "VideosPanel.tsx": ["scan", "clear", "skip", "continue", "back", "sync"],
        "LgptManager.tsx": ["scan", "clear", "skip", "continue", "back", "sync"],
        "BiosManager.tsx": ["skip", "continue", "back", "sync"],  # browse-based: no scan/clear
    }
    for panel, kinds in expectations.items():
        p = SRC / "components" / panel
        assert p.exists(), f"{panel} missing"
        text = p.read_text(encoding="utf-8")
        for kind in kinds:
            assert _has_action_button(text, kind), (
                f"{panel} does not use unified panel-btn {kind}"
            )


def test_action_buttons_above_browse():
    """Point 2: the action row (Scan/Clear/Skip/Continue/Back/Sync) must be
    rendered ABOVE the Browse/source-picker section in every panel."""
    for panel in PANELS:
        p = SRC / "components" / panel
        if not p.exists():
            continue
        text = p.read_text(encoding="utf-8")
        i_actions = text.find('className="panel-actions"')
        # Every content panel binds its Browse via onClick= handleBrowse/handlePick*
        import re as _re
        m = _re.search(r"onClick=\{(?:\(\) => )?handle(Browse|Pick\w*)", text)
        i_browse = m.start() if m else -1
        assert i_actions != -1, f"{panel}: panel-actions row missing"
        assert i_browse != -1, f"{panel}: Browse button missing"
        assert i_actions < i_browse, (
            f"{panel}: action buttons must be ABOVE the Browse section"
        )


def test_music_search_only_after_scan():
    """Point 1: the Music search input must be gated by the scan result (only
    active once the music folder is loaded), like Games/Videos gate by plan."""
    text = (SRC / "components" / "MusicPanel.tsx").read_text(encoding="utf-8")
    # The search input must live inside a scanResult conditional
    assert "{scanResult && (" in text
    i_search = text.find('placeholder={t.searchPlaceholder || "Search file..."}')
    assert i_search != -1, "Music search input missing"
    gated = "{scanResult && (" in text[:i_search]
    assert gated, "Music search must be gated by scanResult (folder loaded)"


def test_bios_search_removed():
    """Point 2: BIOS search must NOT exist (user selects each BIOS directly)."""
    text = (SRC / "components" / "BiosManager.tsx").read_text(encoding="utf-8")
    assert "Search BIOS" not in text, "BIOS search must be removed"
    assert "searchQuery" not in text, "BIOS searchQuery must be removed"


def test_sync_button_always_active():
    """Sync to SD must NEVER be disabled — with no files staged it navigates to
    SD Card and the sync flow reports observably what is missing."""
    for panel in PANELS:
        p = SRC / "components" / panel
        if not p.exists():
            continue
        text = p.read_text(encoding="utf-8")
        i = text.find('className="panel-btn sync"')
        assert i != -1, f"{panel}: sync button missing"
        # The button element around the sync class must not contain disabled=
        btn_start = text.rfind("<button", 0, i)
        btn_end = text.find(">", i)
        attrs = text[btn_start:btn_end]
        assert "disabled" not in attrs, f"{panel}: sync button must stay active: {attrs[:80]}"


def test_sd_status_bar_everywhere():
    """Point 4: every content panel shows the shared REAL SD status bar which
    refreshes after sync (refreshSignal)."""
    for panel in ["GamesPanel.tsx", "MusicPanel.tsx", "VideosPanel.tsx", "BiosManager.tsx", "LgptManager.tsx"]:
        text = (SRC / "components" / panel).read_text(encoding="utf-8")
        assert "SdStatusBar" in text, f"{panel} must render SdStatusBar"
        assert "refreshSignal" in text, f"{panel} must pass refreshSignal to SdStatusBar"
    # App bumps the signal after every completed sync
    app = (SRC / "App.tsx").read_text(encoding="utf-8")
    assert "setSdRefreshSignal" in app, "App must bump sdRefreshSignal after sync"
    assert "autoSyncSignal" in app, "App must wire Sync-to-SD navigation signal"


def test_no_adhoc_inline_action_buttons_remain():
    """Scan/Clear/Skip/Continue buttons must not carry ad-hoc inline
    backgroundColor styles (the unified CSS classes are the single source)."""
    for panel in PANELS:
        p = SRC / "components" / panel
        if not p.exists():
            continue
        text = p.read_text(encoding="utf-8")
        # Find buttons with action labels and check for inline backgroundColor
        for m in re.finditer(r"<button[^>]*>(\s*(Scanning…|Scanning\.|Scan|Clear|Skip|Continue)[^<]*)", text):
            attrs = m.group(0)
            assert "backgroundColor" not in attrs, (
                f"{panel} action button uses inline style: {attrs[:80]}"
            )
