"""Phase 2B duplicate & conflict resolution — deterministic, metadata-rich, explicit decisions, zero SD writes."""
import pathlib, tempfile, hashlib, sys, zipfile
REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))
from treefrog import profile, scanner, planner

def _make_zip(path, entries):
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
        for name, data in entries:
            z.writestr(name, data)

def test_identical_loose_files():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms").mkdir()
        # Two identical files same content, same name in different subfolders? Actually identical loose files in source
        (src / "a.gba").write_bytes(b"same")
        (src / "b.gba").write_bytes(b"same")
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        # Should have one copy and one duplicate
        assert any(e["action"] == "skip_duplicate" for e in plan["entries"])
        # Check metadata
        dup = [e for e in plan["entries"] if e["action"] == "skip_duplicate"][0]
        assert "source_hash" in dup and dup["source_hash"] is not None
        assert dup["content_type"] is not None
        assert "duplicate" in dup["reason"].lower()
        # Zero SD writes
        assert not (sd / "roms" / "GBA" / "a.gba").exists()

def test_identical_different_filenames():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms").mkdir()
        (src / "game1.gba").write_bytes(b"identical")
        (src / "game2.gba").write_bytes(b"identical")
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        dups = [e for e in plan["entries"] if e["action"] == "skip_duplicate"]
        assert len(dups) >= 1
        # Should be duplicate/alias, not conflict
        for e in dups:
            assert e["source_hash"] is not None
            assert e["destination_hash"] is None or e["destination_hash"] is None  # no dest yet
            assert "different path + same hash" in e["reason"] or "duplicate" in e["reason"].lower()

def test_same_filename_different_contents():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms" / "GBA").mkdir(parents=True)
        (sd / "roms" / "GBA" / "existing.gba").write_bytes(b"original")
        (src / "existing.gba").write_bytes(b"different")
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        conflicts = [e for e in plan["entries"] if e["action"] == "conflict"]
        assert len(conflicts) == 1
        c = conflicts[0]
        assert c["source_hash"] is not None
        assert c["destination_hash"] is not None
        assert c["source_hash"] != c["destination_hash"]
        assert "different hash" in c["reason"].lower()
        assert c["content_type"] is not None
        assert c["default_action"] == "conflict"
        assert c["resolution"] == "conflict"
        assert c["resolved_action"] == "conflict"

def test_grouped_cue_bin_duplicates():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms").mkdir()
        # Create two identical CUE/BIN groups in different archives
        zp1 = src / "game1.zip"
        zp2 = src / "game2.zip"
        _make_zip(zp1, [("game.cue", b'FILE "game.bin" BINARY'), ("game.bin", b"identical bin")])
        _make_zip(zp2, [("other.cue", b'FILE "other.bin" BINARY'), ("other.bin", b"identical bin")])
        # But grouped detection groups by folder, so each zip has one group with 2 files
        # The combined hash for each group should be identical if bin content identical and cue content identical (but cue differs in filename)
        # For this test, make cues identical content and bins identical
        # Actually we made game.cue vs other.cue with same content? They differ only in filename inside? Let's make them identical content but different archive
        # We already did same content for bin, cue content is slightly different (filename inside cue). Let's make identical
        # Recreate with identical cue content
        zp1.unlink()
        zp2.unlink()
        _make_zip(zp1, [("game.cue", b"same cue"), ("game.bin", b"same bin")])
        _make_zip(zp2, [("game.cue", b"same cue"), ("game.bin", b"same bin")])
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        # One group should be duplicate
        # The planner for grouped archives creates one entry per group, so two zips each produce one grouped entry, second should be duplicate
        dups = [e for e in plan["entries"] if e["action"] == "skip_duplicate"]
        # Might be duplicate due to identical combined hash
        assert len(dups) >= 1 or any(e["action"] == "skip_duplicate" for e in plan["entries"]), plan["entries"]
        # Check that grouped duplicate has members
        for e in plan["entries"]:
            if e["action"] == "skip_duplicate" and e.get("members"):
                assert len(e["members"]) == 2

def test_archive_vs_extracted_payload_duplicates():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms").mkdir()
        # Loose file
        loose = src / "game.gba"
        loose.write_bytes(b"payload")
        # Zip containing same payload
        zp = src / "game.zip"
        _make_zip(zp, [("inner.gba", b"payload")])
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        # Should not count as two independent copies when logical content identical
        # One should be duplicate
        dups = [e for e in plan["entries"] if e["action"] == "skip_duplicate"]
        assert len(dups) >= 1, f"expected duplicate archive vs extracted, got {plan['entries']}"
        # Check that duplicate has source_hash and reason indicates duplicate extracted payload
        for e in dups:
            assert e["source_hash"] is not None

def test_destination_unchanged():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms" / "GBA").mkdir(parents=True)
        (sd / "roms" / "GBA" / "same.gba").write_bytes(b"same")
        (src / "same.gba").write_bytes(b"same")
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        unchanged = [e for e in plan["entries"] if e["action"] == "skip_unchanged"]
        assert len(unchanged) == 1
        u = unchanged[0]
        assert u["source_hash"] == u["destination_hash"]
        assert u["source_hash"] is not None
        assert "unchanged" in u["reason"].lower()
        assert u["content_type"] is not None

