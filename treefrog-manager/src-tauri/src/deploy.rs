use crate::Plan;
use crate::profile::LoadedProfile;
use crate::sd_target;
use std::path::Path;
use log::{info, warn};
use tauri::{Emitter, AppHandle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeployProgress {
    pub total: usize,
    pub completed: usize,
    pub current: Option<String>,
    pub errors: Vec<String>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeployResult {
    pub success: bool,
    pub deployed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakdown: Option<Vec<serde_json::Value>>,
}

fn ensure_parent_exists(dest: &Path) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn safe_copy_file(src: &Path, dest: &Path) -> anyhow::Result<()> {
    // Validate the full destination relative to SD root (strip drive letter if present)
    let dest_str = dest.to_string_lossy().to_string().replace('\\', "/");
    // For validation, extract the part after the SD root (e.g., G:/roms/GBA/game.gba -> roms/GBA/game.gba)
    // Find the first occurrence of "roms/" or "cubegm/" or "lgpt/" or "frogui/" to get the relative part
    let relative = if dest_str.contains("roms/") {
        dest_str.split("roms/").last().map(|s| format!("roms/{}", s)).unwrap_or(dest_str.clone())
    } else if dest_str.contains("cubegm/") {
        dest_str.split("cubegm/").last().map(|s| format!("cubegm/{}", s)).unwrap_or(dest_str.clone())
    } else if dest_str.contains("lgpt/") {
        dest_str.split("lgpt/").last().map(|s| format!("lgpt/{}", s)).unwrap_or(dest_str.clone())
    } else if dest_str.contains("frogui/") {
        dest_str.split("frogui/").last().map(|s| format!("frogui/{}", s)).unwrap_or(dest_str.clone())
    } else {
        // Fallback: use file name with roms/ prefix for validation
        if let Some(file_name) = dest.file_name().and_then(|n| n.to_str()) {
            format!("roms/{}", file_name)
        } else {
            dest_str.clone()
        }
    };
    sd_target::validate_destination_path(&relative).map_err(|e| anyhow::anyhow!("invalid destination {}: {}", dest.display(), e))?;
    ensure_parent_exists(dest)?;
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));
    // Use a unique temp file per operation to avoid collisions
    let tmp_name = format!(".treefrog_staging_{}_{}_{}.tmp", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(), dest.file_name().unwrap_or_default().to_string_lossy());
    let tmp_path = parent.join(tmp_name);
    std::fs::copy(src, &tmp_path)?;
    // Integrity check: temp file size must match source exactly
    if std::fs::metadata(&tmp_path)?.len() != std::fs::metadata(src)?.len() {
        let _ = std::fs::remove_file(&tmp_path);
        anyhow::bail!("copy verification failed: size mismatch for {} -> {}", src.display(), dest.display());
    }
    // On Windows FAT32/exFAT, rename is atomic if same directory, but not across volumes
    // Since tmp is in same parent as dest, rename should be atomic
    match std::fs::rename(&tmp_path, dest) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Fallback: copy and remove tmp if rename fails (e.g., cross-device)
            let _ = std::fs::remove_file(&tmp_path);
            Err(anyhow::anyhow!("atomic rename failed: {}", e))
        }
    }
}

