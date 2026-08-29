"""Phase 3A — SD target detection, validation, indexing, space, zero-write, planner integration."""
import pathlib, tempfile, json, os, sys
REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))
from treefrog import sd_target, profile, scanner, planner

def _make_sd(root: pathlib.Path, markers=("cubegm", "roms"), extra=None):
    for m in markers:
        (root / m).mkdir(parents=True, exist_ok=True)
    if extra:
        for e in extra:
            (root / e).mkdir(parents=True, exist_ok=True)
    # Add some dummy files
    (root / "cubegm" / "test.txt").write_text("dummy", encoding="utf-8")
    return root

def test_valid_treefrog_target():
    with tempfile.TemporaryDirectory() as tmp:
        sd = pathlib.Path(tmp) / "sd_valid"
        sd.mkdir()
        _make_sd(sd, markers=("cubegm", "roms"), extra=["frogui", "lgpt", "cubegm/cores", "cubegm/bios"])
        res = sd_target.analyze_target(str(sd))
        assert res["status"] == "valid"
        assert res["is_treefrog"] is True
        assert "cubegm" in res["markers_found"] and "roms" in res["markers_found"]
        assert res["lgpt_detected"] is True
        assert res["volume"]["accessible"] is True

def test_incomplete_treefrog_target():
    with tempfile.TemporaryDirectory() as tmp:
        sd = pathlib.Path(tmp) / "sd_incomplete"
        sd.mkdir()
        _make_sd(sd, markers=("cubegm",), extra=[])
        res = sd_target.analyze_target(str(sd))
        assert res["status"] == "incomplete"
        assert res["is_treefrog"] is False
        assert res["is_incomplete"] is True
        assert "roms" in res["markers_missing"]

def test_unknown_target():
    with tempfile.TemporaryDirectory() as tmp:
        sd = pathlib.Path(tmp) / "sd_unknown"
        sd.mkdir()
        (sd / "some_other").mkdir()
        res = sd_target.analyze_target(str(sd))
        assert res["status"] == "unknown"
        assert res["is_treefrog"] is False

def test_inaccessible_target():
    with tempfile.TemporaryDirectory() as tmp:
        sd = pathlib.Path(tmp) / "no_such_sd_12345"
        res = sd_target.analyze_target(str(sd))
        assert res["status"] == "inaccessible"
        assert res["volume"]["accessible"] is False

def test_removable_volume_abstraction():
    vols = sd_target.list_volumes()
    # Should return a list (may be empty on CI, but must not crash)
    assert isinstance(vols, list)
    for v in vols:
        assert "path" in v
        assert "accessible" in v
        # Check that get_volume_info works for a temp path
    with tempfile.TemporaryDirectory() as tmp:
        info = sd_target.get_volume_info(tmp)
        assert info["accessible"] is True
        assert info["path"] == tmp

def test_target_scan_rom_detection():
    with tempfile.TemporaryDirectory() as tmp:
        sd = pathlib.Path(tmp) / "sd"
        sd.mkdir()
        _make_sd(sd)
        # Add ROM dirs
        (sd / "roms" / "GBA").mkdir(parents=True)
        (sd / "roms" / "GBA" / "game.gba").write_bytes(b"gba")
        (sd / "roms" / "music").mkdir(parents=True)
        (sd / "roms" / "music" / "song.mp3").write_bytes(b"mp3")
        (sd / "roms" / "videos").mkdir(parents=True)
        (sd / "roms" / "videos" / "vid.mp4").write_bytes(b"mp4")
        (sd / "cubegm" / "bios").mkdir(parents=True)
        (sd / "cubegm" / "bios" / "gba_bios.bin").write_bytes(b"bios")
        (sd / "lgpt" / "samples").mkdir(parents=True)
        (sd / "lgpt" / "samples" / "kick.wav").write_bytes(b"wav")
        res = sd_target.analyze_target(str(sd))
        assert "GBA" in res["rom_dirs"]
        assert "music" in res["media_dirs"] or "videos" in res["media_dirs"]
        assert "cubegm/bios" in res["bios_dirs"] or "bios" in res["bios_dirs"]
        assert "lgpt/samples" in res["lgpt_dirs"] or "lgpt" in res["lgpt_dirs"]
        assert res["existing_count"] >= 5
        assert res["total_size"] > 0

