import pathlib, os
from . import classify as cl

def scan(source_root: str, profile):
    root = pathlib.Path(source_root)
    if not root.exists():
        raise FileNotFoundError(f"source not found: {source_root}")
    out = []
    # For LGPT projects, we need to handle directories as logical units
    # First, collect all files, but also check for LGPT project directories
    lgpt_projects_detected = set()
    for p in root.rglob("*"):
        if p.is_dir():
            # Check if this directory is an LGPT project (contains lgptsav.dat or .lgpt or has multiple files and is under projects)
            try:
                # Check for project markers
                has_marker = any((p / n).exists() for n in ["lgptsav.dat", "project.lgpt", "save.dat"])
                # Also check if the directory itself is classified as lgpt_project
                # We can classify the directory path
                if has_marker or ("projects" in str(p).lower() and len(list(p.iterdir())) > 0):
                    # Check if any file inside would be classified as lgpt_project if we classify the dir
                    # For now, treat this directory as a project logical unit
                    # We will handle it after the file loop
                    pass
            except:
                pass
            continue
        if p.is_symlink():
            continue
        # Skip junk/placeholder files that are never content and cause collisions
        fname_lower = p.name.lower()
        if fname_lower.startswith('.') or fname_lower in ("thumbs.db", "desktop.ini", ".keep", ".gitkeep", ".ds_store") or fname_lower.endswith(".tmp") or fname_lower.endswith(".temp"):
            continue
        # classify
        c = cl.classify(p, profile)
        # size
        try:
            size = p.stat().st_size
        except:
            continue
        rel = p.relative_to(root).as_posix() if p.is_relative_to(root) else str(p)
        # multi-file group: CUE/BIN detection
        group = None
        if p.suffix.lower() == ".cue":
            # parse cue for BIN references
            try:
                content = p.read_text(encoding="utf-8", errors="ignore")
                bins = []
                for line in content.splitlines():
                    if "FILE" in line.upper() and ".bin" in line.lower():
                        # extract quoted bin name
                        if '"' in line:
                            start = line.find('"')+1
                            end = line.find('"', start)
                            bin_name = line[start:end]
                            sibling = p.parent / bin_name
                            if sibling.exists():
                                bins.append(sibling)
                if bins:
                    group = [p] + bins
                else:
                    # fallback same stem .bin
                    sibling = p.parent / f"{p.stem}.bin"
                    if sibling.exists():
                        group = [p, sibling]
            except:
                pass
        out.append({"source_path": p, "relative_hint": rel, "size": size, "classification": c, "group": group})
    # Handle LGPT projects as logical units: find project directories
    for p in root.rglob("*"):
        if not p.is_dir():
            continue
        if p.is_symlink():
            continue
        # Check if this directory should be treated as an LGPT project logical unit
        # Criteria: directory contains lgptsav.dat or is directly under lgpt/projects or has multiple files and is named like a project
        try:
            # Only consider directories that are not the root and have at least one file
            if p == root:
                continue
            # Check for project marker or if parent is projects
            has_marker = (p / "lgptsav.dat").exists() or (p / "project.lgpt").exists() or any(f.suffix.lower() == ".lgpt" for f in p.iterdir() if f.is_file())
            is_project_dir = "projects" in str(p).lower() or has_marker
            # Also check if all files inside would be classified as lgpt_project or if directory itself would be
            # For now, if it has a marker or is under projects, treat as project
            if is_project_dir or has_marker:
                # Check if we already have an entry for a file inside this project that would be part of it
                # Instead, create a single entry for the project directory itself
                # Use the directory as source_path, with classification as lgpt_project
                c = cl.classify(p, profile)
                if c["kind"] == "lgpt_project":
                    # Avoid duplicate if we already have this project
                    if p not in lgpt_projects_detected:
                        lgpt_projects_detected.add(p)
                        # Calculate size as sum of all files in the project
                        try:
                            total_size = sum(f.stat().st_size for f in p.rglob("*") if f.is_file())
                        except:
                            total_size = 0
                        rel = p.relative_to(root).as_posix() if p.is_relative_to(root) else str(p)
                        # Collect members
                        try:
                            members = [f.relative_to(p).as_posix() for f in p.rglob("*") if f.is_file()]
                        except:
                            members = []
                        out.append({"source_path": p, "relative_hint": rel, "size": total_size, "classification": c, "group": None, "members": members, "is_project_dir": True})
        except:
            pass
    # For LGPT projects that were detected as directories, we should remove the individual file entries that are inside those projects to avoid double counting
    # Find all file entries that are inside any detected project directory and remove them if the project is a logical unit
    if lgpt_projects_detected:
        filtered = []
        for item in out:
            src = item["source_path"]
            # If this file is inside a detected project directory, skip it (since the project dir will be the logical unit)
            inside_project = any(src.is_relative_to(proj) and src != proj for proj in lgpt_projects_detected if hasattr(src, 'is_relative_to'))
            # For older Python, use relative_to with try
            if not inside_project:
                # Also check via string prefix for older
                for proj in lgpt_projects_detected:
                    try:
                        src.relative_to(proj)
                        if src != proj:
                            inside_project = True
                            break
                    except:
                        pass
            if not inside_project:
                filtered.append(item)
            else:
                # This file is part of a project logical unit, so skip the individual file entry
                pass
        # Also need to handle the case where the project dir entry itself is in filtered, keep it
        # The project dir entries are already in out, so filtered will have them
        out = filtered
    # dedup group leaders
    seen = set()
    deduped = []
    for item in out:
        if item["group"]:
            key = "|".join(str(m) for m in item["group"])
            if key in seen:
                continue
            seen.add(key)
        deduped.append(item)
    return deduped