pub fn deploy_plan(plan: &Plan, sd_root: &str, _profile: &LoadedProfile, force: bool, app: Option<&AppHandle>) -> anyhow::Result<DeployResult> {
    let sd_path = Path::new(sd_root);
    if !sd_path.exists() {
        anyhow::bail!("SD root not found: {}", sd_root);
    }
    let analysis = sd_target::analyze_target(sd_root)?;
    if analysis.status == "inaccessible" {
        anyhow::bail!("SD target inaccessible: {}", sd_root);
    }
    if !analysis.is_treefrog {
        if analysis.status == "unknown" {
            anyhow::bail!("SD target not recognized as TreeFrogUI (missing cubegm/ + roms/): {}", sd_root);
        }
    }
    let space = sd_target::calculate_space(plan, analysis.free_bytes);
    if space.status == "insufficient_space" {
        anyhow::bail!("Insufficient space: required {} available {}", space.required_bytes, space.available_bytes.unwrap_or(0));
    }
    let dests: Vec<String> = plan.entries.iter().map(|e| e.destination.clone()).collect();
    let collisions = sd_target::check_case_collision(&dests);
    if !collisions.is_empty() {
        anyhow::bail!("Case collision detected: {} collides with {}", collisions[0].0, collisions[0].1);
    }

    let mut deployed = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut breakdown_rows: Vec<serde_json::Value> = Vec::new();
    let mut written_dests: std::collections::HashMap<String, Option<String>> = std::collections::HashMap::new();

    let total = plan.entries.iter().filter(|e| matches!(e.resolved_action.as_ref().unwrap_or(&e.action).as_str(), "copy" | "extract" | "convert_then_copy")).count();
    let mut completed = 0usize;
    
    for entry in &plan.entries {
        let action = entry.resolved_action.as_ref().unwrap_or(&entry.action);
        info!(
            "Processing: {} -> {} (action: {}, reason: {})",
            entry.source,
            entry.destination,
            action,
            entry.reason
        );
        let dest_rel = &entry.destination;
        // Validate (for relative dest)
        let dest_for_validation = if dest_rel.contains(':') || dest_rel.starts_with('/') || dest_rel.starts_with('\\') {
            // If it's an absolute Windows path, extract the file name for validation
            Path::new(dest_rel).file_name().and_then(|n| n.to_str()).unwrap_or(dest_rel).to_string()
        } else {
            dest_rel.clone()
        };
        if let Err(e) = sd_target::validate_destination_path(&dest_for_validation) {
            // Try validating as roms/... + file name
            if let Err(e2) = sd_target::validate_destination_path(&format!("roms/{}", dest_for_validation)) {
                errors.push(format!("{}: {} (also {})", dest_rel, e, e2));
                failed += 1;
                continue;
            }
        }
        let dest_abs = sd_path.join(dest_rel);

        // ---- Fresh on-disk verification: never trust a stale skip decision ----
        let dest_exists_now = dest_abs.exists();
        let downgraded: String = match action.as_str() {
            // "unchanged" REQUIRES the file to actually exist on the SD right now.
            "skip_unchanged" if !dest_exists_now => {
                info!("Downgrade skip_unchanged -> copy (destination missing on SD): {}", dest_abs.display());
                "copy".to_string()
            }
            other => other.to_string(),
        };

        // Force mode: user explicitly wants to (re)copy everything.
        let action_final: String = if force {
            match downgraded.as_str() {
                "skip_unchanged" | "skip_duplicate" | "skip" => {
                    info!("Force copy: {} -> {}", entry.source, dest_abs.display());
                    "copy".to_string()
                }
                _ => downgraded,
            }
        } else {
            downgraded
        };

        // Runtime double-write guard: never write the same destination twice in one job.
        let is_write = matches!(action_final.as_str(), "copy" | "extract" | "convert_then_copy" | "replace");
        if is_write {
            let norm_dest = dest_abs.to_string_lossy().to_lowercase();
            if let Some(prev_hash) = written_dests.get(&norm_dest).cloned() {
                let cur_hash = entry.hash.clone().or_else(|| entry.source_hash.clone());
                skipped += 1;
                let (act, msg) = if prev_hash.is_some() && prev_hash == cur_hash {
                    ("skip_duplicate", format!("duplicate within job skipped: {} (same content already deployed to {})", entry.source, dest_rel))
                } else {
                    ("conflict", format!("conflict within job skipped: {} (destination {} already written by another entry with different content -> manual review)", entry.source, dest_rel))
                };
                warnings.push(msg.clone());
                warn!("{}", msg);
                breakdown_rows.push(serde_json::json!({
                    "source": entry.source,
                    "destination": dest_rel,
                    "dest_abs": dest_abs.to_string_lossy(),
                    "dest_exists": dest_exists_now,
                    "action": act,
                    "reason": msg,
                    "content_type": entry.content_type,
                }));
                continue;
            }
        }

        // Record breakdown row with absolute verified path (loop-collected)
        breakdown_rows.push(serde_json::json!({
            "source": entry.source,
            "destination": dest_rel,
            "dest_abs": dest_abs.to_string_lossy(),
            "dest_exists": dest_exists_now,
            "action": action_final,
            "reason": entry.reason,
            "content_type": entry.content_type,
        }));

        match action_final.as_str() {
            "copy" | "replace" => {
                let raw_src = entry.source.clone();
                let is_group = raw_src.contains("::group:") || raw_src.contains(" (group ");
                if is_group {
                    let base = raw_src.split("::").next().unwrap_or(&raw_src);
                    let base = base.split(" (group ").next().unwrap_or(base).trim();
                    let cue_path = std::path::PathBuf::from(base);
                    let src_dir = cue_path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
                    let dest_dir = dest_abs.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| dest_abs.clone());

                    let mut members = entry.members.clone().or_else(|| entry.group.clone()).unwrap_or_default();
                    if let Some(cn) = cue_path.file_name().and_then(|n| n.to_str()) {
                        if !members.iter().any(|m| m == cn) { members.insert(0, cn.to_string()); }
                    }

                    let mut ok = true;
                    for m in &members {
                        let s = src_dir.join(m);
                        let d = dest_dir.join(m);
                        if !s.exists() { errors.push(format!("group member not found: {}", s.display())); ok = false; continue; }
                        match safe_copy_file(&s, &d) {
                            Ok(()) => { written_dests.insert(d.to_string_lossy().to_lowercase(), None); }
                            Err(e) => { errors.push(format!("copy {} -> {}: {}", s.display(), d.display(), e)); ok = false; }
                        }
                    }
                    if ok {
                        deployed += 1;
                        written_dests.insert(dest_abs.to_string_lossy().to_lowercase(), entry.hash.clone());
                    } else { failed += 1; }
                    // breakdown row con action final y miembros
                    continue;
                }
                let src_str = entry.source.split("::").next().unwrap_or(&entry.source);
                let src = Path::new(src_str);
                if !src.exists() {
                    errors.push(format!("source not found: {}", src.display()));
                    failed += 1;
                    continue;
                }
                match safe_copy_file(src, &dest_abs) {
                    Ok(()) => {
                        deployed += 1;
                        written_dests.insert(dest_abs.to_string_lossy().to_lowercase(), entry.hash.clone().or_else(|| entry.source_hash.clone()));
                        // Emitir progreso después de cada archivo copiado
                        if matches!(action_final.as_str(), "copy" | "extract" | "convert_then_copy") {
                            completed += 1;
                            if let Some(app_handle) = app {
                                let _ = app_handle.emit("deploy-progress", serde_json::json!({
                                    "current": completed,
                                    "total": total,
                                    "percentage": (completed as f64 / total as f64 * 100.0) as u32,
                                    "current_file": entry.source,
                                    "message": format!("Transfiriendo {}/{} archivos...", completed, total)
                                }));
                            }
                        }
                    },
                    Err(e) => {
                        errors.push(format!("copy {} -> {}: {}", src.display(), dest_abs.display(), e));
                        failed += 1;
                    }
                }
            },
            "extract" => {
                let archive_src = Path::new(entry.source.split("::").next().unwrap_or(&entry.source));
                if !archive_src.exists() {
                    errors.push(format!("archive not found: {}", archive_src.display()));
                    failed += 1;
                    continue;
                }
                // For grouped logical units (CUE/BIN), the destination is a folder like roms/PS/Game
                // For single files, it's a file like roms/SFC/game.sfc
                // We need to preserve hierarchy: use a unique TempDir per archive
                let tmp_dir = tempfile::TempDir::new().map_err(|e| anyhow::anyhow!("tempdir failed: {}", e))?;
                match crate::archive::safe_extract_to_temp(archive_src, tmp_dir.path(), &crate::archive::Limits::default()) {
                    Ok(extracted) => {
                        let mut ok = true;
                        // For grouped, dest_abs is a directory; for single, it's a file's parent
                        let dest_base = if dest_abs.extension().is_some() && !dest_abs.is_dir() {
                            // Single file: dest_abs is like roms/SFC/game.sfc, so parent is roms/SFC
                            dest_abs.parent().unwrap_or(sd_path).to_path_buf()
                        } else {
                            // Grouped: dest_abs is like roms/PS/Game (directory)
                            dest_abs.clone()
                        };
                        for p in extracted {
                            // Preserve relative path from temp dir
                            let rel = p.strip_prefix(tmp_dir.path()).unwrap_or(&p);
                            let dest_file = dest_base.join(rel);
                            // Validate before copy
                            if let Err(e) = sd_target::validate_destination_path(&rel.to_string_lossy().to_string().replace('\\', "/")) {
                                errors.push(format!("extracted path invalid {} -> {}: {}", p.display(), dest_file.display(), e));
                                ok = false;
                                continue;
                            }
                            if let Err(e) = safe_copy_file(&p, &dest_file) {
                                errors.push(format!("extract copy {} -> {}: {}", p.display(), dest_file.display(), e));
                                ok = false;
                            } else {
                                written_dests.insert(dest_file.to_string_lossy().to_lowercase(), None);
                            }
                        }
                        if ok {
                            deployed += 1;
                            written_dests.insert(dest_abs.to_string_lossy().to_lowercase(), entry.hash.clone().or_else(|| entry.source_hash.clone()));
                            if matches!(action_final.as_str(), "copy" | "extract" | "convert_then_copy") {
                                completed += 1;
                                if let Some(app_handle) = app {
                                    let _ = app_handle.emit("deploy-progress", serde_json::json!({
                                        "current": completed,
                                        "total": total,
                                        "percentage": (completed as f64 / total as f64 * 100.0) as u32,
                                        "current_file": entry.source,
                                        "message": format!("Transfiriendo {}/{} archivos...", completed, total)
                                    }));
                                }
                            }
                        } else { failed += 1; }
                    },
                    Err(e) => {
                        // Fallback: copy archive as is if it's a payload (e.g., cps1 zip)
                        if entry.destination.ends_with(".zip") || entry.content_type.as_deref() == Some("archive-payload") {
                            match safe_copy_file(archive_src, &dest_abs) {
                                Ok(()) => deployed += 1,
                                Err(e2) => {
                                    errors.push(format!("extract {}: {} (fallback copy also failed: {})", archive_src.display(), e, e2));
                                    failed += 1;
                                }
                            }
                        } else {
                            errors.push(format!("extract {}: {}", archive_src.display(), e));
                            failed += 1;
                        }
                    }
                }
            },
            "convert_then_copy" => {
                let src = Path::new(&entry.source);
                if !src.exists() {
                    errors.push(format!("video source not found: {}", src.display()));
                    failed += 1;
                    continue;
                }
                // For video conversion, the planner has already determined that the source needs conversion
                // via ffprobe -> FFmpeg -> re-probe. In a full implementation, we would call video::convert here
                // to generate the converted file in a temp staging area, then copy the converted file.
                // For this milestone (read-only dry-run), we just copy the source and warn that conversion is provisional.
                warnings.push(format!("video conversion for {} is PROVISIONAL_UNVALIDATED (would convert via FFmpeg to {} before deploy)", src.display(), entry.converted_name.as_deref().unwrap_or("converted.mp4")));
                match safe_copy_file(src, &dest_abs) {
                    Ok(()) => {
                        deployed += 1;
                        written_dests.insert(dest_abs.to_string_lossy().to_lowercase(), entry.hash.clone().or_else(|| entry.source_hash.clone()));
                        // Emitir progreso después de cada archivo copiado
                        if matches!(action_final.as_str(), "copy" | "extract" | "convert_then_copy") {
                            completed += 1;
                            if let Some(app_handle) = app {
                                let _ = app_handle.emit("deploy-progress", serde_json::json!({
                                    "current": completed,
                                    "total": total,
                                    "percentage": (completed as f64 / total as f64 * 100.0) as u32,
                                    "current_file": entry.source,
                                    "message": format!("Transfiriendo {}/{} archivos...", completed, total)
                                }));
                            }
                        }
                    },
                    Err(e) => {
                        errors.push(format!("video copy {} -> {}: {}", src.display(), dest_abs.display(), e));
                        failed += 1;
                    }
                }
            },
            "skip" | "skip_unchanged" | "skip_duplicate" => {
                skipped += 1;
            },
            "conflict" | "manual_review" | "unsupported_archive" | "unsupported" | "conversion_error" => {
                warnings.push(format!("{} requires manual decision: {} -> {} ({})", entry.source, entry.destination, entry.action, entry.reason));
                skipped += 1;
            },
            _ => {
                warnings.push(format!("unknown action {} for {} -> {}", action, entry.source, entry.destination));
                skipped += 1;
            }
        }
    }

    info!(
        "Deploy complete: {} deployed, {} skipped, {} failed",
        deployed, skipped, failed
    );

    // Log detailed breakdown
    if skipped > 0 {
        let skipped_entries: Vec<_> = plan.entries.iter()
            .filter(|e| {
                let action = e.resolved_action.as_ref().unwrap_or(&e.action);
                matches!(action.as_str(), "skip" | "skip_unchanged" | "skip_duplicate" | "conflict" | "manual_review")
            })
            .collect();
        
        for entry in skipped_entries {
            let action = entry.resolved_action.as_ref().unwrap_or(&entry.action);
            warn!(
                "Skipped: {} -> {} (action: {}, reason: {})",
                entry.source, entry.destination, action, entry.reason
            );
        }
    }

    if failed > 0 {
        for error in &errors {
            warn!("Deploy error: {}", error);
        }
    }

    let success = failed == 0;

    Ok(DeployResult {
        success,
        deployed,
        skipped,
        failed,
        errors,
        warnings,
        breakdown: Some(breakdown_rows.clone()),
    })
}
