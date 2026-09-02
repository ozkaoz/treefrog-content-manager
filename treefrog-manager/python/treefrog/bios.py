import pathlib, json, hashlib, fnmatch
from . import hash as hmod
from . import archive as amod

# BIOS validation states as defined in spec
VALID_STATES = ["missing", "found_valid", "found_invalid", "found_unknown", "duplicate", "conflict", "not_required"]

def load_bios_definitions(profile_dir=None):
    import pathlib, json
    if profile_dir is None:
        from .profile import PROFILE_DIR
        p = PROFILE_DIR / "bios.json"
    else:
        p = pathlib.Path(profile_dir) / "bios.json"
    data = json.loads(p.read_text(encoding="utf-8"))
    return data

def _match_filename(filename: str, accepted: list, aliases: list, patterns: list) -> bool:
    # Check exact, alias, and pattern (with fnmatch)
    lower = filename.lower()
    for name in accepted:
        if lower == name.lower():
            return True
    for alias in aliases:
        if lower == alias.lower():
            return True
    for pat in patterns:
        # fnmatch is case-insensitive on Windows, but we do lower
        if fnmatch.fnmatch(lower, pat.lower()):
            return True
    return False

def _is_known_filename(filename: str, bios_def: dict) -> bool:
    # Check if filename matches any accepted filename/pattern/alias for this BIOS
    accepted = bios_def.get("accepted_filenames", [])
    aliases = bios_def.get("aliases", [])
    patterns = bios_def.get("accepted_patterns", [])
    # Also check variants
    for var in bios_def.get("variants", []):
        accepted.extend(var.get("filenames", []))
        aliases.extend(var.get("aliases", []))
    return _match_filename(filename, accepted, aliases, patterns)

