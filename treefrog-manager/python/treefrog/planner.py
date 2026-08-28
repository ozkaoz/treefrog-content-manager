import pathlib, hashlib, tempfile, os
from . import hash as hmod
from . import archive as amod
from . import classify as cl

def _dest_exists(p: pathlib.Path) -> bool:
    return p.exists()

def _classify_inner_member(member_name: str, profile):
    # member_name is archive entry name like "a/b/game.gba"
    # we classify by extension only, but also check for grouping
    p = pathlib.Path(member_name)
    return cl.classify(p, profile)

def _group_members(entries, profile):
    """Group archive members that form one logical game (CUE/BIN etc).
    Returns list of groups: each group is list of entries that belong together.
    For now, CUE+BIN in same folder.
    """
    # Map folder -> entries
    by_folder = {}
    for e in entries:
        if e["is_dir"]:
            continue
        folder = str(pathlib.PurePath(e["name"]).parent).replace('\\','/')
        by_folder.setdefault(folder, []).append(e)
    groups = []
    used = set()
    for folder, ents in by_folder.items():
        # Look for .cue files
        cues = [e for e in ents if pathlib.Path(e["name"]).suffix.lower() == ".cue"]
        for cue in cues:
            cue_name = cue["name"]
            # Already used?
            if cue_name in used:
                continue
            # Parse cue content heuristic: we don't have actual cue content here, but we can group by sibling .bin with same basename
            # Also check if cue references FILE "xxx.bin" - we don't have content, so use basename grouping
            stem = pathlib.Path(cue_name).stem
            # Find sibling .bin files with same stem or any .bin in same folder
            siblings = [e for e in ents if e["name"] not in used and pathlib.Path(e["name"]).suffix.lower() == ".bin" and pathlib.Path(e["name"]).stem == stem]
            # Also include .bin that might be referenced but different stem? For now group all .bin in same folder with this cue if only one cue
            if not siblings and len(cues) == 1:
                # If single cue in folder, group all .bin in that folder with it
                siblings = [e for e in ents if e["name"] not in used and pathlib.Path(e["name"]).suffix.lower() == ".bin"]
            if siblings:
                grp = [cue] + siblings
                for m in grp:
                    used.add(m["name"])
                groups.append(grp)
            else:
                # Cue without bin (maybe bin missing), treat cue alone
                used.add(cue_name)
                groups.append([cue])
        # Now handle remaining entries not grouped (including .bin without cue)
        for e in ents:
            if e["name"] not in used:
                groups.append([e])
                used.add(e["name"])
    # For entries not in any folder grouping (should already be covered), ensure they are singletons
    return groups

def _decide_archive_mode(archive_path: pathlib.Path, inner_entries, profile):
    ext = archive_path.suffix.lower()
    archive_policy = profile.get("archive_policy_full", {}) if profile else {}
    per_system = archive_policy.get("per_system", {}) if isinstance(archive_policy, dict) else {}
    handlers = archive_policy.get("handlers", {}) if isinstance(archive_policy, dict) else {}
    # Check handler availability first
    handler = handlers.get(ext) if isinstance(handlers, dict) else None
    if handler and not handler.get("implemented", True):
        return "unsupported"
    # If no inner entries (empty archive), manual_review
    if not inner_entries:
        return "manual"
    # Early grouped detection: CUE+BIN in same folder should be grouped regardless of system mixed
    # Check for any folder containing both .cue and .bin with same stem or single cue + bin
    has_grouped = False
    by_folder = {}
    for e in inner_entries:
        if e["is_dir"]:
            continue
        folder = str(pathlib.PurePath(e["name"]).parent).replace('\\','/')
        by_folder.setdefault(folder, []).append(e)
    for folder, ents in by_folder.items():
        has_cue = any(pathlib.Path(x["name"]).suffix.lower()==".cue" for x in ents)
        has_bin = any(pathlib.Path(x["name"]).suffix.lower()==".bin" for x in ents)
        if has_cue and has_bin:
            has_grouped = True
            break
    if has_grouped:
        return "grouped"
    # Check for nested archives: any inner entry is itself an archive
    has_nested = any(pathlib.Path(e["name"]).suffix.lower() in (".zip",".7z",".rar") for e in inner_entries if not e["is_dir"])
    if has_nested:
        limits = archive_policy.get("safety", {}).get("limits", {}) if isinstance(archive_policy, dict) else {}
        max_depth = limits.get("max_depth", 1) if isinstance(limits, dict) else 1
        if max_depth <= 1:
            return "manual"
    # Determine system inference: classify each inner member
    systems_for_members = []
    for e in inner_entries:
        if e["is_dir"]:
            continue
        inner_ext = pathlib.Path(e["name"]).suffix.lower()
        ext_to_system = profile.get("ext_to_system", {}) if profile else {}
        sys_ids = ext_to_system.get(inner_ext, [])
        if sys_ids:
            systems_for_members.append(sys_ids[0])
        elif inner_ext in (".cue",".bin",".chd",".m3u"):
            systems_for_members.append("grouped_hint")
        else:
            systems_for_members.append(None)
    known_systems = [s for s in systems_for_members if s not in (None, "grouped_hint")]
    if not known_systems:
        for sys_id, modes in per_system.items():
            if isinstance(modes, dict) and modes.get(ext) == "payload":
                return "payload"
        return "payload"
    unique_known = set(known_systems)
    if len(unique_known) == 1:
        sys_id = list(unique_known)[0]
        per = per_system.get(sys_id, {})
        if isinstance(per, dict):
            mode = per.get(ext)
            if mode:
                return mode
        if "grouped_hint" in systems_for_members:
            return "grouped"
        return "extract_and_classify"
    # Mixed systems: still extract_and_classify each to its correct system folder (not manual)
    # Only manual if ambiguous payload vs container, not for normal mixed ROM zip
    if len(unique_known) > 1:
        if "grouped_hint" in systems_for_members:
            return "grouped"
        return "extract_and_classify"
    return "extract_and_classify"

