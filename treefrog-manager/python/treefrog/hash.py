import hashlib, pathlib

def sha256_file(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(64*1024), b""):
            h.update(chunk)
    return h.hexdigest()

# Duplicate handling algorithm per profile
# same path + same hash -> unchanged
# different path + same hash -> duplicate_content default skip
# same path + different hash -> conflict
# new path + new hash -> copy

def classify_duplicate(same_path: bool, same_hash: bool, exists: bool) -> str:
    if not exists:
        return "new"
    if same_path and same_hash:
        return "unchanged"
    if not same_path and same_hash:
        return "duplicate_content"
    if same_path and not same_hash:
        return "conflict"
    # different path + different hash but dest exists with different content -> conflict
    return "conflict"