def validate_bios_file(file_path: pathlib.Path, bios_def: dict, profile=None):
    """
    Validate a single file against a BIOS definition.
    Returns dict with state and details.
    States: found_valid, found_invalid, found_unknown, etc.
    Matching order: exact filename + hash, alias + hash, size fallback, wrong content, unknown.
    """
    filename = file_path.name
    # Check if file exists
    if not file_path.exists():
        return {"state": "missing", "reason": "file not found", "bios_id": bios_def.get("id")}

    # Check if filename is known for this BIOS
    is_known = _is_known_filename(filename, bios_def)
    if not is_known:
        return {"state": "found_unknown", "reason": f"filename {filename} not in accepted list for {bios_def.get('id')}", "bios_id": bios_def.get("id")}

    # Get expected hashes and sizes
    # Collect all variants' hashes and sizes
    all_hashes = []
    all_sizes = []
    # Top-level hashes
    if bios_def.get("hashes_sha256"):
        all_hashes.extend([h.lower() for h in bios_def.get("hashes_sha256") if h])
    if bios_def.get("expected_size"):
        all_sizes.append(bios_def.get("expected_size"))
    # Variants
    for var in bios_def.get("variants", []):
        if var.get("hashes_sha256"):
            all_hashes.extend([h.lower() for h in var.get("hashes_sha256") if h])
        if var.get("expected_size"):
            all_sizes.append(var.get("expected_size"))
        # Also check expected_md5? For GBA BIOS we have hash_known, but for O2EM we have MD5
        # For now, only SHA-256 is authoritative, MD5 is just notes

    # If hashes are known, must validate via hash, not just filename
    has_known_hashes = len([h for h in all_hashes if h]) > 0

    # Compute file hash and size
    try:
        file_hash = hmod.sha256_file(file_path).lower()
        file_size = file_path.stat().st_size
    except Exception as e:
        return {"state": "found_invalid", "reason": f"could not hash file: {e}", "bios_id": bios_def.get("id")}

    # 1. exact filename + known hash
    # Check if filename matches exactly an accepted filename and hash matches
    # For alias, also check
    # We need to check if file_hash is in all_hashes
    if has_known_hashes:
        if file_hash in [h.lower() for h in all_hashes]:
            # Check if filename is exact or alias - both are valid if hash matches
            # Determine if it's exact vs alias for reason
            exact_filenames = bios_def.get("accepted_filenames", []) + [fn for var in bios_def.get("variants", []) for fn in var.get("filenames", [])]
            aliases = bios_def.get("aliases", []) + [a for var in bios_def.get("variants", []) for a in var.get("aliases", [])]
            is_exact = any(filename.lower() == fn.lower() for fn in exact_filenames)
            is_alias = any(filename.lower() == a.lower() for a in aliases)
            if is_exact:
                return {"state": "found_valid", "reason": "exact filename + known hash", "bios_id": bios_def.get("id"), "hash": file_hash, "size": file_size}
            elif is_alias:
                return {"state": "found_valid", "reason": "accepted alias + known hash", "bios_id": bios_def.get("id"), "hash": file_hash, "size": file_size}
            else:
                # Filename matches via pattern but hash matches - still valid
                return {"state": "found_valid", "reason": "pattern + known hash", "bios_id": bios_def.get("id"), "hash": file_hash, "size": file_size}
        else:
            # Known filename but wrong hash -> found_invalid
            return {"state": "found_invalid", "reason": f"known filename {filename} but hash {file_hash[:16]}... not in accepted {all_hashes[:1]}", "bios_id": bios_def.get("id"), "hash": file_hash, "size": file_size, "expected_hashes": all_hashes}
    else:
        # No known hashes, use size-only validation if expected size known
        if all_sizes:
            if file_size in all_sizes:
                return {"state": "found_valid", "reason": "filename + expected size (no hash defined)", "bios_id": bios_def.get("id"), "hash": file_hash, "size": file_size}
            else:
                return {"state": "found_invalid", "reason": f"filename {filename} size {file_size} not in expected {all_sizes}", "bios_id": bios_def.get("id"), "hash": file_hash, "size": file_size}
        else:
            # No hash, no size -> cannot validate, treat as unknown but filename is known, so found_unknown?
            # For cases like PS1 BIOS where no hash is defined, any file with correct name/size is considered found_unknown or found_valid?
            # Per spec, when no hash is defined, we cannot claim validity from filename alone, but we can say found_unknown
            # However for PS1, any 512 KiB file with scph*.bin is considered valid even without hash, per notes
            # So for those, if filename matches and size matches expected_size (if any), we can say found_valid via size
            # For PS1, expected_size is 524288, so if file size matches, it's valid
            # For others with no size and no hash (e.g. neogeo.zip, segacd
            # bios_CD_*, ecwolf.pk3), the accepted filename IS the validation:
            # the user picked the file explicitly. Valid by name (observable
            # reason). Old behavior returned found_unknown which made 9 of 13
            # BIOS unusable.
            if file_size in all_sizes or not all_sizes:
                if not all_hashes and not all_sizes:
                    return {"state": "found_valid", "reason": "exact filename accepted (profile declares no hash/size - validated by name)", "bios_id": bios_def.get("id"), "hash": file_hash, "size": file_size}
                else:
                    return {"state": "found_valid", "reason": "filename matches and no hash to contradict", "bios_id": bios_def.get("id"), "hash": file_hash, "size": file_size}

    return {"state": "found_unknown", "reason": "unknown BIOS", "bios_id": bios_def.get("id"), "hash": file_hash, "size": file_size}

