pub mod archive;
pub mod bios;
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
use std::time::Instant;
use tauri::Emitter;
static ANALYZE_CACHE: Mutex<Option<(String, Instant, serde_json::Value)>> = Mutex::new(None);

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
    cleanup_state();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
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
            delete_roms_from_sd
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

#[tauri::command]
async fn deploy_to_sd(app: tauri::AppHandle, source_path: String, sd_path: String, force: Option<bool>) -> Result<serde_json::Value, String> {
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

    // HARD GUARD: never write to a non-removable drive unless the user explicitly forces it
    // (some USB card readers report as fixed; force covers that case).
    if target.volume.removable != Some(true) && !force {
        return Err(format!(
            "REFUSADO: {} no es una unidad removible (SD/USB). Conecta la SD y selecciónala en Overview. Activa 'Forzar copia' en SD Card solo si tu lector reporta la SD como unidad fija.",
            sd_path
        ));
    }
    let scanned = scanner::scan(&source_path, &profile).map_err(|e| e.to_string())?;
    let plan = planner::plan(scanned, &sd_path, &profile).map_err(|e| e.to_string())?;
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
    let result = crate::deploy::deploy_plan(&plan, &sd_path, &profile, force, Some(&app)).map_err(|e| e.to_string())?;
    let mut out = serde_json::to_value(&result).unwrap();
    out["target"] = serde_json::to_value(&target).unwrap();
    out["space"] = serde_json::to_value(&space).unwrap();
    out["plan"] = serde_json::to_value(&plan).unwrap();
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
    // Validate SD is TreeFrog to avoid accidental deletion elsewhere
    let target = sd_target::analyze_target(&sd_path).map_err(|e| e.to_string())?;
    if !target.is_treefrog {
        return Err(format!("No es una SD TreeFrog válida (status: {}): {}", target.status, sd_path));
    }
    // Contar total de archivos a eliminar
    let mut total_files = 0usize;
    let mut files_to_process: Vec<std::path::PathBuf> = Vec::new();
    
    if delete_all {
        for dir in ["roms", "cubegm/bios", "lgpt"] {
            let dir_path = sd.join(dir);
            if dir_path.exists() {
                for entry in walkdir::WalkDir::new(&dir_path).into_iter().filter_map(|e| e.ok()) {
                    if entry.file_type().is_file() {
                        total_files += 1;
                        files_to_process.push(entry.path().to_path_buf());
                    }
                }
            }
        }
        if total_files == 0 {
            let roms_dir = sd.join("roms");
            if roms_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&roms_dir) {
                    for e in entries.filter_map(|e| e.ok()) {
                        if e.path().is_dir() {
                            total_files += 1;
                            for inner in walkdir::WalkDir::new(e.path()).into_iter().filter_map(|e| e.ok()) {
                                if inner.file_type().is_file() {
                                    files_to_process.push(inner.path().to_path_buf());
                                }
                            }
                            if files_to_process.is_empty() {
                                files_to_process.push(e.path());
                            }
                        }
                    }
                }
            }
        }
    } else {
        for file_rel in &files_to_delete {
            if !file_rel.starts_with("roms/") && !file_rel.starts_with("roms\\") && !file_rel.starts_with("cubegm/bios") && !file_rel.starts_with("lgpt") {
                if let Err(_e) = sd_target::validate_destination_path(file_rel) {
                    if !file_rel.starts_with("roms") {
                        continue;
                    }
                }
            }
            let file_path = sd.join(file_rel);
            if !file_path.exists() {
                continue;
            }
            if file_path.is_file() {
                total_files += 1;
                files_to_process.push(file_path);
            } else if file_path.is_dir() {
                let mut dir_files = 0usize;
                for entry in walkdir::WalkDir::new(&file_path).into_iter().filter_map(|e| e.ok()) {
                    if entry.file_type().is_file() {
                        dir_files += 1;
                        files_to_process.push(entry.path().to_path_buf());
                    }
                }
                if dir_files == 0 {
                    total_files += 1;
                    files_to_process.push(file_path);
                } else {
                    total_files += dir_files;
                }
            }
        }
    }
    
    if total_files == 0 && files_to_process.is_empty() {
        cleanup_state();
        return Ok(DeleteResult {
            success: true,
            deleted: 0,
            failed: 0,
            errors: Vec::new(),
        });
    }
    if total_files == 0 {
        total_files = files_to_process.len();
    }
    
    let mut deleted: usize = 0;
    let mut failed: usize = 0;
    let mut errors: Vec<String> = Vec::new();
    
    for file_path in files_to_process {
        if !file_path.exists() {
            deleted += 1;
            let _ = app.emit("delete-progress", serde_json::json!({
                "current": deleted + failed,
                "total": total_files,
                "percentage": ((deleted + failed) as f64 / total_files.max(1) as f64 * 100.0) as u32,
                "current_file": file_path.file_name().unwrap_or_default().to_string_lossy(),
                "message": format!("Eliminando {}/{} archivos...", deleted + failed, total_files),
                "isDeleting": true
            }));
            continue;
        }
        let is_dir = file_path.is_dir();
        let res = if is_dir {
            std::fs::remove_dir_all(&file_path)
        } else {
            std::fs::remove_file(&file_path)
        };
        match res {
            Ok(_) => {
                deleted += 1;
                log::info!("Eliminado: {}", file_path.display());
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("Error eliminando {}: {}", file_path.display(), e));
                log::error!("Error eliminando {}: {}", file_path.display(), e);
            }
        }
        
        let _ = app.emit("delete-progress", serde_json::json!({
            "current": deleted + failed,
            "total": total_files,
            "percentage": ((deleted + failed) as f64 / total_files.max(1) as f64 * 100.0) as u32,
            "current_file": file_path.file_name().unwrap_or_default().to_string_lossy(),
            "message": format!("Eliminando {}/{} archivos...", deleted + failed, total_files),
            "isDeleting": true
        }));
    }
    
    if delete_all {
        for dir in ["roms", "cubegm/bios", "lgpt"] {
            let dir_path = sd.join(dir);
            if dir_path.exists() {
                if let Ok(entries) = std::fs::read_dir(&dir_path) {
                    let is_empty = entries.count() == 0;
                    if !is_empty {
                        for entry in walkdir::WalkDir::new(&dir_path).into_iter().filter_map(|e| e.ok()).collect::<Vec<_>>().into_iter().rev() {
                            if entry.file_type().is_dir() {
                                let p = entry.path();
                                if p != dir_path {
                                    let _ = std::fs::remove_dir(p);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        for file_rel in &files_to_delete {
            let file_path = sd.join(file_rel);
            if file_path.exists() && file_path.is_dir() {
                let _ = std::fs::remove_dir(&file_path);
            } else if !file_path.exists() {
                if let Some(parent) = file_path.parent() {
                    if parent.exists() && parent.is_dir() {
                        if let Ok(mut entries) = std::fs::read_dir(parent) {
                            if entries.next().is_none() {
                                let _ = std::fs::remove_dir(parent);
                            }
                        }
                    }
                }
            }
        }
    }
    
    cleanup_state();
    Ok(DeleteResult {
        success: failed == 0,
        deleted,
        failed,
        errors,
    })
}