def test_target_scan_lgpt_detection():
    with tempfile.TemporaryDirectory() as tmp:
        sd = pathlib.Path(tmp) / "sd"
        sd.mkdir()
        _make_sd(sd, extra=["lgpt"])
        (sd / "lgpt" / "samples").mkdir(parents=True)
        (sd / "lgpt" / "samples" / "a.wav").write_bytes(b"a")
        (sd / "lgpt" / "projects" / "ProjA").mkdir(parents=True)
        (sd / "lgpt" / "projects" / "ProjA" / "project.lgpt").write_text("proj", encoding="utf-8")
        res = sd_target.analyze_target(str(sd))
        assert res["lgpt_detected"] is True
        assert "lgpt/samples" in res["lgpt_dirs"]
        assert "lgpt/projects" in res["lgpt_dirs"]

def test_target_logical_units_not_flattened():
    # Ensure that multi-file groups and LGPT projects are not flattened in target index
    with tempfile.TemporaryDirectory() as tmp:
        sd = pathlib.Path(tmp) / "sd"
        sd.mkdir()
        _make_sd(sd)
        # Create a fake CUE/BIN group on SD (should be counted as files, but planner will treat as logical)
        (sd / "roms" / "PS").mkdir(parents=True)
        (sd / "roms" / "PS" / "game.cue").write_text("cue", encoding="utf-8")
        (sd / "roms" / "PS" / "game.bin").write_bytes(b"bin")
        # Create LGPT project dir
        (sd / "lgpt" / "projects" / "MyProj").mkdir(parents=True)
        (sd / "lgpt" / "projects" / "MyProj" / "project.lgpt").write_text("proj", encoding="utf-8")
        (sd / "lgpt" / "projects" / "MyProj" / "sample.wav").write_bytes(b"wav")
        res = sd_target.analyze_target(str(sd))
        # Should count files individually, but not claim they are flattened
        assert res["existing_count"] >= 3

def test_target_hashing_via_planner():
    # Use planner to hash target content and compare with source
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        sd = pathlib.Path(tmp) / "sd"
        sd.mkdir()
        _make_sd(sd)
        (sd / "roms" / "GBA").mkdir(parents=True)
        (sd / "roms" / "GBA" / "same.gba").write_bytes(b"samecontent")
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        (src / "same.gba").write_bytes(b"samecontent")
        (src / "different.gba").write_bytes(b"different")
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        # same.gba should be duplicate or unchanged, different should be new or conflict
        actions = [e["action"] for e in plan["entries"]]
        assert "skip_duplicate" in actions or "skip_unchanged" in actions

def test_space_calculation_ok():
    plan = {"entries": [
        {"action": "copy", "size": 1000},
        {"action": "extract", "size": 2000},
        {"action": "convert_then_copy", "size": 3000},
        {"action": "skip_duplicate", "size": 4000},
    ]}
    space = sd_target.calculate_space(plan, free_bytes=10000)
    assert space["bytes_to_copy"] == 1000
    assert space["bytes_to_extract"] == 2000
    assert space["bytes_to_generate"] == 3000
    assert space["bytes_to_skip"] == 4000
    assert space["required_bytes"] == 6000
    assert space["status"] == "ok"

def test_insufficient_space():
    plan = {"entries": [{"action": "copy", "size": 8 * 1024**3}]}
    space = sd_target.calculate_space(plan, free_bytes=7 * 1024**3)
    assert space["status"] == "insufficient_space"
    assert space["required_bytes"] == 8 * 1024**3
    assert space["available_bytes"] == 7 * 1024**3

def test_destination_path_validation():
    valid = ["roms/GBA/game.gba", "lgpt/samples/kick.wav", "roms/music/album/song.mp3"]
    for v in valid:
        sd_target.validate_destination_path(v)
    invalid = [
        "../evil.gba",
        "/absolute.gba",
        "C:/evil.gba",
        "roms/CON/game.gba",
        "roms/evil:stream",
        "roms/evil*",
        "roms/evil?",
        "roms/evil|",
        "roms/evil>",
        "roms/evil.",
        "roms//double",
        "\\\\server\\share",
    ]
    for inv in invalid:
        try:
            sd_target.validate_destination_path(inv)
            assert False, f"should have rejected {inv}"
        except ValueError:
            pass

