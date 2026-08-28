import pathlib, json, sys
REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))
from treefrog import profile

def test_manifest_exists():
    assert (REPO / "profiles" / "treefrogui" / "manifest.json").exists()
    m = json.loads((REPO / "profiles" / "treefrogui" / "manifest.json").read_text(encoding="utf-8"))
    assert m["schema_version"] == "1.0.0"
    assert "profile.json" in m["files"]

def test_profile_loads():
    p = profile.load_profile()
    assert p["profile_version"] == "1.0.0"
    assert len(p["systems"]) > 70  # full coverage, not bootstrap 2-3 (75 current)
    # case-sensitive alias check: FC vs fc distinct? alias map lowercases but file system preserves case for display
    assert "fc" in p["alias_to_system"]
    assert "gba" in p["alias_to_system"] or "gba" in str(p["alias_to_system"])

def test_systems_cover_all():
    p = profile.load_profile()
    ids = {s["id"] for s in p["systems"]}
    for need in ["nes_fceumm","snes","gba_gpsp","md_picodrive","ps_psx","pce","amiga","c64","msx","spec","cps1","neogeo","pico8","arduboy","o2em"]:
        assert need in ids, f"missing {need}"

def test_media_bios_lgpt_video_presets():
    media = profile.load_media()
    assert "roms/music" == media["media"]["music"]["destination"]
    assert media["media"]["music"]["preserve_subfolders"] is True
    bios = profile.load_bios()
    assert bios["destination_root"] == "cubegm/bios"
    assert any("scph" in str(r.get("accepted_patterns")) for r in bios["bios_rules"])
    lgpt = profile.load_lgpt()
    assert lgpt["destinations"]["samples"] == "lgpt/samples"
    assert lgpt["destinations"]["projects"] == "lgpt/projects"
    video = profile.load_video_presets()
    assert video["presets"][0]["status"] == "PROVISIONAL_UNVALIDATED"

def test_sd_markers():
    sd = profile.load_sd_markers()
    markers = [m["path"] for m in sd["detection"]["markers"]]
    assert "cubegm/" in markers and "roms/" in markers

def test_profile_has_archive_safety():
    p = profile.load_profile()
    pol = p["profile"]["archive_policy"]
    assert pol["safety"]["prevent_traversal"] is True
    assert pol["nested_archives"]["max_entries_per_archive"] == 1024
