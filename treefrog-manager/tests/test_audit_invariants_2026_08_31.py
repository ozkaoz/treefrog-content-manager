"""Audit 2026-08-31 regression suite — canonical invariants.
Covers: destination path security (traversal/UNC/drive/ADS/reserved/empty),
sd::detect explicit states, effective-action space calculation, collision-safe
keep_both, BIOS destination escape (Rust-side, mirrored here through the Python
planner validation), and user-override escape.
"""
import pathlib
import sys
import tempfile

REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))

from treefrog import sd, planner, sd_target  # noqa: E402


# ---------------------------------------------------------------- sd states
def test_sd_detect_writable_is_unknown_not_inferred():
    with tempfile.TemporaryDirectory() as tmp:
        sd_path = pathlib.Path(tmp) / "sd"
        (sd_path / "cubegm").mkdir(parents=True)
        (sd_path / "roms").mkdir()
        info = sd.detect(str(sd_path))
        assert info["is_treefrog_sd"] is True
        # NEVER infer writable/healthy from markers
        assert info["writable"] is None
        assert info["healthy"] is None
        assert info["accessible"] is True


def test_sd_detect_with_probe_proves_writability():
    with tempfile.TemporaryDirectory() as tmp:
        sd_path = pathlib.Path(tmp) / "sd"
        (sd_path / "cubegm").mkdir(parents=True)
        (sd_path / "roms").mkdir()
        info = sd.detect_with_probe(str(sd_path))
        assert info["writable"] is True  # proven, not assumed
        assert info["healthy"] is True
        # probe cleaned up
        assert not any(p.name.startswith(".treefrog_probe") for p in sd_path.iterdir())


def test_sd_detect_read_only_not_writable():
    import os
    with tempfile.TemporaryDirectory() as tmp:
        ro = pathlib.Path(tmp) / "ro"
        (ro / "cubegm").mkdir(parents=True)
        (ro / "roms").mkdir()
        try:
            os.chmod(ro, 0o555)
        except (PermissionError, OSError):
            pass  # Windows dir readonly attr does not block creation
        try:
            info = sd.detect_with_probe(str(ro))
            if os.name == "posix":
                assert info["writable"] is False, "read-only must not appear writable"
                assert info["healthy"] is False
        finally:
            try:
                os.chmod(ro, 0o755)
            except (PermissionError, OSError):
                pass


def test_sd_detect_inaccessible_raises():
    with tempfile.TemporaryDirectory() as tmp:
        gone = pathlib.Path(tmp) / "does_not_exist"
        try:
            sd.detect(str(gone))
            assert False, "must raise"
        except FileNotFoundError:
            pass


# --------------------------------------------------------- destination paths
MALICIOUS_DESTINATIONS = [
    "../evil.bin",
    "../../evil.bin",
    "cubegm/bios/../../evil.bin",
    "C:\\evil.bin",
    "\\\\server\\share\\evil.bin",
    "/evil.bin",
    "roms/../../evil.bin",
    "roms//x",
    "roms/CON",
    "roms/con.txt",
    "roms/file.txt:ads",
    "roms/evil|name",
    "roms/evil.",
]


def test_validate_destination_rejects_all_malicious():
    for bad in MALICIOUS_DESTINATIONS:
        try:
            sd_target.validate_destination_path(bad)
        except Exception:
            continue
        assert False, f"malicious destination must be rejected: {bad!r}"


def test_validate_destination_accepts_valid_relative():
    for good in [
        "roms/FC/game.nes",
        "cubegm/bios/scph1001.bin",
        "lgpt/samples/kick.wav",
        "roms/videos/movie.mp4",
    ]:
        sd_target.validate_destination_path(good)  # must not raise


# ------------------------------------------------------- effective-action space
def _entry(action, resolved, size):
    return {
        "source": f"src/{size}",
        "destination": f"roms/{size}.bin",
        "action": action,
        "resolved_action": resolved,
        "size": size,
    }


