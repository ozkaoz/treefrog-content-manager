import pathlib, zipfile, re, os, tempfile

LIMITS = {"max_entries": 1024, "max_expansion_bytes": 1024*1024*1024, "max_depth": 1, "max_total_files_per_job": 10000}

# Handler registry — ZIP implemented; 7z/RAR explicitly unsupported (precise reason)
HANDLERS = {
    ".zip": {"implemented": True, "library": "zipfile"},
    # 7z/RAR are EXPLICITLY unsupported (no maintained safe adapter).
    # They surface as unsupported_archive with a precise reason and can
    # never be extracted or copied as supported content. Only ZIP is supported.
    ".7z": {"implemented": False, "reason": "7z archives are not supported (only ZIP is supported)"},
    ".rar": {"implemented": False, "reason": "RAR archives are not supported (only ZIP is supported)"},
}

class ArchiveError(ValueError):
    pass

class UnsupportedArchive(ArchiveError):
    pass

class SafetyViolation(ArchiveError):
    pass

class CollisionError(SafetyViolation):
    pass

class NestedArchiveBomb(SafetyViolation):
    pass

# Regex for Windows drive-letter absolute paths like C:/, C:\, C:, D:foo
_DRIVE_RE = re.compile(r'^[a-zA-Z]:([/\\]|$)')
_UNC_RE = re.compile(r'^\\\\[^\\]+\\[^\\]+')

def _is_windows_absolute(name: str) -> bool:
    if _DRIVE_RE.match(name):
        return True
    if _UNC_RE.match(name):
        return True
    # also \\?\ and similar
    if name.startswith('\\\\?\\'):
        return True
    return False

def _check_entry_safety(name: str, info, limits):
    # Absolute paths
    if name.startswith('/') or name.startswith('\\'):
        raise SafetyViolation(f"absolute path entry: {name}")
    if _is_windows_absolute(name):
        raise SafetyViolation(f"windows drive-letter absolute entry: {name}")
    # also PurePath absolute (posix/windows)
    if pathlib.PurePath(name).is_absolute():
        raise SafetyViolation(f"absolute path entry (pure): {name}")
    # Traversal
    # Use PurePath parts to detect ParentDir
    # Need to handle both / and \ separators: normalize to /
    normalized = name.replace('\\', '/')
    parts = pathlib.PurePath(normalized).parts
    if '..' in parts:
        raise SafetyViolation(f"traversal entry: {name}")
    # Also check for any part == '..' after split
    for p in parts:
        if p == '..':
            raise SafetyViolation(f"traversal entry: {name}")
    # Symlink / hardlink hazards
    # zip external_attr: top 16 bits are unix mode
    mode = (info.external_attr >> 16) & 0o170000 if hasattr(info, 'external_attr') else 0
    is_symlink = mode == 0o120000
    # Also check via zip info is_symlink helper if available (Python 3.8+ zipfile has is_symlink? not, but we check attribute)
    # Hardlink-like: if mode is not regular file (0o100000), dir (0o040000), symlink (0o120000) and not 0, treat as unsafe
    if is_symlink:
        raise SafetyViolation(f"symlink hazard: {name}")
    if mode != 0 and mode not in (0o100000, 0o040000, 0o120000):
        # Could be FIFO, device, hardlink etc
        raise SafetyViolation(f"hardlink/unsafe file type hazard: {name} mode={oct(mode)}")
    # Also reject entries with Windows ADS (alternate data stream) like "file.txt:stream"
    if ':' in name and not _DRIVE_RE.match(name):
        # On Windows, colon in filename indicates ADS, but on POSIX it's allowed. For safety on TreeFrog (Linux), colon is suspicious inside archive member.
        # We treat colon inside path (not drive letter) as hazard if it appears after first char and not part of drive
        # Allow colon only if it's part of something like "file:name" ??? For safety, reject colon in member name on Windows-style archives
        # But to avoid false positive for legitimate files, only reject if name contains ":\\0 or //"? We'll check for colon followed by anything that looks like ADS
        # Simple: if ':' in pathlib.PurePath(name).name and not name.lower().endswith(('.cue', '.bin')): treat as hazard if contains ':'
        # For now, flag colon in filename as potential ADS hazard when on Windows-style archive
        # We'll be conservative: if ':' in name and '\\' not in name, still flag if ':' not at position 1 (drive)
        # Actually many valid files don't contain colon; so we can flag any colon that is not drive-letter as hazard
        if ':' in name:
            raise SafetyViolation(f"hardlink/ADS hazard (colon in name): {name}")

