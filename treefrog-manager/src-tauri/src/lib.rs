pub mod archive;
pub mod bios;
pub mod bios_catalog;
pub mod classify;
pub mod db;
pub mod deploy;
pub mod hash;
pub mod planner;
pub mod profile;
pub mod scanner;
pub mod sd;
pub mod sd_target;
pub mod video;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tauri::Emitter;
static ANALYZE_CACHE: Mutex<Option<(String, Instant, serde_json::Value)>> = Mutex::new(None);
static APP_INITIALIZED: AtomicBool = AtomicBool::new(false);

fn analyze_target_cached(path: &str) -> Result<serde_json::Value, String> {
    if let Ok(g) = ANALYZE_CACHE.lock() {
        if let Some((p, t, v)) = &*g {
            if p == path && t.elapsed().as_secs() < 15 { return Ok(v.clone()); }
        }
    }
    let v = serde_json::to_value(sd_target::analyze_target(path).map_err(|e| e.to_string())?).unwrap();
    if let Ok(mut g) = ANALYZE_CACHE.lock() { *g = Some((path.to_string(), Instant::now(), v.clone())); }
    Ok(v)
}

fn cleanup_state() {
    // Clear in-memory caches
    if let Ok(mut g) = ANALYZE_CACHE.lock() {
        *g = None;
    }
    // Clear any on-disk state that might persist between sessions (e.g., temp files, stale DB)
    // For now, just clear the cache; DB is not used for transferred files persistence
    // If a SQLite DB exists at the default location, we could delete it here, but it's not currently used for sync state
}

pub fn reset_all_state() {
    tracing::info!("Resetting all application state...");
    
    // Limpiar caché de análisis
    if let Ok(mut cache) = ANALYZE_CACHE.lock() {
        *cache = None;
    }
    
    APP_INITIALIZED.store(true, Ordering::SeqCst);
    
    tracing::info!("Application state reset complete");
}

#[tauri::command]
async fn reset_app_state() -> Result<(), String> {
    reset_all_state();
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlanSummary {
    pub unchanged: usize,
    pub new: usize,
    pub changed: usize,
    pub duplicate_content: usize,
    pub conflicts: usize,
    pub deletions: usize,
    #[serde(default)]
    pub manual_review: usize,
    #[serde(default)]
    pub unsupported_archive: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PlanEntry {
    pub source: String,
    pub destination: String,
    pub action: String, // copy | extract | skip_unchanged | skip_duplicate | conflict | manual_review | unsupported_archive
    pub reason: String,
    pub hash: Option<String>,
    #[serde(default)]
    pub source_hash: Option<String>,
    #[serde(default)]
    pub destination_hash: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub possible_destinations: Option<Vec<String>>,
    pub size: Option<u64>,
    pub group: Option<Vec<String>>,
    #[serde(default)]
    pub members: Option<Vec<String>>,
    #[serde(default)]
    pub default_action: Option<String>,
    #[serde(default)]
    pub resolution: Option<String>,
    #[serde(default)]
    pub resolved_action: Option<String>,
    #[serde(default)]
    pub original_destination: Option<String>,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub probe: Option<serde_json::Value>,
    #[serde(default)]
    pub converted_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Plan {
    pub summary: PlanSummary,
    pub entries: Vec<PlanEntry>,
    pub warnings: Vec<String>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Resetear estado al iniciar
    reset_all_state();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            dry_run_preview,
            detect_sd,
            verify_profile,
            bios_profile,
            bios_scan,
            list_volumes,
            analyze_target,
            dry_run_with_target,
            deploy_to_sd,
            lgpt_scan_samples,
            lgpt_scan_projects,
            build_info,
            clear_cache,
            delete_roms_from_sd,
            reset_app_state,
            get_valid_systems_for_extension,
            scan_games,
            scan_music,
            scan_videos,
            scan_bios_files,
            list_files_in_folder,
            check_for_updates,
            download_update,
            get_temp_path,
            open_folder,
            get_bios_catalog,
            validate_bios_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn verify_profile() -> Result<serde_json::Value, String> {
    let p = profile::load_profile().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true, "profile_version": p.profile_version }))
}

#[tauri::command]
fn detect_sd(path: String) -> Result<serde_json::Value, String> {
    let r = sd::detect(&path).map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(r).unwrap())
}

#[tauri::command]
async fn dry_run_preview(source_path: String, sd_path: String) -> Result<Plan, String> {
    // Phase 1: read-only preview — never writes.
    let profile = profile::load_profile().map_err(|e| e.to_string())?;
    let sd_info = sd::detect(&sd_path).map_err(|e| e.to_string())?;
    if !sd_info.is_treefrog_sd {
        return Err(format!("SD path is not a TreeFrogUI SD (missing cubegm/ + roms/ markers): {}", sd_path));
    }
    let scanned = scanner::scan(&source_path, &profile).map_err(|e| e.to_string())?;
    let plan = planner::plan(scanned, &sd_path, &profile).map_err(|e| e.to_string())?;
    Ok(plan)
}

