import pathlib, shutil, tempfile, os

def deploy_plan(plan, sd_root, profile=None):
    sd_path = pathlib.Path(sd_root)
    if not sd_path.exists():
        raise FileNotFoundError(f"SD root not found: {sd_root}")
    deployed = 0
    skipped = 0
    failed = 0
    errors = []
    warnings = []
    for e in plan.get("entries", []):
        action = e.get("resolved_action") or e.get("action")
        dest_rel = e.get("destination")
        try:
            from .sd_target import validate_destination_path
            validate_destination_path(dest_rel)
        except Exception as ex:
            errors.append(f"{dest_rel}: {ex}")
            failed += 1
            continue
        dest_abs = sd_path / dest_rel
        if action in ("copy", "replace"):
            src = pathlib.Path(e["source"].split("::")[0])
            if not src.exists():
                errors.append(f"source not found: {src}")
                failed += 1
                continue
            try:
                dest_abs.parent.mkdir(parents=True, exist_ok=True)
                tmp = dest_abs.parent / f".treefrog_staging_{os.getpid()}_{dest_abs.name}.tmp"
                shutil.copy2(str(src), str(tmp))
                tmp.rename(dest_abs)
                deployed += 1
            except Exception as ex:
                errors.append(f"copy {src} -> {dest_abs}: {ex}")
                failed += 1
        elif action == "extract":
            src = pathlib.Path(e["source"].split("::")[0])
            if not src.exists():
                errors.append(f"archive not found: {src}")
                failed += 1
                continue
            try:
                dest_abs.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(str(src), str(dest_abs))
                deployed += 1
            except Exception as ex:
                errors.append(f"extract {src}: {ex}")
                failed += 1
        elif action == "convert_then_copy":
            src = pathlib.Path(e["source"])
            if not src.exists():
                errors.append(f"video source not found: {src}")
                failed += 1
                continue
            try:
                dest_abs.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(str(src), str(dest_abs))
                deployed += 1
            except Exception as ex:
                errors.append(f"video copy {src} -> {dest_abs}: {ex}")
                failed += 1
        elif action in ("skip", "skip_unchanged", "skip_duplicate"):
            skipped += 1
        elif action in ("conflict", "manual_review", "unsupported_archive", "unsupported", "conversion_error"):
            warnings.append(f"{e['source']} requires manual decision: {e['destination']} ({e['action']})")
            skipped += 1
        else:
            warnings.append(f"unknown action {action} for {e['source']} -> {e['destination']}")
            skipped += 1
    success = failed == 0
    return {
        "success": success,
        "deployed": deployed,
        "skipped": skipped,
        "failed": failed,
        "errors": errors,
        "warnings": warnings,
    }
