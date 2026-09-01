pub mod archive;
pub mod bios;
pub mod bios_catalog;
pub mod classify;
pub mod db;
pub mod deploy;
pub mod hash;
pub mod paths;
pub mod planner;
pub mod profile;
pub mod scanner;
pub mod sd;
pub mod sd_target;
pub mod video;

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use tauri::Emitter;
static ANALYZE_CACHE: Mutex<Option<(String, Instant, serde_json::Value)>> = Mutex::new(None);
static APP_INITIALIZED: AtomicBool = AtomicBool::new(false);

fn analyze_target_cached(path: &str) -> Result<serde_json::Value, String> {
    if let Ok(g) = ANALYZE_CACHE.lock() {
        if let Some((p, t, v)) = &*g {
            if p == path && t.elapsed().as_secs() < 15 {
                return Ok(v.clone());
            }
        }
    }
    let v =
        serde_json::to_value(sd_target::analyze_target(path).map_err(|e| e.to_string())?).unwrap();
    if let Ok(mut g) = ANALYZE_CACHE.lock() {
        *g = Some((path.to_string(), Instant::now(), v.clone()));
    }
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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
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

/// The ONE definition of the action that will actually be executed for an
/// entry. Every subsystem (deployment, progress totals, summary calculation,
/// space calculation, collision detection, dry-run reporting, frontend-facing
/// counts) must use this — never mix `action` and `resolved_action` by hand.
pub fn effective_action(entry: &PlanEntry) -> &str {
    entry.resolved_action.as_deref().unwrap_or(&entry.action)
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
            validate_bios_file,
            scan_music_structured,
            resolve_plan,
            app_version,
            get_content_counts
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn verify_profile() -> Result<serde_json::Value, String> {
    let p = profile::load_profile().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true, "profile_version": p.profile_version }))
}

/// Backend authority for plan resolution: the frontend collects user choices
/// (per entry index/source/destination) and sends them here. ALL business
/// rules (resolve, rename, effective action, collisions) execute in Rust.
/// The React layer never reimplements resolution semantics.
#[tauri::command]
fn resolve_plan(
    plan: Plan,
    sd_path: String,
    decisions: std::collections::HashMap<String, String>,
) -> Result<Plan, String> {
    // Canonical validation of incoming plan destinations before resolution.
    let sd_root = std::path::Path::new(&sd_path);
    for e in &plan.entries {
        crate::paths::resolve_validated_destination(sd_root, &e.destination)
            .map_err(|err| format!("invalid destination {}: {}", e.destination, err))?;
    }
    let resolved = planner::apply_resolutions_ctx(plan, &decisions, &sd_path);
    // Post-resolution validation: renames must still be canonically safe.
    for e in &resolved.entries {
        crate::paths::resolve_validated_destination(sd_root, &e.destination)
            .map_err(|err| format!("resolved destination invalid {}: {}", e.destination, err))?;
    }
    Ok(resolved)
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
        return Err(format!(
            "SD path is not a TreeFrogUI SD (missing cubegm/ + roms/ markers): {}",
            sd_path
        ));
    }
    let scanned = scanner::scan(&source_path, &profile).map_err(|e| e.to_string())?;
    let plan = planner::plan(scanned, &sd_path, &profile).map_err(|e| e.to_string())?;
    Ok(plan)
}

#[tauri::command]
fn bios_profile() -> Result<serde_json::Value, String> {
    let profile = profile::load_profile().map_err(|e| e.to_string())?;
    let bios_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../profiles/treefrogui/bios.json"),
        )
        .or_else(|_| std::fs::read_to_string("profiles/treefrogui/bios.json"))
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let definitions = bios_json
        .get("bios_definitions")
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![]));
    Ok(
        serde_json::json!({ "profile_version": profile.profile_version, "definitions": definitions }),
    )
}

