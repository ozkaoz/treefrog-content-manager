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
    return False


def test_all_panels_use_unified_button_classes():
    css = (SRC / "styles.css").read_text(encoding="utf-8")
    # The unified system exists in CSS
    for sel in (
        "button.panel-btn.scan",
        "button.panel-btn.clear",
        "button.panel-btn.skip",
        "button.panel-btn.continue",
        ".panel-actions",
    ):
        assert sel in css, f"styles.css missing unified selector: {sel}"

    expectations = {
        "GamesPanel.tsx": ["scan", "clear", "skip", "continue"],
        "MusicPanel.tsx": ["scan", "clear", "skip", "continue"],
        "VideosPanel.tsx": ["scan", "clear", "skip", "continue"],
        "LgptManager.tsx": ["scan", "clear", "skip", "continue"],
        "BiosManager.tsx": ["skip", "continue"],  # no scan/clear (browse-based)
    }
    for panel, kinds in expectations.items():
        p = SRC / "components" / panel
        assert p.exists(), f"{panel} missing"
        text = p.read_text(encoding="utf-8")
        for kind in kinds:
            assert _has_action_button(text, kind), (
                f"{panel} does not use unified panel-btn {kind}"
            )


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