def inspect_zip(path: pathlib.Path, limits=None):
    if limits is None:
        limits = LIMITS
    if not zipfile.is_zipfile(path):
        raise ValueError(f"not a zip: {path}")
    entries = []
    total = 0
    with zipfile.ZipFile(path, "r") as z:
        infos = z.infolist()
        if len(infos) > limits["max_entries"]:
            raise SafetyViolation(f"archive exceeds max entries {len(infos)} > {limits['max_entries']}")
        # For collision detection: normalized lowercased dest -> original name
        seen = {}
        for info in infos:
            name = info.filename
            # Empty name or directory only? Still check safety for directories
            _check_entry_safety(name, info, limits)
            # Skip directories for expansion tally but still need collision check?
            if info.is_dir():
                # Directories still need safety but not counted for expansion
                # Also check for collision on directory names? Still track
                norm = name.replace('\\', '/').lower().rstrip('/')
                if norm in seen:
                    raise CollisionError(f"collision: duplicate normalized path {name} vs {seen[norm]}")
                seen[norm] = name
                entries.append({"name": name, "is_dir": True, "size": 0, "compressed_size": info.compress_size})
                continue
            # Check for nested archive bomb: if entry is itself an archive and depth would exceed limit
            # For Phase 2A we just note it; planner will decide manual_review if depth exceeded
            # Here we just count, but if we detect nested archive we let planner handle; but we still enforce total file count
            # Expansion size
            total += info.file_size
            if total > limits["max_expansion_bytes"]:
                raise SafetyViolation(f"archive exceeds max expansion {total} > {limits['max_expansion_bytes']}")
            # Compression ratio bomb check (if compressed_size >0)
            if info.compress_size > 0 and info.file_size > 0:
                ratio = info.file_size / max(1, info.compress_size)
                if ratio > limits.get("max_compression_ratio", 100) and info.file_size > 1024*1024:
                    # Only flag if file is large enough to be bomb-ish
                    raise SafetyViolation(f"excessive compression ratio {ratio:.1f} for {name}")
            # Collision detection on file entries
            norm = name.replace('\\', '/').lower()
            if norm in seen:
                raise CollisionError(f"collision: normalized path duplicate {name} vs {seen[norm]}")
            seen[norm] = name
            # Also check for hardlink-like: duplicate content via same CRC? Could indicate hardlink, but treat as collision already
            entries.append({"name": name, "is_dir": False, "size": info.file_size, "compressed_size": info.compress_size, "crc": info.CRC})
        # Also check total files per job limit (not per archive but overall job, handled by planner)
    return entries

def inspect_archive(path: pathlib.Path, profile=None, limits=None):
    """Abstraction dispatcher: returns entries or raises UnsupportedArchive."""
    ext = path.suffix.lower()
    handler = HANDLERS.get(ext)
    if handler is None:
        raise UnsupportedArchive(f"unsupported archive format: {ext}")
    if not handler.get("implemented"):
        raise UnsupportedArchive(handler.get("reason", f"archive handler not available for {ext}"))
    if ext == ".zip":
        return inspect_zip(path, limits)
    raise UnsupportedArchive(f"handler for {ext} not implemented")

