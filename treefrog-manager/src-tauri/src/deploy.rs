use crate::effective_action;
use crate::profile::LoadedProfile;
use crate::sd_target;
use crate::Plan;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

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

fn emit_phase(app: Option<&AppHandle>, phase: &str, current: usize, total: usize, file: &str) {
    if let Some(app_handle) = app {
        let _ = app_handle.emit("deploy-progress", serde_json::json!({
            "current": current,
            "total": total,
            "percentage": if total == 0 { 100 } else { ((current as f64 / total as f64) * 100.0) as u32 },
            "current_file": file,
            "phase": phase,
            "message": format!("{} {}/{} ...", phase, current, total),
            "isDeleting": false
        }));
    }
}

/// THE ONE SAFE WRITER. Resolves and validates the destination with the
/// canonical path model (paths::resolve_validated_destination) — secure even
/// when called directly. The staged temp file + atomic rename guarantees the
/// final path is only ever written after successful copy + size verification.
fn safe_copy_file_resolved(src: &Path, dest_resolved: &Path) -> anyhow::Result<()> {
    if let Some(parent) = dest_resolved.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let parent = dest_resolved.parent().unwrap_or_else(|| Path::new("."));
    // Unique temp file per operation (no collisions between parallel writers)
    let tmp_name = format!(
        ".treefrog_staging_{}_{}_{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        dest_resolved
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    );
    let tmp_path = parent.join(tmp_name);
    std::fs::copy(src, &tmp_path)?;
    // Integrity check: staged file size must match source exactly
    if std::fs::metadata(&tmp_path)?.len() != std::fs::metadata(src)?.len() {
        let _ = std::fs::remove_file(&tmp_path);
        anyhow::bail!(
            "copy verification failed: size mismatch for {} -> {}",
            src.display(),
            dest_resolved.display()
        );
    }
    // Atomic rename within the same directory
    match std::fs::rename(&tmp_path, dest_resolved) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_path);
            // Windows/exFAT: rename onto an existing file can fail; fall back
            // to replace-by-copy with the staged temp file as source.
            match std::fs::copy(&tmp_path, dest_resolved) {
                Ok(_) => {
                    let _ = std::fs::remove_file(&tmp_path);
                    Ok(())
                }
                Err(_) => Err(anyhow::anyhow!("atomic rename failed: {}", e)),
            }
        }
    }
}

/// Back-compat wrapper: validate + resolve destination from (sd_root, dest_rel)
/// and delegate to the resolved writer. Callers MUST pass the SD root.
/// Kept for direct-call compatibility (e.g. tests, future callers); secure by
/// itself — never writes outside the canonical resolved destination.
#[allow(dead_code)]
fn safe_copy_file(src: &Path, sd_root: &Path, dest_rel: &str) -> anyhow::Result<()> {
    let dest_resolved = crate::paths::resolve_validated_destination(sd_root, dest_rel)
        .map_err(|e| anyhow::anyhow!("invalid destination '{}': {}", dest_rel, e))?;
    safe_copy_file_resolved(src, &dest_resolved)
}

fn validate_rom_file(src: &Path) -> anyhow::Result<()> {
    let metadata = std::fs::metadata(src)?;
    if metadata.len() == 0 {
        anyhow::bail!("Archivo vacío: {}", src.display());
    }
    let mut file = std::fs::File::open(src)?;
    let mut buffer = [0; 1024];
    match file.read(&mut buffer) {
        Ok(0) => anyhow::bail!("No se pudo leer el archivo: {}", src.display()),
        Ok(_) => Ok(()),
        Err(e) => anyhow::bail!("Error leyendo archivo {}: {}", src.display(), e),
    }
}

