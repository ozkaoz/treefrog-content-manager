import pathlib

MUSIC_EXTS = {".mp3",".m4a",".aac",".wav",".flac",".ogg",".opus"}
VIDEO_EXTS = {".mp4",".mkv",".avi",".mov",".m4v",".wmv",".mpg",".mpeg",".ts",".webm"}
IMAGE_EXTS = {".jpg",".jpeg",".png",".bmp",".gif",".webp",".tiff",".tif",".tga",".ico"}
EBOOK_EXTS = {".epub",".mobi",".pdf",".cbz",".fb2",".xps"}
ARCHIVE_EXTS = {".zip",".7z",".rar"}
# NOTE (2026-09-01): the old BIOS_HINTS hardcoded list was REMOVED — BIOS
# classification is the single declarative model (bios.json via the dedicated
# BIOS tab). A general ROM scan only classifies as BIOS files that explicitly
# live in a cubegm/bios source folder. This avoids false positives (e.g. a ROM
# named scph-xxx.bin) and duplicating x86BOOT.img which TreeFrogUI ships.
# Artwork folders managed by Mini Scraper: files inside are NEVER deployed.
ARTWORK_DIRS = (".res", "imgs", "images")


def is_artwork_path(path: pathlib.Path) -> bool:
    return any(p.name.lower() in ARTWORK_DIRS for p in path.parents) or path.parent.name.lower() in ARTWORK_DIRS