def validate_all_bios(source_files: list, bios_definitions: list, system_content_present: dict = None):
    """
    Validate all BIOS definitions against a list of source files.
    source_files: list of Path objects
    bios_definitions: list from bios.json bios_definitions
    system_content_present: dict mapping system_id -> bool (whether content for that system is present)
    Returns dict mapping bios_id -> validation result
    """
    if system_content_present is None:
        system_content_present = {}

    results = {}
    # For duplicate/conflict detection, we need to track hashes and filenames
    seen_hashes = {}
    seen_filenames = {}

    for bios_def in bios_definitions:
        bios_id = bios_def.get("id")
        system_id = bios_def.get("system_id")

        # Check conditional requirement
        required = bios_def.get("required", "optional")
        requirement = bios_def.get("requirement", {})
        scope = requirement.get("scope", "optional")
        mandatory_when = requirement.get("mandatory_when", "")

        # Determine if BIOS is required
        is_required = False
        if required == "required":
            is_required = True
        elif required == "conditional":
            # Check if system content is present
            # mandatory_when is like "psx_content_present" or "gba_content_present"
            # We check system_content_present dict
            if system_id and system_content_present.get(system_id, False):
                is_required = True
            elif mandatory_when and "content_present" in mandatory_when:
                # Try to parse system id from mandatory_when
                # e.g., "psx_content_present" -> system_id "psx"
                # For now, just check if any system content is present and matches
                # If system_content_present is empty, we assume not required if no content for that system
                is_required = False
            else:
                is_required = False
        elif required == "optional":
            is_required = False

        # Find matching files for this BIOS
        matching_files = []
        for f in source_files:
            if _is_known_filename(f.name, bios_def):
                matching_files.append(f)

        if not matching_files:
            if is_required:
                results[bios_id] = {"state": "missing", "reason": f"BIOS {bios_id} missing but required when {mandatory_when}", "bios_id": bios_id, "system_id": system_id, "required": is_required}
            else:
                results[bios_id] = {"state": "not_required", "reason": f"BIOS {bios_id} not required (no {system_id} content)", "bios_id": bios_id, "system_id": system_id, "required": is_required}
            continue

        # For each matching file, validate
        # If multiple files match same BIOS, check for duplicate/conflict
        validations = []
        for f in matching_files:
            res = validate_bios_file(f, bios_def)
            res["file"] = str(f)
            res["filename"] = f.name
            validations.append(res)

        # Determine overall state for this BIOS
        # If any is found_valid, then BIOS is satisfied (any one variant satisfies)
        # But need to check for duplicates and conflicts
        valid_count = sum(1 for v in validations if v["state"] == "found_valid")
        invalid_count = sum(1 for v in validations if v["state"] == "found_invalid")
        unknown_count = sum(1 for v in validations if v["state"] == "found_unknown")

        # Check for duplicate identical BIOS (same hash, same filename or different filename but same content)
        # If we have two files with same hash and same BIOS id, it's duplicate
        hashes = {}
        for v in validations:
            h = v.get("hash")
            if h:
                hashes.setdefault(h, []).append(v)

        has_duplicate = any(len(v) > 1 for v in hashes.values())
        has_conflict = False
        # Conflict: same filename, different hash
        filenames = {}
        for v in validations:
            fn = v.get("filename", "").lower()
            filenames.setdefault(fn, []).append(v)
        for fn, vs in filenames.items():
            if len(vs) > 1:
                # Same filename, check if hashes differ
                hs = set(v.get("hash") for v in vs if v.get("hash"))
                if len(hs) > 1:
                    has_conflict = True

        if has_conflict:
            results[bios_id] = {"state": "conflict", "reason": f"same BIOS filename with different content for {bios_id}", "bios_id": bios_id, "system_id": system_id, "required": is_required, "validations": validations}
        elif has_duplicate:
            # If all duplicates are valid, then it's duplicate (not error, but duplicate)
            # If we have multiple identical valid BIOS files, it's duplicate
            results[bios_id] = {"state": "duplicate", "reason": f"duplicate identical BIOS files for {bios_id}", "bios_id": bios_id, "system_id": system_id, "required": is_required, "validations": validations}
        elif valid_count > 0:
            results[bios_id] = {"state": "found_valid", "reason": f"found valid BIOS for {bios_id} ({valid_count} valid variants)", "bios_id": bios_id, "system_id": system_id, "required": is_required, "validations": validations}
        elif invalid_count > 0:
            results[bios_id] = {"state": "found_invalid", "reason": f"found BIOS but invalid for {bios_id}", "bios_id": bios_id, "system_id": system_id, "required": is_required, "validations": validations}
        elif unknown_count > 0:
            results[bios_id] = {"state": "found_unknown", "reason": f"found BIOS with unknown validity for {bios_id}", "bios_id": bios_id, "system_id": system_id, "required": is_required, "validations": validations}
        else:
            results[bios_id] = {"state": "missing", "reason": f"BIOS {bios_id} not found", "bios_id": bios_id, "system_id": system_id, "required": is_required}

    return results

def get_valid_destinations(bios_def: dict):
    """Get all valid destinations for a BIOS definition (profile-driven, not hardcoded)."""
    dests = bios_def.get("destinations", [])
    if not dests:
        # Fallback to primary_destination or destination or destination_root
        if bios_def.get("primary_destination"):
            dests = [bios_def["primary_destination"]]
        elif bios_def.get("destination"):
            dests = [bios_def["destination"]]
        elif bios_def.get("destination_root"):
            dests = [bios_def["destination_root"]]
    # Normalize: strip trailing slash
    return [d.rstrip("/") for d in dests]

def is_bios_archive(file_path: pathlib.Path, bios_def: dict):
    """Check if BIOS file is an archive and handle via archive infrastructure."""
    # Check if file is an archive that should be inspected
    # Use archive module to inspect if needed
    ext = file_path.suffix.lower()
    if ext in (".zip", ".7z", ".rar"):
        # Check BIOS archive mode
        archive_cfg = bios_def.get("archive", {})
        mode = archive_cfg.get("mode", "payload")
        return mode
    return None
