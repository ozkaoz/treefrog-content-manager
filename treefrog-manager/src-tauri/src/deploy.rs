use crate::Plan;
use crate::profile::LoadedProfile;
use crate::sd_target;
use std::path::Path;
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
}

fn ensure_parent_exists(dest: &Path) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn safe_copy_file(src: &Path, dest: &Path) -> anyhow::Result<()> {
    let dest_str = dest.to_string_lossy().to_string().replace('\\', "/");
    // For absolute dest, we need to validate the relative part
    // If dest is absolute (like G:/roms/...), extract the relative part after the SD root
    // For validation, we use the file name and parent logic, but we should not validate the drive letter
    // Instead, validate the part after the SD root mount point
    // For now, just validate the file name and directory names
    if let Some(file_name) = dest.file_name().and_then(|n| n.to_str()) {
        // Check for illegal chars in file name only for this simplified version
        sd_target::validate_destination_path(&format!("roms/{}", file_name)).map_err(|e| anyhow::anyhow!("invalid file name: {}", e))?;
    }
    ensure_parent_exists(dest)?;
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));
    let tmp_name = format!(".treefrog_staging_{}_{}.tmp", std::process::id(), dest.file_name().unwrap_or_default().to_string_lossy());
    let tmp_path = parent.join(tmp_name);
    std::fs::copy(src, &tmp_path)?;
    std::fs::rename(&tmp_path, dest)?;
    Ok(())
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
                let dest_dir = if dest_abs.extension().is_some() {
                    dest_abs.parent().unwrap_or(sd_path).to_path_buf()
                } else {
                    dest_abs.clone()
                };
                // For now, if it's an archive payload, just copy; otherwise try to extract
                // Use the archive module to decide
                match crate::archive::safe_extract_to_temp(archive_src, &std::env::temp_dir().join(format!("tf_extract_{}", std::process::id())), &crate::archive::Limits::default()) {
                    Ok(extracted) => {
                        // Copy each extracted file to dest_dir
                        let mut ok = true;
                        for p in extracted {
                            if let Some(rel) = p.file_name() {
                                let dest_file = dest_dir.join(rel);
                                if let Err(e) = safe_copy_file(&p, &dest_file) {
                                    errors.push(format!("extract copy {} -> {}: {}", p.display(), dest_file.display(), e));
                                    ok = false;
                                }
                            }
                        }
                        if ok { deployed += 1; } else { failed += 1; }
                    },
                    Err(e) => {
                        // Fallback: copy archive as is if it's a payload
                        if entry.destination.ends_with(".zip") {
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
                // In real implementation, this would be the converted file in staging
                // For now, just copy the source
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

    let success = failed == 0;
    Ok(DeployResult {
        success,
        deployed,
        skipped,
        failed,
        errors,
        warnings,
    })
}
