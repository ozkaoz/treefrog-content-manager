use crate::profile::LoadedProfile;
use crate::scanner::ScannedFile;
use crate::hash;
use crate::archive;
use std::path::Path;
use std::collections::{HashMap, HashSet};

use crate::Plan;
use crate::PlanEntry;
use crate::PlanSummary;

fn group_members(entries: &[archive::ArchiveEntry]) -> Vec<Vec<archive::ArchiveEntry>> {
    let mut by_folder: HashMap<String, Vec<archive::ArchiveEntry>> = HashMap::new();
    for e in entries.iter().filter(|e| !e.is_dir) {
        let folder = Path::new(&e.name).parent().map(|p| p.to_string_lossy().to_string().replace('\\', "/")).unwrap_or_default();
        by_folder.entry(folder).or_default().push(e.clone());
    }
    let mut groups: Vec<Vec<archive::ArchiveEntry>> = Vec::new();
    let mut used: HashSet<String> = HashSet::new();
    for (_folder, ents) in by_folder.iter() {
        let cues: Vec<_> = ents.iter().filter(|e| Path::new(&e.name).extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase() == "cue").unwrap_or(false)).cloned().collect();
        for cue in &cues {
            if used.contains(&cue.name) { continue; }
            let stem = Path::new(&cue.name).file_stem().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            let mut siblings: Vec<archive::ArchiveEntry> = ents.iter().filter(|e| !used.contains(&e.name) && Path::new(&e.name).extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase() == "bin").unwrap_or(false) && Path::new(&e.name).file_stem().and_then(|s| s.to_str()).map(|s| s.to_lowercase() == stem).unwrap_or(false)).cloned().collect();
            if siblings.is_empty() && cues.len() == 1 {
                siblings = ents.iter().filter(|e| !used.contains(&e.name) && Path::new(&e.name).extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase() == "bin").unwrap_or(false)).cloned().collect();
            }
            if !siblings.is_empty() {
                let mut grp = vec![cue.clone()];
                grp.extend(siblings.clone());
                for m in &grp { used.insert(m.name.clone()); }
                groups.push(grp);
            } else {
                used.insert(cue.name.clone());
                groups.push(vec![cue.clone()]);
            }
        }
        for e in ents {
            if !used.contains(&e.name) {
                groups.push(vec![e.clone()]);
                used.insert(e.name.clone());
            }
        }
    }
    groups
}

fn decide_archive_mode(archive_path: &Path, inner: &[archive::ArchiveEntry], profile: &LoadedProfile) -> String {
    let ext = archive_path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e.to_lowercase())).unwrap_or_default();
    let policy = &profile.archive_policy_full;
    let handlers = policy.get("handlers").and_then(|v| v.as_object());
    if let Some(h) = handlers.and_then(|m| m.get(&ext)) {
        if h.get("implemented").and_then(|v| v.as_bool()) == Some(false) {
            return "unsupported".to_string();
        }
    }
    if inner.is_empty() {
        return "manual".to_string();
    }
    let has_nested = inner.iter().any(|e| !e.is_dir && {
        let ie = Path::new(&e.name).extension().and_then(|s| s.to_str()).map(|s| format!(".{}", s.to_lowercase())).unwrap_or_default();
        matches!(ie.as_str(), ".zip" | ".7z" | ".rar")
    });
    if has_nested {
        let max_depth = policy.get("safety").and_then(|v| v.get("limits")).and_then(|v| v.get("max_depth")).and_then(|v| v.as_u64()).unwrap_or(1);
        if max_depth <= 1 {
            return "manual".to_string();
        }
    }
    // Early grouped detection: CUE+BIN in same folder should be grouped regardless of system mixed
    {
        let mut by_folder: HashMap<String, Vec<&archive::ArchiveEntry>> = HashMap::new();
        for e in inner.iter().filter(|e| !e.is_dir) {
            let folder = Path::new(&e.name).parent().map(|p| p.to_string_lossy().to_string().replace('\\', "/")).unwrap_or_default();
            by_folder.entry(folder).or_default().push(e);
        }
        for (_folder, ents) in by_folder {
            let has_cue = ents.iter().any(|e| Path::new(&e.name).extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()=="cue").unwrap_or(false));
            let has_bin = ents.iter().any(|e| Path::new(&e.name).extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()=="bin").unwrap_or(false));
            if has_cue && has_bin {
                return "grouped".to_string();
            }
        }
    }
    let mut systems_for_members: Vec<Option<String>> = Vec::new();
    for e in inner.iter().filter(|e| !e.is_dir) {
        let inner_ext = Path::new(&e.name).extension().and_then(|s| s.to_str()).map(|s| format!(".{}", s.to_lowercase())).unwrap_or_default();
        if let Some(ids) = profile.ext_to_system.get(&inner_ext) {
            systems_for_members.push(Some(ids[0].clone()));
        } else if matches!(inner_ext.as_str(), ".cue" | ".bin" | ".chd" | ".m3u") {
            systems_for_members.push(Some("grouped_hint".to_string()));
        } else {
            systems_for_members.push(None);
        }
    }
    let known: Vec<String> = systems_for_members.iter().filter_map(|o| o.clone()).filter(|s| s != "grouped_hint").collect();
    if known.is_empty() {
        // Unknown inner extensions: still try to extract (members will be
        // classified individually; unknown ones go to roms/UNKNOWN for review).
        return "extract_and_classify".to_string();
    }
    let unique: HashSet<String> = known.iter().cloned().collect();
    if unique.len() == 1 {
        let sys_id = unique.iter().next().unwrap();
        // Arcade-class systems keep the archive as payload (MAME/FBA/NeoGeo need the zip intact).
        // Every other system: EXTRACT and classify each ROM into its correct folder.
        const PAYLOAD_SYSTEMS: &[&str] = &[
            "mame", "mame2000", "mame2003", "mame2010", "mame2016",
            "fbneo", "fba", "cps1", "cps2", "cps3", "neogeo", "arcade",
        ];
        let is_arcade = PAYLOAD_SYSTEMS.contains(&sys_id.as_str());
        if let Some(per) = policy.get("per_system").and_then(|v| v.as_object()).and_then(|m| m.get(sys_id)) {
            if let Some(mode) = per.get(&ext).and_then(|v| v.as_str()) {
                if mode == "payload" && !is_arcade {
                    return "extract_and_classify".to_string();
                }
                return mode.to_string();
            }
        }
        if is_arcade {
            return "payload".to_string();
        }
        if systems_for_members.iter().any(|s| s.as_deref() == Some("grouped_hint")) {
            return "grouped".to_string();
        }
        return "extract_and_classify".to_string();
    }
    // Mixed systems: still extract_and_classify each to its correct system folder
    if systems_for_members.iter().any(|s| s.as_deref() == Some("grouped_hint")) {
        return "grouped".to_string();
    }
    "extract_and_classify".to_string()
}

