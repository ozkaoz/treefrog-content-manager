import json, pathlib

REPO_ROOT = pathlib.Path(__file__).resolve().parents[3]
PROFILE_DIR = REPO_ROOT / "profiles" / "treefrogui"

def load_profile():
    profile = json.loads((PROFILE_DIR / "profile.json").read_text(encoding="utf-8"))
    systems = json.loads((PROFILE_DIR / "systems.json").read_text(encoding="utf-8"))
    # Build maps
    ext_to_system = {}
    alias_to_system = {}
    for sys in systems["systems"]:
        for ext in sys.get("extensions", []):
            k = ext.lower()
            ext_to_system.setdefault(k, []).append(sys["id"])
        for alias in sys.get("folder_aliases", []):
            alias_to_system[alias.lower()] = sys["id"]
    # systems by id
    sys_by_id = {s["id"]: s for s in systems["systems"]}
    return {
        "profile_version": profile.get("profile_version"),
        "systems": systems["systems"],
        "sys_by_id": sys_by_id,
        "ext_to_system": ext_to_system,
        "alias_to_system": alias_to_system,
        "archive_policy": profile.get("archive_policy", {}),
        "profile": profile,
        "systems_raw": systems,
    }

def load_media():
    return json.loads((PROFILE_DIR / "media.json").read_text(encoding="utf-8"))
def load_bios():
    return json.loads((PROFILE_DIR / "bios.json").read_text(encoding="utf-8"))
def load_lgpt():
    return json.loads((PROFILE_DIR / "lgpt.json").read_text(encoding="utf-8"))
def load_video_presets():
    return json.loads((PROFILE_DIR / "video_presets.json").read_text(encoding="utf-8"))
def load_sd_markers():
    return json.loads((PROFILE_DIR / "sd_markers.json").read_text(encoding="utf-8"))
