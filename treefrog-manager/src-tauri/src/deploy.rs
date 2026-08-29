use crate::Plan;
use crate::profile::LoadedProfile;
use crate::sd_target;
use std::path::Path;
use log::{info, warn};
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

pub fn deploy_plan(plan: &Plan, sd_root: &str, _profile: &LoadedProfile) -> anyhow::Result<DeployResult> {
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
        match action.as_str() {
            "copy" | "replace" => {
                let src_str = entry.source.split("::").next().unwrap_or(&entry.source);
                let src = Path::new(src_str);
                if !src.exists() {
                    errors.push(format!("source not found: {}", src.display()));
                    failed += 1;
                    continue;
                }
                match safe_copy_file(src, &dest_abs) {
                    Ok(()) => deployed += 1,
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
                            }
                        }
                        if ok { deployed += 1; } else { failed += 1; }
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
                    Ok(()) => deployed += 1,
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

    // Build detailed breakdown for UI
    let breakdown: Vec<serde_json::Value> = plan.entries.iter().map(|e| {
        let action = e.resolved_action.as_ref().unwrap_or(&e.action);
        serde_json::json!({
            "source": e.source,
            "destination": e.destination,
            "action": action,
            "reason": e.reason,
            "content_type": e.content_type,
        })
    }).collect();

    Ok(DeployResult {
        success,
        deployed,
        skipped,
        failed,
        errors,
        warnings,
        breakdown: Some(breakdown),
    })
}