def is_archive_runtime_payload(path: pathlib.Path, inner_entries, profile):
    # Profile-driven decision: consult archive_policy per_system
    # If profile has archive_policy_full per_system mapping, use it
    archive_policy = {}
    if profile and isinstance(profile, dict):
        archive_policy = profile.get("archive_policy_full", {}) or profile.get("archive_policy", {})
    per_system = archive_policy.get("per_system", {}) if isinstance(archive_policy, dict) else {}
    # Heuristic fallback: if inner contains known ROM exts -> extract, else payload
    ext_to_system = profile.get("ext_to_system", {}) if profile else {}
    has_known_inner = False
    for e in inner_entries:
        if e["is_dir"]:
            continue
        inner_ext = pathlib.Path(e["name"]).suffix.lower()
        if inner_ext in ext_to_system and inner_ext not in (".zip",".7z",".rar"):
            has_known_inner = True
            break
        # also check for cue/bin etc that are not in ext_to_system but are known
        if inner_ext in (".cue",".bin",".chd",".m3u",".sfc",".nes",".gba",".gb",".gbc",".md",".sms",".gg"):
            has_known_inner = True
            break
    # If no profile per_system override, use heuristic
    # Try to find if any per_system rule says payload for this archive ext
    # For now, if has_known_inner -> not payload (extract)
    if has_known_inner:
        return False
    # No known inner -> treat as payload (e.g., arcade zip containing opaque blobs)
    # Check if any system has payload mode for this ext: if so, payload else also payload as fallback
    return True

def safe_extract_to_temp(archive_path: pathlib.Path, temp_dir: pathlib.Path, limits=None):
    """Extract archive safely to temp_dir, validating all paths stay inside temp_dir. Returns list of extracted paths. Never overwrites source."""
    if limits is None:
        limits = LIMITS
    # Use inspect to validate first
    entries = inspect_archive(archive_path, limits=limits)
    extracted = []
    # Now extract each entry
    with zipfile.ZipFile(archive_path, "r") as z:
        for info in z.infolist():
            name = info.filename
            # Re-check safety (defense in depth)
            _check_entry_safety(name, info, limits)
            # Resolve destination inside temp_dir
            # Normalize separators
            normalized = name.replace('\\', '/')
            dest = temp_dir / normalized
            # Ensure dest is within temp_dir
            try:
                dest.resolve().relative_to(temp_dir.resolve())
            except ValueError:
                raise SafetyViolation(f"extraction would escape temp dir: {name} -> {dest}")
            # Also check string prefix
            if not str(dest.resolve()).startswith(str(temp_dir.resolve())):
                raise SafetyViolation(f"extraction escape: {dest}")
            # Never overwrite source file: ensure dest is not the archive itself (should be inside temp, not source)
            if dest.resolve() == archive_path.resolve():
                raise SafetyViolation(f"would overwrite source archive: {dest}")
            # Create parent dirs
            if info.is_dir():
                dest.mkdir(parents=True, exist_ok=True)
            else:
                dest.parent.mkdir(parents=True, exist_ok=True)
                # Collision check: if file already exists in temp, it's a collision (should not overwrite silently)
                if dest.exists():
                    raise CollisionError(f"output collision in temp: {dest} already exists")
                # Extract via read/write to avoid zipfile extractall which may not check
                data = z.read(name)
                # Double-check expansion already validated, but verify write size
                if len(data) > limits["max_expansion_bytes"]:
                    raise SafetyViolation("single file expansion exceeds limit")
                dest.write_bytes(data)
                extracted.append(dest)
    return extracted

def safe_join(dest_root: pathlib.Path, dest_dir: str, file_name: str) -> pathlib.Path:
    if ".." in file_name or pathlib.Path(file_name).is_absolute():
        raise ValueError(f"unsafe file_name: {file_name}")
    if _is_windows_absolute(file_name):
        raise ValueError(f"unsafe file_name windows absolute: {file_name}")
    dest = dest_root / dest_dir / file_name
    # prevent escape: dest must be within dest_root
    try:
        dest.resolve().relative_to(dest_root.resolve())
    except ValueError:
        raise ValueError(f"dest escapes root: {dest} not in {dest_root}")
    except Exception:
        if ".." in dest_dir:
            raise ValueError(f"dest_dir traversal: {dest_dir}")
    # Also string prefix check for non-existent paths
    if not str(dest).startswith(str(dest_root)):
        # More robust: check normalized
        try:
            dest.relative_to(dest_root)
        except:
            raise ValueError(f"dest escapes root (string): {dest}")
    return dest

def get_handler_for_extension(ext: str):
    return HANDLERS.get(ext.lower())