def _detect_collisions(destinations):
    # destinations: list of dest_rel strings
    seen = {}
    collisions = []
    for d in destinations:
        norm = d.replace('\\','/').lower()
        if norm in seen:
            collisions.append((d, seen[norm]))
        else:
            seen[norm] = d
    return collisions

def _content_type_for_classification(c, profile):
    kind = c.get("kind")
    if kind == "rom":
        sys_id = c.get("system_id") or "unknown"
        return f"rom/{sys_id}"
    if kind == "music":
        return "music"
    if kind == "video":
        return "video"
    if kind == "image":
        return "image"
    if kind == "ebook":
        return "ebook"
    if kind == "bios":
        return "bios"
    if kind == "lgpt_sample":
        return "lgpt/sample"
    if kind == "lgpt_project":
        return "lgpt/project"
    if kind == "archive":
        return "archive"
    return "unknown"

# Phase 2B resolution model
VALID_RESOLUTIONS = {"skip", "replace", "keep_both", "keep_destination", "keep_source"}

def _default_resolution_for_action(action: str) -> str:
    if action == "skip_duplicate":
        return "skip"
    if action == "skip_unchanged":
        return "skip"
    if action == "conflict":
        return "conflict"
    if action == "manual_review":
        return "manual_review"
    if action == "unsupported_archive":
        return "skip"
    if action in ("copy", "extract"):
        return "copy"
    return "skip"

def _apply_single_resolution(entry, resolution: str):
    orig_action = entry.get("action")
    dest = entry.get("destination")
    if not resolution or resolution not in VALID_RESOLUTIONS:
        return entry
    resolved = dict(entry)
    resolved["resolution"] = resolution
    if "default_action" not in resolved:
        resolved["default_action"] = orig_action
    if resolution == "skip":
        resolved["resolved_action"] = "skip"
        resolved["reason"] = entry.get("reason", "") + " [resolved: skip]"
    elif resolution == "keep_destination":
        resolved["resolved_action"] = "skip"
        resolved["reason"] = entry.get("reason", "") + " [resolved: keep_destination]"
    elif resolution in ("replace", "keep_source"):
        if orig_action in ("conflict", "manual_review", "skip_duplicate"):
            resolved["resolved_action"] = "copy" if orig_action == "skip_duplicate" else "replace"
            resolved["reason"] = entry.get("reason", "") + f" [resolved: {resolution}]"
        else:
            resolved["resolved_action"] = "copy"
            resolved["reason"] = entry.get("reason", "") + f" [resolved: {resolution}]"
    elif resolution == "keep_both":
        p = pathlib.Path(dest)
        if "." in p.name and p.suffix:
            new_name = f"{p.stem}_1{p.suffix}"
            new_dest = str(p.parent / new_name) if str(p.parent) != "." else new_name
        else:
            new_dest = dest + "_1"
        new_dest = new_dest.replace("\\", "/")
        resolved["destination"] = new_dest
        resolved["resolved_action"] = "copy" if orig_action in ("skip_duplicate", "conflict", "copy", "extract") else "extract"
        if orig_action == "extract":
            resolved["resolved_action"] = "extract"
        resolved["reason"] = entry.get("reason", "") + " [resolved: keep_both -> renamed]"
        resolved["original_destination"] = dest
    else:
        resolved["resolved_action"] = orig_action
    return resolved