#[allow(dead_code)]
fn detect_collisions(dests: &[String]) -> Vec<(String,String)> {
    let mut seen: HashMap<String,String> = HashMap::new();
    let mut out = Vec::new();
    for d in dests {
        let norm = d.replace('\\', "/").to_lowercase();
        if let Some(prev) = seen.get(&norm) {
            out.push((d.clone(), prev.clone()));
        } else {
            seen.insert(norm, d.clone());
        }
    }
    out
}

fn content_type_for_classification(kind: &crate::classify::Kind, system_id: &Option<String>) -> String {
    match kind {
        crate::classify::Kind::Rom => format!("rom/{}", system_id.clone().unwrap_or("unknown".to_string())),
        crate::classify::Kind::Music => "music".to_string(),
        crate::classify::Kind::Video => "video".to_string(),
        crate::classify::Kind::Image => "image".to_string(),
        crate::classify::Kind::Ebook => "ebook".to_string(),
        crate::classify::Kind::Bios => "bios".to_string(),
        crate::classify::Kind::LgptSample => "lgpt/sample".to_string(),
        crate::classify::Kind::LgptProject => "lgpt/project".to_string(),
        crate::classify::Kind::Archive => "archive".to_string(),
        crate::classify::Kind::Unknown => "unknown".to_string(),
    }
}

#[allow(dead_code)]
const VALID_RESOLUTIONS: &[&str] = &["skip", "replace", "keep_both", "keep_destination", "keep_source"];

fn default_resolution_for_action(action: &str) -> String {
    match action {
        "skip_duplicate" => "skip".to_string(),
        "skip_unchanged" => "skip".to_string(),
        "conflict" => "conflict".to_string(),
        "manual_review" => "manual_review".to_string(),
        "unsupported_archive" => "skip".to_string(),
        "copy" | "extract" => "copy".to_string(),
        _ => "skip".to_string(),
    }
}

fn apply_single_resolution(entry: &crate::PlanEntry, resolution: &str) -> crate::PlanEntry {
    let mut resolved = entry.clone();
    resolved.resolution = Some(resolution.to_string());
    if resolved.default_action.is_none() {
        resolved.default_action = Some(entry.action.clone());
    }
    match resolution {
        "skip" => {
            resolved.resolved_action = Some("skip".to_string());
            resolved.reason = format!("{} [resolved: skip]", entry.reason);
        },
        "keep_destination" => {
            resolved.resolved_action = Some("skip".to_string());
            resolved.reason = format!("{} [resolved: keep_destination]", entry.reason);
        },
        "replace" | "keep_source" => {
            let ra = if entry.action == "skip_duplicate" { "copy".to_string() } else { "replace".to_string() };
            resolved.resolved_action = Some(ra);
            resolved.reason = format!("{} [resolved: {}]", entry.reason, resolution);
        },
        "keep_both" => {
            let dest = entry.destination.clone();
            let p = Path::new(&dest);
            let new_dest = if p.extension().is_some() && p.file_name().is_some() {
                let stem = p.file_stem().unwrap().to_string_lossy().to_string();
                let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
                let parent = p.parent().map(|pp| pp.to_string_lossy().to_string()).unwrap_or_default();
                if parent.is_empty() || parent == "." {
                    format!("{}_1{}", stem, ext)
                } else {
                    format!("{}/{}_1{}", parent, stem, ext)
                }
            } else {
                format!("{}_1", dest)
            };
            resolved.destination = new_dest.replace('\\', "/");
            resolved.original_destination = Some(dest);
            resolved.resolved_action = Some(if entry.action == "extract" { "extract".to_string() } else { "copy".to_string() });
            resolved.reason = format!("{} [resolved: keep_both -> renamed]", entry.reason);
        },
        _ => {
            resolved.resolved_action = Some(entry.action.clone());
        }
    }
    resolved
}

pub fn apply_resolutions(plan: crate::Plan, decisions: &std::collections::HashMap<String, String>) -> crate::Plan {
    let mut new_entries = Vec::new();
    for (idx, entry) in plan.entries.iter().enumerate() {
        let key_idx = idx.to_string();
        let key_src = entry.source.clone();
        let key_dst = entry.destination.clone();
        let key_combined = format!("{}->{}", entry.source, entry.destination);
        let resolution = decisions.get(&key_idx)
            .or_else(|| decisions.get(&key_src))
            .or_else(|| decisions.get(&key_dst))
            .or_else(|| decisions.get(&key_combined))
            .or_else(|| decisions.get(&idx.to_string()));
        if let Some(res) = resolution {
            new_entries.push(apply_single_resolution(entry, res));
        } else {
            let mut e = entry.clone();
            if e.resolved_action.is_none() {
                e.resolved_action = Some(e.action.clone());
                e.resolution = Some(default_resolution_for_action(&e.action));
                if e.default_action.is_none() {
                    e.default_action = Some(e.action.clone());
                }
            }
            new_entries.push(e);
        }
    }
    crate::Plan { summary: plan.summary.clone(), entries: new_entries, warnings: plan.warnings.clone() }
}