/// Real video conversion pipeline (P1):
///   probe source -> (already evaluated at plan time) -> stage temp output ->
///   run ffmpeg -> probe converted output -> validate compatibility ->
///   only then deploy the staged output to the SD destination.
/// The ORIGINAL source is never modified and never copied as-is for
/// convert_then_copy. Failure/cancellation removes the staged output.
fn deploy_converted_video(
    entry_source: &str,
    sd_root: &Path,
    dest_rel: &str,
    profile: &LoadedProfile,
    app: Option<&AppHandle>,
    entry_hash: &mut Option<String>,
) -> anyhow::Result<()> {
    let src = Path::new(entry_source);
    if !src.exists() {
        anyhow::bail!("video source not found: {}", src.display());
    }
    // Phase event: probing
    emit_phase(app, "Probing", 0, 1, entry_source);
    // Re-probe source for freshness (planner decision may be stale)
    let probe = crate::video::probe(&src.to_string_lossy())
        .map_err(|e| anyhow::anyhow!("ffprobe failed for {}: {}", src.display(), e))?;
    let eval = crate::video::evaluate_compatibility(&probe, &profile.video_preset);
    if eval.status == "compatible" {
        // Source became compatible since planning: copy the ORIGINAL (allowed,
        // explicit and observable in the reason).
        let dest_resolved = crate::paths::resolve_validated_destination(sd_root, dest_rel)
            .map_err(|e| anyhow::anyhow!("invalid destination '{}': {}", dest_rel, e))?;
        return safe_copy_file_resolved(src, &dest_resolved);
    }
    if eval.status != "conversion_required" {
        anyhow::bail!(
            "video no longer convertible (status: {}): {}",
            eval.status,
            eval.reason
        );
    }
    // Phase event: converting
    emit_phase(app, "Converting", 0, 1, entry_source);
    // Temp staging area — output is validated BEFORE any SD write.
    let staging = tempfile::TempDir::new().map_err(|e| anyhow::anyhow!("tempdir failed: {}", e))?;
    let conv = crate::video::convert(src, staging.path(), &profile.video_preset);
    // TempDir drops automatically on error paths below (removes staged output).
    if !conv.success {
        anyhow::bail!(
            "video conversion failed for {}: {}",
            src.display(),
            conv.error
                .unwrap_or_else(|| "unknown conversion error".to_string())
        );
    }
    let converted = conv
        .output_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("conversion produced no output for {}", src.display()))?;
    // converted output already ffprobe-validated by video::convert; recompute
    // hash so the deployment records the DEPLOYED content's identity.
    *entry_hash = crate::hash::sha256_file(converted).ok();
    // Phase event: deploying
    emit_phase(app, "Deploying", 0, 1, entry_source);
    let dest_resolved = crate::paths::resolve_validated_destination(sd_root, dest_rel)
        .map_err(|e| anyhow::anyhow!("invalid destination '{}': {}", dest_rel, e))?;
    safe_copy_file_resolved(converted, &dest_resolved)
}

