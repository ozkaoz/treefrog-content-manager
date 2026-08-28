import pathlib, tempfile, sys
REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))
from treefrog import sd

def test_detect_valid_sd():
    with tempfile.TemporaryDirectory() as tmp:
        sd_path = pathlib.Path(tmp) / "sd"
        (sd_path / "cubegm").mkdir(parents=True)
        (sd_path / "roms").mkdir(parents=True)
        info = sd.detect(str(sd_path))
        assert info["is_treefrog_sd"] is True
        assert "cubegm" in info["markers_found"]

def test_detect_invalid_sd():
    with tempfile.TemporaryDirectory() as tmp:
        bad = pathlib.Path(tmp) / "empty"
        bad.mkdir()
        info = sd.detect(str(bad))
        assert info["is_treefrog_sd"] is False
        assert "cubegm" in info["markers_missing"]

def test_write_probe():
    with tempfile.TemporaryDirectory() as tmp:
        sd_path = pathlib.Path(tmp) / "sd2"
        (sd_path / "cubegm").mkdir(parents=True)
        (sd_path / "roms").mkdir(parents=True)
        # probe should succeed on temp writable dir
        assert sd.write_probe(str(sd_path)) is True
        # ensure probe file cleaned up
        assert not any(p.name.startswith(".treefrog_probe") for p in sd_path.iterdir())