#[allow(unused_assignments, unused_variables)]
pub fn plan(scanned: Vec<ScannedFile>, sd_root: &str, profile: &LoadedProfile) -> anyhow::Result<Plan> {
    let sd_path = Path::new(sd_root);
    let mut entries: Vec<PlanEntry> = Vec::new();
    let mut unchanged = 0usize;
    let mut new_c = 0usize;
    let mut changed = 0usize;
    let mut duplicate = 0usize;
    let mut conflicts = 0usize;
    let mut manual = 0usize;
    let mut unsupported = 0usize;
    let mut hash_to_dest: HashMap<String, String> = HashMap::new();

    let mut job_seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Deterministic order
    let mut scanned_sorted = scanned;
    scanned_sorted.sort_by(|a,b| a.source_path.cmp(&b.source_path));

    // Limits
    let limits = archive::Limits::default();
    let max_total = limits.max_total_files_per_job as usize;
    let mut total_planned = 0usize;

    for sf in scanned_sorted {
        let (dest_rel, _hint, reason) = resolve_destination(&sf, profile, sd_path)?;
        let dest_abs = sd_path.join(&dest_rel);

        if sf.classification.kind == crate::classify::Kind::Archive {
            let ext = sf.source_path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e.to_lowercase())).unwrap_or_default();
            // Check handler first via archive abstraction
            let handler = archive::get_handler_for_ext(&ext);
            if handler.is_none() || (ext == ".7z" || ext == ".rar") {
                // 7z/rar are stubbed as unsupported
                if matches!(ext.as_str(), ".7z" | ".rar") {
                    let dest2 = if sf.classification.destination.is_empty() { format!("roms/UNKNOWN/{}", sf.source_path.file_name().unwrap().to_string_lossy()) } else { format!("{}/{}", sf.classification.destination, sf.source_path.file_name().unwrap().to_string_lossy()) };
                    entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dest2, action: "unsupported_archive".into(), reason: format!("archive handler not available for {} (stub)", ext), hash: None, size: Some(sf.size), group: None, ..Default::default()});
                    unsupported += 1;
                    continue;
                }
            }

            let inspected = archive::inspect_archive(&sf.source_path, &limits);
            let inner = match inspected {
                Ok(v) => v,
                Err(e) => {
                    let msg = e.to_string();
                    let action = match &e {
                        archive::ArchiveError::Unsupported(_) => { unsupported += 1; "unsupported_archive" },
                        _ => { manual += 1; "manual_review" },
                    };
                    let dr = if sf.classification.destination.is_empty() { format!("roms/UNKNOWN/{}", sf.source_path.file_name().unwrap().to_string_lossy()) } else { format!("{}/{}", sf.classification.destination, sf.source_path.file_name().unwrap().to_string_lossy()) };
                    entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dr, action: action.into(), reason: format!("archive error: {}", msg), hash: None, size: Some(sf.size), group: None, ..Default::default()});
                    if action == "unsupported_archive" { /* already counted */ } else if action == "manual_review" { /* already */ }
                    continue;
                }
            };

            let inner_file_count = inner.iter().filter(|e| !e.is_dir).count();
            if total_planned + inner_file_count > max_total {
                let dr = format!("{}/{}", sf.classification.destination, sf.source_path.file_name().unwrap().to_string_lossy());
                entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dr, action: "manual_review".into(), reason: format!("exceeds max_total_files_per_job {} (bomb guard)", max_total), hash: None, size: Some(sf.size), group: None, ..Default::default()});
                manual += 1;
                continue;
            }

            let mode = decide_archive_mode(&sf.source_path, &inner, profile);
            if mode == "unsupported" {
                let dr = format!("{}/{}", sf.classification.destination, sf.source_path.file_name().unwrap().to_string_lossy());
                entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dr, action: "unsupported_archive".into(), reason: format!("mode unsupported for {}", ext), hash: None, size: Some(sf.size), group: None, ..Default::default()});
                unsupported += 1;
                continue;
            }
            if mode == "payload" {
                let dest_rel2 = if sf.classification.destination.is_empty() { format!("roms/UNKNOWN/{}", sf.source_path.file_name().unwrap().to_string_lossy()) } else { format!("{}/{}", sf.classification.destination, sf.source_path.file_name().unwrap().to_string_lossy()) };
                let dest_abs2 = sd_path.join(&dest_rel2);
                let exists = dest_abs2.exists();
                let mut src_hash = None;
                let mut same_hash = false;
                if exists {
                    let dst_size = std::fs::metadata(&dest_abs2).map(|m| m.len()).unwrap_or(0);
                    if dst_size == sf.size {
                        if let (Ok(sh), Ok(dh)) = (hash::sha256_file(&sf.source_path), hash::sha256_file(&dest_abs2)) {
                            same_hash = sh == dh;
                            src_hash = Some(sh);
                        }
                    }
                }
                let class = if exists { hash::classify(None, exists, same_hash, exists) } else { hash::DuplicateClass::New };
                let (act, rsn) = match class {
                    hash::DuplicateClass::Unchanged => { unchanged+=1; ("skip_unchanged", "same path + same hash -> unchanged (payload)") },
                    hash::DuplicateClass::DuplicateContent => { duplicate+=1; ("skip_duplicate", "different path + same hash -> duplicate (payload)") },
                    hash::DuplicateClass::Conflict => { conflicts+=1; ("conflict", "same path + different hash -> conflict (payload)") },
                    hash::DuplicateClass::New => {
                        // In-job dedup only for archive payload
                        let h = hash::sha256_file(&sf.source_path).ok();
                        if let Some(hv) = h {
                            if hash_to_dest.contains_key(&hv) {
                                duplicate+=1; ("skip_duplicate", "duplicate archive payload")
                            } else {
                                hash_to_dest.insert(hv.clone(), dest_rel2.clone());
                                new_c+=1; ("copy", "archive-is-payload -> copy intact")
                            }
                        } else {
                            new_c+=1; ("copy", "archive-is-payload -> copy intact")
                        }
                    },
                };
                let hash_val = if matches!(act, "copy" | "skip_duplicate") { hash::sha256_file(&sf.source_path).ok() } else { src_hash };
                entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dest_rel2, action: act.into(), reason: rsn.into(), hash: hash_val, size: Some(sf.size), group: None, ..Default::default()});
                total_planned += 1;
                continue;
            }
            if mode == "manual" {
                let dr = format!("{}/{}", sf.classification.destination, sf.source_path.file_name().unwrap().to_string_lossy());
                entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dr, action: "manual_review".into(), reason: "archive requires explicit user decision (profile manual / mixed / nested)".into(), hash: None, size: Some(sf.size), group: None, ..Default::default()});
                manual += 1;
                continue;
            }
            // container / extract_and_classify / grouped
            // Group handling
            let groups = if mode == "grouped" {
                group_members(&inner)
            } else {
                // Check if inner contains cue+bin even in extract mode -> group opportunistically
                let has_cue = inner.iter().any(|e| !e.is_dir && Path::new(&e.name).extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()=="cue").unwrap_or(false));
                let has_bin = inner.iter().any(|e| !e.is_dir && Path::new(&e.name).extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()=="bin").unwrap_or(false));
                if has_cue && has_bin {
                    group_members(&inner)
                } else {
                    inner.iter().filter(|e| !e.is_dir).map(|e| vec![e.clone()]).collect()
                }
            };
            // Build destinations for collision detection
            let mut group_infos: Vec<(Vec<archive::ArchiveEntry>, String)> = Vec::new();
            for grp in groups {
                if grp.len() == 1 {
                    let e = &grp[0];
                    let inner_ext = Path::new(&e.name).extension().and_then(|s| s.to_str()).map(|s| format!(".{}", s.to_lowercase())).unwrap_or_default();
                    // classify inner
                    let member_class = {
                        let p = Path::new(&e.name);
                        crate::classify::classify(p, profile)
                    };
                    let mut eff_base = member_class.destination.clone();
                    if eff_base.is_empty() || eff_base == "roms/UNKNOWN" {
                        if let Some(ids) = profile.ext_to_system.get(&inner_ext) {
                            if let Some(sys) = profile.systems.iter().find(|s| s.id == ids[0]) {
                                eff_base = format!("roms/{}", sys.folder_aliases[0]);
                            }
                        }
                    }
                    if eff_base.is_empty() {
                        eff_base = "roms/UNKNOWN".to_string();
                    }
                    // PS1 CUE/BIN heuristic: if eff_base is MD/segacd/UNKNOWN but group contains CUE, default to PS (most common for TreeFrogUI)
                    if (eff_base == "roms/MD" || eff_base == "roms/segacd" || eff_base == "roms/UNKNOWN") && e.name.to_lowercase().ends_with(".cue") {
                        if let Some(sys) = profile.systems.iter().find(|s| s.id == "ps_psx") {
                            eff_base = format!("roms/{}", sys.folder_aliases[0]);
                        }
                    }
                    let fname = Path::new(&e.name).file_name().unwrap().to_string_lossy().to_string();
                    let dr = format!("{}/{}", eff_base, fname);
                    group_infos.push((grp.clone(), dr));
                } else {
                    let cue = grp.iter().find(|e| Path::new(&e.name).extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()=="cue").unwrap_or(false)).unwrap_or(&grp[0]);
                    let inner_ext = Path::new(&cue.name).extension().and_then(|s| s.to_str()).map(|s| format!(".{}", s.to_lowercase())).unwrap_or_default();
                    let member_class = crate::classify::classify(Path::new(&cue.name), profile);
                    let mut eff_base = member_class.destination.clone();
                    if eff_base.is_empty() || eff_base == "roms/UNKNOWN" {
                        if let Some(ids) = profile.ext_to_system.get(&inner_ext) {
                            if let Some(sys) = profile.systems.iter().find(|s| s.id == ids[0]) {
                                eff_base = format!("roms/{}", sys.folder_aliases[0]);
                            }
                        }
                    }
                    if eff_base.is_empty() {
                        eff_base = "roms/UNKNOWN".to_string();
                    }
                    // PS1 CUE/BIN grouped heuristic: default to PS if CUE present and eff_base is generic
                    if (eff_base == "roms/MD" || eff_base == "roms/segacd" || eff_base == "roms/UNKNOWN") && grp.iter().any(|e| e.name.to_lowercase().ends_with(".cue")) {
                        if let Some(sys) = profile.systems.iter().find(|s| s.id == "ps_psx") {
                            eff_base = format!("roms/{}", sys.folder_aliases[0]);
                        }
                    }
                    let group_name = Path::new(&cue.name).file_stem().unwrap().to_string_lossy().to_string();
                    let dr = format!("{}/{}", eff_base, group_name);
                    group_infos.push((grp.clone(), dr));
                }
            }
            // Single temp extraction per archive; member hashes computed once
            let temp_dir = tempfile::TempDir::new().map_err(|e| anyhow::anyhow!("tempdir failed: {}", e))?;
            let _extracted_ok = crate::archive::safe_extract_to_temp(&sf.source_path, temp_dir.path(), &limits);
            let member_hash = |name: &str| -> Option<String> {
                let fname = std::path::Path::new(name).file_name()?.to_string_lossy().to_string();
                for p in walkdir::WalkDir::new(temp_dir.path()).follow_links(false).into_iter().filter_map(|e| e.ok()) {
                    if p.file_name().to_string_lossy() == fname {
                        return hash::sha256_file(p.path()).ok();
                    }
                }
                None
            };
            // Compute hashes for each group and dedup inside archive before collision handling
            let mut group_infos_with_hash: Vec<(Vec<archive::ArchiveEntry>, String, Option<String>)> = Vec::new();
            for (grp, dr) in group_infos {
                let gh = if grp.len() == 1 {
                    member_hash(&grp[0].name)
                } else {
                    let mut hasher = sha2::Sha256::new();
                    use sha2::Digest;
                    let mut found_hashes: Vec<String> = Vec::new();
                    for member in &grp {
                        if let Some(h) = member_hash(&member.name) {
                            found_hashes.push(h);
                        }
                    }
                    if found_hashes.is_empty() { None } else {
                        found_hashes.sort();
                        for h in &found_hashes { hasher.update(h.as_bytes()); }
                        Some(hex::encode(hasher.finalize()))
                    }
                };
                group_infos_with_hash.push((grp, dr, gh));
            }
            let mut seen_dest: std::collections::HashMap<String, Option<String>> = std::collections::HashMap::new();
            let mut final_infos: Vec<(Vec<archive::ArchiveEntry>, String, Option<String>)> = Vec::new();
            for (grp, dr, gh) in group_infos_with_hash {
                let norm = dr.to_lowercase();
                match seen_dest.get(&norm) {
                    Some(prev) if prev.is_some() && gh.is_some() && prev.as_ref() == gh.as_ref() => {
                        // Identical content inside the archive -> deploy ONE copy only
                        entries.push(PlanEntry {
                            source: format!("{}::{}", sf.source_path.display(), grp[0].name),
                            destination: dr.clone(),
                            action: "skip_duplicate".into(),
                            reason: "duplicate content inside archive -> only one copy deployed".into(),
                            hash: gh.clone(),
                            size: Some(grp.iter().map(|e| e.size).sum()),
                            group: if grp.len() > 1 { Some(grp.iter().map(|e| e.name.clone()).collect()) } else { None },
                            ..Default::default()
                        });
                        duplicate += 1;
                        continue;
                    }
                    Some(_) => {
                        // Same destination, DIFFERENT content -> real collision -> manual review (only this member)
                        entries.push(PlanEntry {
                            source: format!("{}::{}", sf.source_path.display(), grp[0].name),
                            destination: dr.clone(),
                            action: "manual_review".into(),
                            reason: format!("collision inside archive: same destination {} with different content", dr),
                            hash: gh.clone(),
                            size: Some(grp.iter().map(|e| e.size).sum()),
                            ..Default::default()
                        });
                        manual += 1;
                        continue;
                    }
                    None => {
                        seen_dest.insert(norm, gh.clone());
                        final_infos.push((grp, dr, gh));
                    }
                }
            }
            // For each group, plan extract with temp hashing for duplicate detection (reuse already computed hash)
            for (grp, dest_rel_inner, inner_hash) in final_infos {
                let dest_abs_inner = sd_path.join(&dest_rel_inner);
                let exists = dest_abs_inner.exists();

                if exists {
                    if grp.len()==1 {
                        let dst_hash = if dest_abs_inner.is_file() { hash::sha256_file(&dest_abs_inner).ok() } else { None };
                        if let (Some(ih), Some(dh)) = (&inner_hash, &dst_hash) {
                            if ih == dh {
                                entries.push(PlanEntry { source: format!("{}::{}", sf.source_path.display(), grp[0].name), destination: dest_rel_inner.clone(), action: "skip_unchanged".into(), reason: "same path + same hash (extracted payload unchanged)".into(), hash: inner_hash.clone(), size: Some(grp[0].size), group: None, ..Default::default()});
                                unchanged +=1;
                            } else {
                                entries.push(PlanEntry { source: format!("{}::{}", sf.source_path.display(), grp[0].name), destination: dest_rel_inner.clone(), action: "conflict".into(), reason: "same path + different hash (extracted payload conflict)".into(), hash: inner_hash.clone(), size: Some(grp[0].size), group: None, ..Default::default()});
                                conflicts +=1; changed +=1;
                            }
                        } else {
                            entries.push(PlanEntry { source: format!("{}::{}", sf.source_path.display(), grp[0].name), destination: dest_rel_inner.clone(), action: "conflict".into(), reason: "extracted payload exists, hash compare unavailable".into(), hash: inner_hash.clone(), size: Some(grp[0].size), group: None, ..Default::default()});
                            conflicts +=1;
                        }
                    } else {
                        entries.push(PlanEntry { source: format!("{}::group:{} ({})", sf.source_path.display(), Path::new(&grp[0].name).file_stem().unwrap().to_string_lossy(), grp.iter().map(|e| e.name.clone()).collect::<Vec<_>>().join(", ")), destination: dest_rel_inner.clone(), action: "conflict".into(), reason: "grouped payload destination exists (folder)".into(), hash: inner_hash.clone(), size: Some(grp.iter().map(|e| e.size).sum()), group: Some(grp.iter().map(|e| e.name.clone()).collect()), ..Default::default()});
                        conflicts +=1;
                    }
                    continue;
                }
                // Check duplicate via hash - only in-job dedup (same hash elsewhere on SD -> copy to required dest)
                if let Some(ref h) = inner_hash {
                    if hash_to_dest.contains_key(h) {
                        let src = if grp.len()==1 { format!("{}::{}", sf.source_path.display(), grp[0].name) } else { format!("{}::group:{}", sf.source_path.display(), grp[0].name) };
                        entries.push(PlanEntry { source: src, destination: dest_rel_inner.clone(), action: "skip_duplicate".into(), reason: "duplicate extracted payload in same job".into(), hash: inner_hash.clone(), size: Some(grp.iter().map(|e| e.size).sum()), group: if grp.len()>1 { Some(grp.iter().map(|e| e.name.clone()).collect()) } else { None }, ..Default::default()});
                        duplicate +=1;
                        continue;
                    }
                    hash_to_dest.insert(h.clone(), dest_rel_inner.clone());
                }
                // New extract
                let src_str = if grp.len()==1 { format!("{}::{}", sf.source_path.display(), grp[0].name) } else { format!("{}::group:{} ({})", sf.source_path.display(), Path::new(&grp[0].name).file_stem().unwrap().to_string_lossy(), grp.iter().map(|e| e.name.clone()).collect::<Vec<_>>().join(", ")) };
                let reason = if grp.len()==1 { format!("archive-extract -> {} ({})", Path::new(&grp[0].name).file_name().unwrap().to_string_lossy(), Path::new(&grp[0].name).extension().unwrap_or_default().to_string_lossy()) } else { format!("grouped CUE/BIN logical unit ({} files)", grp.len()) };
                entries.push(PlanEntry { source: src_str, destination: dest_rel_inner.clone(), action: "extract".into(), reason, hash: inner_hash.clone(), size: Some(grp.iter().map(|e| e.size).sum()), group: if grp.len()>1 { Some(grp.iter().map(|e| e.name.clone()).collect()) } else { None }, ..Default::default()});
                new_c +=1;
                total_planned += grp.len();
            }
            continue;
        }

        // Video pipeline: check if this is a video file (via classification)
        if sf.classification.kind == crate::classify::Kind::Video {
            let video_preset = &profile.video_preset;
            let probe_result = crate::video::probe(&sf.source_path.to_string_lossy());
            let (status, reason_str) = match probe_result {
                Ok(probe) => {
                    let eval = crate::video::evaluate_compatibility(&probe, video_preset);
                    (eval.status, eval.reason)
                },
                Err(e) => ("inspection_error".to_string(), format!("video inspection error: {}", e)),
            };
            let dest_rel_video = format!("{}/{}", sf.classification.destination, sf.source_path.file_name().unwrap().to_string_lossy());
            if status == "inspection_error" {
                entries.push(crate::PlanEntry {
                    source: sf.source_path.to_string_lossy().to_string(),
                    destination: dest_rel_video.clone(),
                    action: "manual_review".to_string(),
                    reason: reason_str.clone(),
                    hash: None,
                    source_hash: None,
                    destination_hash: None,
                    content_type: Some("video".to_string()),
                    size: Some(sf.size),
                    group: None,
                    members: None,
                    default_action: Some("manual_review".to_string()),
                    resolution: Some("manual_review".to_string()),
                    resolved_action: Some("manual_review".to_string()),
                    original_destination: None,
                    ..Default::default()
                });
                manual += 1;
                continue;
            } else if status == "unsupported" {
                entries.push(crate::PlanEntry {
                    source: sf.source_path.to_string_lossy().to_string(),
                    destination: dest_rel_video.clone(),
                    action: "unsupported".to_string(),
                    reason: reason_str.clone(),
                    hash: None,
                    source_hash: None,
                    destination_hash: None,
                    content_type: Some("video".to_string()),
                    size: Some(sf.size),
                    group: None,
                    members: None,
                    default_action: Some("unsupported".to_string()),
                    resolution: Some("manual_review".to_string()),
                    resolved_action: Some("manual_review".to_string()),
                    original_destination: None,
                    ..Default::default()
                });
                manual += 1;
                continue;
            } else if status == "compatible" {
                let dest_abs_v = sd_path.join(&dest_rel_video);
                let exists_v = dest_abs_v.exists();
                let src_hash_v = hash::sha256_file(&sf.source_path).ok();
                let dst_hash_v = if exists_v { hash::sha256_file(&dest_abs_v).ok() } else { None };
                let same_hash_v = if let (Some(a), Some(b)) = (&src_hash_v, &dst_hash_v) {
                    if std::fs::metadata(&dest_abs_v).map(|m| m.len()).unwrap_or(0) != sf.size { false } else { a == b }
                } else { false };
                let class_v = if exists_v { hash::classify(None, exists_v, same_hash_v, exists_v) } else { hash::DuplicateClass::New };
                let duplicate_elsewhere_v = if !exists_v {
                    if let Some(h) = &src_hash_v {
                        if hash_to_dest.contains_key(h) { true } else { hash_to_dest.insert(h.clone(), dest_rel_video.clone()); false }
                    } else { false }
                } else { false };
                let (action_v, reason_v) = if duplicate_elsewhere_v {
                    duplicate += 1;
                    ("skip_duplicate".to_string(), "different path + same hash -> duplicate (video)".to_string())
                } else {
                    match class_v {
                        hash::DuplicateClass::Unchanged => { unchanged += 1; ("skip_unchanged".to_string(), "same path + same hash -> unchanged (video compatible)".to_string()) },
                        hash::DuplicateClass::Conflict => { conflicts += 1; changed += 1; ("conflict".to_string(), "same path + different hash -> conflict (video)".to_string()) },
                        _ => { new_c += 1; ("copy".to_string(), "video compatible -> copy".to_string()) },
                    }
                };
                entries.push(crate::PlanEntry {
                    source: sf.source_path.to_string_lossy().to_string(),
                    destination: dest_rel_video.clone(),
                    action: action_v.to_string(),
                    reason: format!("{} | {}", reason_str, reason_v),
                    hash: src_hash_v.clone(),
                    source_hash: src_hash_v.clone(),
                    destination_hash: dst_hash_v.clone(),
                    content_type: Some("video".to_string()),
                    size: Some(sf.size),
                    group: None,
                    members: None,
                    default_action: Some(action_v.clone()),
                    resolution: Some(default_resolution_for_action(&action_v)),
                    resolved_action: Some(action_v.clone()),
                    original_destination: None,
                    ..Default::default()
                });
                continue;
            } else if status == "conversion_required" {
                let ffmpeg_cfg = video_preset.get("ffmpeg").and_then(|v| v.as_object());
                let output_ext = ffmpeg_cfg.and_then(|m| m.get("output_extension")).and_then(|x| x.as_str()).unwrap_or(".mp4");
                let base = sf.source_path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
                let safe_base: String = base.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
                let safe_base = if safe_base.is_empty() { "output".to_string() } else { safe_base };
                let converted_name = format!("{}.converted{}", safe_base, output_ext);
                let converted_dest = format!("{}/{}", sf.classification.destination, converted_name);
                let dest_abs_conv = sd_path.join(&converted_dest);
                let exists_conv = dest_abs_conv.exists();
                if exists_conv {
                    entries.push(crate::PlanEntry {
                        source: sf.source_path.to_string_lossy().to_string(),
                        destination: converted_dest.clone(),
                        action: "conflict".to_string(),
                        reason: format!("{} | conversion_required -> {} already exists (conflict)", reason_str, converted_name),
                        hash: None,
                        source_hash: None,
                        destination_hash: None,
                        content_type: Some("video".to_string()),
                        size: Some(sf.size),
                        group: None,
                        members: None,
                        default_action: Some("conflict".to_string()),
                        resolution: Some("conflict".to_string()),
                        resolved_action: Some("conflict".to_string()),
                        original_destination: None,
                        ..Default::default()
                    });
                    conflicts += 1;
                } else {
                    entries.push(crate::PlanEntry {
                        source: sf.source_path.to_string_lossy().to_string(),
                        destination: converted_dest.clone(),
                        action: "convert_then_copy".to_string(),
                        reason: format!("{} | conversion_required via {} (provisional) -> {}", reason_str, video_preset.get("id").and_then(|x| x.as_str()).unwrap_or("unknown"), converted_name),
                        hash: None,
                        source_hash: None,
                        destination_hash: None,
                        content_type: Some("video".to_string()),
                        size: Some(sf.size),
                        group: None,
                        members: None,
                        default_action: Some("convert_then_copy".to_string()),
                        resolution: Some("copy".to_string()),
                        resolved_action: Some("convert_then_copy".to_string()),
                        original_destination: None,
                        ..Default::default()
                    });
                    new_c += 1;
                }
                continue;
            } else {
                entries.push(crate::PlanEntry {
                    source: sf.source_path.to_string_lossy().to_string(),
                    destination: dest_rel.clone(),
                    action: "manual_review".to_string(),
                    reason: reason_str.clone(),
                    hash: None,
                    source_hash: None,
                    destination_hash: None,
                    content_type: Some("video".to_string()),
                    size: Some(sf.size),
                    group: None,
                    members: None,
                    default_action: Some("manual_review".to_string()),
                    resolution: Some("manual_review".to_string()),
                    resolved_action: Some("manual_review".to_string()),
                    original_destination: None,
                    ..Default::default()
                });
                manual += 1;
                continue;
            }
        }
        // Normal file: hash compare (always compute hashes for UI, even when sizes differ)
        let exists = dest_abs.exists();
        let same_path = exists;
        let (src_hash, dst_hash) = if exists {
            let sh = hash::sha256_file(&sf.source_path).ok();
            let dh = hash::sha256_file(&dest_abs).ok();
            (sh, dh)
        } else {
            (None, None)
        };
        let same_hash = if exists {
            if let (Some(a), Some(b)) = (&src_hash, &dst_hash) {
                // Also check size: if sizes differ, hashes cannot be equal, but we already have hashes
                if std::fs::metadata(&dest_abs).map(|m| m.len()).unwrap_or(0) != sf.size {
                    false
                } else {
                    a == b
                }
            } else { false }
        } else { false };

        // Hash ONLY when the destination exists (unchanged/conflict decision).
        let exists = dest_abs.exists();
        let (src_hash, dst_hash, same_hash) = if exists {
            let sh = hash::sha256_file(&sf.source_path).ok();
            let dh = hash::sha256_file(&dest_abs).ok();
            let same = match (&sh, &dh) {
                (Some(a), Some(b)) => std::fs::metadata(&dest_abs).map(|m| m.len()).unwrap_or(0) == sf.size && a == b,
                _ => false,
            };
            (sh, dh, same)
        } else {
            (None, None, false)
        };

        // Cheap in-job dedupe: same name + same size within this job -> skip_duplicate
        let file_name_lower = sf.source_path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        let dedupe_key = format!("{}|{}", file_name_lower, sf.size);
        let duplicate_in_job = !exists && !job_seen.insert(dedupe_key);

        let class = if duplicate_in_job {
            hash::DuplicateClass::DuplicateContent
        } else {
            hash::classify(None, exists, same_hash, exists)
        };

        let (action, reason2) = match class {
            hash::DuplicateClass::Unchanged => { unchanged+=1; ("skip_unchanged".to_string(), "same path + same hash -> unchanged".to_string()) },
            hash::DuplicateClass::DuplicateContent => { duplicate+=1; ("skip_duplicate".to_string(), "same name+size within job -> only one copy deployed".to_string()) },
            hash::DuplicateClass::Conflict => { conflicts+=1; if exists { changed+=1; } ("conflict".to_string(), "same path + different hash -> conflict".to_string()) },
            hash::DuplicateClass::New => { new_c+=1; ("copy".to_string(), reason.clone()) },
        };
        let r = reason2.clone();

        let src_str = if let Some(ref members) = sf.group_members { format!("{} (group {} files)", sf.source_path.display(), members.len()) } else { sf.source_path.to_string_lossy().to_string() };
        let ct = content_type_for_classification(&sf.classification.kind, &sf.classification.system_id);
        let members_vec = sf.group_members.as_ref().map(|v| v.iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect::<Vec<_>>());
        entries.push(PlanEntry {
            source: src_str,
            destination: dest_rel.clone(),
            action: action.clone().into(),
            reason: r.clone(),
            hash: src_hash.clone(),
            source_hash: src_hash.clone(),
            destination_hash: dst_hash.clone(),
            content_type: Some(ct),
            size: Some(sf.size),
            group: members_vec.clone(),
            members: members_vec,
            default_action: Some(action.clone()),
            resolution: Some(default_resolution_for_action(&action)),
            resolved_action: Some(action.clone()),
            original_destination: None, ..Default::default()});
    }

    // Enrich entries with Phase 2B metadata for UI (content_type, hashes, resolution)
    for e in &mut entries {
        if e.content_type.is_none() {
            // For grouped logical units, keep grouped type
            if let Some(m) = &e.members {
                if m.len() > 1 {
                    e.content_type = Some("grouped/CUE_BBIN".to_string());
                    continue;
                }
            }
            if let Some(g) = &e.group {
                if g.len() > 1 {
                    e.content_type = Some("grouped/CUE_BBIN".to_string());
                    continue;
                }
            }
            let dest = e.destination.clone();
            let ct = if dest.starts_with("roms/") {
                let parts: Vec<&str> = dest.split('/').collect();
                if parts.len() >= 2 { format!("rom/{}", parts[1]) } else { "rom/unknown".to_string() }
            } else if dest.starts_with("lgpt/") {
                if dest.contains("projects") { "lgpt/project".to_string() } else { "lgpt/sample".to_string() }
            } else if dest.starts_with("roms/music") { "music".to_string() }
            else if e.action == "copy" && e.destination.ends_with(".zip") { "archive-payload".to_string() }
            else if dest.contains("archive") { "archive".to_string() }
            else { "unknown".to_string() };
            e.content_type = Some(ct);
        }
        if e.source_hash.is_none() {
            e.source_hash = e.hash.clone();
        }
        // destination_hash already set where applicable; keep None otherwise
        if e.default_action.is_none() {
            e.default_action = Some(e.action.clone());
        }
        if e.resolution.is_none() {
            e.resolution = Some(default_resolution_for_action(&e.action));
        }
        if e.resolved_action.is_none() {
            e.resolved_action = Some(e.action.clone());
        }
        if e.members.is_none() {
            e.members = e.group.clone();
        }
        if let Some(m) = &mut e.members { m.sort(); }
        if let Some(g) = &mut e.group { g.sort(); }
        // Ensure hash fields are consistent
        if e.hash.is_none() && e.source_hash.is_some() {
            e.hash = e.source_hash.clone();
        }
    }

    // ---- Destination-level resolution within the job ----
    // The same destination must never appear twice (previously aborted as "case collision").
    // Keep the FIRST entry; later ones become:
    //   - skip_duplicate  (same hash  -> only one copy deployed)
    //   - conflict        (different content -> manual review)
    // One problematic file must NEVER block the rest of the sync.
    {
        let mut seen: std::collections::HashMap<String, Option<String>> = std::collections::HashMap::new();
        for e in entries.iter_mut() {
            let norm = e.destination.to_lowercase();
            if let Some(prev_hash) = seen.get(&norm).cloned() {
                let cur_hash = e.hash.clone().or_else(|| e.source_hash.clone());
                let same = match (&prev_hash, &cur_hash) {
                    (Some(a), Some(b)) => a == b,
                    _ => false,
                };
                if same {
                    e.action = "skip_duplicate".to_string();
                    e.reason = format!("{} [same destination within job -> only one copy deployed]", e.reason);
                } else {
                    e.action = "conflict".to_string();
                    e.resolution = Some("manual_review".to_string());
                    e.reason = format!("{} [same destination with different content within job -> manual review]", e.reason);
                }
                e.resolved_action = Some(e.action.clone());
                e.default_action = Some(e.action.clone());
            } else {
                seen.insert(norm, e.hash.clone().or_else(|| e.source_hash.clone()));
            }
        }
    }

    // Final safety net: no destination may be empty or start with '/'
    for e in &mut entries {
        if e.destination.is_empty() || e.destination.starts_with('/') {
            let fname = e.destination.trim_start_matches('/').to_string();
            let fname = if fname.is_empty() {
                std::path::Path::new(&e.source).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "file".to_string())
            } else {
                std::path::Path::new(&fname).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(fname)
            };
            e.destination = format!("roms/UNKNOWN/{}", fname);
            e.reason = format!("{} [destination sanitized to roms/UNKNOWN]", e.reason);
        }
    }

    // Deterministic: sort entries by source then destination
    entries.sort_by(|a,b| a.source.cmp(&b.source).then(a.destination.cmp(&b.destination)));

    // Recompute summary from FINAL actions (single source of truth)
    let summary = PlanSummary {
        unchanged: entries.iter().filter(|e| e.action == "skip_unchanged").count(),
        new: entries.iter().filter(|e| matches!(e.action.as_str(), "copy" | "extract")).count(),
        changed: entries.iter().filter(|e| e.action == "convert_then_copy").count(),
        duplicate_content: entries.iter().filter(|e| e.action == "skip_duplicate").count(),
        conflicts: entries.iter().filter(|e| e.action == "conflict").count(),
        deletions: 0,
        manual_review: entries.iter().filter(|e| matches!(e.action.as_str(), "manual_review" | "unsupported" | "unsupported_archive")).count(),
        unsupported_archive: entries.iter().filter(|e| matches!(e.action.as_str(), "unsupported_archive" | "unsupported")).count(),
    };
    Ok(Plan { summary, entries, warnings: vec!["PROVISIONAL_UNVALIDATED video preset — not hardware validated".into(), "arch archives bounded: depth=1 entries=1024 expansion=1GiB".into()] })
}