pub fn deploy_plan(
    plan: &Plan,
    sd_root: &str,
    profile: &LoadedProfile,
    force: bool,
    app: Option<&AppHandle>,
) -> anyhow::Result<DeployResult> {
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
            anyhow::bail!(
                "SD target not recognized as TreeFrogUI (missing cubegm/ + roms/): {}",
                sd_root
            );
        }
    }
    let space = sd_target::calculate_space(plan, analysis.free_bytes);
    if space.status == "insufficient_space" {
        anyhow::bail!(
            "Insufficient space: required {} available {}",
            space.required_bytes,
            space.available_bytes.unwrap_or(0)
        );
    }
    // Only entries that write can collide (effective action decides).
    let write_dests: Vec<String> = plan
        .entries
        .iter()
        .filter(|e| {
            matches!(
                effective_action(e),
                "copy" | "extract" | "convert_then_copy" | "replace"
            )
        })
        .map(|e| e.destination.clone())
        .collect();
    let collisions = sd_target::check_case_collision(&write_dests);
    if !collisions.is_empty() {
        anyhow::bail!(
            "Case collision detected: {} collides with {}",
            collisions[0].0,
            collisions[0].1
        );
    }

    let mut deployed = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut breakdown_rows: Vec<serde_json::Value> = Vec::new();
    let mut written_dests: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();

    let total = plan
        .entries
        .iter()
        .filter(|e| {
            matches!(
                effective_action(e),
                "copy" | "extract" | "convert_then_copy" | "replace"
            )
        })
        .count();
    let mut completed = 0usize;

    for entry in &plan.entries {
        let action = effective_action(entry).to_string();
        info!(
            "Processing: {} -> {} (action: {}, reason: {})",
            entry.source, entry.destination, action, entry.reason
        );
        let dest_rel = &entry.destination;
        // UNKNOWN destinations are NEVER written (state machine invariant).
        if dest_rel.contains("roms/UNKNOWN") {
            tracing::warn!("Skipping file with UNKNOWN destination: {}", entry.source);
            warnings.push(format!(
                "Archivo omitido por destino desconocido: {}",
                entry.source
            ));
            skipped += 1;
            continue;
        }
        // Canonical destination validation + resolution (single model).
        let dest_resolved = match crate::paths::resolve_validated_destination(sd_path, dest_rel) {
            Ok(p) => p,
            Err(e) => {
                errors.push(format!("{}: {}", dest_rel, e));
                failed += 1;
                continue;
            }
        };
        let dest_abs = dest_resolved.clone();

        // ---- Fresh on-disk verification: never trust a stale skip decision ----
        let dest_exists_now = dest_abs.exists();
        let downgraded: String = match action.as_str() {
            // "unchanged" REQUIRES the file to actually exist on the SD right now.
            "skip_unchanged" if !dest_exists_now => {
                info!(
                    "Downgrade skip_unchanged -> copy (destination missing on SD): {}",
                    dest_abs.display()
                );
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
        let is_write = matches!(
            action_final.as_str(),
            "copy" | "extract" | "convert_then_copy" | "replace"
        );
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

        // Record breakdown row with absolute verified path
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
                    let cue_path = PathBuf::from(base);
                    let src_dir = cue_path
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_default();
                    let dest_dir = dest_abs
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| dest_abs.clone());
                    // Group base destination in RELATIVE canonical form
                    let dest_dir_rel = entry
                        .destination
                        .rsplit_once('/')
                        .map(|(dir, _)| dir.to_string())
                        .unwrap_or_default();

                    let mut members = entry
                        .members
                        .clone()
                        .or_else(|| entry.group.clone())
                        .unwrap_or_default();
                    if let Some(cn) = cue_path.file_name().and_then(|n| n.to_str()) {
                        if !members.iter().any(|m| m == cn) {
                            members.insert(0, cn.to_string());
                        }
                    }

                    let mut ok = true;
                    for m in &members {
                        // Group member names must be valid single components
                        // (no traversal, no separators, no reserved names).
                        if let Err(e) = crate::paths::validate_relative_destination(m) {
                            errors.push(format!("group member name invalid {}: {}", m, e));
                            ok = false;
                            continue;
                        }
                        let s = src_dir.join(m);
                        let member_dest = if dest_dir_rel.is_empty() {
                            m.clone()
                        } else {
                            format!("{}/{}", dest_dir_rel, m)
                        };
                        let d = crate::paths::resolve_validated_destination(sd_path, &member_dest)
                            .unwrap_or_else(|_| dest_dir.join(m));
                        if !s.exists() {
                            errors.push(format!("group member not found: {}", s.display()));
                            ok = false;
                            continue;
                        }
                        if let Err(e) = validate_rom_file(&s) {
                            errors.push(format!("validation failed {}: {}", s.display(), e));
                            warn!("validation failed {}: {}", s.display(), e);
                            ok = false;
                            continue;
                        }
                        if let Ok(meta) = std::fs::metadata(&s) {
                            info!(
                                "Copying ROM (grupo): {} -> {} ({} bytes)",
                                s.display(),
                                d.display(),
                                meta.len()
                            );
                        }
                        match safe_copy_file_resolved(&s, &d) {
                            Ok(()) => {
                                written_dests.insert(d.to_string_lossy().to_lowercase(), None);
                            }
                            Err(e) => {
                                errors.push(format!(
                                    "copy {} -> {}: {}",
                                    s.display(),
                                    d.display(),
                                    e
                                ));
                                ok = false;
                            }
                        }
                    }
                    if ok {
                        deployed += 1;
                        written_dests.insert(
                            dest_abs.to_string_lossy().to_lowercase(),
                            entry.hash.clone(),
                        );
                    } else {
                        failed += 1;
                    }
                    continue;
                }
                let src_str = entry.source.split("::").next().unwrap_or(&entry.source);
                let src = Path::new(src_str);
                if !src.exists() {
                    errors.push(format!("source not found: {}", src.display()));
                    failed += 1;
                    continue;
                }
                if let Err(e) = validate_rom_file(src) {
                    errors.push(format!("validation failed {}: {}", src.display(), e));
                    warn!("validation failed {}: {}", src.display(), e);
                    failed += 1;
                    continue;
                }
                if let Ok(meta) = std::fs::metadata(src) {
                    info!(
                        "Copying ROM: {} -> {} ({} bytes)",
                        src.display(),
                        dest_abs.display(),
                        meta.len()
                    );
                }
                match safe_copy_file_resolved(src, &dest_abs) {
                    Ok(()) => {
                        deployed += 1;
                        written_dests.insert(
                            dest_abs.to_string_lossy().to_lowercase(),
                            entry.hash.clone().or_else(|| entry.source_hash.clone()),
                        );
                        completed += 1;
                        emit_phase(app, "Transferring", completed, total, &entry.source);
                    }
                    Err(e) => {
                        errors.push(format!(
                            "copy {} -> {}: {}",
                            src.display(),
                            dest_abs.display(),
                            e
                        ));
                        failed += 1;
                    }
                }
            }
            "extract" => {
                let archive_src =
                    Path::new(entry.source.split("::").next().unwrap_or(&entry.source));
                if !archive_src.exists() {
                    errors.push(format!("archive not found: {}", archive_src.display()));
                    failed += 1;
                    continue;
                }
                // For grouped logical units (CUE/BIN), the destination is a folder like roms/PS/Game
                // For single files, it's a file like roms/SFC/game.sfc
                // We need to preserve hierarchy: use a unique TempDir per archive
                let tmp_dir = tempfile::TempDir::new()
                    .map_err(|e| anyhow::anyhow!("tempdir failed: {}", e))?;
                match crate::archive::safe_extract_to_temp(
                    archive_src,
                    tmp_dir.path(),
                    &crate::archive::Limits::default(),
                ) {
                    Ok(extracted) => {
                        let mut ok = true;
                        // Destination base in RELATIVE form (canonical model):
                        // - grouped: entry.destination is the target folder (roms/PS/Game)
                        // - single:   entry.destination is the file (roms/SFC/game.sfc) -> its parent
                        let dest_base_rel = if dest_abs.extension().is_some() && !dest_abs.is_dir()
                        {
                            entry
                                .destination
                                .rsplit_once('/')
                                .map(|(dir, _)| dir.to_string())
                                .unwrap_or_default()
                        } else {
                            entry.destination.clone().trim_end_matches('/').to_string()
                        };
                        for p in extracted {
                            // Preserve relative path from temp dir
                            let rel = p.strip_prefix(tmp_dir.path()).unwrap_or(&p);
                            let rel_str = rel.to_string_lossy().replace('\\', "/");
                            // Canonical validation of the member name BEFORE join
                            if let Err(e) = crate::paths::validate_relative_destination(&rel_str) {
                                errors.push(format!(
                                    "extracted path invalid {} -> {}: {}",
                                    p.display(),
                                    rel_str,
                                    e
                                ));
                                ok = false;
                                continue;
                            }
                            // Combined destination (base + member), canonically validated
                            let member_dest = if dest_base_rel.is_empty() {
                                rel_str.clone()
                            } else {
                                format!("{}/{}", dest_base_rel, rel_str)
                            };
                            let dest_file_resolved =
                                match crate::paths::resolve_validated_destination(
                                    sd_path,
                                    &member_dest,
                                ) {
                                    Ok(d) => d,
                                    Err(e) => {
                                        errors.push(format!(
                                            "extracted member destination invalid {} -> {}: {}",
                                            p.display(),
                                            member_dest,
                                            e
                                        ));
                                        ok = false;
                                        continue;
                                    }
                                };
                            if let Err(e) = validate_rom_file(&p) {
                                errors.push(format!("validation failed {}: {}", p.display(), e));
                                warn!("validation failed {}: {}", p.display(), e);
                                ok = false;
                                continue;
                            }
                            if let Ok(meta) = std::fs::metadata(&p) {
                                info!(
                                    "Copying ROM (extract): {} -> {} ({} bytes)",
                                    p.display(),
                                    dest_file_resolved.display(),
                                    meta.len()
                                );
                            }
                            if let Err(e) = safe_copy_file_resolved(&p, &dest_file_resolved) {
                                errors.push(format!(
                                    "extract copy {} -> {}: {}",
                                    p.display(),
                                    dest_file_resolved.display(),
                                    e
                                ));
                                ok = false;
                            } else {
                                written_dests.insert(
                                    dest_file_resolved.to_string_lossy().to_lowercase(),
                                    None,
                                );
                            }
                        }
                        if ok {
                            deployed += 1;
                            written_dests.insert(
                                dest_abs.to_string_lossy().to_lowercase(),
                                entry.hash.clone().or_else(|| entry.source_hash.clone()),
                            );
                            completed += 1;
                            emit_phase(app, "Transferring", completed, total, &entry.source);
                        } else {
                            failed += 1;
                        }
                    }
                    Err(e) => {
                        // UNSUPPORTED formats are NEVER silently copied as
                        // supported content. Only payload archives (zip kept
                        // intact per profile) fall back to a plain copy.
                        let is_payload = entry.destination.ends_with(".zip")
                            || entry.content_type.as_deref() == Some("archive-payload");
                        if is_payload {
                            match safe_copy_file_resolved(archive_src, &dest_abs) {
                                Ok(()) => deployed += 1,
                                Err(e2) => {
                                    errors.push(format!(
                                        "extract {}: {} (payload copy also failed: {})",
                                        archive_src.display(),
                                        e,
                                        e2
                                    ));
                                    failed += 1;
                                }
                            }
                        } else {
                            errors.push(format!("extract {}: {}", archive_src.display(), e));
                            failed += 1;
                        }
                    }
                }
            }
            "convert_then_copy" => {
                // REAL conversion: original is never deployed for this action.
                let mut entry_hash = entry.hash.clone();
                match deploy_converted_video(
                    &entry.source,
                    sd_path,
                    &entry.destination,
                    profile,
                    app,
                    &mut entry_hash,
                ) {
                    Ok(()) => {
                        deployed += 1;
                        written_dests.insert(
                            dest_abs.to_string_lossy().to_lowercase(),
                            entry_hash.clone().or_else(|| entry.source_hash.clone()),
                        );
                        completed += 1;
                        emit_phase(app, "Transferring", completed, total, &entry.source);
                    }
                    Err(e) => {
                        errors.push(format!(
                            "video conversion {} -> {}: {}",
                            entry.source,
                            dest_abs.display(),
                            e
                        ));
                        failed += 1;
                    }
                }
            }
            "skip" | "skip_unchanged" | "skip_duplicate" => {
                skipped += 1;
            }
            "conflict"
            | "manual_review"
            | "unsupported_archive"
            | "unsupported"
            | "conversion_error" => {
                warnings.push(format!(
                    "{} requires manual decision: {} -> {} ({})",
                    entry.source, entry.destination, entry.action, entry.reason
                ));
                skipped += 1;
            }
            _ => {
                warnings.push(format!(
                    "unknown action {} for {} -> {}",
                    action, entry.source, entry.destination
                ));
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
        let skipped_entries: Vec<_> = plan
            .entries
            .iter()
            .filter(|e| {
                matches!(
                    effective_action(e),
                    "skip" | "skip_unchanged" | "skip_duplicate" | "conflict" | "manual_review"
                )
            })
            .collect();

        for entry in skipped_entries {
            warn!(
                "Skipped: {} -> {} (action: {}, reason: {})",
                entry.source,
                entry.destination,
                effective_action(entry),
                entry.reason
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

#[cfg(test)]
mod deploy_security_tests {
    use super::*;

    /// The writer must be secure when called DIRECTLY: any traversal attempt
    /// is rejected regardless of caller validation.
    #[test]
    fn safe_writer_rejects_traversal_when_called_directly() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd_root = tmp.path().join("sd");
        std::fs::create_dir_all(&sd_root).unwrap();
        let src = tmp.path().join("rom.bin");
        std::fs::write(&src, b"rom").unwrap();
        for bad in [
            "../evil.bin",
            "a/../evil.bin",
            "/evil.bin",
            "C:\\evil.bin",
            "\\\\srv\\s\\e.bin",
        ] {
            assert!(
                safe_copy_file(&src, &sd_root, bad).is_err(),
                "must reject: {bad}"
            );
        }
        // Valid relative dest writes INSIDE the root
        safe_copy_file(&src, &sd_root, "roms/FC/rom.bin").unwrap();
        assert!(sd_root.join("roms/FC/rom.bin").exists());
        // Nothing escaped
        assert!(!tmp.path().join("evil.bin").exists());
    }

    /// Staging + atomic rename: a successful copy leaves no staging temp files.
    #[test]
    fn successful_copy_leaves_no_staging_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd_root = tmp.path().join("sd");
        std::fs::create_dir_all(sd_root.join("roms")).unwrap();
        let src = tmp.path().join("fake.bin");
        std::fs::write(&src, b"x").unwrap();
        safe_copy_file(&src, &sd_root, "roms/game.bin").unwrap();
        assert!(sd_root.join("roms/game.bin").exists());
        for e in std::fs::read_dir(sd_root.join("roms")).unwrap() {
            let name = e.unwrap().file_name().to_string_lossy().to_string();
            assert!(
                !name.starts_with(".treefrog_staging_"),
                "staging file leaked: {name}"
            );
        }
    }

    /// BIOS deployments go through the same writer: end-to-end BIOS deploy via
    /// a canonical Plan proves BIOS uses identical write safety rules.
    #[test]
    fn bios_uses_same_plan_deploy_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd_root = tmp.path().join("sd");
        std::fs::create_dir_all(sd_root.join("cubegm/bios")).unwrap();
        std::fs::create_dir_all(sd_root.join("roms")).unwrap();
        let src = tmp.path().join("bios");
        std::fs::create_dir_all(&src).unwrap();
        let bios_file = src.join("gba_bios.bin");
        std::fs::write(&bios_file, b"bios").unwrap();

        let plan = crate::Plan {
            summary: crate::PlanSummary::default(),
            entries: vec![crate::PlanEntry {
                source: bios_file.to_string_lossy().to_string(),
                destination: "cubegm/bios/gba_bios.bin".to_string(),
                action: "copy".to_string(),
                reason: "BIOS (user-supplied)".to_string(),
                content_type: Some("bios".to_string()),
                size: Some(4),
                ..Default::default()
            }],
            warnings: vec![],
        };
        // analyze_target requires a TreeFrogUI-like target; create markers
        let profile = crate::profile::load_profile().unwrap();
        let result = deploy_plan(
            &plan,
            sd_root.to_string_lossy().as_ref(),
            &profile,
            false,
            None,
        )
        .unwrap();
        assert_eq!(result.deployed, 1);
        assert!(sd_root.join("cubegm/bios/gba_bios.bin").exists());
    }
}
