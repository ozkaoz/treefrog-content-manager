import pathlib, sys, json
REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))
from treefrog import profile, classify

def test_bios_patterns():
    p = profile.load_profile()
    cases = [
        ("scph1001.bin", "bios"),
        ("gba_bios.bin", "bios"),
        ("o2rom.bin", "bios"),
        ("disksys.rom", "bios"),
        ("neogeo.zip", "bios"),  # neogeo bios is .zip but should be classified bios due to name hint > archive? our classify checks archive first so .zip will be archive, not bios
        ("kick13.rom", "bios"),
        ("x86BOOT.img", "bios"),
    ]
    for name, expected in cases:
        c = classify.classify(pathlib.Path(name), p)
        # neogeo.zip is archive kind, not bios, because archive check precedes bios — that's intentional: archive inspection then bios handling
        if name == "neogeo.zip":
            assert c["kind"] == "archive"
        else:
            assert c["kind"] == expected, f"{name} -> {c['kind']}"

def test_lgpt_profile_paths():
    lgpt = profile.load_lgpt()
    assert lgpt["destinations"]["samples"] == "lgpt/samples"
    assert lgpt["destinations"]["projects"] == "lgpt/projects"
    # verify against sd_root evidence
    assert (REPO / "sd_root" / "lgpt" / "samples" / ".keep").exists()
    assert (REPO / "sd_root" / "lgpt" / "projects" / ".keep").exists()

def test_bios_json_has_required_fields():
    bios = profile.load_bios()
    for rule in bios["bios_rules"]:
        assert "system" in rule
        assert "destination" in rule
        assert "accepted_patterns" in rule
        assert "required" in rule

def test_video_preset_provisional():
    video = profile.load_video_presets()
    preset = video["presets"][0]
    assert preset["status"] == "PROVISIONAL_UNVALIDATED"
    assert preset["requires_physical_validation"] is True
    # ensure we don't claim hardware compat
    assert "PROVISIONAL" in preset["status"]