#[allow(dead_code)]
fn dest_abs_exists(p: &Path) -> bool { p.exists() }

#[allow(dead_code)]
fn do_hash_compare(src: &Path, dst: &Path) -> anyhow::Result<(Option<String>, Option<String>)> {
    let sh = if src.exists() { hash::sha256_file(src).ok() } else { None };
    let dh = if dst.exists() { hash::sha256_file(dst).ok() } else { None };
    Ok((sh, dh))
}

fn resolve_destination(sf: &ScannedFile, _profile: &LoadedProfile, _sd_root: &Path) -> anyhow::Result<(String, String, String)> {
    let kind = &sf.classification.kind;
    // NEVER allow an empty destination base: it would produce "/file" (invalid absolute path)
    let dest_base_owned = if sf.classification.destination.is_empty() {
        "roms/UNKNOWN".to_string()
    } else {
        sf.classification.destination.clone()
    };
    let dest_base = &dest_base_owned;
    let file_name = sf.source_path.file_name().unwrap().to_string_lossy().to_string();
    match kind {
        crate::classify::Kind::Music => {
            let rel_dir = Path::new(&sf.relative_hint).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            let dest = if rel_dir.is_empty() { format!("{}/{}", dest_base, file_name) } else {
                let parent = Path::new(&sf.relative_hint).parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if parent.is_empty() { format!("{}/{}", dest_base, file_name) } else { format!("{}/{}/{}", dest_base, parent, file_name) }
            };
            Ok((dest, "copy".into(), "music — preserve subfolders (playlist)".into()))
        },
        crate::classify::Kind::Rom => {
            let dest = format!("{}/{}", dest_base, file_name);
            // Defensa en profundidad: colapsar duplicidad accidental de carpeta raíz (ej. roms/roms/GBA/...)
            let safe_dest = dest.replace("roms/roms/", "roms/").replace("roms\\roms\\", "roms/");
            Ok((safe_dest, "copy".into(), format!("new path + new hash -> {}", dest_base)))
        },
        crate::classify::Kind::Video => {
            let dest = format!("{}/{}", dest_base, file_name);
            let safe_dest = dest.replace("roms/roms/", "roms/").replace("roms\\roms\\", "roms/");
            Ok((safe_dest, "copy".into(), "video — will be probed via ffprobe at sync time".into()))
        },
        crate::classify::Kind::Image => {
            let dest = if dest_base==".res" { format!("{}/{}", dest_base, file_name) } else { format!("{}/{}", dest_base, file_name) };
            Ok((dest, "copy".into(), "image".into()))
        },
        crate::classify::Kind::Ebook => {
            let dest = format!("{}/{}", dest_base, file_name);
            Ok((dest, "copy".into(), "ebook".into()))
        },
        crate::classify::Kind::Bios => {
            let dest = format!("{}/{}", dest_base, file_name);
            Ok((dest, "copy".into(), "BIOS — user-supplied, verify size/hash per bios.json".into()))
        },
        crate::classify::Kind::LgptSample => {
            let dest = format!("lgpt/samples/{}", file_name);
            Ok((dest, "copy".into(), "LGPT sample".into()))
        },
        crate::classify::Kind::LgptProject => {
            let stem = sf.source_path.file_stem().unwrap().to_string_lossy().to_string();
            let dest = format!("lgpt/projects/{}", stem);
            Ok((dest, "copy".into(), "LGPT project — preserve as directory group".into()))
        },
        crate::classify::Kind::Archive => {
            let dest = if dest_base.is_empty() { format!("roms/UNKNOWN/{}", file_name) } else { format!("{}/{}", dest_base, file_name) };
            Ok((dest, "inspect".into(), "archive — inspect entries before copy".into()))
        },
        _ => {
            // Preserve relative subpath so two unknown files with the same name
            // in different folders never collide at the same destination.
            let rel = sf.relative_hint.replace('\\', "/");
            let dest = if rel.is_empty() || rel == file_name {
                format!("{}/{}", dest_base, file_name)
            } else {
                format!("{}/{}", dest_base, rel)
            };
            Ok((dest, "copy".into(), "unknown -> roms/UNKNOWN preserving relative path (needs review)".into()))
        }
    }
}

