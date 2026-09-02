"""Regresión de BIOS sin hash/tamaño (2026-09-01): neogeo.zip, segacd, pcfx,
ecwolf.pk3 etc. deben ser SELECCIONABLES (found_valid), no found_unknown."""
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))

from treefrog import bios as bmod, profile  # noqa: E402


def _defs():
    return profile.load_bios().get("bios_definitions", [])


def test_neogeo_zip_selectable():
    """El reporte del usuario: 'Selected: neogeo.zip — filename known but no
    hash/size to validate'. Con el fix debe ser found_valid con reason
    observable (validated by name)."""
    defs = _defs()
    neogeo = next(d for d in defs if d["id"] == "neogeo_bios")
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        f = pathlib.Path(tmp) / "neogeo.zip"
        f.write_bytes(b"fake neogeo bios")
        res = bmod.validate_bios_file(f, neogeo)
        assert res["state"] == "found_valid", res
        assert "no hash/size" in res["reason"].lower() or "by name" in res["reason"].lower(), res["reason"]


def test_all_bios_defs_selectable_when_no_criteria():
    """TODA BIOS cuya definición no declara hash ni tamaño debe aceptar un
    archivo con nombre aceptado (el usuario la eligió explícitamente)."""
    defs = _defs()
    no_criteria = [d for d in defs if not d.get("hashes_sha256") and not d.get("expected_size")
                   and not any(v.get("hashes_sha256") or v.get("expected_size") for v in d.get("variants", []))]
    assert len(no_criteria) >= 9, f"se esperaban >=9 BIOS sin criterios, hay {len(no_criteria)}"
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        for d in no_criteria:
            # tomar un filename concreto (sin wildcard)
            names = [n for n in d.get("accepted_filenames", []) if "*" not in n and "?" not in n]
            if not names:
                # de variantes
                for v in d.get("variants", []):
                    names.extend(n for n in v.get("filenames", []) if "*" not in n)
            if not names:
                continue  # solo wildcards — el BIOS tab las lista, no auto-valida
            f = pathlib.Path(tmp) / names[0]
            f.write_bytes(b"bios")
            res = bmod.validate_bios_file(f, d)
            assert res["state"] == "found_valid", f'{d["id"]} con {names[0]}: {res}'


def test_bios_with_hash_still_validates_hash():
    """La BIOS con hash (gba) sigue validando por hash — el fix no relaja eso."""
    defs = _defs()
    gba = next(d for d in defs if d["id"] == "gba_bios")
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        f = pathlib.Path(tmp) / "gba_bios.bin"
        f.write_bytes(b"wrong content")
        res = bmod.validate_bios_file(f, gba)
        assert res["state"] == "found_invalid", res


def test_bios_with_size_still_validates_size():
    """PS1 (scph1001.bin, expected_size 524288) valida por tamaño."""
    defs = _defs()
    ps1 = next(d for d in defs if d["id"] == "ps1_bios")
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        f = pathlib.Path(tmp) / "scph1001.bin"
        f.write_bytes(b"x" * 524288)
        res = bmod.validate_bios_file(f, ps1)
        assert res["state"] == "found_valid", res
        f2 = pathlib.Path(tmp) / "scph1001_wrong.bin"
        f2.write_bytes(b"x" * 1000)
        res2 = bmod.validate_bios_file(f2, ps1)
        assert res2["state"] == "found_invalid", res2
