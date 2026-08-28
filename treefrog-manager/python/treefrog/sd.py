import pathlib

def detect(path: str):
    p = pathlib.Path(path)
    if not p.exists():
        raise FileNotFoundError(f"SD path not found: {path}")
    markers = ["cubegm","roms"]
    found = [m for m in markers if (p / m).exists()]
    missing = [m for m in markers if m not in found]
    is_sd = "cubegm" in found and "roms" in found
    return {"path": path, "is_treefrog_sd": is_sd, "markers_found": found, "markers_missing": missing, "writable": True if is_sd else None, "healthy": True if is_sd else None}

def write_probe(path: str) -> bool:
    p = pathlib.Path(path)
    probe = p / f".treefrog_probe_{__import__('os').getpid()}.tmp"
    try:
        probe.write_text("probe", encoding="utf-8")
        probe.unlink()
        return True
    except:
        return False