/// STRUCTURAL GUARANTEE: no destination may appear twice among entries that
/// write to the SD. Later writers are downgraded (skip_duplicate / conflict).
/// This makes "case collision" aborts impossible by construction.
pub fn resolve_write_collisions(plan: crate::Plan) -> crate::Plan {
    let mut seen: std::collections::HashMap<String, Option<String>> = std::collections::HashMap::new();
    let mut entries = Vec::with_capacity(plan.entries.len());
    for mut e in plan.entries {
        let a = e.resolved_action.as_ref().unwrap_or(&e.action).clone();
        let is_write = matches!(a.as_str(), "copy" | "extract" | "convert_then_copy" | "replace");
        if is_write {
            let norm = e.destination.to_lowercase();
            if let Some(prev) = seen.get(&norm).cloned() {
                let cur = e.hash.clone().or_else(|| e.source_hash.clone());
                let same = match (&prev, &cur) { (Some(a), Some(b)) => a == b, _ => false };
                if same {
                    e.action = "skip_duplicate".into();
                    e.reason = format!("{} [duplicate destination within job -> only one copy deployed]", e.reason);
                } else {
                    e.action = "conflict".into();
                    e.resolution = Some("manual_review".into());
                    e.reason = format!("{} [same destination, different content within job -> manual review]", e.reason);
                }
                e.resolved_action = Some(e.action.clone());
                e.default_action = Some(e.action.clone());
            } else {
                seen.insert(norm, e.hash.clone().or_else(|| e.source_hash.clone()));
            }
        }
        entries.push(e);
    }
    let summary = crate::PlanSummary {
        unchanged: entries.iter().filter(|e| e.action == "skip_unchanged").count(),
        new: entries.iter().filter(|e| matches!(e.action.as_str(), "copy" | "extract")).count(),
        changed: entries.iter().filter(|e| e.action == "convert_then_copy").count(),
        duplicate_content: entries.iter().filter(|e| e.action == "skip_duplicate").count(),
        conflicts: entries.iter().filter(|e| e.action == "conflict").count(),
        deletions: 0,
        manual_review: entries.iter().filter(|e| matches!(e.action.as_str(), "manual_review" | "unsupported" | "unsupported_archive")).count(),
        unsupported_archive: entries.iter().filter(|e| matches!(e.action.as_str(), "unsupported_archive" | "unsupported")).count(),
    };
    crate::Plan { summary, entries, warnings: plan.warnings }
}

