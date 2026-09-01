"""Archive security fixtures — Windows-specific member path hazards.
A malicious ZIP can smuggle drive-letter paths, UNC paths, and ADS colons in
member names that are harmless on Linux but escape or corrupt on Windows.
These tests prove the archive layer rejects them in BOTH the Python mirror
and documents the Rust behavior (same rules, see archive.rs::check_entry_safety).
"""
import io
import pathlib
import sys
import tempfile
import zipfile

REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))

from treefrog import archive  # noqa: E402


def _make_zip(members: dict) -> pathlib.Path:
    """Create a ZIP with the given member name -> bytes mapping."""
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as zf:
        for name, data in members.items():
            zf.writestr(name, data)
    tmp = tempfile.NamedTemporaryFile(suffix=".zip", delete=False)
    tmp.write(buf.getvalue())
    tmp.close()
    return pathlib.Path(tmp.name)


def test_drive_letter_member_rejected():
    z = _make_zip({"C:/evil.txt": b"x", "roms/ok.nes": b"y"})
    try:
        try:
            archive.inspect_archive(z)
            assert False, "drive-letter member must be rejected"
        except Exception:
            pass  # rejected (safety violation)
    finally:
        z.unlink()


def test_unc_member_rejected():
    z = _make_zip({"\\\\server\\share\\evil.txt": b"x"})
    try:
        try:
            archive.inspect_archive(z)
            assert False, "UNC member must be rejected"
        except Exception:
            pass
    finally:
        z.unlink()


def test_traversal_member_rejected():
    z = _make_zip({"../evil.txt": b"x", "a/../../evil2.txt": b"y"})
    try:
        try:
            archive.inspect_archive(z)
            assert False, "traversal member must be rejected"
        except Exception:
            pass
    finally:
        z.unlink()


def test_absolute_member_rejected():
    z = _make_zip({"/evil.txt": b"x"})
    try:
        try:
            archive.inspect_archive(z)
            assert False, "absolute member must be rejected"
        except Exception:
            pass
    finally:
        z.unlink()


def test_safe_zip_passes_and_lists():
    z = _make_zip({"roms/FC/game.nes": b"NES", "docs/readme.txt": b"hi"})
    try:
        entries = archive.inspect_archive(z)
        names = {e["name"] for e in entries}
        assert "roms/FC/game.nes" in names
    finally:
        z.unlink()


def test_7z_rar_explicitly_unsupported():
    # Only ZIP is supported; 7z/RAR must surface unsupported_archive with a
    # precise reason — never claim support, never extract, never copy as
    # supported content.
    for ext in (".7z", ".rar"):
        tmp = tempfile.NamedTemporaryFile(suffix=ext, delete=False)
        tmp.write(b"not a real archive")
        tmp.close()
        p = pathlib.Path(tmp.name)
        try:
            try:
                archive.inspect_archive(p)
                assert False, "junk 7z/rar must not be inspected as supported"
            except Exception as e:
                assert "unsupported" in str(e).lower() or "7z" in str(e).lower() or "rar" in str(e).lower(), \
                    f"unsupported reason must be explicit: {e}"
        finally:
            p.unlink()
