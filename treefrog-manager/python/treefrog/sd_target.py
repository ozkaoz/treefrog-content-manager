import pathlib, os, json, shutil

def get_volume_info(path: str):
    p = pathlib.Path(path)
    accessible = p.exists()
    error = None
    free = None
    total = None
    label = None
    filesystem = None
    removable = None
    if not accessible:
        error = f"path not found: {path}"
    else:
        try:
            # Try to read dir to check accessibility
            next(p.iterdir(), None)
        except Exception as e:
            error = f"read_dir failed: {e}"
            accessible = False
        # Get free space via shutil
        try:
            usage = shutil.disk_usage(str(p))
            total = usage.total
            free = usage.free
        except:
            pass
        # Try to get label/filesystem on Windows via ctypes if needed, keep None for now
    return {
        "path": path,
        "label": label,
        "filesystem": filesystem,
        "total_bytes": total,
        "free_bytes": free,
        "removable": removable,
        "accessible": accessible,
        "error": error,
    }

def list_volumes():
    # On Windows, enumerate drives A-Z; on other OS, return empty
    import string, pathlib
    out = []
    try:
        import ctypes
        # Windows: use GetLogicalDrives
        drives = []
        for letter in string.ascii_uppercase:
            drive = f"{letter}:\\"
            if pathlib.Path(drive).exists():
                out.append(get_volume_info(drive))
    except:
        pass
    # Fallback: if on Windows and no volumes found, try to list via shutil for current path
    return out

def analyze_target(path: str):
    p = pathlib.Path(path)
    vol = get_volume_info(path)
    errors = []
    if vol["error"]:
        errors.append(vol["error"])
    if not vol["accessible"]:
        return {
            "path": path,
            "volume": vol,
            "status": "inaccessible",
            "is_treefrog": False,
            "is_incomplete": False,
            "markers_found": [],
            "markers_missing": ["cubegm", "roms"],
            "lgpt_detected": False,
            "rom_dirs": [],
            "media_dirs": [],
            "bios_dirs": [],
            "lgpt_dirs": [],
            "existing_count": 0,
            "total_size": 0,
            "free_bytes": vol["free_bytes"],
            "capacity_bytes": vol["total_bytes"],
            "filesystem": vol["filesystem"],
            "label": vol["label"],
            "errors": errors,
            "stable_id": None,
            "physical_device": None,
        }
    # Load markers from sd_markers.json if available, else defaults
    required = ["cubegm", "roms"]
    found = []
    missing = []
    for m in required:
        if (p / m).exists():
            found.append(m)
        else:
            missing.append(m)
    # Optional
    for m in ["frogui", "lgpt", "cubegm/cores", "cubegm/bios"]:
        if (p / m).exists() and m not in found:
            found.append(m)
    is_treefrog = "cubegm" in found and "roms" in found
    is_incomplete = not is_treefrog and ("cubegm" in found or "roms" in found)
    if not vol["accessible"]:
        status = "inaccessible"
    elif is_treefrog:
        status = "valid"
    elif is_incomplete:
        status = "incomplete"
    else:
        status = "unknown"
    lgpt_detected = (p / "lgpt").exists()
    rom_dirs = []
    media_dirs = []
    bios_dirs = []
    lgpt_dirs = []
    existing_count = 0
    total_size = 0
    roms_path = p / "roms"
    if roms_path.exists():
        try:
            for ent in roms_path.iterdir():
                if ent.is_dir() and not ent.is_symlink():
                    name = ent.name
                    low = name.lower()
                    if low in ("music", "videos", "images", "ebook"):
                        media_dirs.append(name)
                    elif low == "bios":
                        bios_dirs.append(name)
                    else:
                        rom_dirs.append(name)
        except:
            pass
        for ent in roms_path.rglob("*"):
            try:
                if ent.is_file() and not ent.is_symlink():
                    existing_count += 1
                    total_size += ent.stat().st_size
            except:
                pass
    if (p / "cubegm" / "bios").exists():
        if "cubegm/bios" not in bios_dirs:
            bios_dirs.append("cubegm/bios")
        for ent in (p / "cubegm" / "bios").rglob("*"):
            try:
                if ent.is_file():
                    existing_count += 1
                    total_size += ent.stat().st_size
            except:
                pass
    if lgpt_detected:
        if (p / "lgpt" / "samples").exists():
            lgpt_dirs.append("lgpt/samples")
        if (p / "lgpt" / "projects").exists():
            lgpt_dirs.append("lgpt/projects")
        if not lgpt_dirs:
            lgpt_dirs.append("lgpt")
        for ent in (p / "lgpt").rglob("*"):
            try:
                if ent.is_file():
                    existing_count += 1
                    total_size += ent.stat().st_size
            except:
                pass
    rom_dirs.sort()
    media_dirs.sort()
    bios_dirs.sort()
    lgpt_dirs.sort()
    # Stable ID: label + filesystem + total as proxy (real would be GUID+serial)
    stable_id = None
    if vol["label"] or vol["filesystem"] or vol["total_bytes"]:
        stable_id = f"{vol['label'] or ''}-{vol['filesystem'] or ''}-{vol['total_bytes'] or ''}"
        if not stable_id.strip("-"):
            stable_id = vol["path"]
        else:
            stable_id = stable_id.strip("-")
    else:
        stable_id = vol["path"]
    physical_device = {
        "device_path": vol["path"],
        "friendly_name": vol["label"],
        "bus_type": "USB" if vol["removable"] else "Fixed" if vol["removable"] is not None else None,
        "removable": bool(vol["removable"]) if vol["removable"] is not None else False,
        "is_usb": bool(vol["removable"]) if vol["removable"] is not None else False,
    } if vol["removable"] is not None else None
    return {
        "path": path,
        "volume": vol,
        "status": status,
        "is_treefrog": is_treefrog,
        "is_incomplete": is_incomplete,
        "markers_found": found,
        "markers_missing": missing,
        "lgpt_detected": lgpt_detected,
        "rom_dirs": rom_dirs,
        "media_dirs": media_dirs,
        "bios_dirs": bios_dirs,
        "lgpt_dirs": lgpt_dirs,
        "existing_count": existing_count,
        "total_size": total_size,
        "free_bytes": vol["free_bytes"],
        "capacity_bytes": vol["total_bytes"],
        "filesystem": vol["filesystem"],
        "label": vol["label"],
        "errors": errors,
        "stable_id": stable_id,
        "physical_device": physical_device,
    }