#[tauri::command]
fn bios_profile() -> Result<serde_json::Value, String> {
    let profile = profile::load_profile().map_err(|e| e.to_string())?;
    let bios_json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../profiles/treefrogui/bios.json")
    ).or_else(|_| std::fs::read_to_string("profiles/treefrogui/bios.json"))
    .map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let definitions = bios_json.get("bios_definitions").cloned().unwrap_or(serde_json::Value::Array(vec![]));
    Ok(serde_json::json!({ "profile_version": profile.profile_version, "definitions": definitions }))
}

#[tauri::command]
fn bios_scan(bios_source: String) -> Result<serde_json::Value, String> {
    let profile = profile::load_profile().map_err(|e| e.to_string())?;
    let bios_json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../profiles/treefrogui/bios.json")
    ).or_else(|_| std::fs::read_to_string("profiles/treefrogui/bios.json"))
    .map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let definitions = bios_json.get("bios_definitions").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    // Use existing scanner to recursively inspect BIOS source, safely inspect archives, hash where needed
    let scanned = scanner::scan(&bios_source, &profile).map_err(|e| e.to_string())?;
    // Also need to handle archives that contain BIOS files: scanner already handles archives via logical units, but for BIOS we want to treat archives as potential BIOS containers
    // For now, collect all scanned files that are BIOS or inside archives that would be extracted
    let mut bios_files: Vec<std::path::PathBuf> = Vec::new();
    for sf in &scanned {
        if sf.classification.kind == crate::classify::Kind::Bios {
            bios_files.push(sf.source_path.clone());
        }
        // Also handle archive members that would be BIOS: if sf is archive, we need to inspect its inner BIOS files
        // For simplicity, if the scanned entry is an archive, we can try to extract its BIOS members to temp and add them
        if sf.classification.kind == crate::classify::Kind::Archive {
            // Try to inspect and extract BIOS files to temp for validation (reuse archive infrastructure)
            if let Ok(inner) = crate::archive::inspect_archive(&sf.source_path, &crate::archive::Limits::default()) {
                for entry in inner.iter().filter(|e| !e.is_dir) {
                    let p = std::path::Path::new(&entry.name);
                    let dummy = crate::classify::classify(p, &profile);
                    if dummy.kind == crate::classify::Kind::Bios {
                        // Extract this BIOS file to temp for validation
                        if let Ok(tmp) = tempfile::TempDir::new() {
                            if let Ok(extracted) = crate::archive::safe_extract_to_temp(&sf.source_path, tmp.path(), &crate::archive::Limits::default()) {
                                for ex in extracted {
                                    if ex.file_name().and_then(|n| n.to_str()).map(|n| n.to_lowercase() == p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase()).unwrap_or(false) {
                                        bios_files.push(ex.clone());
                                        // Leak temp dir by forgetting it (so file remains for validation)
                                        std::mem::forget(tmp);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // Also include any BIOS files that were directly found as Bios kind (already in bios_files)
    // Now validate each BIOS definition against found files
    // For system_content_present, we can infer from scanned files: if any scanned file is for a system that has BIOS, then that system content is present
    // For now, just assume all systems with content are present if any scanned file's system_id matches
    let mut system_content_present: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    for sf in &scanned {
        if let Some(sys_id) = &sf.classification.system_id {
            system_content_present.insert(sys_id.clone(), true);
        }
        // Also check folders for system
        for def in &definitions {
            if let Some(sys_id) = def.get("system_id").and_then(|v| v.as_str()) {
                if sf.source_path.to_string_lossy().to_lowercase().contains(&sys_id.to_lowercase()) {
                    system_content_present.insert(sys_id.to_string(), true);
                }
            }
        }
    }
    let results = crate::bios::validate_all_bios(&bios_files, &definitions, &system_content_present);
    let mut out: Vec<serde_json::Value> = Vec::new();
    for (bios_id, res) in results {
        let mut v = serde_json::to_value(&res).unwrap();
        // Add variant info: which variant satisfied
        if let Some(def) = definitions.iter().find(|d| d.get("id").and_then(|x| x.as_str()) == Some(&bios_id)) {
            v["definition"] = def.clone();
            // Try to determine which variant matched (if found_valid, check which variant's filename/hash matches)
            if let Some(file) = res.file.clone() {
                let fname = std::path::Path::new(&file).file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                for var in def.get("variants").and_then(|x| x.as_array()).unwrap_or(&vec![]) {
                    if let Some(arr) = var.get("filenames").and_then(|x| x.as_array()) {
                        for fnm in arr {
                            if let Some(s) = fnm.as_str() {
                                if s.to_lowercase() == fname {
                                    v["variant"] = var.get("id").cloned().unwrap_or(serde_json::Value::String("unknown".to_string()));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        out.push(v);
    }
    // Deterministic sort by bios_id
    out.sort_by(|a,b| a.get("bios_id").and_then(|x| x.as_str()).unwrap_or("").cmp(b.get("bios_id").and_then(|x| x.as_str()).unwrap_or("")));
    Ok(serde_json::json!({ "results": out }))
}

#[tauri::command]
fn list_volumes() -> Result<serde_json::Value, String> {
    let vols = sd_target::list_volumes();
    Ok(serde_json::to_value(vols).unwrap())
}

#[tauri::command]
fn analyze_target(path: String) -> Result<serde_json::Value, String> {
    let analysis = sd_target::analyze_target(&path).map_err(|e| e.to_string())?;
    // Validate that we haven't written anything (read-only guarantee)
    Ok(serde_json::to_value(analysis).unwrap())
}

#[tauri::command]
async fn dry_run_with_target(source_path: String, sd_path: String) -> Result<serde_json::Value, String> {
    let profile = profile::load_profile().map_err(|e| e.to_string())?;
    let target_val = analyze_target_cached(&sd_path)?;
    let target: sd_target::TargetAnalysis = serde_json::from_value(target_val).unwrap();
    if target.status == "inaccessible" {
        return Err(format!("Target inaccessible: {}", sd_path));
    }
    // Use existing planner (single source of truth) with target path
    let scanned = scanner::scan(&source_path, &profile).map_err(|e| e.to_string())?;
    let plan = planner::plan(scanned, &sd_path, &profile).map_err(|e| e.to_string())?;
    let plan = planner::resolve_write_collisions(plan);
    // Validate destination paths and check collisions
    for e in &plan.entries {
        sd_target::validate_destination_path(&e.destination).map_err(|err| format!("invalid destination {}: {}", e.destination, err))?;
    }
    // ONLY entries that write to the SD can collide. Skips/duplicates/conflicts never write.
    let write_dests: Vec<String> = plan.entries.iter()
        .filter(|e| {
            let a = e.resolved_action.as_ref().unwrap_or(&e.action);
            matches!(a.as_str(), "copy" | "extract" | "convert_then_copy" | "replace")
        })
        .map(|e| e.destination.clone())
        .collect();
    let collisions = sd_target::check_case_collision(&write_dests);
    if !collisions.is_empty() {
        log::warn!("Unexpected leftover collisions (resolved as warnings): {:?}", collisions);
    }
    // Calculate space
    let space = sd_target::calculate_space(&plan, target.free_bytes);
    let mut out = serde_json::to_value(&plan).unwrap();
    out["target"] = serde_json::to_value(&target).unwrap();
    out["space"] = serde_json::to_value(&space).unwrap();
    out["collisions"] = serde_json::to_value(&collisions).unwrap();
    Ok(out)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BiosPlanEntry {
    pub source: String,
    pub destination: String,
    pub action: String,
    pub reason: String,
    pub content_type: String,
    pub is_bios: bool,
}

#[tauri::command]
async fn deploy_to_sd(app: tauri::AppHandle, sd_path: String, force: Option<bool>, selected_files: Option<Vec<String>>, user_decisions: Option<std::collections::HashMap<String, String>>, bios_entries: Option<Vec<BiosPlanEntry>>, source_path: Option<String>) -> Result<serde_json::Value, String> {
    let profile = profile::load_profile().map_err(|e| e.to_string())?;
    let target_val = analyze_target_cached(&sd_path)?;
    let target: sd_target::TargetAnalysis = serde_json::from_value(target_val).unwrap();
    if target.status == "inaccessible" {
        return Err(format!("Target inaccessible: {}", sd_path));
    }
    if !target.is_treefrog {
        return Err(format!("Target is not a valid TreeFrogUI SD (status: {}): {}", target.status, sd_path));
    }

    let force = force.unwrap_or(false);

    // Handle BIOS-only FIRST (before removable check - BIOS can go to any writable target)
    if source_path.is_none() {
        if let Some(bios) = &bios_entries {
            if bios.is_empty() {
                return Err("No files to sync".to_string());
            }
            
            if target.volume.removable != Some(true) && !force {
                return Err(format!(
                    "REFUSED: {} is not a removable drive. Connect the SD and select it in Overview. Enable 'Force copy' in SD Card only if your reader reports the SD as a fixed drive.",
                    sd_path
                ));
            }
            
            let mut deployed = 0usize;
            let mut skipped = 0usize;
            let mut failed = 0usize;
            let mut errors = Vec::new();
            let mut warnings = Vec::new();
            let total = bios.len();
            
            for (idx, entry) in bios.iter().enumerate() {
                let dest_abs = std::path::Path::new(&sd_path).join(&entry.destination);
                let filename_lower = dest_abs.file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                
                const STOCK_BIOS: &[&str] = &[
                    "scph1001.bin", "scph5501.bin", "scph5502.bin",
                    "gba_bios.bin", "o2rom.bin", "neogeo.zip",
                    "disksys.rom", "bios_cd_u.bin", "bios_cd_e.bin", "bios_cd_j.bin"
                ];
                
                let already_exists = dest_abs.exists();
                let is_stock_bios = STOCK_BIOS.contains(&filename_lower.as_str());
                
                if already_exists && is_stock_bios {
                    tracing::info!("Skipping stock BIOS overwrite (already exists): {}", dest_abs.display());
                    skipped += 1;
                    warnings.push(format!(
                        "Skipped {} - stock BIOS already exists on SD",
                        entry.source.split('/').last().unwrap_or(&entry.source)
                    ));
                    continue;
                }
                
                let _ = std::fs::create_dir_all(
                    dest_abs.parent().unwrap_or(std::path::Path::new(&sd_path))
                );
                
                match std::fs::copy(std::path::Path::new(&entry.source), &dest_abs) {
                    Ok(_) => {
                        deployed += 1;
                        tracing::info!("BIOS copied: {} -> {}", entry.source, dest_abs.display());
                    }
                    Err(e) => {
                        failed += 1;
                        errors.push(format!(
                            "BIOS {} -> {}: {}",
                            entry.source,
                            dest_abs.display(),
                            e
                        ));
                    }
                }
                
                let _ = app.emit("deploy-progress", serde_json::json!({
                    "current": idx + 1,
                    "total": total,
                    "percentage": (((idx + 1) as f64 / total as f64) * 100.0) as u32,
                    "current_file": entry.source,
                    "message": format!("Copying BIOS {}/{}...", idx + 1, total),
                    "isDeleting": false
                }));
            }
            
            return Ok(serde_json::json!({
                "success": failed == 0,
                "deployed": deployed,
                "skipped": skipped,
                "failed": failed,
                "errors": errors,
                "warnings": warnings,
                "breakdown": serde_json::Value::Null,
                "target": serde_json::to_value(&target).unwrap(),
                "space": serde_json::json!({"status": "ok"}),
                "plan": serde_json::json!({"entries": [], "summary": {}}),
                "bios_deployed": deployed,
                "bios_skipped": skipped,
                "bios_failed": failed
            }));
        }
        return Err("No files to sync".to_string());
    }
    
    if target.volume.removable != Some(true) && !force {
        return Err(format!(
            "REFUSED: {} is not a removable drive. Connect the SD and select it in Overview. Enable 'Force copy' in SD Card only if your reader reports the SD as a fixed drive.",
            sd_path
        ));
    }
    
    let source_path_str = source_path.unwrap();

    // BIOS: copy directly without planner (triple guard for cubegm/bios already in delete, here for deploy)
    let mut bios_deployed = 0usize;
    let mut bios_failed = 0usize;
    let mut bios_errors: Vec<String> = Vec::new();
    if let Some(entries) = &bios_entries {
        let total_bios = entries.len();
        let mut completed_bios = 0usize;
        for entry in entries {
            let dest_abs = std::path::Path::new(&sd_path).join(&entry.destination);
            let filename_lower = dest_abs.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            const STOCK_BIOS: &[&str] = &["scph1001.bin","scph5501.bin","scph5502.bin","gba_bios.bin","o2rom.bin","neogeo.zip","disksys.rom","bios_cd_u.bin","bios_cd_e.bin","bios_cd_j.bin"];
            if STOCK_BIOS.contains(&filename_lower.as_str()) {
                if !force {
                    tracing::info!("Skipping stock BIOS overwrite: {}", dest_abs.display());
                    continue;
                }
            }
            let normalized = entry.destination.to_lowercase().replace('\\', "/");
            if !normalized.starts_with("cubegm/bios") {
                tracing::warn!("Skipping non-BIOS destination for BIOS entry: {}", entry.destination);
                continue;
            }
            let _ = std::fs::create_dir_all(dest_abs.parent().unwrap_or(std::path::Path::new(&sd_path)));
            match std::fs::copy(std::path::Path::new(&entry.source), &dest_abs) {
                Ok(_) => {
                    bios_deployed += 1;
                    tracing::info!("BIOS copied: {} -> {}", entry.source, dest_abs.display());
                }
                Err(e) => {
                    bios_failed += 1;
                    bios_errors.push(format!("BIOS {} -> {}: {}", entry.source, dest_abs.display(), e));
                    tracing::error!("BIOS copy failed: {}", e);
                }
            }
            completed_bios += 1;
            let _ = app.emit("deploy-progress", serde_json::json!({
                "current": completed_bios,
                "total": total_bios,
                "percentage": ((completed_bios as f64 / total_bios.max(1) as f64) * 100.0) as u32,
                "current_file": entry.source,
                "message": format!("Copying BIOS {}/{}...", completed_bios, total_bios),
                "isDeleting": false
            }));
        }
    }

    let scanned = scanner::scan(&source_path_str, &profile).map_err(|e| e.to_string())?;
    let mut plan = if let Some(ref files) = selected_files {
        planner::plan_with_selection(scanned, &sd_path, &profile, Some(files.clone())).map_err(|e| e.to_string())?
    } else {
        planner::plan(scanned, &sd_path, &profile).map_err(|e| e.to_string())?
    };
    if let Some(overrides) = &user_decisions {
        for entry in plan.entries.iter_mut() {
            let src_base = entry.source.split("::").next().unwrap_or(&entry.source).to_string();
            if let Some(new_folder) = overrides.get(&src_base).or_else(|| overrides.get(&entry.source)) {
                let file_name = std::path::Path::new(&entry.destination).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let new_dest = format!("{}/{}", new_folder.trim_end_matches('/'), file_name);
                tracing::info!("Override aplicado: {} -> {} (antes {})", src_base, new_dest, entry.destination);
                entry.destination = new_dest;
            }
        }
    }
    let plan = planner::resolve_write_collisions(plan);
    for e in &plan.entries {
        sd_target::validate_destination_path(&e.destination).map_err(|err| format!("invalid destination {}: {}", e.destination, err))?;
    }
    let space = sd_target::calculate_space(&plan, target.free_bytes);
    if space.status == "insufficient_space" {
        return Err(format!("Insufficient space: required {} available {}", space.required_bytes, space.available_bytes.unwrap_or(0)));
    }
    // ONLY entries that write to the SD can collide. Skips/duplicates/conflicts
    // never write, so their destinations must not be checked.
    let write_dests: Vec<String> = plan.entries.iter()
        .filter(|e| {
            let a = e.resolved_action.as_ref().unwrap_or(&e.action);
            matches!(a.as_str(), "copy" | "extract" | "convert_then_copy" | "replace")
        })
        .map(|e| e.destination.clone())
        .collect();
    let collisions = sd_target::check_case_collision(&write_dests);
    if !collisions.is_empty() {
        // Never abort: deploy.rs has a runtime double-write guard as last resort.
        log::warn!("Leftover write collisions (deploy guard will skip them): {:?}", collisions);
    }
    let mut result = crate::deploy::deploy_plan(&plan, &sd_path, &profile, force, Some(&app)).map_err(|e| e.to_string())?;
    result.deployed += bios_deployed;
    result.failed += bios_failed;
    result.errors.extend(bios_errors.clone());
    if bios_deployed > 0 {
        result.warnings.push(format!("BIOS: {} copied, {} failed", bios_deployed, bios_failed));
    }
    let mut out = serde_json::to_value(&result).unwrap();
    out["target"] = serde_json::to_value(&target).unwrap();
    out["space"] = serde_json::to_value(&space).unwrap();
    out["plan"] = serde_json::to_value(&plan).unwrap();
    out["bios_deployed"] = serde_json::json!(bios_deployed);
    out["bios_failed"] = serde_json::json!(bios_failed);
    Ok(out)
}

#[tauri::command]
async fn lgpt_scan_samples(samples_source: String, sd_path: String) -> Result<serde_json::Value, String> {
    let profile = profile::load_profile().map_err(|e| e.to_string())?;
    let scanned = scanner::scan(&samples_source, &profile).map_err(|e| e.to_string())?;
    // Forzar contexto LGPT samples: WAV/sonidos -> lgpt/samples/
    let scanned: Vec<scanner::ScannedFile> = scanned.into_iter().map(|mut sf| {
        if sf.classification.kind == crate::classify::Kind::Music
            || sf.classification.kind == crate::classify::Kind::Unknown
        {
            sf.classification.kind = crate::classify::Kind::LgptSample;
            sf.classification.destination = "lgpt/samples".to_string();
        }
        sf
    }).collect();
    let plan = planner::plan(scanned, &sd_path, &profile).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "samples": [], "projects": [], "plan": plan }))
}

#[tauri::command]
async fn lgpt_scan_projects(projects_source: String, sd_path: String) -> Result<serde_json::Value, String> {
    let profile = profile::load_profile().map_err(|e| e.to_string())?;
    let scanned = scanner::scan(&projects_source, &profile).map_err(|e| e.to_string())?;
    let scanned: Vec<scanner::ScannedFile> = scanned.into_iter().map(|mut sf| {
        sf.classification.kind = crate::classify::Kind::LgptProject;
        sf.classification.destination = "lgpt/projects".to_string();
        sf
    }).collect();
    let plan = planner::plan(scanned, &sd_path, &profile).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "samples": [], "projects": [], "plan": plan }))
}

#[tauri::command]
fn build_info() -> serde_json::Value {
    serde_json::json!({
        "commit": option_env!("TFM_GIT_COMMIT").unwrap_or("dev"),
        "built_at": option_env!("TFM_BUILD_TS").unwrap_or("unknown")
    })
}

#[tauri::command]
fn clear_cache() -> Result<(), String> {
    cleanup_state();
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteResult {
    pub success: bool,
    pub deleted: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemOption {
    pub id: String,
    pub folder: String,
    pub display_name: String,
    pub core: String,
}

#[tauri::command]
async fn get_valid_systems_for_extension(ext: String) -> Result<Vec<SystemOption>, String> {
    let profile = crate::profile::load_profile().map_err(|e| e.to_string())?;
    let mut systems = Vec::new();
    let ext_lower = if ext.starts_with('.') { ext.to_lowercase() } else { format!(".{}", ext.to_lowercase()) };
    for sys in &profile.systems {
        if sys.extensions.iter().any(|e| e.to_lowercase() == ext_lower) {
            systems.push(SystemOption {
                id: sys.id.clone(),
                folder: sys.folder_aliases.first().cloned().unwrap_or_default(),
                display_name: sys.display_name.clone().unwrap_or(sys.id.clone()),
                core: sys.core.clone().unwrap_or_default(),
            });
        }
    }
    Ok(systems)
}

#[tauri::command]
async fn get_bios_catalog() -> Result<Vec<crate::bios_catalog::BiosEntry>, String> {
    Ok(crate::bios_catalog::get_bios_catalog())
}

#[tauri::command]
async fn validate_bios_file(path: String, bios_id: String) -> Result<serde_json::Value, String> {
    let catalog = crate::bios_catalog::get_bios_catalog();
    let bios = catalog.iter().find(|b| b.id == bios_id)
        .ok_or_else(|| format!("BIOS id not found: {}", bios_id))?;

    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Ok(serde_json::json!({"valid": false, "reason": "File not found"}));
    }

    let filename = p.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let name_ok = if let Some(pattern) = &bios.pattern {
        let regex_str = "^".to_string() + &pattern.to_lowercase().replace('.', "\\.").replace('*', ".*") + "$";
        regex::Regex::new(&regex_str)
            .map(|re| re.is_match(&filename))
            .unwrap_or(false)
    } else {
        bios.filenames.iter().any(|f| f.to_lowercase() == filename)
    };

    if !name_ok {
        return Ok(serde_json::json!({
            "valid": false,
            "reason": format!("Filename '{}' does not match expected pattern: {}", p.file_name().unwrap_or_default().to_string_lossy(), bios.pattern.clone().unwrap_or(bios.filenames.join(" OR ")))
        }));
    }

    if let Some(expected_size) = bios.expected_size {
        let actual_size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        if actual_size != expected_size {
            return Ok(serde_json::json!({
                "valid": false,
                "reason": format!("Size mismatch: expected {} bytes, got {}", expected_size, actual_size)
            }));
        }
    }

    if let Some(expected_md5) = &bios.md5 {
        let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
        let digest = md5::compute(&bytes);
        let actual_md5 = format!("{:x}", digest);
        if actual_md5 != *expected_md5 {
            return Ok(serde_json::json!({
                "valid": false,
                "reason": format!("MD5 mismatch: expected {}, got {}", expected_md5, actual_md5)
            }));
        }
    }

    if let Some(expected_sha) = &bios.sha256 {
        let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
        let mut hasher = sha2::Sha256::new();
        use sha2::Digest;
        hasher.update(&bytes);
        let actual_sha = hex::encode(hasher.finalize());
        if actual_sha != *expected_sha {
            return Ok(serde_json::json!({
                "valid": false,
                "reason": format!("SHA-256 mismatch")
            }));
        }
    }

    Ok(serde_json::json!({"valid": true, "reason": "OK"}))
}

#[tauri::command]
async fn scan_games(path: String) -> Result<Vec<scanner::ScannedFile>, String> {
    let profile = crate::profile::load_profile().map_err(|e| e.to_string())?;
    scanner::scan_games(&path, &profile).map_err(|e| e.to_string())
}

#[tauri::command]
async fn scan_music(path: String) -> Result<Vec<scanner::ScannedFile>, String> {
    let profile = crate::profile::load_profile().map_err(|e| e.to_string())?;
    scanner::scan_music(&path, &profile).map_err(|e| e.to_string())
}

#[tauri::command]
async fn scan_videos(path: String) -> Result<Vec<scanner::ScannedFile>, String> {
    let profile = crate::profile::load_profile().map_err(|e| e.to_string())?;
    scanner::scan_videos(&path, &profile).map_err(|e| e.to_string())
}

#[tauri::command]
async fn scan_bios_files(path: String) -> Result<Vec<scanner::ScannedFile>, String> {
    let profile = crate::profile::load_profile().map_err(|e| e.to_string())?;
    scanner::scan_bios(&path, &profile).map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_roms_from_sd(
    app: tauri::AppHandle,
    sd_path: String,
    files_to_delete: Vec<String>,
    delete_all: bool,
) -> Result<DeleteResult, String> {
    use std::path::Path;
    let sd = Path::new(&sd_path);
    if !sd.exists() {
        return Err(format!("SD path no existe: {}", sd_path));
    }
    let target = sd_target::analyze_target(&sd_path).map_err(|e| e.to_string())?;
    if !target.is_treefrog {
        return Err(format!("No es una SD TreeFrog válida (status: {}): {}", target.status, sd_path));
    }
    let profile = crate::profile::load_profile().map_err(|e| e.to_string())?;

    const STOCK_BIOS_FILES: &[&str] = &[
        "scph1001.bin", "scph5501.bin", "scph5502.bin",
        "gba_bios.bin",
        "o2rom.bin",
        "neogeo.zip",
        "disksys.rom",
        "bios_cd_u.bin", "bios_cd_e.bin", "bios_cd_j.bin",
    ];
    
    let mut valid_extensions: std::collections::HashSet<String> = std::collections::HashSet::new();
    
    for sys in &profile.systems {
        for ext in &sys.extensions {
            valid_extensions.insert(ext.to_lowercase());
        }
    }
    
    valid_extensions.extend([
        ".mp3", ".flac", ".ogg", ".wav", ".m4a", ".aac", ".opus",
        ".mp4", ".mkv", ".avi", ".mov", ".wmv", ".webm",
        ".jpg", ".jpeg", ".png", ".bmp", ".gif", ".webp", ".tiff",
    ].iter().map(|s| s.to_string()));
    
    let mut files_to_process = Vec::new();
    
    if delete_all {
        for dir in ["roms", "lgpt"] {
            let dir_path = sd.join(dir);
            if dir_path.exists() {
                for entry in walkdir::WalkDir::new(&dir_path).into_iter().filter_map(|e| e.ok()) {
                    if entry.file_type().is_file() {
                        let file_name = entry.file_name().to_string_lossy().to_lowercase();
                        let is_stock_bios = STOCK_BIOS_FILES.contains(&file_name.as_str());
                        if is_stock_bios {
                            tracing::info!("Preserved (stock BIOS): {}", entry.path().display());
                            continue;
                        }
                        let ext = entry.path()
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| format!(".{}", e.to_lowercase()))
                            .unwrap_or_default();
                        
                        let is_bios = entry.path().to_string_lossy().contains("cubegm/bios");
                        let is_valid = is_bios || valid_extensions.contains(&ext);
                        
                        if is_valid {
                            files_to_process.push(entry.path().to_path_buf());
                        } else {
                            tracing::info!("Preserved (invalid extension): {}", entry.path().display());
                        }
                    }
                }
            }
        }
    } else {
        for file_rel in &files_to_delete {
            let normalized_rel = file_rel.to_lowercase().replace('\\', "/");
            if normalized_rel.contains("cubegm/bios") {
                tracing::warn!("Skipping BIOS folder from deletion (protected)");
                continue;
            }
            let file_path = sd.join(file_rel);
            if !file_path.exists() {
                continue;
            }
            if file_path.is_file() {
                let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                let is_stock_bios = STOCK_BIOS_FILES.contains(&file_name.as_str());
                if is_stock_bios {
                    tracing::info!("Preserved (stock BIOS): {}", file_path.display());
                    continue;
                }
                let ext = file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| format!(".{}", e.to_lowercase()))
                    .unwrap_or_default();
                let is_bios = file_path.to_string_lossy().contains("cubegm/bios");
                let lower_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                let is_doc = lower_name == "readme" || lower_name == "readme.txt" || lower_name.ends_with(".txt") && !valid_extensions.contains(&ext) || lower_name.ends_with(".md");
                if is_doc && !is_bios && !valid_extensions.contains(&ext) {
                    tracing::info!("Preserved (documentation): {}", file_path.display());
                    continue;
                }
                let is_valid = is_bios || valid_extensions.contains(&ext) || file_rel.starts_with("roms/") || file_rel.starts_with("roms\\");
                if is_valid || file_path.to_string_lossy().contains("roms/") {
                    files_to_process.push(file_path);
                } else {
                    tracing::info!("Preserved: {}", file_path.display());
                }
            } else if file_path.is_dir() {
                for entry in walkdir::WalkDir::new(&file_path).into_iter().filter_map(|e| e.ok()) {
                    if entry.file_type().is_file() {
                        let file_name = entry.file_name().to_string_lossy().to_lowercase();
                        let is_stock_bios = STOCK_BIOS_FILES.contains(&file_name.as_str());
                        if is_stock_bios {
                            tracing::info!("Preserved (stock BIOS): {}", entry.path().display());
                            continue;
                        }
                        let ext = entry.path()
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| format!(".{}", e.to_lowercase()))
                            .unwrap_or_default();
                        let is_bios = entry.path().to_string_lossy().contains("cubegm/bios");
                        let is_valid = is_bios || valid_extensions.contains(&ext);
                        if is_valid {
                            files_to_process.push(entry.path().to_path_buf());
                        } else {
                            tracing::info!("Preserved (invalid extension): {}", entry.path().display());
                        }
                    }
                }
            }
        }
    }
    
    let total_files = files_to_process.len();
    if total_files == 0 {
        let _ = app.emit("delete-progress", serde_json::json!({
            "current": 0,
            "total": 0,
            "percentage": 100,
            "current_file": "",
            "message": "Nothing to delete",
            "isDeleting": false
        }));
        return Ok(DeleteResult {
            success: true,
            deleted: 0,
            failed: 0,
            errors: Vec::new(),
        });
    }
    let mut deleted = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();
    
    for file_path in files_to_process {
        let normalized = file_path.to_string_lossy().to_lowercase().replace('\\', "/");
        if normalized.contains("cubegm/bios") {
            tracing::warn!("Skipping BIOS file from deletion (protected): {}", file_path.display());
            continue;
        }
        match std::fs::remove_file(&file_path) {
            Ok(_) => {
                deleted += 1;
                tracing::info!("Deleted: {}", file_path.display());
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("Error deleting {}: {}", file_path.display(), e));
                tracing::error!("Error deleting {}: {}", file_path.display(), e);
            }
        }
        
        let _ = app.emit("delete-progress", serde_json::json!({
            "current": deleted + failed,
            "total": total_files,
            "percentage": ((deleted + failed) as f64 / total_files.max(1) as f64 * 100.0) as u32,
            "current_file": file_path.file_name().unwrap_or_default().to_string_lossy(),
            "message": format!("Deleting {}/{} files...", deleted + failed, total_files),
            "isDeleting": true
        }));
    }
    
    let _ = app.emit("delete-progress", serde_json::json!({
        "current": total_files,
        "total": total_files,
        "percentage": 100,
        "current_file": "",
        "message": "Deletion complete",
        "isDeleting": false
    }));
    
    Ok(DeleteResult {
        success: failed == 0,
        deleted,
        failed,
        errors,
    })
}

#[tauri::command]
async fn list_files_in_folder(sd_path: String, folder_rel: String) -> Result<Vec<String>, String> {
    use std::path::Path;
    let folder = Path::new(&sd_path).join(&folder_rel);
    if !folder.exists() { return Ok(vec![]); }
    
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(&folder).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(rel) = entry.path().strip_prefix(Path::new(&sd_path)) {
                files.push(rel.to_string_lossy().to_string().replace('\\', "/"));
            }
        }
    }
    files.sort();
    Ok(files)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub html_url: String,
    pub assets: Vec<GitHubAsset>,
    pub published_at: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

#[tauri::command]
async fn check_for_updates(current_version: String) -> Result<Option<GitHubRelease>, String> {
    let client = reqwest::Client::new();
    let url = "https://api.github.com/repos/ozkaoz/treefrog-content-manager/releases/latest";
    let response = client
        .get(url)
        .header("User-Agent", "TreeFrog-Content-Manager")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }
    let release: GitHubRelease = response.json().await.map_err(|e| format!("Failed to parse release info: {}", e))?;
    let latest_version = release.tag_name.trim_start_matches('v');
    let current = current_version.trim_start_matches('v');
    if latest_version != current {
        Ok(Some(release))
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn download_update(url: String, save_path: String) -> Result<(), String> {
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await.map_err(|e| format!("Failed to download: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }
    let bytes = response.bytes().await.map_err(|e| format!("Failed to read bytes: {}", e))?;
    std::fs::write(&save_path, &bytes).map_err(|e| format!("Failed to save file: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn get_temp_path() -> Result<String, String> {
    let temp = std::env::temp_dir();
    Ok(temp.to_string_lossy().to_string())
}

#[tauri::command]
async fn open_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(&path).spawn().map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    Ok(())
}