def classify(path: pathlib.Path, profile):
    ext = path.suffix.lower()
    name_lower = path.name.lower()
    # LGPT - profile-driven, check before generic music (WAV baseline)
    # Use profile's lgpt destinations if available
    lgpt_samples_dest = "lgpt/samples"
    lgpt_projects_dest = "lgpt/projects"
    try:
        from .profile import load_lgpt
        lgpt_cfg = load_lgpt()
        lgpt_samples_dest = lgpt_cfg.get("destinations", {}).get("samples", lgpt_samples_dest)
        lgpt_projects_dest = lgpt_cfg.get("destinations", {}).get("projects", lgpt_projects_dest)
    except:
        pass
    # Check for LGPT sample first (WAV baseline, but accept others if under lgpt)
    if "lgpt" in str(path).lower() and ext in (".wav", ".flac", ".aiff", ".aif", ".mp3", ".ogg"):
        return {"kind":"lgpt_sample","system_id":None,"destination":lgpt_samples_dest,"multi_file":False,"archive_valid":False}
    # Check for LGPT project directory
    if path.is_dir():
        try:
            has_marker = any((path / n).exists() for n in ["lgptsav.dat", "project.lgpt", "save.dat"])
            is_in_projects = "projects" in str(path).lower()
            if has_marker or is_in_projects:
                # Check if directory looks like a project
                if has_marker or len(list(path.iterdir())) > 0:
                    return {"kind":"lgpt_project","system_id":None,"destination":lgpt_projects_dest,"multi_file":True,"archive_valid":False}
        except:
            pass
    if ext == ".lgpt":
        return {"kind":"lgpt_project","system_id":None,"destination":lgpt_projects_dest,"multi_file":True,"archive_valid":False}
    # archives
    if ext in ARCHIVE_EXTS:
        return {"kind":"archive","system_id":None,"destination":"","multi_file":False,"archive_valid":False}
    if ext in MUSIC_EXTS:
        return {"kind":"music","system_id":None,"destination":"roms/music","multi_file":False,"archive_valid":False}
    if ext in VIDEO_EXTS:
        return {"kind":"video","system_id":None,"destination":"roms/videos","multi_file":False,"archive_valid":False}
    if ext in IMAGE_EXTS:
        # Artwork (Mini Scraper): NEVER deploy — the old code wrote it to
        # `.res/.res/...` at the SD ROOT (outside content roots, real bug).
        if is_artwork_path(path):
            return {"kind":"unknown","system_id":None,"destination":"","multi_file":False,"archive_valid":False}
        return {"kind":"image","system_id":None,"destination":"roms/images","multi_file":False,"archive_valid":False}
    if ext in EBOOK_EXTS:
        return {"kind":"ebook","system_id":None,"destination":"roms/Ebook","multi_file":False,"archive_valid":False}
    # LGPT - profile-driven destinations, not hardcoded in UI (but classify may use profile if available)
    # Prefer WAV baseline for samples
    lgpt_samples_dest = profile.get("lgpt_destinations", {}).get("samples", "lgpt/samples") if isinstance(profile, dict) else "lgpt/samples"
    lgpt_projects_dest = profile.get("lgpt_destinations", {}).get("projects", "lgpt/projects") if isinstance(profile, dict) else "lgpt/projects"
    # Also try to get from profile's lgpt json
    try:
        from .profile import load_lgpt
        lgpt_cfg = load_lgpt()
        lgpt_samples_dest = lgpt_cfg.get("destinations", {}).get("samples", lgpt_samples_dest)
        lgpt_projects_dest = lgpt_cfg.get("destinations", {}).get("projects", lgpt_projects_dest)
    except:
        pass
    if "lgpt" in str(path).lower() and ext in (".wav", ".flac", ".aiff", ".aif", ".mp3", ".ogg"):
        # Prefer WAV baseline, but accept others if profile allows
        return {"kind":"lgpt_sample","system_id":None,"destination":lgpt_samples_dest,"multi_file":False,"archive_valid":False}
    # Projects: treat as logical unit where directory contains multiple files (e.g., lgptsav.dat)
    # Check if path is a directory that looks like a project (contains lgptsav.dat or .lgpt files)
    if path.is_dir():
        # Check if directory contains project files
        try:
            # Look for lgptsav.dat or .lgpt or any file inside
            has_project_marker = any((path / n).exists() for n in ["lgptsav.dat", "project.lgpt", "save.dat"])
            # Or if parent is projects
            is_in_projects = "projects" in str(path).lower()
            if has_project_marker or is_in_projects:
                return {"kind":"lgpt_project","system_id":None,"destination":lgpt_projects_dest,"multi_file":True,"archive_valid":False}
            # Also check if directory has multiple files (likely a project)
            if len(list(path.iterdir())) > 1:
                # Heuristic: if directory is under a projects-like path, treat as project
                if "project" in str(path).lower():
                    return {"kind":"lgpt_project","system_id":None,"destination":lgpt_projects_dest,"multi_file":True,"archive_valid":False}
        except:
            pass
    if ext == ".lgpt":
        return {"kind":"lgpt_project","system_id":None,"destination":lgpt_projects_dest,"multi_file":True,"archive_valid":False}
    # BIOS: single declarative model (bios.json). Two paths to Bios:
    # 1. Files explicitly inside a cubegm/bios folder (user layout —
    #    matches both "cubegm/bios/..." and ".../cubegm/bios/...").
    # 2. Loose files whose EXACT filename matches bios.json accepted_filenames.
    posix = str(path).lower().replace("\\", "/")
    if "/cubegm/bios/" in posix or posix.startswith("cubegm/bios/"):
        return {"kind":"bios","system_id":None,"destination":"cubegm/bios","multi_file":False,"archive_valid":False}
    try:
        from .profile import load_bios
        bios_defs = load_bios().get("bios_definitions", [])
        accepted = set()
        for d in bios_defs:
            for f in d.get("accepted_filenames", []):
                accepted.add(f.lower())
            for var in d.get("variants", []):
                for f in var.get("filenames", []):
                    accepted.add(f.lower())
        if name_lower in accepted:
            return {"kind":"bios","system_id":None,"destination":"cubegm/bios","multi_file":False,"archive_valid":False}
    except Exception:
        pass
    # ROM by profile
    ext_to_system = profile["ext_to_system"]
    if ext in ext_to_system:
        sys_id = ext_to_system[ext][0]
        sys_entry = profile["sys_by_id"].get(sys_id, {})
        folder = sys_entry.get("folder_aliases", ["UNKNOWN"])[0]
        dest = f"roms/{folder}"
        multi = bool(sys_entry.get("multi_file", False))
        archive_valid = ext in [e.lower() for e in sys_entry.get("archive_payload_valid",[])]
        return {"kind":"rom","system_id":sys_id,"destination":dest,"multi_file":multi,"archive_valid":archive_valid}
    # unknown
    return {"kind":"unknown","system_id":None,"destination":"roms/UNKNOWN","multi_file":False,"archive_valid":False}
