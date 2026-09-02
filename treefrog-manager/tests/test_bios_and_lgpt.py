import pathlib, sys, json
REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))
from treefrog import profile, classify

def test_bios_patterns():
    # NOTE (2026-09-01 audit): the hardcoded BIOS name-hints were REMOVED.
    # New contract: a general scan only classifies as BIOS files that
    # explicitly live inside a cubegm/bios source folder. Loose BIOS-named
    # files are handled by the dedicated BIOS tab (bios.json validation:
    # filename + size + SHA-256). This avoids false positives (a ROM named
    # scph-*.bin) and duplicating x86BOOT.img which TreeFrogUI already ships.
    p = profile.load_profile()
    loose_cases = [
        "scph1001.bin",
        "gba_bios.bin",
        "o2rom.bin",
        "disksys.rom",
        "neogeo.zip",  # archive kind (archive check precedes)
        "kick13.rom",
        "x86BOOT.img",  # TreeFrogUI ships it already; never auto-capture
    ]
    for name in loose_cases:
        c = classify.classify(pathlib.Path(name), p)
        if name == "neogeo.zip":
            # neogeo.zip es archive (el check de archive precede al de BIOS)
            assert c["kind"] == "archive"
        elif name in ("scph1001.bin", "gba_bios.bin", "o2rom.bin", "disksys.rom", "kick13.rom", "x86BOOT.img"):
            # nombres EXACTOS declarados en bios.json -> bios (modelo declarativo)
            assert c["kind"] == "bios", f"exact bios.json name {name} must be bios, got {c['kind']}"
        else:
            assert c["kind"] != "bios", f"loose {name} must NOT be auto-captured as bios"

    # Substring false positive -> NO bios
    c = classify.classify(pathlib.Path("scph-greatest-hits.bin"), p)
    assert c["kind"] != "bios"

    # Inside an explicit cubegm/bios folder -> bios
    c = classify.classify(pathlib.Path("cubegm/bios/my_custom.bin"), p)
    assert c["kind"] == "bios"
    assert c["destination"] == "cubegm/bios"

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