def apply_resolutions(plan, decisions: dict):
    if not decisions:
        return plan
    new_entries = []
    for idx, entry in enumerate(plan.get("entries", [])):
        key = None
        if str(idx) in decisions:
            key = str(idx)
        elif entry.get("source") in decisions:
            key = entry.get("source")
        elif entry.get("destination") in decisions:
            key = entry.get("destination")
        combined = f"{entry.get('source')}->{entry.get('destination')}"
        if combined in decisions:
            key = combined
        resolution = decisions.get(key) if key else None
        if idx in decisions:
            resolution = decisions[idx]
        if resolution:
            new_entries.append(_apply_single_resolution(entry, resolution))
        else:
            e = dict(entry)
            if "resolved_action" not in e:
                e["resolved_action"] = e.get("action")
                e["resolution"] = _default_resolution_for_action(e.get("action"))
                if "default_action" not in e:
                    e["default_action"] = e.get("action")
            new_entries.append(e)
    new_plan = dict(plan)
    new_plan["entries"] = new_entries
    resolved_summary = {
        "skip": sum(1 for e in new_entries if e.get("resolved_action") == "skip"),
        "copy": sum(1 for e in new_entries if e.get("resolved_action") == "copy"),
        "extract": sum(1 for e in new_entries if e.get("resolved_action") == "extract"),
        "replace": sum(1 for e in new_entries if e.get("resolved_action") == "replace"),
        "conflict": sum(1 for e in new_entries if e.get("resolved_action") == "conflict"),
        "manual_review": sum(1 for e in new_entries if e.get("resolved_action") == "manual_review"),
    }
    new_plan["resolved_summary"] = resolved_summary
    return new_plan

