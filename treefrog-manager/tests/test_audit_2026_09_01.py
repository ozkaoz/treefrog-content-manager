"""Regresión auditoría 2026-09-01: artwork .res nunca desplegado + BIOS no capturado por nombre + Vectrex."""
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))

from treefrog import classify as clf, profile, scanner, planner  # noqa: E402


def test_artwork_res_never_deployed():
    p = profile.load_profile()
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        (src / "GBA" / ".res").mkdir(parents=True)
        (src / "GBA" / ".res" / "game.png").write_bytes(b"png")
        (src / "GBA" / "Imgs").mkdir(parents=True)
        (src / "GBA" / "Imgs" / "cover.png").write_bytes(b"png")
        (src / "GBA" / "game.gba").write_bytes(b"gba")
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms").mkdir()
        scanned = scanner.scan(str(src), p)
        # artwork -> unknown, sin destino
        for s in scanned:
            if ".res" in s["source_path"].parts or "Imgs" in s["source_path"].parts:
                assert s["classification"]["kind"] == "unknown", s["source_path"]
                assert s["classification"]["destination"] == "", s["source_path"]
        plan = planner.plan(scanned, str(sd), p)
        # El artwork NO aparece como copia
        for e in plan["entries"]:
            assert not e["destination"].startswith(".res"), e["destination"]
            assert "Imgs" not in e["destination"] or e["action"].startswith("skip"), e["destination"]
        # El ROM sí se despliega correctamente
        assert any(e["destination"] == "roms/GBA/game.gba" for e in plan["entries"]), plan["entries"]


def test_bios_not_captured_by_name_in_general_scan():
    p = profile.load_profile()
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        (src / "scph-greatest-hits.bin").write_bytes(b"rom")  # ROM con nombre BIOS (substring)
        (src / "scph-greatest-hits2.rom").write_bytes(b"rom")  # substring scph en .rom
        # dentro de cubegm/bios explícita sí es BIOS
        (src / "cubegm" / "bios").mkdir(parents=True)
        (src / "cubegm" / "bios" / "scph1001.bin").write_bytes(b"bios")
        scanned = scanner.scan(str(src), p)
        by_name = {s["source_path"].name: s["classification"]["kind"] for s in scanned
                   if s["source_path"].parent.name != "bios"}
        assert by_name.get("scph-greatest-hits.bin") != "bios", "ROM scph*.bin (substring) no debe ser BIOS"
        assert by_name.get("scph-greatest-hits2.rom") != "bios", "ROM .rom con scph (substring) no debe ser BIOS"
        kinds = {s["source_path"].name: s["classification"]["kind"] for s in scanned}
        assert kinds.get("scph1001.bin") == "bios", "dentro de cubegm/bios debe ser BIOS"


def test_vectrex_system_present():
    p = profile.load_profile()
    assert "vec" in p["alias_to_system"], "systems.json debe tener vec (Vectrex)"
    assert ".vec" in p["ext_to_system"], ".vec debe mapear al sistema"
