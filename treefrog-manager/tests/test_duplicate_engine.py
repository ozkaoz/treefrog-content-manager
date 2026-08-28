import pathlib, tempfile, sys
REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))
from treefrog import hash as hmod

def test_sha256():
    with tempfile.TemporaryDirectory() as tmp:
        p = pathlib.Path(tmp) / "file.bin"
        p.write_bytes(b"hello world")
        h = hmod.sha256_file(p)
        assert h == "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        # same content different path -> same hash
        q = pathlib.Path(tmp) / "copy.bin"
        q.write_bytes(b"hello world")
        assert hmod.sha256_file(q) == h

def test_classify():
    assert hmod.classify_duplicate(True, True, True) == "unchanged"
    assert hmod.classify_duplicate(False, True, True) == "duplicate_content"
    assert hmod.classify_duplicate(True, False, True) == "conflict"
    assert hmod.classify_duplicate(False, False, False) == "new"
    assert hmod.classify_duplicate(True, False, False) == "new"  # not exists -> new even if same_path True

def test_duplicate_means_same_content_not_filename():
    with tempfile.TemporaryDirectory() as tmp:
        a = pathlib.Path(tmp) / "a.gba"
        b = pathlib.Path(tmp) / "b.gba"
        a.write_bytes(b"identical")
        b.write_bytes(b"identical")
        ha = hmod.sha256_file(a)
        hb = hmod.sha256_file(b)
        assert ha == hb
        # different filename but same hash -> duplicate_content
        assert hmod.classify_duplicate(False, True, True) == "duplicate_content"
        # same filename but different hash -> conflict
        b.write_bytes(b"different")
        hb2 = hmod.sha256_file(b)
        assert ha != hb2