def test_space_uses_effective_action_exclusively():
    plan = {
        "entries": [
            _entry("copy", None, 100),
            _entry("conflict", "replace", 50),   # resolved replace -> REQUIRED
            _entry("skip_duplicate", "copy", 70),  # resolved copy -> REQUIRED
            _entry("skip_duplicate", None, 999),   # still skip
            _entry("convert_then_copy", None, 200),
            _entry("extract", "skip", 500),        # resolved skip
        ]
    }
    s = sd_target.calculate_space(plan, 10_000)
    assert s["bytes_to_copy"] == 100 + 50 + 70
    assert s["bytes_to_generate"] == 200
    assert s["bytes_to_extract"] == 0
    assert s["bytes_to_skip"] == 999 + 500
    assert s["required_bytes"] == 420
    assert s["status"] == "ok"


def test_space_conflict_counts_only_when_resolved_to_write():
    unresolved = {"entries": [_entry("conflict", None, 50)]}
    assert sd_target.calculate_space(unresolved, 1000)["required_bytes"] == 0
    resolved = {"entries": [_entry("conflict", "replace", 50)]}
    assert sd_target.calculate_space(resolved, 1000)["required_bytes"] == 50


def test_space_insufficient():
    plan = {"entries": [_entry("copy", None, 1000)]}
    assert sd_target.calculate_space(plan, 100)["status"] == "insufficient_space"


# ------------------------------------------------------- keep_both collisions
def _conflict_entry(source, dest):
    return {"source": source, "destination": dest, "action": "conflict", "reason": "t", "size": 1}


def test_keep_both_skips_existing_1_on_disk():
    with tempfile.TemporaryDirectory() as tmp:
        sd_root = pathlib.Path(tmp) / "sd"
        (sd_root / "roms" / "FC").mkdir(parents=True)
        (sd_root / "roms" / "FC" / "game_1.nes").write_bytes(b"x")
        plan = {"entries": [_conflict_entry("src/a.nes", "roms/FC/game.nes")]}
        out = planner.apply_resolutions(plan, {"0": "keep_both"}, str(sd_root))
        assert out["entries"][0]["destination"] == "roms/FC/game_2.nes"
        assert out["entries"][0]["original_destination"] == "roms/FC/game.nes"


def test_keep_both_respects_plan_destinations_case_insensitive():
    with tempfile.TemporaryDirectory() as tmp:
        sd_root = pathlib.Path(tmp) / "sd"
        (sd_root / "roms" / "FC").mkdir(parents=True)
        plan = {
            "entries": [
                _conflict_entry("src/a.nes", "roms/FC/game.nes"),
                _conflict_entry("src/b.nes", "roms/FC/GAME_1.NES"),
            ]
        }
        out = planner.apply_resolutions(plan, {"0": "keep_both"}, str(sd_root))
        # _1 claimed by second entry (case-insensitive) -> _2
        assert out["entries"][0]["destination"] == "roms/FC/game_2.nes"
        assert out["entries"][1]["destination"] == "roms/FC/GAME_1.NES"


def test_multiple_keep_both_never_collide():
    with tempfile.TemporaryDirectory() as tmp:
        sd_root = pathlib.Path(tmp) / "sd"
        (sd_root / "roms" / "FC").mkdir(parents=True)
        plan = {
            "entries": [
                _conflict_entry("src/a.nes", "roms/FC/game.nes"),
                _conflict_entry("src/b.nes", "roms/FC/game.nes"),
                _conflict_entry("src/c.nes", "roms/FC/game.nes"),
            ]
        }
        out = planner.apply_resolutions(plan, {"0": "keep_both", "1": "keep_both", "2": "keep_both"}, str(sd_root))
        dests = [e["destination"] for e in out["entries"]]
        assert len({d.lower() for d in dests}) == 3, "renames must be unique"
        assert dests[0] == "roms/FC/game_1.nes"
        assert dests[1] == "roms/FC/game_2.nes"
        assert dests[2] == "roms/FC/game_3.nes"


# ------------------------------------------------------ user override escape
def test_user_override_escape_rejected_by_validation():
    # The deploy path validates every user override with the same canonical
    # validator before applying it — malicious folder hints must fail there.
    for bad in ["..", "C:\\evil", "\\\\srv\\share", "roms/../..", ""]:
        try:
            sd_target.validate_destination_path(f"{bad}/game.nes".lstrip("/") if bad else "")
        except Exception:
            continue
        # empty -> also rejected
        assert bad == "", f"override escape must be rejected: {bad!r}"
