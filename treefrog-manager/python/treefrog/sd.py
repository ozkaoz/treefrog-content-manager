import pathlib


def detect(path: str):
    """Detect TreeFrogUI markers. Explicit states — writable/healthy are NEVER
    inferred from markers: writable is None (unknown) until a real write probe
    proves it; healthy is only true when accessible AND writable == True."""
    p = pathlib.Path(path)
    accessible = p.exists() and p.is_dir()
    if not accessible:
        raise FileNotFoundError(f"SD path not found or not a directory: {path}")
    markers = ["cubegm", "roms"]
    found = [m for m in markers if (p / m).exists()]
    missing = [m for m in markers if m not in found]
    is_sd = "cubegm" in found and "roms" in found
    return {
        "path": path,
        "is_treefrog_sd": is_sd,
        "markers_found": found,
        "markers_missing": missing,
        "accessible": accessible,
        "writable": None,  # unknown until probed — never inferred
        "healthy": None,   # unknown until proven
    }


def detect_with_probe(path: str):
    """detect + explicit non-destructive write probe. writable/healthy are
    PROVEN (Some True/False), not assumed."""
    info = detect(path)
    info["writable"] = write_probe(path)
    info["healthy"] = bool(info["accessible"] and info["writable"] is True)
    return info


def write_probe(path: str) -> bool:
    """Non-destructive write probe: create unique temp file, remove it.
    True = proof of writability; False = read-only/unwritable (never an
    assumption)."""
    p = pathlib.Path(path)
    probe = p / f".treefrog_probe_{__import__('os').getpid()}.tmp"
    try:
        probe.write_text("probe", encoding="utf-8")
        probe.unlink()
        return True
    except Exception:
        return False
