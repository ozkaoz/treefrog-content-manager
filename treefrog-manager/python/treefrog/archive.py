import pathlib, zipfile

LIMITS = {"max_entries": 1024, "max_expansion_bytes": 1024*1024*1024, "max_depth": 1}

def inspect_zip(path: pathlib.Path, limits=None):
    if limits is None:
        limits = LIMITS
    if not zipfile.is_zipfile(path):
        raise ValueError(f"not a zip: {path}")
    entries = []
    total = 0
    with zipfile.ZipFile(path, "r") as z:
        if len(z.infolist()) > limits["max_entries"]:
            raise ValueError(f"archive exceeds max entries {len(z.infolist())} > {limits['max_entries']}")
        for info in z.infolist():
            name = info.filename
            # safety: absolute / traversal / symlink
            if name.startswith("/") or name.startswith("\\") or pathlib.PurePath(name).is_absolute():
                raise ValueError(f"absolute path entry: {name}")
            # traversal
            if ".." in pathlib.PurePath(name).parts:
                raise ValueError(f"traversal entry: {name}")
            # symlink hazard: zip external_attr symlink check (unix file type)
            # external_attr >>16 is unix mode; check symlink bit 0o120000
            is_symlink = (info.external_attr >> 16) & 0o170000 == 0o120000
            if is_symlink or info.is_dir() and False:
                raise ValueError(f"symlink hazard: {name}")
            total += info.file_size
            if total > limits["max_expansion_bytes"]:
                raise ValueError(f"archive exceeds max expansion {total} > {limits['max_expansion_bytes']}")
            entries.append({"name": name, "is_dir": info.is_dir(), "size": info.file_size})
    return entries

def is_archive_runtime_payload(path: pathlib.Path, inner_entries, profile):
    # if inner contains known ROM exts (non-archive), extract; else payload
    ext_to_system = profile["ext_to_system"]
    has_known_inner = False
    for e in inner_entries:
        if e["is_dir"]:
            continue
        inner_ext = pathlib.Path(e["name"]).suffix.lower()
        if inner_ext in ext_to_system and inner_ext not in (".zip",".7z",".rar"):
            has_known_inner = True
            break
    if has_known_inner:
        return False
    # if no known inner, treat as payload (e.g., cps1 zip)
    return True

def safe_join(dest_root: pathlib.Path, dest_dir: str, file_name: str) -> pathlib.Path:
    if ".." in file_name or pathlib.Path(file_name).is_absolute():
        raise ValueError(f"unsafe file_name: {file_name}")
    dest = dest_root / dest_dir / file_name
    # prevent escape: dest must be within dest_root (simple prefix)
    try:
        dest.relative_to(dest_root.resolve())
    except Exception:
        # Use manual check if not exists yet
        if ".." in dest_dir:
            raise ValueError(f"dest_dir traversal: {dest_dir}")
    return dest