def validate_destination_path(dest: str):
    if not dest:
        raise ValueError("empty destination")
    if ".." in dest.split("/"):
        raise ValueError(f"traversal detected: {dest}")
    if dest.startswith("/") or dest.startswith("\\"):
        raise ValueError(f"absolute path not allowed: {dest}")
    if len(dest) >= 2 and dest[1] == ":" and dest[0].isalpha():
        raise ValueError(f"drive injection not allowed: {dest}")
    if dest.startswith("\\\\"):
        raise ValueError(f"UNC not allowed: {dest}")
    if ":" in dest:
        raise ValueError(f"ADS not allowed: {dest}")
    reserved = ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"]
    for part in dest.split("/"):
        if not part:
            raise ValueError(f"empty path component in {dest}")
        base = part.split(".")[0].upper()
        if base in reserved:
            raise ValueError(f"reserved name not allowed: {part}")
        if part.endswith(".") or part.endswith(" "):
            raise ValueError(f"trailing dot/space not allowed: {part}")
        for ch in '<>:"\\|?*':
            if ch in part:
                raise ValueError(f"illegal character '{ch}' in {part}")
    if "\\" in dest:
        raise ValueError(f"backslash not allowed in destination: {dest}")

def check_case_collision(dests):
    seen = {}
    out = []
    for d in dests:
        norm = d.lower()
        if norm in seen:
            out.append((d, seen[norm]))
        else:
            seen[norm] = d
    return out

def calculate_space(plan, free_bytes=None):
    to_copy = 0
    to_extract = 0
    to_generate = 0
    to_skip = 0
    for e in plan.get("entries", []):
        size = e.get("size") or 0
        action = e.get("action")
        if action == "copy":
            to_copy += size
        elif action == "extract":
            to_extract += size
        elif action == "convert_then_copy":
            to_generate += size
        elif action in ("skip_unchanged", "skip_duplicate", "skip"):
            to_skip += size
        # resolved_action
        ra = e.get("resolved_action")
        if ra and ra != action:
            if ra in ("copy", "replace"):
                to_copy += size
            elif ra == "extract":
                to_extract += size
            elif ra == "convert_then_copy":
                to_generate += size
    required = to_copy + to_extract + to_generate
    if free_bytes is not None:
        status = "insufficient_space" if required > free_bytes else "ok"
    else:
        status = "unknown"
    return {
        "bytes_to_copy": to_copy,
        "bytes_to_extract": to_extract,
        "bytes_to_generate": to_generate,
        "bytes_to_skip": to_skip,
        "required_bytes": required,
        "available_bytes": free_bytes,
        "status": status,
    }