def plan(scanned, sd_root: str, profile):
    sd_path = pathlib.Path(sd_root)
    entries = []
    unchanged = new = changed = duplicate = conflicts = manual = unsupported = 0
    hash_to_dest = {}
    # Pre-index SD file hashes for duplicate detection
    scanned_sizes = {sf["size"] for sf in scanned}
    sd_hash_map = {}  # sha256 -> dest_rel
    if sd_path.exists():
        for p in sd_path.rglob("*"):
            if p.is_file() and p.stat().st_size in scanned_sizes:
                try:
                    h = hmod.sha256_file(p)
                    rel = p.relative_to(sd_path).as_posix()
                    sd_hash_map[h] = rel
                except:
                    pass

    # Sort scanned deterministically
    scanned_sorted = sorted(scanned, key=lambda x: str(x["source_path"]))

    # For per-job limits
    archive_policy = profile.get("archive_policy_full", {}) if profile else {}
    limits = archive_policy.get("safety", {}).get("limits", {}) if isinstance(archive_policy, dict) else {}
    max_total_files = limits.get("max_total_files_per_job", 10000) if isinstance(limits, dict) else 10000
    total_planned_files = 0

    for sf in scanned_sorted:
        c = sf["classification"]
        kind = c["kind"]
        dest_base = c["destination"]
        file_name = sf["source_path"].name

        # Archive handling
        if kind == "archive":
            ext = sf["source_path"].suffix.lower()
            # Check handler availability first
            handler = amod.get_handler_for_extension(ext)
            if handler and not handler.get("implemented", True):
                dest_rel = f"{dest_base}/{file_name}" if dest_base else f"roms/UNKNOWN/{file_name}"
                entries.append({"source": str(sf["source_path"]), "destination": dest_rel, "action": "unsupported_archive", "reason": f"archive handler not available for {ext} (stub)", "hash": None, "size": sf["size"], "group": None})
                unsupported += 1
                continue
            # Try inspect
            try:
                inner = amod.inspect_archive(sf["source_path"], profile)
            except amod.UnsupportedArchive as e:
                dest_rel = f"{dest_base}/{file_name}" if dest_base else f"roms/UNKNOWN/{file_name}"
                entries.append({"source": str(sf["source_path"]), "destination": dest_rel, "action": "unsupported_archive", "reason": str(e), "hash": None, "size": sf["size"], "group": None})
                unsupported += 1
                continue
            except (amod.SafetyViolation, amod.CollisionError, amod.NestedArchiveBomb) as e:
                # Safety violations go to manual_review, not conflict
                dest_rel = f"{dest_base}/{file_name}" if dest_base else f"roms/UNKNOWN/{file_name}"
                entries.append({"source": str(sf["source_path"]), "destination": dest_rel, "action": "manual_review", "reason": f"archive safety violation: {e}", "hash": None, "size": sf["size"], "group": None})
                manual += 1
                continue
            except Exception as e:
                dest_rel = f"{dest_base}/{file_name}" if dest_base else f"roms/UNKNOWN/{file_name}"
                entries.append({"source": str(sf["source_path"]), "destination": dest_rel, "action": "manual_review", "reason": f"archive inspection failed: {e}", "hash": None, "size": sf["size"], "group": None})
                manual += 1
                continue

            # Check per-job total files limit
            # Count inner non-dir entries
            inner_file_count = len([e for e in inner if not e["is_dir"]])
            if total_planned_files + inner_file_count > max_total_files:
                dest_rel = f"{dest_base}/{file_name}" if dest_base else f"roms/UNKNOWN/{file_name}"
                entries.append({"source": str(sf["source_path"]), "destination": dest_rel, "action": "manual_review", "reason": f"exceeds max_total_files_per_job {max_total_files} (bomb guard)", "hash": None, "size": sf["size"], "group": None})
                manual += 1
                continue

            mode = _decide_archive_mode(sf["source_path"], inner, profile)
            if mode == "unsupported":
                dest_rel = f"{dest_base}/{file_name}" if dest_base else f"roms/UNKNOWN/{file_name}"
                entries.append({"source": str(sf["source_path"]), "destination": dest_rel, "action": "unsupported_archive", "reason": f"mode unsupported for {ext}", "hash": None, "size": sf["size"], "group": None})
                unsupported += 1
                continue
            if mode == "payload":
                # copy intact
                dest_rel = f"{dest_base}/{file_name}" if dest_base else f"roms/UNKNOWN/{file_name}"
                dest_abs = sd_path / dest_rel
                exists = dest_abs.exists()
                src_hash = None
                dst_hash = None
                same_hash = False
                if exists:
                    dst_size = dest_abs.stat().st_size if dest_abs.exists() else -1
                    if dst_size == sf["size"]:
                        try:
                            src_hash = hmod.sha256_file(sf["source_path"])
                            dst_hash = hmod.sha256_file(dest_abs)
                            same_hash = (src_hash == dst_hash)
                        except:
                            pass
                cls = hmod.classify_duplicate(exists, same_hash, exists) if exists else "new"
                if cls == "unchanged":
                    unchanged+=1; action="skip_unchanged"; reason="same path + same hash -> unchanged (payload)"
                elif cls == "duplicate_content":
                    duplicate+=1; action="skip_duplicate"; reason="different path + same hash -> duplicate content default skip (payload)"
                elif cls == "conflict":
                    conflicts+=1; action="conflict"; reason="same path + different hash -> conflict (payload)"
                else:
                    # Check duplicate via sd_hash_map for payload archive itself
                    try:
                        src_hash = hmod.sha256_file(sf["source_path"])
                        if src_hash in sd_hash_map:
                            duplicate+=1; action="skip_duplicate"; reason="different path + same hash -> duplicate content (payload archive already on SD)"
                        elif src_hash in hash_to_dest:
                            duplicate+=1; action="skip_duplicate"; reason="duplicate archive in same job"
                        else:
                            hash_to_dest[src_hash]=dest_rel
                            new+=1; action="copy"; reason="archive-is-payload -> copy intact"
                    except:
                        new+=1; action="copy"; reason="archive-is-payload -> copy intact"
                entries.append({"source": str(sf["source_path"]), "destination": dest_rel, "action": action, "reason": reason, "hash": src_hash, "size": sf["size"], "group": None})
                total_planned_files += 1
            elif mode in ("container","extract_and_classify","grouped"):
                # Need to handle grouping and collisions
                # First, group members if mode == grouped or contains cue/bin
                if mode == "grouped":
                    groups = _group_members(inner, profile)
                else:
                    # For extract_and_classify, still check for cue/bin grouping opportunistically
                    # If inner contains cue+bin, group them even in extract mode
                    has_cue = any(pathlib.Path(e["name"]).suffix.lower()==".cue" for e in inner if not e["is_dir"])
                    has_bin = any(pathlib.Path(e["name"]).suffix.lower()==".bin" for e in inner if not e["is_dir"])
                    if has_cue and has_bin:
                        groups = _group_members(inner, profile)
                    else:
                        groups = [[e] for e in inner if not e["is_dir"]]
                # For each group, create a logical entry
                # Need to detect collisions among destinations for this archive
                dests_for_collision = []
                group_entries = []
                for grp in groups:
                    # Each group is list of ArchiveEntry dicts
                    # For single-file group, destination is based on that file's classification
                    # For multi-file group (CUE+BIN), destination is folder based on cue's system
                    if len(grp) == 1:
                        e = grp[0]
                        inner_ext = pathlib.Path(e["name"]).suffix.lower()
                        # Classify inner member
                        member_class = _classify_inner_member(e["name"], profile)
                        # Determine dest_base for this member
                        # Use member_class destination
                        eff_base = member_class["destination"]
                        if not eff_base or eff_base == "roms/UNKNOWN":
                            # Try to infer from inner ext mapping
                            sys_ids = profile.get("ext_to_system", {}).get(inner_ext, [])
                            if sys_ids:
                                sys_entry = profile.get("sys_by_id", {}).get(sys_ids[0], {})
                                eff_base = f"roms/{sys_entry.get('folder_aliases',['UNKNOWN'])[0]}"
                            else:
                                eff_base = "roms/UNKNOWN"
                        fname = pathlib.Path(e["name"]).name
                        dest_rel_inner = f"{eff_base}/{fname}"
                        group_entries.append((grp, dest_rel_inner, e))
                        dests_for_collision.append(dest_rel_inner)
                    else:
                        # Multi-file group: treat as one logical unit, destination is folder
                        # Find primary (cue) to infer system
                        cue_entry = next((x for x in grp if pathlib.Path(x["name"]).suffix.lower()==".cue"), grp[0])
                        inner_ext = pathlib.Path(cue_entry["name"]).suffix.lower()
                        member_class = _classify_inner_member(cue_entry["name"], profile)
                        eff_base = member_class["destination"]
                        if not eff_base or eff_base == "roms/UNKNOWN":
                            sys_ids = profile.get("ext_to_system", {}).get(inner_ext, [])
                            if sys_ids:
                                sys_entry = profile.get("sys_by_id", {}).get(sys_ids[0], {})
                                eff_base = f"roms/{sys_entry.get('folder_aliases',['UNKNOWN'])[0]}"
                            else:
                                eff_base = "roms/UNKNOWN"
                        # For grouped, destination is the folder plus group name (cue stem)
                        group_name = pathlib.Path(cue_entry["name"]).stem
                        dest_rel_inner = f"{eff_base}/{group_name}"
                        # For collision, we consider the folder as destination
                        group_entries.append((grp, dest_rel_inner, cue_entry))
                        dests_for_collision.append(dest_rel_inner)
                # Collision detection among this archive's members
                collisions = _detect_collisions(dests_for_collision)
                if collisions:
                    dest_rel = f"{dest_base}/{file_name}" if dest_base else f"roms/UNKNOWN/{file_name}"
                    entries.append({"source": str(sf["source_path"]), "destination": dest_rel, "action": "manual_review", "reason": f"output collision inside archive: {collisions[0][0]} collides with {collisions[0][1]}", "hash": None, "size": sf["size"], "group": None})
                    manual += 1
                    continue
                # Also check for nested archive bomb already handled via mode==manual, but double-check
                # Now plan each group
                for grp, dest_rel_inner, primary in group_entries:
                    # For single file groups, we can attempt to hash inner content via temp extraction to detect duplicate archive vs extracted payload
                    # Extract primary member to temp and hash for duplicate detection
                    dest_abs_inner = sd_path / dest_rel_inner
                    # For grouped, dest is folder; check if folder exists?
                    # For single, file exists check
                    exists = dest_abs_inner.exists()
                    # For duplicate detection of extracted payload, we need to hash inner content
                    # We extract to temp workspace (never to SD) and hash
                    inner_hash = None
                    try:
                        # Only for single-file groups we can hash the extracted bytes without full temp extraction?
                        # For now, use temp extraction for accurate hashing
                        with tempfile.TemporaryDirectory() as tmpdir:
                            tmp_path = pathlib.Path(tmpdir)
                            extracted = amod.safe_extract_to_temp(sf["source_path"], tmp_path)
                            # Find the extracted file corresponding to this group
                            # For single, find by name
                            if len(grp) == 1:
                                # Find extracted file with same basename
                                target_name = pathlib.Path(grp[0]["name"]).name
                                found = None
                                for p in tmp_path.rglob("*"):
                                    if p.name == target_name:
                                        found = p
                                        break
                                if found and found.exists():
                                    inner_hash = hmod.sha256_file(found)
                                    # Also check if this inner hash duplicates any loose file or other archive inner
                                    if inner_hash in sd_hash_map:
                                        # Duplicate extracted payload already on SD
                                        entries.append({"source": f"{sf['source_path']}::{primary['name']}", "destination": dest_rel_inner, "action": "skip_duplicate", "reason": f"duplicate extracted payload (inner {pathlib.Path(primary['name']).suffix} already on SD at {sd_hash_map[inner_hash]})", "hash": inner_hash, "size": primary["size"], "group": [x["name"] for x in grp] if len(grp)>1 else None})
                                        duplicate += 1
                                        continue
                                    if inner_hash in hash_to_dest:
                                        entries.append({"source": f"{sf['source_path']}::{primary['name']}", "destination": dest_rel_inner, "action": "skip_duplicate", "reason": "duplicate extracted payload in same job", "hash": inner_hash, "size": primary["size"], "group": [x["name"] for x in grp] if len(grp)>1 else None})
                                        duplicate += 1
                                        continue
                                    hash_to_dest[inner_hash]=dest_rel_inner
                            else:
                                # Grouped: hash combined
                                # For grouped, hash is hash of sorted member hashes
                                combined = hashlib.sha256()
                                for member in sorted(grp, key=lambda x: x["name"]):
                                    member_path = None
                                    for p in tmp_path.rglob("*"):
                                        if p.name == pathlib.Path(member["name"]).name:
                                            member_path = p
                                            break
                                    if member_path and member_path.exists():
                                        h = hmod.sha256_file(member_path)
                                        combined.update(h.encode())
                                inner_hash = combined.hexdigest() if combined else None
                                if inner_hash and inner_hash in sd_hash_map:
                                    entries.append({"source": f"{sf['source_path']}::group:{group_name}", "destination": dest_rel_inner, "action": "skip_duplicate", "reason": f"duplicate grouped payload already on SD", "hash": inner_hash, "size": sum(x["size"] for x in grp), "group": [x["name"] for x in grp]})
                                    duplicate += 1
                                    continue
                                if inner_hash and inner_hash in hash_to_dest:
                                    entries.append({"source": f"{sf['source_path']}::group:{group_name}", "destination": dest_rel_inner, "action": "skip_duplicate", "reason": "duplicate grouped payload in same job", "hash": inner_hash, "size": sum(x["size"] for x in grp), "group": [x["name"] for x in grp]})
                                    duplicate += 1
                                    continue
                                if inner_hash:
                                    hash_to_dest[inner_hash]=dest_rel_inner
                    except Exception as e:
                        # If temp extraction fails, fall back to entry size based planning
                        pass

                    # Now check existence for this logical unit
                    if exists:
                        # For single file, check same path same hash via inner_hash vs dest file hash
                        if len(grp) == 1:
                            try:
                                # Need dest file hash
                                dst_hash = hmod.sha256_file(dest_abs_inner) if dest_abs_inner.is_file() else None
                                if inner_hash and dst_hash and inner_hash == dst_hash:
                                    entries.append({"source": f"{sf['source_path']}::{primary['name']}", "destination": dest_rel_inner, "action": "skip_unchanged", "reason": "same path + same hash (extracted payload unchanged)", "hash": inner_hash, "size": primary["size"], "group": None})
                                    unchanged += 1
                                else:
                                    entries.append({"source": f"{sf['source_path']}::{primary['name']}", "destination": dest_rel_inner, "action": "conflict", "reason": "same path + different hash (extracted payload conflict)", "hash": inner_hash, "size": primary["size"], "group": None})
                                    conflicts += 1
                                    changed += 1
                            except:
                                entries.append({"source": f"{sf['source_path']}::{primary['name']}", "destination": dest_rel_inner, "action": "conflict", "reason": "extracted payload exists, hash compare failed", "hash": inner_hash, "size": primary["size"], "group": None})
                                conflicts += 1
                        else:
                            # Grouped folder exists
                            entries.append({"source": f"{sf['source_path']}::group:{group_name}", "destination": dest_rel_inner, "action": "conflict", "reason": "grouped payload destination exists (folder)", "hash": inner_hash, "size": sum(x["size"] for x in grp), "group": [x["name"] for x in grp]})
                            conflicts += 1
                    else:
                        # New
                        if len(grp) == 1:
                            entries.append({"source": f"{sf['source_path']}::{primary['name']}", "destination": dest_rel_inner, "action": "extract", "reason": f"archive-extract -> {primary['name']} ({pathlib.Path(primary['name']).suffix})", "hash": inner_hash, "size": primary["size"], "group": None})
                        else:
                            entries.append({"source": f"{sf['source_path']}::group:{group_name} ({', '.join(x['name'] for x in grp)})", "destination": dest_rel_inner, "action": "extract", "reason": f"grouped CUE/BIN logical unit ({len(grp)} files)", "hash": inner_hash, "size": sum(x["size"] for x in grp), "group": [x["name"] for x in grp]})
                        new += 1
                        total_planned_files += len(grp)
            elif mode == "manual":
                dest_rel = f"{dest_base}/{file_name}" if dest_base else f"roms/UNKNOWN/{file_name}"
                entries.append({"source": str(sf["source_path"]), "destination": dest_rel, "action": "manual_review", "reason": "archive requires explicit user decision (profile manual / mixed / nested / unresolved)", "hash": None, "size": sf["size"], "group": None})
                manual += 1
            else:
                dest_rel = f"{dest_base}/{file_name}" if dest_base else f"roms/UNKNOWN/{file_name}"
                entries.append({"source": str(sf["source_path"]), "destination": dest_rel, "action": "manual_review", "reason": f"unhandled archive mode {mode}", "hash": None, "size": sf["size"], "group": None})
                manual += 1
            continue

        # Resolve destination for normal files
        # Music: preserve subfolders (playlist)
        if kind == "music":
            parent = pathlib.Path(sf["relative_hint"]).parent
            if parent != pathlib.Path(".") and str(parent) != "":
                playlist = parent.name if len(parent.parts)==1 else parent.parts[-1]
                dest_rel = f"{dest_base}/{playlist}/{file_name}"
            else:
                dest_rel = f"{dest_base}/{file_name}"
        elif kind in ("lgpt_project",):
            stem = sf["source_path"].stem
            dest_rel = f"lgpt/projects/{stem}"
        elif kind in ("lgpt_sample",):
            dest_rel = f"lgpt/samples/{file_name}"
        elif kind in ("bios",):
            dest_rel = f"{dest_base}/{file_name}"
        else:
            dest_rel = f"{dest_base}/{file_name}"

        dest_abs = sd_path / dest_rel
        exists = dest_abs.exists()
        same_hash = False
        src_hash = None
        dst_hash = None

        if exists:
            try:
                src_hash = hmod.sha256_file(sf["source_path"])
                dst_hash = hmod.sha256_file(dest_abs) if dest_abs.is_file() else None
                # Sizes differ => cannot be same hash, but still provide hashes for UI
                if dest_abs.is_file() and dest_abs.stat().st_size != sf["size"]:
                    same_hash = False
                else:
                    same_hash = (src_hash == dst_hash) if dst_hash else False
            except:
                same_hash = False
            cls = hmod.classify_duplicate(True, same_hash, True)
            if cls == "unchanged":
                unchanged+=1; action="skip_unchanged"; reason="same path + same hash -> unchanged"
            elif cls == "conflict":
                conflicts+=1; changed+=1; action="conflict"; reason="same path + different hash -> conflict"
            else:
                new+=1; action="copy"; reason="new path + new hash -> copy"
        else:
            try:
                src_hash = hmod.sha256_file(sf["source_path"])
                if src_hash in sd_hash_map:
                    duplicate+=1; action="skip_duplicate"; reason="different path + same hash -> duplicate content default skip"
                elif src_hash in hash_to_dest:
                    duplicate+=1; action="skip_duplicate"; reason="different path + same hash -> duplicate content default skip"
                else:
                    hash_to_dest[src_hash] = dest_rel
                    new+=1; action="copy"; reason=f"new path + new hash -> {dest_base}"
            except:
                new+=1; action="copy"; reason=f"new path + new hash -> {dest_base}"

        src_str = str(sf["source_path"])
        if sf.get("group"):
            src_str = f"{src_str} (group {len(sf['group'])} files)"
            group_names = [p.name for p in sf["group"]]
        else:
            group_names = None

        # Determine content_type for UI
        content_type = _content_type_for_classification(c, profile)
        # For grouped, content_type is grouped
        if sf.get("group"):
            content_type = "grouped/CUE_BBIN" if "cue" in sf["source_path"].suffix.lower() or any("cue" in str(g).lower() for g in (sf.get("group") or [])) else content_type

        entry = {
            "source": src_str,
            "destination": dest_rel,
            "action": action,
            "reason": reason,
            "hash": src_hash,
            "source_hash": src_hash,
            "destination_hash": dst_hash,
            "content_type": content_type,
            "size": sf["size"],
            "group": group_names,
            "members": group_names,
            "default_action": action,
            "resolution": _default_resolution_for_action(action),
            "resolved_action": action,
        }
        # For conflict/duplicate/manual, keep both hashes for UI
        entries.append(entry)

    # Enrich any entries that were created earlier (archive paths) with missing metadata for UI consistency
    for e in entries:
        # Content type: grouped takes precedence
        if "content_type" not in e or not e["content_type"]:
            if e.get("members") and len(e["members"]) > 1:
                e["content_type"] = "grouped/CUE_BBIN"
            elif e.get("group") and len(e["group"]) > 1:
                e["content_type"] = "grouped/CUE_BBIN"
            else:
                dest = e.get("destination", "")
                if "cps" in dest or "neogeo" in dest or "m2k" in dest:
                    e["content_type"] = "rom/arcade"
                elif dest.startswith("roms/"):
                    parts = dest.split("/")
                    if len(parts) >= 2:
                        e["content_type"] = f"rom/{parts[1]}"
                    else:
                        e["content_type"] = "rom/unknown"
                elif dest.startswith("lgpt/"):
                    e["content_type"] = "lgpt/project" if "projects" in dest else "lgpt/sample"
                elif dest.startswith("roms/music"):
                    e["content_type"] = "music"
                else:
                    e["content_type"] = "unknown"
        if "source_hash" not in e or e["source_hash"] is None:
            e["source_hash"] = e.get("hash")
        if "destination_hash" not in e or e["destination_hash"] is None:
            # For conflict where destination exists, try to provide destination_hash for UI
            if e.get("action") in ("conflict", "manual_review") and e.get("destination"):
                # Try to hash destination if it exists on SD and we have not already
                # We have sd_path available, but in enrichment we don't have it directly; keep as None if not already set
                # The planner already set it for non-archive conflicts, for archive extracts it was set via inner_hash logic
                e["destination_hash"] = e.get("destination_hash")
            else:
                e["destination_hash"] = e.get("destination_hash")
        if "default_action" not in e:
            e["default_action"] = e.get("action")
        if "resolution" not in e:
            e["resolution"] = _default_resolution_for_action(e.get("action"))
        if "resolved_action" not in e:
            e["resolved_action"] = e.get("action")
        if "members" not in e:
            e["members"] = e.get("group")
        # Ensure deterministic keys: sort members if present
        if e.get("members"):
            e["members"] = sorted(e["members"])
            e["group"] = sorted(e["group"]) if e.get("group") else e["members"]

    # Final deterministic sort by source then destination (stable)
    entries.sort(key=lambda x: (x.get("source", ""), x.get("destination", "")))

    summary = {"unchanged": unchanged, "new": new, "changed": changed, "duplicate_content": duplicate, "conflicts": conflicts, "deletions": 0, "manual_review": manual, "unsupported_archive": unsupported}
    warnings = ["PROVISIONAL_UNVALIDATED video preset — not hardware validated", "arch archives bounded: depth=1 entries=1024 expansion=1GiB"]
    return {"summary": summary, "entries": entries, "warnings": warnings}
