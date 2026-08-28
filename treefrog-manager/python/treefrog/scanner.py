import pathlib, os
from . import classify as cl

def scan(source_root: str, profile):
    root = pathlib.Path(source_root)
    if not root.exists():
        raise FileNotFoundError(f"source not found: {source_root}")
    out = []
    for p in root.rglob("*"):
        if p.is_dir():
            continue
        if p.is_symlink():
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