#[cfg(test)]
mod collision_tests {
    fn mk(dest: &str, hash: Option<&str>) -> crate::PlanEntry {
        crate::PlanEntry {
            source: format!("src/{}", dest),
            destination: dest.to_string(),
            action: "copy".to_string(),
            reason: "test".to_string(),
            hash: hash.map(|s| s.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn duplicate_destinations_are_downgraded_never_abort() {
        let plan = crate::Plan {
            summary: crate::PlanSummary { unchanged: 0, new: 3, changed: 0, duplicate_content: 0, conflicts: 0, deletions: 0, manual_review: 0, unsupported_archive: 0 },
            entries: vec![
                mk("roms/FC/King of Fighters 99, The (Unl).nes", Some("h1")),
                mk("roms/FC/KING OF FIGHTERS 99, THE (UNL).NES", Some("h1")), // case-collision, same hash
                mk("roms/FC/other.nes", Some("h2")),
            ],
            warnings: vec![],
        };
        let fixed = super::resolve_write_collisions(plan);
        let writers = fixed.entries.iter().filter(|e| matches!(e.action.as_str(), "copy" | "extract" | "convert_then_copy" | "replace")).collect::<Vec<_>>();
        assert_eq!(writers.len(), 2, "only unique destinations may write");
        assert_eq!(fixed.entries.iter().filter(|e| e.action == "skip_duplicate").count(), 1);
        // writers' destinations are case-unique
        let mut set = std::collections::HashSet::new();
        for w in writers { assert!(set.insert(w.destination.to_lowercase())); }
    }

    #[test]
    fn different_content_same_destination_becomes_conflict() {
        let plan = crate::Plan {
            summary: crate::PlanSummary { unchanged: 0, new: 2, changed: 0, duplicate_content: 0, conflicts: 0, deletions: 0, manual_review: 0, unsupported_archive: 0 },
            entries: vec![mk("roms/FC/a.nes", Some("h1")), mk("roms/FC/a.nes", Some("hX"))],
            warnings: vec![],
        };
        let fixed = super::resolve_write_collisions(plan);
        assert_eq!(fixed.entries.iter().filter(|e| e.action == "conflict").count(), 1);
        assert_eq!(fixed.entries.iter().filter(|e| matches!(e.action.as_str(), "copy" | "extract")).count(), 1);
    }
}