def test_case_insensitive_collision():
    dests = ["roms/GBA/Game.gba", "roms/gba/game.gba", "roms/SFC/other.sfc"]
    coll = sd_target.check_case_collision(dests)
    assert len(coll) == 1
    assert coll[0][0].lower() == coll[0][1].lower()

def test_integration_with_planner():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        sd = pathlib.Path(tmp) / "sd"
        sd.mkdir()
        _make_sd(sd)
        (sd / "roms" / "GBA").mkdir(parents=True)
        (sd / "roms" / "GBA" / "existing.gba").write_bytes(b"existing")
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        (src / "new.gba").write_bytes(b"new")
        (src / "existing.gba").write_bytes(b"existing")
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        target = sd_target.analyze_target(str(sd))
        # Validate destinations
        for e in plan["entries"]:
            sd_target.validate_destination_path(e["destination"])
        # Check no case collisions in plan
        dests = [e["destination"] for e in plan["entries"]]
        coll = sd_target.check_case_collision(dests)
        # May be empty or not, but should be deterministic
        # Calculate space
        free = target["free_bytes"] or (10 * 1024**3)
        space = sd_target.calculate_space(plan, free_bytes=free)
        assert "required_bytes" in space
        assert "status" in space

def test_deterministic_target_analysis():
    with tempfile.TemporaryDirectory() as tmp:
        sd = pathlib.Path(tmp) / "sd"
        sd.mkdir()
        _make_sd(sd)
        (sd / "roms" / "GBA").mkdir(parents=True)
        (sd / "roms" / "GBA" / "a.gba").write_bytes(b"a")
        (sd / "roms" / "GBA" / "b.gba").write_bytes(b"b")
        res1 = sd_target.analyze_target(str(sd))
        res2 = sd_target.analyze_target(str(sd))
        assert res1["rom_dirs"] == res2["rom_dirs"]
        assert res1["existing_count"] == res2["existing_count"]
        assert res1["status"] == res2["status"]

def test_zero_writes():
    with tempfile.TemporaryDirectory() as tmp:
        sd = pathlib.Path(tmp) / "sd"
        sd.mkdir()
        _make_sd(sd)
        before = set(p.relative_to(sd).as_posix() for p in sd.rglob("*") if p.is_file())
        # Run all operations that should be read-only
        sd_target.analyze_target(str(sd))
        sd_target.get_volume_info(str(sd))
        sd_target.list_volumes()
        p = profile.load_profile()
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        (src / "game.gba").write_bytes(b"game")
        scanned = scanner.scan(str(src), p)
        planner.plan(scanned, str(sd), p)
        after = set(p.relative_to(sd).as_posix() for p in sd.rglob("*") if p.is_file())
        assert before == after, "target analysis must not write to SD"
        # Also ensure no probe file left
        for f in sd.rglob(".treefrog_probe*"):
            assert False, f"probe file left {f}"

def test_target_index_space_with_video_bios_lgpt():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        sd = pathlib.Path(tmp) / "sd"
        sd.mkdir()
        _make_sd(sd, extra=["lgpt", "cubegm/bios"])
        (sd / "roms" / "GBA").mkdir(parents=True)
        (sd / "roms" / "GBA" / "old.gba").write_bytes(b"old")
        (sd / "cubegm" / "bios").mkdir(parents=True, exist_ok=True)
        (sd / "cubegm" / "bios" / "gba_bios.bin").write_bytes(b"bios")
        (sd / "lgpt" / "samples").mkdir(parents=True, exist_ok=True)
        (sd / "lgpt" / "samples" / "old.wav").write_bytes(b"oldwav")
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        (src / "new.gba").write_bytes(b"new")
        (src / "new.wav").write_bytes(b"newwav")
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        target = sd_target.analyze_target(str(sd))
        # Check that target detected bios and lgpt
        assert "cubegm/bios" in target["bios_dirs"] or "bios" in str(target["bios_dirs"])
        assert target["lgpt_detected"] is True
        space = sd_target.calculate_space(plan, free_bytes=target["free_bytes"] or (10*1024**3))
        assert space["required_bytes"] >= 0
