import pathlib, hashlib
from . import hash as hmod
from . import archive as amod

def _dest_exists(p: pathlib.Path) -> bool:
    return p.exists()

def plan(scanned, sd_root: str, profile):
    sd_path = pathlib.Path(sd_root)
    entries = []
    unchanged = new = changed = duplicate = conflicts = 0
    hash_to_dest = {}
    # Pre-index SD file hashes for duplicate detection (same content not same filename)
    # Build map size -> list of paths, then hash only sizes that appear in scanned set
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

    for sf in scanned:
        c = sf["classification"]
        kind = c["kind"]
        dest_base = c["destination"]
        file_name = sf["source_path"].name

        # Archive handling
        if kind == "archive":
            # inspect
            try:
                inner = amod.inspect_zip(sf["source_path"])
                is_payload = amod.is_archive_runtime_payload(sf["source_path"], inner, profile)
                if is_payload:
                    # copy intact — dest is generic or based on hint? For MVP use roms/UNKNOWN if base empty
                    dest_rel = f"{dest_base}/{file_name}" if dest_base else f"roms/UNKNOWN/{file_name}"
                    dest_abs = sd_path / dest_rel
                    exists = dest_abs.exists()
                    src_hash = None
                    dst_hash = None
                    same_hash = False
                    if exists:
                        # cheap size then hash
                        dst_size = dest_abs.stat().st_size if dest_abs.exists() else -1
                        if dst_size == sf["size"]:
                            try:
                                src_hash = hmod.sha256_file(sf["source_path"])
                                dst_hash = hmod.sha256_file(dest_abs)
                                same_hash = (src_hash == dst_hash)
                            except: pass
                    cls = hmod.classify_duplicate(exists, same_hash, exists) if exists else "new"
                    if cls == "unchanged":
                        unchanged+=1; action="skip_unchanged"; reason="same path + same hash -> unchanged"
                    elif cls == "duplicate_content":
                        duplicate+=1; action="skip_duplicate"; reason="different path + same hash -> duplicate skip"
                    elif cls == "conflict":
                        conflicts+=1; action="conflict"; reason="same path + different hash -> conflict"
                    else:
                        new+=1; action="copy"; reason="archive payload valid → copy intact (new)"
                    entries.append({"source": str(sf["source_path"]), "destination": dest_rel, "action": action, "reason": reason, "hash": src_hash, "size": sf["size"], "group": None})
                else:
                    # extract inner
                    for inner_e in [e for e in inner if not e["is_dir"]]:
                        inner_ext = pathlib.Path(inner_e["name"]).suffix.lower()
                        if inner_ext in profile["ext_to_system"] or inner_ext in (".sfc",".nes",".gba",".gb",".gbc",".md",".bin",".cue"):
                            fname = pathlib.Path(inner_e["name"]).name
                            # dest_base for extract: use scanned dest_base or generic roms/SFC etc? For generic archive we need to infer from inner ext
                            # If original dest_base empty, infer from inner ext mapping
                            eff_dest_base = dest_base
                            if not eff_dest_base or eff_dest_base == "":
                                # map inner ext to folder
                                sys_ids = profile["ext_to_system"].get(inner_ext, [])
                                if sys_ids:
                                    sys_entry = profile["sys_by_id"].get(sys_ids[0], {})
                                    eff_dest_base = f"roms/{sys_entry.get('folder_aliases',['UNKNOWN'])[0]}"
                                else:
                                    eff_dest_base = "roms/UNKNOWN"
                            # If dest_base was UNKNOWN but inner suggests better, use inner's folder? Keep eff_dest_base
                            dest_rel_inner = f"{eff_dest_base}/{fname}"
                            dest_abs_inner = sd_path / dest_rel_inner
                            exists = dest_abs_inner.exists()
                            act = "conflict" if exists else "extract"
                            if act == "extract":
                                new+=1
                            else:
                                conflicts+=1
                            entries.append({"source": f"{sf['source_path']}::{inner_e['name']}", "destination": dest_rel_inner, "action": act, "reason": f"archive extract safe (inner {inner_ext})", "hash": None, "size": inner_e["size"], "group": None})
            except Exception as e:
                dest_rel = f"{dest_base}/{file_name}" if dest_base else f"roms/UNKNOWN/{file_name}"
                entries.append({"source": str(sf["source_path"]), "destination": dest_rel, "action": "conflict", "reason": f"archive inspection failed: {e}", "hash": None, "size": sf["size"], "group": None})
                conflicts+=1
            continue

        # Resolve destination for normal files
        # Music: preserve subfolders (playlist)
        if kind == "music":
            parent = pathlib.Path(sf["relative_hint"]).parent
            # preserve immediate parent folder as playlist name if any
            if parent != pathlib.Path(".") and str(parent) != "":
                # Use parent name (last folder) as playlist
                playlist = parent.name if len(parent.parts)==1 else parent.parts[-1]
                dest_rel = f"{dest_base}/{playlist}/{file_name}"
            else:
                dest_rel = f"{dest_base}/{file_name}"
        elif kind in ("lgpt_project",):
            stem = sf["source_path"].stem
            dest_rel = f"lgpt/projects/{stem}"
            # group handling: if directory project, ensure we treat as group
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
            dst_size = dest_abs.stat().st_size if dest_abs.is_file() else -1
            if dst_size != sf["size"]:
                # cheap mismatch -> conflict without hashing
                same_hash = False
            else:
                try:
                    src_hash = hmod.sha256_file(sf["source_path"])
                    dst_hash = hmod.sha256_file(dest_abs)
                    same_hash = (src_hash == dst_hash)
                except:
                    same_hash = False
            cls = hmod.classify_duplicate(True, same_hash, True)
            if cls == "unchanged":
                unchanged+=1; action="skip_unchanged"; reason="same path + same hash -> unchanged"
            elif cls == "conflict":
                conflicts+=1; changed+=1; action="conflict"; reason="same path + different hash -> conflict"
            else:
                # shouldn't reach duplicate_content for same path
                new+=1; action="copy"; reason="new path + new hash -> copy"
        else:
            # check duplicate elsewhere via hash_to_dest (different path same hash) + SD pre-index
            try:
                src_hash = hmod.sha256_file(sf["source_path"])
                if src_hash in sd_hash_map:
                    # content already exists on SD at different path
                    # but if dest_rel == sd_hash_map[src_hash] it's actually same path? we already handled exists case above
                    # So this is duplicate content elsewhere
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

        entries.append({"source": src_str, "destination": dest_rel, "action": action, "reason": reason, "hash": src_hash, "size": sf["size"], "group": group_names})

    summary = {"unchanged": unchanged, "new": new, "changed": changed, "duplicate_content": duplicate, "conflicts": conflicts, "deletions": 0}
    warnings = ["PROVISIONAL_UNVALIDATED video preset — not hardware validated", "arch archives bounded: depth=1 entries=1024 expansion=1GiB"]
    return {"summary": summary, "entries": entries, "warnings": warnings}
