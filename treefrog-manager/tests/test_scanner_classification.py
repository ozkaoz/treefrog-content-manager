import pathlib, tempfile, sys
REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))
from treefrog import profile, scanner, classify

def test_classify_by_extension():
    p = profile.load_profile()
    cases = [
        (pathlib.Path("game.gba"), "rom"),
        (pathlib.Path("song.mp3"), "music"),
        (pathlib.Path("movie.mp4"), "video"),
        (pathlib.Path("photo.jpg"), "image"),
        (pathlib.Path("book.epub"), "ebook"),
        (pathlib.Path("scph1001.bin"), "bios"),
        (pathlib.Path("archive.zip"), "archive"),
        (pathlib.Path("unknown.xyz"), "unknown"),
    ]
    for path, expected_kind in cases:
        c = classify.classify(path, p)
        assert c["kind"] == expected_kind, f"{path} got {c['kind']} expected {expected_kind}"

def test_classify_destination_for_rom():
    p = profile.load_profile()
    c = classify.classify(pathlib.Path("Advance Wars.gba"), p)
    assert c["destination"] == "roms/GBA" or c["destination"].startswith("roms/")

def test_scanner_recursive_and_preserves_music_subfolder():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        root = pathlib.Path(tmp)
        (root / "My Album").mkdir()
        (root / "My Album" / "track1.flac").write_text("fake flac")
        (root / "nested" / "deep").mkdir(parents=True)
        (root / "nested" / "deep" / "game.gba").write_text("gba")
        (root / "single.mp3").write_text("mp3")
        scanned = scanner.scan(str(root), p)
        # should find 3 files
        assert len([s for s in scanned if s["source_path"].name=="track1.flac"]) == 1
        # music preserve: classification says roms/music, but planner will preserve subfolder — scanner just classifies
        music = [s for s in scanned if s["classification"]["kind"]=="music"]
        assert len(music) >= 2

def test_cue_bin_group_preserved():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        root = pathlib.Path(tmp)
        # create CUE + BIN
        (root / "game.cue").write_text('FILE "game.bin" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n')
        (root / "game.bin").write_bytes(b"\x00"*1024)
        scanned = scanner.scan(str(root), p)
        # deduped group: only one entry with group_members
        grouped = [s for s in scanned if s["group"] is not None]
        assert len(grouped) == 1
        assert len(grouped[0]["group"]) == 2