def test_explicit_conflict_replacement():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms" / "GBA").mkdir(parents=True)
        (sd / "roms" / "GBA" / "conflict.gba").write_bytes(b"old")
        (src / "conflict.gba").write_bytes(b"new")
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        conflict = [e for e in plan["entries"] if e["action"] == "conflict"][0]
        idx = plan["entries"].index(conflict)
        # Apply explicit replace decision
        new_plan = planner.apply_resolutions(plan, {idx: "replace"})
        resolved = new_plan["entries"][idx]
        assert resolved["resolution"] == "replace"
        assert resolved["resolved_action"] == "replace"
        assert "replace" in resolved["reason"].lower()
        # Ensure original plan unchanged (single source of truth, apply returns new plan)
        assert plan["entries"][idx]["action"] == "conflict"

def test_explicit_keep_destination():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms" / "GBA").mkdir(parents=True)
        (sd / "roms" / "GBA" / "keep.gba").write_bytes(b"old")
        (src / "keep.gba").write_bytes(b"new")
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        idx = [i for i, e in enumerate(plan["entries"]) if e["action"] == "conflict"][0]
        new_plan = planner.apply_resolutions(plan, {idx: "keep_destination"})
        assert new_plan["entries"][idx]["resolved_action"] == "skip"
        assert new_plan["entries"][idx]["resolution"] == "keep_destination"

def test_explicit_keep_both():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms" / "GBA").mkdir(parents=True)
        (sd / "roms" / "GBA" / "both.gba").write_bytes(b"old")
        (src / "both.gba").write_bytes(b"new")
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        idx = [i for i,e in enumerate(plan["entries"]) if e["action"]=="conflict"][0]
        new_plan = planner.apply_resolutions(plan, {idx: "keep_both"})
        resolved = new_plan["entries"][idx]
        assert resolved["resolved_action"] in ("copy", "extract")
        assert resolved["destination"] != "roms/GBA/both.gba"
        assert "_1" in resolved["destination"]
        assert "keep_both" in resolved["reason"].lower()

def test_explicit_skip():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms").mkdir()
        (src / "a.gba").write_bytes(b"data")
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        # Find a copy/extract entry
        idx = [i for i,e in enumerate(plan["entries"]) if e["action"] in ("copy","extract")][0]
        new_plan = planner.apply_resolutions(plan, {idx: "skip"})
        assert new_plan["entries"][idx]["resolved_action"] == "skip"
        assert new_plan["entries"][idx]["resolution"] == "skip"

def test_deterministic_results():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms").mkdir()
        for i in range(5):
            (src / f"game{i}.gba").write_bytes(f"content{i}".encode())
        scanned = scanner.scan(str(src), p)
        plan1 = planner.plan(scanned, str(sd), p)
        plan2 = planner.plan(scanned, str(sd), p)
        assert plan1["summary"] == plan2["summary"]
        assert [e["source"] for e in plan1["entries"]] == [e["source"] for e in plan2["entries"]]
        assert [e["destination"] for e in plan1["entries"]] == [e["destination"] for e in plan2["entries"]]
        # Also check that apply_resolutions is deterministic
        new1 = planner.apply_resolutions(plan1, {0: "keep_both"})
        new2 = planner.apply_resolutions(plan2, {0: "keep_both"})
        assert new1["entries"][0]["destination"] == new2["entries"][0]["destination"]

def test_collision_metadata_exposed():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms").mkdir()
        # Create a zip that would cause collision inside archive is tested in phase2a, but here test that planner entries expose metadata
        (src / "game.gba").write_bytes(b"data")
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        e = plan["entries"][0]
        # Check that frontend-relevant metadata is present
        assert "source" in e
        assert "destination" in e
        assert "action" in e
        assert "reason" in e
        assert "source_hash" in e
        assert "destination_hash" in e
        assert "content_type" in e
        assert "default_action" in e
        assert "resolution" in e
        assert "resolved_action" in e
        # For non-conflict, destination_hash may be None, but key should exist
        assert "content_type" in e and e["content_type"] is not None

def test_zero_sd_writes():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms" / "GBA").mkdir(parents=True)
        (sd / "roms" / "GBA" / "existing.gba").write_bytes(b"old")
        (src / "existing.gba").write_bytes(b"new")
        (src / "new.gba").write_bytes(b"new2")
        # Record SD file list before (relative)
        before = set(p.relative_to(sd).as_posix() for p in sd.rglob("*") if p.is_file())
        scanned = scanner.scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        # Also test apply_resolutions does not write
        _ = planner.apply_resolutions(plan, {0: "replace"})
        after = set(p.relative_to(sd).as_posix() for p in sd.rglob("*") if p.is_file())
        assert before == after, "planner must not write to SD"
        # Also check that no temp files leaked to SD (check relative paths only, not absolute temp dir)
        assert not any("tmp" in p.relative_to(sd).as_posix().lower() for p in sd.rglob("*") if p.is_file())