#[tauri::command]
fn bios_scan(bios_source: String) -> Result<serde_json::Value, String> {
    let profile = profile::load_profile().map_err(|e| e.to_string())?;
    let bios_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../profiles/treefrogui/bios.json"),
        )
        .or_else(|_| std::fs::read_to_string("profiles/treefrogui/bios.json"))
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let definitions = bios_json
        .get("bios_definitions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
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
            if let Ok(inner) =
                crate::archive::inspect_archive(&sf.source_path, &crate::archive::Limits::default())
            {
                for entry in inner.iter().filter(|e| !e.is_dir) {
                    let p = std::path::Path::new(&entry.name);
                    let dummy = crate::classify::classify(p, &profile);
                    if dummy.kind == crate::classify::Kind::Bios {
                        // Extract this BIOS file to temp for validation
                        if let Ok(tmp) = tempfile::TempDir::new() {
                            if let Ok(extracted) = crate::archive::safe_extract_to_temp(
                                &sf.source_path,
                                tmp.path(),
                                &crate::archive::Limits::default(),
                            ) {
                                for ex in extracted {
                                    if ex
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .map(|n| {
                                            n.to_lowercase()
                                                == p.file_name()
                                                    .and_then(|n| n.to_str())
                                                    .unwrap_or("")
                                                    .to_lowercase()
                                        })
                                        .unwrap_or(false)
                                    {
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
    let mut system_content_present: std::collections::HashMap<String, bool> =
        std::collections::HashMap::new();
    for sf in &scanned {
        if let Some(sys_id) = &sf.classification.system_id {
            system_content_present.insert(sys_id.clone(), true);
        }
        // Also check folders for system
        for def in &definitions {
            if let Some(sys_id) = def.get("system_id").and_then(|v| v.as_str()) {
                if sf
                    .source_path
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&sys_id.to_lowercase())
                {
                    system_content_present.insert(sys_id.to_string(), true);
                }
            }
        }
    }
    let results =
        crate::bios::validate_all_bios(&bios_files, &definitions, &system_content_present);
    let mut out: Vec<serde_json::Value> = Vec::new();
    for (bios_id, res) in results {
        let mut v = serde_json::to_value(&res).unwrap();
        // Add variant info: which variant satisfied
        if let Some(def) = definitions
            .iter()
            .find(|d| d.get("id").and_then(|x| x.as_str()) == Some(&bios_id))
        {
            v["definition"] = def.clone();
            // Try to determine which variant matched (if found_valid, check which variant's filename/hash matches)
            if let Some(file) = res.file.clone() {
                let fname = std::path::Path::new(&file)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                for var in def
                    .get("variants")
                    .and_then(|x| x.as_array())
                    .unwrap_or(&vec![])
                {
                    if let Some(arr) = var.get("filenames").and_then(|x| x.as_array()) {
                        for fnm in arr {
                            if let Some(s) = fnm.as_str() {
                                if s.to_lowercase() == fname {
                                    v["variant"] = var.get("id").cloned().unwrap_or(
                                        serde_json::Value::String("unknown".to_string()),
                                    );
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
    out.sort_by(|a, b| {
        a.get("bios_id")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .cmp(b.get("bios_id").and_then(|x| x.as_str()).unwrap_or(""))
    });
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
async fn dry_run_with_target(
    source_path: String,
    sd_path: String,
) -> Result<serde_json::Value, String> {
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
        sd_target::validate_destination_path(&e.destination)
            .map_err(|err| format!("invalid destination {}: {}", e.destination, err))?;
    }
    // ONLY entries that write to the SD can collide. Skips/duplicates/conflicts never write.
    let write_dests: Vec<String> = plan
        .entries
        .iter()
        .filter(|e| {
            let a = e.resolved_action.as_ref().unwrap_or(&e.action);
            matches!(
                a.as_str(),
                "copy" | "extract" | "convert_then_copy" | "replace"
            )
        })
        .map(|e| e.destination.clone())
        .collect();
    let collisions = sd_target::check_case_collision(&write_dests);
    if !collisions.is_empty() {
        log::warn!(
            "Unexpected leftover collisions (resolved as warnings): {:?}",
            collisions
        );
    }
    // Calculate space
    let space = sd_target::calculate_space(&plan, target.free_bytes);
    let mut out = serde_json::to_value(&plan).unwrap();
    out["target"] = serde_json::to_value(&target).unwrap();
    out["space"] = serde_json::to_value(&space).unwrap();
    out["collisions"] = serde_json::to_value(&collisions).unwrap();
    Ok(out)
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BiosPlanEntry {
    pub source: String,
    pub destination: String,
    pub action: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub is_bios: Option<bool>,
    #[serde(default)]
    pub size: Option<u64>,
}

/// Embedded BIOS profile for the portable EXE (no external files required).
/// Same pattern as profile.rs: the file system is checked first (dev/updates),
/// and the embedded copy is the portable fallback — never an empty catalog.
const EMBEDDED_BIOS_JSON: &str = include_str!("../../../profiles/treefrogui/bios.json");

/// Single loader for the declarative BIOS profile (bios.json) — the ONLY
/// authoritative BIOS definition source. Hardcoded lists are forbidden.
pub fn bios_profile_json() -> serde_json::Value {
    let candidates = [
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../profiles/treefrogui/bios.json"),
        std::path::PathBuf::from("profiles/treefrogui/bios.json"),
    ];
    for c in &candidates {
        if let Ok(s) = std::fs::read_to_string(c) {
            if let Ok(v) = serde_json::from_str(&s) {
                return v;
            }
        }
    }
    // Portable fallback: embedded bios.json (the catalog must NEVER be empty
    // just because the exe runs without a profiles/ folder next to it).
    serde_json::from_str(EMBEDDED_BIOS_JSON)
        .unwrap_or_else(|_| serde_json::json!({ "bios_definitions": [] }))
}

/// Public re-export for sibling modules (bios_catalog).
pub fn bios_profile_json_public() -> serde_json::Value {
    bios_profile_json()
}

/// BIOS stock-guard: names of stock BIOS files (declared required/conditional
/// in bios.json) that are never silently overwritten once present on the SD.
fn bios_stock_guard_names() -> Vec<String> {
    let json = bios_profile_json();
    let mut names = Vec::new();
    if let Some(defs) = json.get("bios_definitions").and_then(|v| v.as_array()) {
        for def in defs {
            let required = def
                .get("required")
                .and_then(|v| v.as_str())
                .unwrap_or("optional");
            if required == "required" || required == "conditional" {
                if let Some(files) = def.get("accepted_filenames").and_then(|v| v.as_array()) {
                    for f in files {
                        if let Some(s) = f.as_str() {
                            let l = s.to_lowercase();
                            // Only concrete filenames (no wildcards) can be stock-guarded
                            if !l.contains('*') && !l.contains('?') {
                                names.push(l);
                            }
                        }
                    }
                }
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// Convert user-selected BIOS files into normal PlanEntry objects that flow
/// through the SAME planner → resolution → destination validation → space
/// validation → deployment lifecycle as every other content type.
/// Malicious destinations are rejected here (canonical path validation).
fn bios_entries_to_plan_entries(
    bios: &[BiosPlanEntry],
    sd_path: &str,
    force: bool,
) -> Result<Vec<PlanEntry>, String> {
    let sd_root = std::path::Path::new(sd_path);
    let stock = bios_stock_guard_names();
    let mut out = Vec::new();
    for b in bios {
        let src = std::path::Path::new(&b.source);
        if !src.is_file() {
            return Err(format!("BIOS source file not found: {}", b.source));
        }
        let filename = src
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if filename.is_empty() {
            return Err(format!("BIOS source has no file name: {}", b.source));
        }
        // The client-supplied destination is used ONLY as a folder hint; the
        // final file name always comes from the source. The hint must pass
        // canonical validation — a malformed or malicious hint is REJECTED
        // (observable error), never silently redirected.
        crate::paths::validate_relative_destination(&b.destination).map_err(|e| {
            format!(
                "invalid BIOS destination '{}' -> {}: {}",
                b.source, b.destination, e
            )
        })?;
        crate::paths::resolve_validated_destination(sd_root, &b.destination)
            .map_err(|e| format!("BIOS destination escapes SD root ({}): {}", b.source, e))?;
        let folder_hint = b
            .destination
            .rsplit_once('/')
            .map(|(dir, _)| dir.to_string())
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| "cubegm/bios".to_string());
        let dest_rel = format!("{}/{}", folder_hint, filename);
        // Canonical validation of the final destination relative path.
        crate::paths::validate_relative_destination(&dest_rel).map_err(|e| {
            format!(
                "invalid BIOS destination '{}' -> {}: {}",
                b.destination, dest_rel, e
            )
        })?;
        // Full resolution + containment against the SD root.
        crate::paths::resolve_validated_destination(sd_root, &dest_rel).map_err(|e| {
            format!(
                "BIOS destination escapes SD root ({} -> {}): {}",
                b.source, dest_rel, e
            )
        })?;
        let size = b
            .size
            .or_else(|| std::fs::metadata(src).ok().map(|m| m.len()))
            .unwrap_or(0);
        let dest_abs = sd_root.join(&dest_rel);
        let already_exists = dest_abs.exists();
        let is_stock = stock.contains(&filename.to_lowercase());
        let (action, reason) = if already_exists && is_stock && !force {
            (
                "skip".to_string(),
                "stock BIOS already exists on SD (kept, not overwritten)".to_string(),
            )
        } else if already_exists && !force {
            (
                "conflict".to_string(),
                "BIOS already exists on SD (resolve to replace/keep_both or enable force)"
                    .to_string(),
            )
        } else if already_exists {
            (
                "replace".to_string(),
                "BIOS exists on SD (forced overwrite)".to_string(),
            )
        } else {
            (
                "copy".to_string(),
                "BIOS (user-supplied) -> cubegm/bios".to_string(),
            )
        };
        out.push(PlanEntry {
            source: b.source.clone(),
            destination: dest_rel,
            action: action.clone(),
            reason: b.reason.clone().unwrap_or(reason),
            hash: crate::hash::sha256_file(src).ok(),
            source_hash: None,
            destination_hash: None,
            content_type: Some("bios".to_string()),
            kind: Some("bios".to_string()),
            possible_destinations: None,
            size: Some(size),
            group: None,
            members: None,
            default_action: Some(action.clone()),
            resolution: Some(action.clone()),
            resolved_action: Some(action.clone()),
            original_destination: None,
            preset: None,
            probe: None,
            converted_name: None,
        });
    }
    Ok(out)
}

/// Record a deployment job into the persistent SQLite store (minimal scope).
/// Returns the job id or an observable error.
fn record_deploy_job(
    plan: &Plan,
    result: &crate::deploy::DeployResult,
    sd_path: &str,
    target_stable_id: Option<&str>,
    profile_version: &str,
) -> anyhow::Result<i64> {
    let conn = crate::db::init_db()?;
    let mut entries = Vec::new();
    for e in &plan.entries {
        let eff = effective_action(e).to_string();
        let status = if result.errors.iter().any(|err| err.contains(&e.destination)) {
            "failed".to_string()
        } else if matches!(
            eff.as_str(),
            "copy" | "extract" | "convert_then_copy" | "replace"
        ) {
            "deployed".to_string()
        } else {
            "skipped".to_string()
        };
        entries.push((
            e.source.clone(),
            e.destination.clone(),
            eff,
            e.hash.clone().or_else(|| e.source_hash.clone()),
            e.size,
            status,
        ));
    }
    crate::db::record_deployment(
        &conn,
        "deploy",
        sd_path,
        target_stable_id,
        profile_version,
        &entries,
    )
}

#[tauri::command]
async fn deploy_to_sd(
    app: tauri::AppHandle,
    sd_path: String,
    force: Option<bool>,
    selected_files: Option<Vec<String>>,
    user_decisions: Option<std::collections::HashMap<String, String>>,
    bios_entries: Option<Vec<BiosPlanEntry>>,
    source_path: Option<String>,
    plan_entries: Option<Vec<PlanEntry>>,
) -> Result<serde_json::Value, String> {
    let profile = profile::load_profile().map_err(|e| e.to_string())?;
    let target_val = analyze_target_cached(&sd_path)?;
    let target: sd_target::TargetAnalysis = serde_json::from_value(target_val).unwrap();
    if target.status == "inaccessible" {
        return Err(format!("Target inaccessible: {}", sd_path));
    }
    if !target.is_treefrog {
        return Err(format!(
            "Target is not a valid TreeFrogUI SD (status: {}): {}",
            target.status, sd_path
        ));
    }

    let force = force.unwrap_or(false);
    let sd_root = std::path::Path::new(&sd_path);

    // BIOS is deployed through the SAME canonical pipeline as all content:
    // converted to PlanEntry objects, validated with the canonical destination
    // model, space-checked, and written by the ONE safe writer in deploy.rs.
    // There is no parallel BIOS write path anymore.
    let bios_plan_entries = match &bios_entries {
        Some(bios) if !bios.is_empty() => bios_entries_to_plan_entries(bios, &sd_path, force)?,
        _ => Vec::new(),
    };
    let has_bios = !bios_plan_entries.is_empty();

    // Panel-provided plan entries (exact user selection from Music/Videos/LGPT
    // previews). Deploying THIS plan — not a re-scan — is what makes the
    // preview the single source of truth: what the user saw is what gets
    // written. All entries still pass the SAME canonical validation.
    let panel_entries = plan_entries.unwrap_or_default();

    if source_path.is_none() {
        // Panel-plan and/or BIOS-only sync (full canonical pipeline below).
        if panel_entries.is_empty() && !has_bios {
            return Err("No files to sync".to_string());
        }
        if target.volume.removable != Some(true) && !force {
            return Err(format!(
                "REFUSED: {} is not a removable drive. Connect the SD and select it in Overview. Enable 'Force copy' in SD Card only if your reader reports the SD as a fixed drive.",
                sd_path
            ));
        }
        let mut plan = crate::Plan {
            summary: crate::PlanSummary::default(),
            entries: panel_entries.clone(),
            warnings: Vec::new(),
        };
        if has_bios {
            let mut merged = plan.entries.clone();
            merged.extend(bios_plan_entries.clone());
            plan.entries = merged;
        }
        // User decisions apply to plan entries too (conflict resolution)
        // — resolved by the BACKEND planner (single source of truth), with
        // collision-safe keep_both against the real SD root.
        if let Some(overrides) = &user_decisions {
            plan = planner::apply_resolutions_ctx(plan, overrides, &sd_path);
        }
        // Canonical validation of every destination against the SD root.
        for e in &plan.entries {
            crate::paths::resolve_validated_destination(sd_root, &e.destination)
                .map_err(|err| format!("invalid destination {}: {}", e.destination, err))?;
        }
        let space = sd_target::calculate_space(&plan, target.free_bytes);
        if space.status == "insufficient_space" {
            return Err(format!(
                "Insufficient space: required {} available {}",
                space.required_bytes,
                space.available_bytes.unwrap_or(0)
            ));
        }
        let result = crate::deploy::deploy_plan(&plan, &sd_path, &profile, force, Some(&app))
            .map_err(|e| e.to_string())?;
        // Persistent deployment record.
        let mut result = result;
        if let Err(e) = record_deploy_job(
            &plan,
            &result,
            &sd_path,
            target.stable_id.as_deref(),
            &profile.profile_version,
        ) {
            result
                .warnings
                .push(format!("deployment record failed (non-fatal): {}", e));
        }
        let mut out = serde_json::to_value(&result).unwrap();
        out["target"] = serde_json::to_value(&target).unwrap();
        out["space"] = serde_json::to_value(&space).unwrap();
        out["plan"] = serde_json::to_value(&plan).unwrap();
        out["bios_deployed"] = serde_json::json!(result.deployed);
        out["bios_skipped"] = serde_json::json!(result.skipped);
        out["bios_failed"] = serde_json::json!(result.failed);
        return Ok(out);
    }

    if target.volume.removable != Some(true) && !force {
        return Err(format!(
            "REFUSED: {} is not a removable drive. Connect the SD and select it in Overview. Enable 'Force copy' in SD Card only if your reader reports the SD as a fixed drive.",
            sd_path
        ));
    }

    let source_path_str = source_path.unwrap();

    // If the frontend provided panel plan entries, deploy THEM (the exact
    // selection the user previewed). Otherwise scan the source folder and
    // build the plan in the backend (legacy Games flow).
    let mut plan = if !panel_entries.is_empty() {
        crate::Plan {
            summary: crate::PlanSummary::default(),
            entries: panel_entries.clone(),
            warnings: Vec::new(),
        }
    } else {
        let scanned = scanner::scan(&source_path_str, &profile).map_err(|e| e.to_string())?;
        if let Some(ref files) = selected_files {
            planner::plan_with_selection(scanned, &sd_path, &profile, Some(files.clone()))
                .map_err(|e| e.to_string())?
        } else {
            planner::plan(scanned, &sd_path, &profile).map_err(|e| e.to_string())?
        }
    };
    if has_bios {
        let mut merged = plan.entries.clone();
        merged.extend(bios_plan_entries.clone());
        plan.entries = merged;
    }
    if let Some(overrides) = &user_decisions {
        for entry in plan.entries.iter_mut() {
            let src_base = entry
                .source
                .split("::")
                .next()
                .unwrap_or(&entry.source)
                .to_string();
            if let Some(new_folder) = overrides
                .get(&src_base)
                .or_else(|| overrides.get(&entry.source))
            {
                let file_name = std::path::Path::new(&entry.destination)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let new_dest = format!("{}/{}", new_folder.trim_end_matches('/'), file_name);
                // User overrides MUST pass the canonical destination validator
                // before they are applied - never trust a raw frontend string.
                crate::paths::validate_relative_destination(&new_dest).map_err(|e| {
                    format!(
                        "invalid user destination override '{}' -> {}: {}",
                        new_folder, new_dest, e
                    )
                })?;
                crate::paths::resolve_validated_destination(sd_root, &new_dest)
                    .map_err(|e| format!("user override escapes SD root ({}): {}", new_dest, e))?;
                tracing::info!(
                    "Override aplicado: {} -> {} (antes {})",
                    src_base,
                    new_dest,
                    entry.destination
                );
                entry.destination = new_dest;
            }
        }
    }
    // BIOS entries join the SAME plan and pass through the same validation.
    if has_bios {
        let mut merged = plan.entries.clone();
        merged.extend(bios_plan_entries.clone());
        plan.entries = merged;
    }
    let plan = planner::resolve_write_collisions(plan);
    // Canonical destination validation: the exact relative strings that will
    // be written, resolved against the SD root.
    for e in &plan.entries {
        crate::paths::resolve_validated_destination(sd_root, &e.destination)
            .map_err(|err| format!("invalid destination {}: {}", e.destination, err))?;
    }
    let space = sd_target::calculate_space(&plan, target.free_bytes);
    if space.status == "insufficient_space" {
        return Err(format!(
            "Insufficient space: required {} available {}",
            space.required_bytes,
            space.available_bytes.unwrap_or(0)
        ));
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
        // Never abort: deploy.rs has a runtime double-write guard as last resort.
        log::warn!(
            "Leftover write collisions (deploy guard will skip them): {:?}",
            collisions
        );
    }
    let mut result = crate::deploy::deploy_plan(&plan, &sd_path, &profile, force, Some(&app))
        .map_err(|e| e.to_string())?;
    if has_bios {
        result.warnings.push(format!(
            "BIOS: {} entries included in canonical plan",
            bios_plan_entries.len()
        ));
    }
    // Persistent deployment record (minimal scope): hash/size/versions/target.
    // Failures here are observable warnings, never silent.
    if let Err(e) = record_deploy_job(
        &plan,
        &result,
        &sd_path,
        target.stable_id.as_deref(),
        &profile.profile_version,
    ) {
        result
            .warnings
            .push(format!("deployment record failed (non-fatal): {}", e));
    }
    let mut out = serde_json::to_value(&result).unwrap();
    out["target"] = serde_json::to_value(&target).unwrap();
    out["space"] = serde_json::to_value(&space).unwrap();
    out["plan"] = serde_json::to_value(&plan).unwrap();
    out["bios_deployed"] = serde_json::json!(bios_plan_entries
        .iter()
        .filter(|e| matches!(e.action.as_str(), "copy" | "replace"))
        .count());
    out["bios_skipped"] = serde_json::json!(bios_plan_entries
        .iter()
        .filter(|e| e.action.starts_with("skip"))
        .count());
    out["bios_failed"] = serde_json::json!(0usize);
    Ok(out)
}

#[tauri::command]
async fn lgpt_scan_samples(
    samples_source: String,
    sd_path: String,
) -> Result<serde_json::Value, String> {
    let profile = profile::load_profile().map_err(|e| e.to_string())?;
    let scanned = scanner::scan(&samples_source, &profile).map_err(|e| e.to_string())?;
    // Forzar contexto LGPT samples: WAV/sonidos -> lgpt/samples/
    let scanned: Vec<scanner::ScannedFile> = scanned
        .into_iter()
        .map(|mut sf| {
            if sf.classification.kind == crate::classify::Kind::Music
                || sf.classification.kind == crate::classify::Kind::Unknown
            {
                sf.classification.kind = crate::classify::Kind::LgptSample;
                sf.classification.destination = "lgpt/samples".to_string();
            }
            sf
        })
        .collect();
    let plan = planner::plan(scanned, &sd_path, &profile).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "samples": [], "projects": [], "plan": plan }))
}

#[tauri::command]
async fn lgpt_scan_projects(
    projects_source: String,
    sd_path: String,
) -> Result<serde_json::Value, String> {
    let profile = profile::load_profile().map_err(|e| e.to_string())?;
    let scanned = scanner::scan(&projects_source, &profile).map_err(|e| e.to_string())?;
    let scanned: Vec<scanner::ScannedFile> = scanned
        .into_iter()
        .map(|mut sf| {
            sf.classification.kind = crate::classify::Kind::LgptProject;
            sf.classification.destination = "lgpt/projects".to_string();
            sf
        })
        .collect();
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

/// SINGLE source of truth for the application version: the Cargo package
/// version (which must match package.json and tauri.conf.json — enforced by CI).
/// The frontend must always display THIS version, never a hardcoded string.
#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Semantic content counts for the Overview screen, derived from the SAME
/// scanner/classification pipeline as the planner (never from UI heuristics
/// like media_dirs.length or existing_count).
#[tauri::command]
fn get_content_counts(sd_path: String) -> Result<serde_json::Value, String> {
    let profile = profile::load_profile().map_err(|e| e.to_string())?;
    let scanned = scanner::scan_directory(&sd_path, &profile, true).map_err(|e| e.to_string())?;
    let mut rom_count = 0usize;
    let mut music_track_count = 0usize;
    let mut video_count = 0usize;
    let mut image_count = 0usize;
    let mut ebook_count = 0usize;
    let mut bios_count = 0usize;
    let mut lgpt_sample_count = 0usize;
    let mut lgpt_project_count = 0usize;
    for sf in &scanned {
        match sf.classification.kind {
            crate::classify::Kind::Rom => rom_count += 1,
            crate::classify::Kind::Music => music_track_count += 1,
            crate::classify::Kind::Video => video_count += 1,
            crate::classify::Kind::Image => image_count += 1,
            crate::classify::Kind::Ebook => ebook_count += 1,
            crate::classify::Kind::Bios => bios_count += 1,
            crate::classify::Kind::LgptSample => lgpt_sample_count += 1,
            crate::classify::Kind::LgptProject => lgpt_project_count += 1,
            _ => {}
        }
    }
    Ok(serde_json::json!({
        "rom_count": rom_count,
        "music_track_count": music_track_count,
        "video_count": video_count,
        "image_count": image_count,
        "ebook_count": ebook_count,
        "bios_count": bios_count,
        "lgpt_sample_count": lgpt_sample_count,
        "lgpt_project_count": lgpt_project_count,
        "total_files": scanned.len()
    }))
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicTrack {
    pub path: String,
    pub filename: String,
    pub size: u64,
    pub folder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicPlaylist {
    pub name: String,
    pub path: String,
    pub tracks: Vec<MusicTrack>,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicScanResult {
    pub standalone_tracks: Vec<MusicTrack>,
    pub playlists: Vec<MusicPlaylist>,
    pub total_tracks: usize,
    pub total_playlists: usize,
}

#[tauri::command]
async fn scan_music_structured(path: String) -> Result<MusicScanResult, String> {
    let root = std::path::Path::new(&path);
    if !root.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let audio_extensions = [".mp3", ".flac", ".ogg", ".wav", ".m4a", ".aac", ".opus"];

    let mut standalone_tracks = Vec::new();
    let mut playlist_map: std::collections::HashMap<String, MusicPlaylist> =
        std::collections::HashMap::new();

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let ext = entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e.to_lowercase()))
            .unwrap_or_default();

        if !audio_extensions.contains(&ext.as_str()) {
            continue;
        }

        let file_path = entry.path();
        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);

        let parent = file_path.parent().unwrap_or(root);
        let relative_parent = parent
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let track = MusicTrack {
            path: file_path.to_string_lossy().to_string(),
            filename: filename.clone(),
            size,
            folder: relative_parent.clone(),
        };

        if relative_parent.is_empty() {
            standalone_tracks.push(track);
        } else {
            let playlist_name = relative_parent
                .split(std::path::MAIN_SEPARATOR)
                .next()
                .unwrap_or(&relative_parent)
                .to_string();

            let playlist = playlist_map
                .entry(playlist_name.clone())
                .or_insert_with(|| MusicPlaylist {
                    name: playlist_name.clone(),
                    path: parent.to_string_lossy().to_string(),
                    tracks: Vec::new(),
                    total_size: 0,
                });

            playlist.tracks.push(track);
            playlist.total_size += size;
        }
    }

    let mut playlists: Vec<MusicPlaylist> = playlist_map.into_values().collect();
    for p in &mut playlists {
        p.tracks.sort_by(|a, b| a.filename.cmp(&b.filename));
    }
    playlists.sort_by(|a, b| a.name.cmp(&b.name));
    standalone_tracks.sort_by(|a, b| a.filename.cmp(&b.filename));

    let total_tracks =
        standalone_tracks.len() + playlists.iter().map(|p| p.tracks.len()).sum::<usize>();
    let total_playlists = playlists.len();

    Ok(MusicScanResult {
        standalone_tracks,
        playlists,
        total_tracks,
        total_playlists,
    })
}

#[tauri::command]
async fn get_valid_systems_for_extension(ext: String) -> Result<Vec<SystemOption>, String> {
    let profile = crate::profile::load_profile().map_err(|e| e.to_string())?;
    let mut systems = Vec::new();
    let ext_lower = if ext.starts_with('.') {
        ext.to_lowercase()
    } else {
        format!(".{}", ext.to_lowercase())
    };
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
    // ONE BIOS validation model: the declarative bios.json definitions,
    // evaluated by bios.rs (filename/alias/wildcard/size/hash/variant rules).
    // The UI catalog is only a projection — validation never reimplements rules.
    let json = bios_profile_json();
    let defs = json
        .get("bios_definitions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let def = defs
        .iter()
        .find(|d| d.get("id").and_then(|x| x.as_str()) == Some(bios_id.as_str()))
        .cloned()
        .ok_or_else(|| format!("BIOS id not found: {}", bios_id))?;

    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Ok(serde_json::json!({ "valid": false, "reason": "File not found" }));
    }

    let validation = crate::bios::validate_bios_file(p, &def);
    let valid = matches!(validation.state, crate::bios::BiosState::FoundValid);
    Ok(serde_json::json!({
        "valid": valid,
        "reason": validation.reason,
        "state": validation.state.to_string(),
        "hash": validation.hash,
        "size": validation.size
    }))
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
        return Err(format!(
            "No es una SD TreeFrog válida (status: {}): {}",
            target.status, sd_path
        ));
    }
    let profile = crate::profile::load_profile().map_err(|e| e.to_string())?;

    // Stock BIOS guard derived from bios.json (single model) — never hardcoded.
    let stock_bios_files: Vec<String> = bios_stock_guard_names();

    let mut valid_extensions: std::collections::HashSet<String> = std::collections::HashSet::new();

    for sys in &profile.systems {
        for ext in &sys.extensions {
            valid_extensions.insert(ext.to_lowercase());
        }
    }

    valid_extensions.extend(
        [
            ".mp3", ".flac", ".ogg", ".wav", ".m4a", ".aac", ".opus", ".mp4", ".mkv", ".avi",
            ".mov", ".wmv", ".webm", ".jpg", ".jpeg", ".png", ".bmp", ".gif", ".webp", ".tiff",
        ]
        .iter()
        .map(|s| s.to_string()),
    );

    let mut files_to_process = Vec::new();

    if delete_all {
        for dir in ["roms", "lgpt"] {
            let dir_path = sd.join(dir);
            if dir_path.exists() {
                for entry in walkdir::WalkDir::new(&dir_path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        let file_name = entry.file_name().to_string_lossy().to_lowercase();
                        let is_stock_bios = stock_bios_files.iter().any(|s| s == &file_name);
                        if is_stock_bios {
                            tracing::info!("Preserved (stock BIOS): {}", entry.path().display());
                            continue;
                        }
                        let ext = entry
                            .path()
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| format!(".{}", e.to_lowercase()))
                            .unwrap_or_default();

                        let is_bios = entry.path().to_string_lossy().contains("cubegm/bios");
                        let is_valid = is_bios || valid_extensions.contains(&ext);

                        if is_valid {
                            files_to_process.push(entry.path().to_path_buf());
                        } else {
                            tracing::info!(
                                "Preserved (invalid extension): {}",
                                entry.path().display()
                            );
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
                let file_name = file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let is_stock_bios = stock_bios_files.iter().any(|s| s == &file_name);
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
                let lower_name = file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let is_doc = lower_name == "readme"
                    || lower_name == "readme.txt"
                    || lower_name.ends_with(".txt") && !valid_extensions.contains(&ext)
                    || lower_name.ends_with(".md");
                if is_doc && !is_bios && !valid_extensions.contains(&ext) {
                    tracing::info!("Preserved (documentation): {}", file_path.display());
                    continue;
                }
                let is_valid = is_bios
                    || valid_extensions.contains(&ext)
                    || file_rel.starts_with("roms/")
                    || file_rel.starts_with("roms\\");
                if is_valid || file_path.to_string_lossy().contains("roms/") {
                    files_to_process.push(file_path);
                } else {
                    tracing::info!("Preserved: {}", file_path.display());
                }
            } else if file_path.is_dir() {
                for entry in walkdir::WalkDir::new(&file_path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        let file_name = entry.file_name().to_string_lossy().to_lowercase();
                        let is_stock_bios = stock_bios_files.iter().any(|s| s == &file_name);
                        if is_stock_bios {
                            tracing::info!("Preserved (stock BIOS): {}", entry.path().display());
                            continue;
                        }
                        let ext = entry
                            .path()
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| format!(".{}", e.to_lowercase()))
                            .unwrap_or_default();
                        let is_bios = entry.path().to_string_lossy().contains("cubegm/bios");
                        let is_valid = is_bios || valid_extensions.contains(&ext);
                        if is_valid {
                            files_to_process.push(entry.path().to_path_buf());
                        } else {
                            tracing::info!(
                                "Preserved (invalid extension): {}",
                                entry.path().display()
                            );
                        }
                    }
                }
            }
        }
    }

    let total_files = files_to_process.len();
    if total_files == 0 {
        let _ = app.emit(
            "delete-progress",
            serde_json::json!({
                "current": 0,
                "total": 0,
                "percentage": 100,
                "current_file": "",
                "message": "Nothing to delete",
                "isDeleting": false
            }),
        );
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
        let normalized = file_path
            .to_string_lossy()
            .to_lowercase()
            .replace('\\', "/");
        if normalized.contains("cubegm/bios") {
            tracing::warn!(
                "Skipping BIOS file from deletion (protected): {}",
                file_path.display()
            );
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

    let _ = app.emit(
        "delete-progress",
        serde_json::json!({
            "current": total_files,
            "total": total_files,
            "percentage": 100,
            "current_file": "",
            "message": "Deletion complete",
            "isDeleting": false
        }),
    );

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
    if !folder.exists() {
        return Ok(vec![]);
    }

    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(&folder)
        .into_iter()
        .filter_map(|e| e.ok())
    {
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
    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {}", e))?;
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
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to download: {}", e))?;
    if !response.status().is_success() {
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read bytes: {}", e))?;
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
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    Ok(())
}

#[cfg(test)]
mod bios_security_tests {
    use super::*;

    fn mk_bios_entry(source: &str, destination: &str) -> BiosPlanEntry {
        BiosPlanEntry {
            source: source.to_string(),
            destination: destination.to_string(),
            action: "copy".to_string(),
            reason: None,
            content_type: Some("bios".to_string()),
            is_bios: Some(true),
            size: None,
        }
    }

    /// Regression: malicious BIOS destinations must NEVER escape the SD root.
    /// Covers: ../ traversal, nested traversal, prefix traversal, drive-letter
    /// paths, UNC paths, absolute unix paths, ADS syntax.
    #[test]
    fn bios_destination_escape_rejected() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd_root = tmp.path().join("sd");
        std::fs::create_dir_all(sd_root.join("cubegm/bios")).unwrap();
        std::fs::create_dir_all(sd_root.join("roms")).unwrap();
        let src = tmp.path().join("bios_src");
        std::fs::create_dir_all(&src).unwrap();
        let bios_file = src.join("scph1001.bin");
        std::fs::write(&bios_file, b"fake bios").unwrap();

        let malicious = [
            "../evil.bin",
            "../../evil.bin",
            "cubegm/bios/../../evil.bin",
            "C:\\evil.bin",
            "\\\\server\\share\\evil.bin",
            "/evil.bin",
            "roms/../../evil.bin",
            "file:ads.bin",
            "CON",
        ];
        for dest in malicious {
            let entry = mk_bios_entry(bios_file.to_string_lossy().as_ref(), dest);
            let result =
                bios_entries_to_plan_entries(&[entry], sd_root.to_string_lossy().as_ref(), false);
            assert!(
                result.is_err(),
                "malicious BIOS destination must be rejected: {dest}"
            );
        }
    }

    /// BIOS entries that pass validation must land exactly under the validated
    /// folder hint inside the SD root; a smuggled escape via folder hint is
    /// rejected with an observable error (never silently redirected).
    #[test]
    fn bios_destination_valid_normalized_to_profile_root() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd_root = tmp.path().join("sd");
        std::fs::create_dir_all(sd_root.join("cubegm/bios")).unwrap();
        std::fs::create_dir_all(sd_root.join("roms")).unwrap();
        let src = tmp.path().join("bios_src");
        std::fs::create_dir_all(&src).unwrap();
        let bios_file = src.join("scph1001.bin");
        std::fs::write(&bios_file, b"fake bios").unwrap();

        // Smuggled escape via folder hint must be REJECTED (observable).
        let entry = mk_bios_entry(bios_file.to_string_lossy().as_ref(), "../evil/scph1001.bin");
        let result =
            bios_entries_to_plan_entries(&[entry], sd_root.to_string_lossy().as_ref(), false);
        assert!(
            result.is_err(),
            "smuggled BIOS folder hint must be rejected"
        );

        // A valid hint passes through and the file name comes from the source.
        let entry = mk_bios_entry(
            bios_file.to_string_lossy().as_ref(),
            "cubegm/bios/whatever.txt",
        );
        let entries =
            bios_entries_to_plan_entries(&[entry], sd_root.to_string_lossy().as_ref(), false)
                .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].destination, "cubegm/bios/scph1001.bin");
        assert_eq!(entries[0].content_type.as_deref(), Some("bios"));
        // Resolves inside root
        let resolved =
            crate::paths::resolve_validated_destination(&sd_root, &entries[0].destination).unwrap();
        let canon_root = sd_root.canonicalize().unwrap();
        assert!(resolved.starts_with(&canon_root));
    }

    /// Stock BIOS guard: existing stock BIOS on the SD must be planned as skip
    /// (not overwrite) unless force is enabled.
    #[test]
    fn bios_stock_guard_skips_existing_stock_bios() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd_root = tmp.path().join("sd");
        std::fs::create_dir_all(sd_root.join("cubegm/bios")).unwrap();
        std::fs::create_dir_all(sd_root.join("roms")).unwrap();
        // Existing stock BIOS on SD
        std::fs::write(sd_root.join("cubegm/bios/scph1001.bin"), b"existing stock").unwrap();
        let src = tmp.path().join("bios_src");
        std::fs::create_dir_all(&src).unwrap();
        let bios_file = src.join("scph1001.bin");
        std::fs::write(&bios_file, b"replacement bios").unwrap();

        let entry = mk_bios_entry(
            bios_file.to_string_lossy().as_ref(),
            "cubegm/bios/scph1001.bin",
        );
        let entries =
            bios_entries_to_plan_entries(&[entry], sd_root.to_string_lossy().as_ref(), false)
                .unwrap();
        assert_eq!(
            entries[0].action, "skip",
            "existing stock BIOS must be skip (guard)"
        );

        // With force it becomes replace
        let entry = mk_bios_entry(
            bios_file.to_string_lossy().as_ref(),
            "cubegm/bios/scph1001.bin",
        );
        let entries =
            bios_entries_to_plan_entries(&[entry], sd_root.to_string_lossy().as_ref(), true)
                .unwrap();
        assert_eq!(
            entries[0].action, "replace",
            "force must allow stock BIOS replace"
        );
    }

    /// Non-stock BIOS already present (different content) becomes a conflict
    /// that flows through the normal resolution model.
    #[test]
    fn bios_existing_non_stock_conflict() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd_root = tmp.path().join("sd");
        std::fs::create_dir_all(sd_root.join("cubegm/bios")).unwrap();
        std::fs::create_dir_all(sd_root.join("roms")).unwrap();
        std::fs::write(sd_root.join("cubegm/bios/custom_bios.bin"), b"existing").unwrap();
        let src = tmp.path().join("bios_src");
        std::fs::create_dir_all(&src).unwrap();
        let bios_file = src.join("custom_bios.bin");
        std::fs::write(&bios_file, b"other content").unwrap();

        let entry = mk_bios_entry(
            bios_file.to_string_lossy().as_ref(),
            "cubegm/bios/custom_bios.bin",
        );
        let entries =
            bios_entries_to_plan_entries(&[entry], sd_root.to_string_lossy().as_ref(), false)
                .unwrap();
        assert_eq!(entries[0].action, "conflict");
    }

    /// effective_action: resolved_action wins when present.
    #[test]
    fn effective_action_prefers_resolved() {
        let e = PlanEntry {
            action: "conflict".to_string(),
            resolved_action: Some("replace".to_string()),
            ..Default::default()
        };
        assert_eq!(effective_action(&e), "replace");
        let e2 = PlanEntry {
            action: "copy".to_string(),
            resolved_action: None,
            ..Default::default()
        };
        assert_eq!(effective_action(&e2), "copy");
    }
}

#[cfg(test)]
mod bios_workflow_integration_tests {
    use super::*;

    /// End-to-end BIOS lifecycle: user selection -> PlanEntry conversion ->
    /// canonical validation -> space check -> deploy -> on-disk result.
    /// BIOS uses the SAME pipeline as all content (no bypass).
    #[test]
    fn bios_full_lifecycle_through_canonical_pipeline() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd_root = tmp.path().join("sd");
        std::fs::create_dir_all(sd_root.join("cubegm/bios")).unwrap();
        std::fs::create_dir_all(sd_root.join("roms")).unwrap();
        let src = tmp.path().join("bios");
        std::fs::create_dir_all(&src).unwrap();
        let gba = src.join("gba_bios.bin");
        std::fs::write(&gba, b"gbabios").unwrap();
        let scph = src.join("scph1001.bin");
        std::fs::write(&scph, b"psxbios").unwrap();

        // 1. BIOS classify: bios.json model validates both files
        let defs = bios_profile_json()
            .get("bios_definitions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let gba_def = defs
            .iter()
            .find(|d| d.get("id").and_then(|x| x.as_str()) == Some("gba_bios"))
            .unwrap()
            .clone();
        let v = crate::bios::validate_bios_file(&gba, &gba_def);
        // size does not match bios.json -> not FoundValid, but state is observable
        assert_ne!(v.state, crate::bios::BiosState::Missing);

        // 2. Plan entries (canonical conversion; user "selects" both)
        let entries = bios_entries_to_plan_entries(
            &[
                BiosPlanEntry {
                    source: gba.to_string_lossy().to_string(),
                    destination: "cubegm/bios/gba_bios.bin".into(),
                    action: "copy".into(),
                    ..Default::default()
                },
                BiosPlanEntry {
                    source: scph.to_string_lossy().to_string(),
                    destination: "cubegm/bios/scph1001.bin".into(),
                    action: "copy".into(),
                    ..Default::default()
                },
            ],
            sd_root.to_string_lossy().as_ref(),
            false,
        )
        .unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries
            .iter()
            .all(|e| e.content_type.as_deref() == Some("bios")));

        // 3. Plan + space + deploy
        let profile = crate::profile::load_profile().unwrap();
        let plan = crate::Plan {
            summary: crate::PlanSummary::default(),
            entries,
            warnings: vec![],
        };
        let space = sd_target::calculate_space(&plan, Some(1_000_000));
        assert_eq!(
            space.status, "ok",
            "BIOS sizes must count as required space"
        );
        assert!(space.required_bytes > 0);
        let result = crate::deploy::deploy_plan(
            &plan,
            sd_root.to_string_lossy().as_ref(),
            &profile,
            false,
            None,
        )
        .unwrap();
        assert_eq!(
            result.deployed, 2,
            "both BIOS must deploy: {:?}",
            result.errors
        );
        assert!(sd_root.join("cubegm/bios/gba_bios.bin").exists());
        assert!(sd_root.join("cubegm/bios/scph1001.bin").exists());

        // 4. Re-running the same BIOS plan: stock guard keeps existing files
        let entries2 = bios_entries_to_plan_entries(
            &[
                BiosPlanEntry {
                    source: gba.to_string_lossy().to_string(),
                    destination: "cubegm/bios/gba_bios.bin".into(),
                    action: "copy".into(),
                    ..Default::default()
                },
                BiosPlanEntry {
                    source: scph.to_string_lossy().to_string(),
                    destination: "cubegm/bios/scph1001.bin".into(),
                    action: "copy".into(),
                    ..Default::default()
                },
            ],
            sd_root.to_string_lossy().as_ref(),
            false,
        )
        .unwrap();
        let plan2 = crate::Plan {
            summary: crate::PlanSummary::default(),
            entries: entries2,
            warnings: vec![],
        };
        let result2 = crate::deploy::deploy_plan(
            &plan2,
            sd_root.to_string_lossy().as_ref(),
            &profile,
            false,
            None,
        )
        .unwrap();
        assert_eq!(
            result2.deployed, 0,
            "existing stock BIOS must not be overwritten"
        );
        assert!(
            result2.skipped >= 2,
            "stock guard must skip: {:?}",
            result2.warnings
        );
    }

    /// BIOS conflict resolution flows through the SAME resolution model
    /// (backend apply_resolutions_ctx with collision-safe keep_both).
    #[test]
    fn bios_conflict_resolved_via_canonical_model() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd_root = tmp.path().join("sd");
        std::fs::create_dir_all(sd_root.join("cubegm/bios")).unwrap();
        std::fs::create_dir_all(sd_root.join("roms")).unwrap();
        // Existing different-content BIOS on the SD
        std::fs::write(sd_root.join("cubegm/bios/custom.bin"), b"existing").unwrap();
        let src = tmp.path().join("bios");
        std::fs::create_dir_all(&src).unwrap();
        let custom = src.join("custom.bin");
        std::fs::write(&custom, b"new content").unwrap();

        let entries = bios_entries_to_plan_entries(
            &[BiosPlanEntry {
                source: custom.to_string_lossy().to_string(),
                destination: "cubegm/bios/custom.bin".into(),
                action: "copy".into(),
                ..Default::default()
            }],
            sd_root.to_string_lossy().as_ref(),
            false,
        )
        .unwrap();
        assert_eq!(
            entries[0].action, "conflict",
            "existing non-stock BIOS is a conflict"
        );

        // User resolves keep_both -> backend renames collision-safely
        let mut plan = crate::Plan {
            summary: crate::PlanSummary::default(),
            entries,
            warnings: vec![],
        };
        let mut decisions = std::collections::HashMap::new();
        decisions.insert("0".to_string(), "keep_both".to_string());
        plan = planner::apply_resolutions_ctx(plan, &decisions, sd_root.to_string_lossy().as_ref());
        assert_eq!(plan.entries[0].destination, "cubegm/bios/custom_1.bin");

        let profile = crate::profile::load_profile().unwrap();
        let result = crate::deploy::deploy_plan(
            &plan,
            sd_root.to_string_lossy().as_ref(),
            &profile,
            false,
            None,
        )
        .unwrap();
        assert_eq!(
            result.deployed, 1,
            "keep_both BIOS must deploy: {:?}",
            result.errors
        );
        assert!(sd_root.join("cubegm/bios/custom_1.bin").exists());
        // Original kept
        assert_eq!(
            std::fs::read(sd_root.join("cubegm/bios/custom.bin")).unwrap(),
            b"existing"
        );
    }
}

#[cfg(test)]
mod per_tab_deploy_tests {
    use super::*;

    fn mk_treefrog_sd(tmp: &tempfile::TempDir) -> std::path::PathBuf {
        let sd_root = tmp.path().join("sd");
        std::fs::create_dir_all(sd_root.join("cubegm/bios")).unwrap();
        std::fs::create_dir_all(sd_root.join("roms")).unwrap();
        std::fs::create_dir_all(sd_root.join("lgpt/samples")).unwrap();
        std::fs::create_dir_all(sd_root.join("lgpt/projects")).unwrap();
        sd_root
    }

    fn simple_entry(source: &std::path::Path, destination: &str, content_type: &str) -> PlanEntry {
        let action = "copy".to_string();
        PlanEntry {
            source: source.to_string_lossy().to_string(),
            destination: destination.to_string(),
            action: action.clone(),
            reason: "panel selection".to_string(),
            hash: crate::hash::sha256_file(source).ok(),
            source_hash: None,
            destination_hash: None,
            content_type: Some(content_type.to_string()),
            kind: Some(content_type.to_string()),
            possible_destinations: None,
            size: Some(source.metadata().map(|m| m.len()).unwrap_or(0)),
            group: None,
            members: None,
            default_action: Some(action.clone()),
            resolution: Some(action.clone()),
            resolved_action: Some(action),
            original_destination: None,
            preset: None,
            probe: None,
            converted_name: None,
        }
    }

    fn deploy(plan: &crate::Plan, sd_root: &std::path::Path) -> crate::deploy::DeployResult {
        let profile = crate::profile::load_profile().unwrap();
        crate::deploy::deploy_plan(
            plan,
            sd_root.to_string_lossy().as_ref(),
            &profile,
            false,
            None,
        )
        .unwrap()
    }

    /// Regression (user report): EVERY content tab must deploy its files to the
    /// correct TreeFrogUI path. Music must deploy (it was silently skipped
    /// before because the panel never reported its source); music playlists
    /// (subfolders under roms/music) must preserve hierarchy per TreeFrogUI.
    #[test]
    fn every_tab_deploys_to_correct_treefrogui_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd_root = mk_treefrog_sd(&tmp);

        // Source tree with one file per content type
        let src = tmp.path().join("src");
        std::fs::create_dir_all(src.join("GBA")).unwrap();
        std::fs::create_dir_all(src.join("MyPlaylist")).unwrap();
        std::fs::create_dir_all(src.join("samples")).unwrap();
        std::fs::create_dir_all(src.join("projects")).unwrap();
        let rom = src.join("GBA/game.gba");
        std::fs::write(&rom, b"GBA").unwrap();
        let music_standalone = src.join("song.mp3");
        std::fs::write(&music_standalone, b"mp3").unwrap();
        let music_playlist = src.join("MyPlaylist/track1.mp3");
        std::fs::write(&music_playlist, b"mp3p").unwrap();
        let sample = src.join("samples/kick.wav");
        std::fs::write(&sample, b"WAV").unwrap();
        let project = src.join("projects/track.lgpt");
        std::fs::write(&project, b"LGPT").unwrap();
        let bios_file = src.join("gba_bios.bin");
        std::fs::write(&bios_file, b"BIOS").unwrap();

        // The combined panel plan (exactly what App.tsx now sends):
        // games -> roms/<SYSTEM>/, music -> roms/music[/playlist]/,
        // videos -> roms/videos/, lgpt -> lgpt/samples|projects/, BIOS separate.
        let entries = vec![
            simple_entry(&rom, "roms/GBA/game.gba", "rom/gba"),
            simple_entry(&music_standalone, "roms/music/song.mp3", "music"),
            simple_entry(&music_playlist, "roms/music/MyPlaylist/track1.mp3", "music"),
            simple_entry(&sample, "lgpt/samples/kick.wav", "lgpt/sample"),
            simple_entry(&project, "lgpt/projects/track.lgpt", "lgpt/project"),
        ];
        let plan = crate::Plan {
            summary: crate::PlanSummary::default(),
            entries,
            warnings: vec![],
        };
        let result = deploy(&plan, &sd_root);
        assert_eq!(
            result.deployed, 5,
            "ALL content tabs must deploy: {:?}",
            result.errors
        );

        // Exact TreeFrogUI paths on the SD
        assert!(
            sd_root.join("roms/GBA/game.gba").exists(),
            "Games -> roms/GBA/"
        );
        assert!(
            sd_root.join("roms/music/song.mp3").exists(),
            "Music standalone -> roms/music/"
        );
        assert!(
            sd_root.join("roms/music/MyPlaylist/track1.mp3").exists(),
            "Music playlist -> roms/music/<playlist>/ (TreeFrogUI playlist semantics)"
        );
        assert!(
            sd_root.join("lgpt/samples/kick.wav").exists(),
            "LGPT samples -> lgpt/samples/"
        );
        assert!(
            sd_root.join("lgpt/projects/track.lgpt").exists(),
            "LGPT projects -> lgpt/projects/"
        );

        // BIOS deploys through the SAME canonical pipeline
        let bios_entries = bios_entries_to_plan_entries(
            &[BiosPlanEntry {
                source: bios_file.to_string_lossy().to_string(),
                destination: "cubegm/bios/gba_bios.bin".into(),
                action: "copy".into(),
                ..Default::default()
            }],
            sd_root.to_string_lossy().as_ref(),
            false,
        )
        .unwrap();
        let bios_plan = crate::Plan {
            summary: crate::PlanSummary::default(),
            entries: bios_entries,
            warnings: vec![],
        };
        let bios_result = deploy(&bios_plan, &sd_root);
        assert_eq!(
            bios_result.deployed, 1,
            "BIOS must deploy: {:?}",
            bios_result.errors
        );
        assert!(
            sd_root.join("cubegm/bios/gba_bios.bin").exists(),
            "BIOS -> cubegm/bios/"
        );
    }

    /// Videos deploy to roms/videos/ with the REAL conversion pipeline when
    /// the preset demands it (compatible videos copy as-is).
    #[test]
    fn videos_tab_deploys_to_roms_videos() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd_root = mk_treefrog_sd(&tmp);
        let vid = tmp.path().join("clip.mp4");
        std::fs::write(&vid, b"not a real video").unwrap();

        // A video entry deploying to roms/videos/ (destination produced by the
        // planner/classifier for video content).
        let entry = PlanEntry {
            action: "copy".to_string(),
            resolved_action: Some("copy".to_string()),
            content_type: Some("video".to_string()),
            ..simple_entry(&vid, "roms/videos/clip.mp4", "video")
        };
        let plan = crate::Plan {
            summary: crate::PlanSummary::default(),
            entries: vec![entry],
            warnings: vec![],
        };
        let result = deploy(&plan, &sd_root);
        assert_eq!(result.deployed, 1, "video must deploy: {:?}", result.errors);
        assert!(
            sd_root.join("roms/videos/clip.mp4").exists(),
            "Videos -> roms/videos/"
        );
    }

    /// Unknown destinations are NEVER written (state machine invariant) — a
    /// panel cannot smuggle files into roms/UNKNOWN.
    #[test]
    fn unknown_destination_never_written() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd_root = mk_treefrog_sd(&tmp);
        let f = tmp.path().join("mystery.xyz");
        std::fs::write(&f, b"???").unwrap();
        let plan = crate::Plan {
            summary: crate::PlanSummary::default(),
            entries: vec![simple_entry(&f, "roms/UNKNOWN/mystery.xyz", "unknown")],
            warnings: vec![],
        };
        let result = deploy(&plan, &sd_root);
        assert_eq!(result.deployed, 0, "UNKNOWN must not deploy");
        assert_eq!(result.skipped, 1);
        assert!(!sd_root.join("roms/UNKNOWN/mystery.xyz").exists());
    }
}

#[cfg(test)]
mod portable_embed_tests {
    use super::*;

    /// Regression (user report: BIOS section empty in the portable exe):
    /// the BIOS catalog must NEVER be empty. When no bios.json exists on the
    /// file system (portable exe without a profiles/ folder), the EMBEDDED
    /// copy must provide the definitions — same portable contract as the
    /// systems profile (profile.rs include_str!).
    #[test]
    fn bios_catalog_never_empty_portable() {
        // Run from a working directory WITHOUT profiles/ next to it.
        let tmp = tempfile::TempDir::new().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();
        let json = bios_profile_json();
        let defs = json
            .get("bios_definitions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        std::env::set_current_dir(prev).unwrap();
        assert!(
            !defs.is_empty(),
            "portable exe must show BIOS entries (embedded bios.json fallback)"
        );
        // Known entries exist (ps1/gba/neogeo are TreeFrogUI stock BIOS)
        let ids: Vec<String> = defs
            .iter()
            .filter_map(|d| d.get("id").and_then(|x| x.as_str()).map(|s| s.to_string()))
            .collect();
        for expected in ["ps1_bios", "gba_bios", "neogeo_bios"] {
            assert!(
                ids.iter().any(|id| id == expected),
                "embedded bios.json must contain {expected}: {ids:?}"
            );
        }
        // The catalog (UI projection) is also non-empty
        let catalog = crate::bios_catalog::get_bios_catalog();
        assert!(!catalog.is_empty(), "BIOS catalog must not be empty");
    }
}
