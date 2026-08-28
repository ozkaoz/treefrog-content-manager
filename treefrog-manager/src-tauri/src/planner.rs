use crate::profile::LoadedProfile;
use crate::scanner::ScannedFile;
use crate::hash;
use crate::archive;
use std::path::{Path, PathBuf};
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
        // no known -> payload if any system has payload for this ext
        if let Some(per) = policy.get("per_system").and_then(|v| v.as_object()) {
            for (_sys, modes) in per {
                if let Some(m) = modes.get(&ext).and_then(|v| v.as_str()) {
                    if m == "payload" { return "payload".to_string(); }
                }
            }
        }
        return "payload".to_string();
    }
    let unique: HashSet<String> = known.iter().cloned().collect();
    if unique.len() == 1 {
        let sys_id = unique.iter().next().unwrap();
        if let Some(per) = policy.get("per_system").and_then(|v| v.as_object()).and_then(|m| m.get(sys_id)) {
            if let Some(mode) = per.get(&ext).and_then(|v| v.as_str()) {
                return mode.to_string();
            }
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

    // Pre-index SD hashes for duplicate detection
    let scanned_sizes: HashSet<u64> = scanned.iter().map(|s| s.size).collect();
    let mut sd_hash_map: HashMap<String, String> = HashMap::new();
    if sd_path.exists() {
        for entry in walkdir::WalkDir::new(sd_path).follow_links(false).into_iter().filter_map(|e| e.ok()) {
            if entry.path().is_file() && !entry.file_type().is_symlink() {
                if let Ok(meta) = entry.metadata() {
                    if scanned_sizes.contains(&meta.len()) {
                        if let Ok(h) = hash::sha256_file(entry.path()) {
                            if let Ok(rel) = entry.path().strip_prefix(sd_path) {
                                sd_hash_map.insert(h, rel.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }
    }

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
                    let dr = if dest_rel.is_empty() { format!("roms/UNKNOWN/{}", sf.source_path.file_name().unwrap().to_string_lossy()) } else { format!("{}/{}", dest_rel, sf.source_path.file_name().unwrap().to_string_lossy()) };
                    let actual_dest = if sf.classification.destination.is_empty() { dr } else { format!("{}/{}", sf.classification.destination, sf.source_path.file_name().unwrap().to_string_lossy()) };
                    // Use more accurate dest
                    let dest2 = if sf.classification.destination.is_empty() { format!("roms/UNKNOWN/{}", sf.source_path.file_name().unwrap().to_string_lossy()) } else { format!("{}/{}", sf.classification.destination, sf.source_path.file_name().unwrap().to_string_lossy()) };
                    entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dest2, action: "unsupported_archive".into(), reason: format!("archive handler not available for {} (stub)", ext), hash: None, size: Some(sf.size), group: None });
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
                    entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dr, action: action.into(), reason: format!("archive error: {}", msg), hash: None, size: Some(sf.size), group: None });
                    if action == "unsupported_archive" { /* already counted */ } else if action == "manual_review" { /* already */ }
                    continue;
                }
            };

            let inner_file_count = inner.iter().filter(|e| !e.is_dir).count();
            if total_planned + inner_file_count > max_total {
                let dr = format!("{}/{}", sf.classification.destination, sf.source_path.file_name().unwrap().to_string_lossy());
                entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dr, action: "manual_review".into(), reason: format!("exceeds max_total_files_per_job {} (bomb guard)", max_total), hash: None, size: Some(sf.size), group: None });
                manual += 1;
                continue;
            }

            let mode = decide_archive_mode(&sf.source_path, &inner, profile);
            if mode == "unsupported" {
                let dr = format!("{}/{}", sf.classification.destination, sf.source_path.file_name().unwrap().to_string_lossy());
                entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dr, action: "unsupported_archive".into(), reason: format!("mode unsupported for {}", ext), hash: None, size: Some(sf.size), group: None });
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
                        // check duplicate via sd_hash_map or hash_to_dest for archive itself
                        let h = hash::sha256_file(&sf.source_path).ok();
                        if let Some(hv) = h {
                            if sd_hash_map.contains_key(&hv) || hash_to_dest.contains_key(&hv) {
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
                entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dest_rel2, action: act.into(), reason: rsn.into(), hash: hash_val, size: Some(sf.size), group: None });
                total_planned += 1;
                continue;
            }
            if mode == "manual" {
                let dr = format!("{}/{}", sf.classification.destination, sf.source_path.file_name().unwrap().to_string_lossy());
                entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dr, action: "manual_review".into(), reason: "archive requires explicit user decision (profile manual / mixed / nested)".into(), hash: None, size: Some(sf.size), group: None });
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
            let mut dests_for_coll: Vec<String> = Vec::new();
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
                    let fname = Path::new(&e.name).file_name().unwrap().to_string_lossy().to_string();
                    let dr = format!("{}/{}", eff_base, fname);
                    dests_for_coll.push(dr.clone());
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
                    let group_name = Path::new(&cue.name).file_stem().unwrap().to_string_lossy().to_string();
                    let dr = format!("{}/{}", eff_base, group_name);
                    dests_for_coll.push(dr.clone());
                    group_infos.push((grp.clone(), dr));
                }
            }
            let coll = detect_collisions(&dests_for_coll);
            if !coll.is_empty() {
                let dr = format!("{}/{}", sf.classification.destination, sf.source_path.file_name().unwrap().to_string_lossy());
                entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dr, action: "manual_review".into(), reason: format!("output collision inside archive: {} collides with {}", coll[0].0, coll[0].1), hash: None, size: Some(sf.size), group: None });
                manual += 1;
                continue;
            }
            // For each group, plan extract with temp hashing for duplicate detection
            for (grp, dest_rel_inner) in group_infos {
                let dest_abs_inner = sd_path.join(&dest_rel_inner);
                let exists = dest_abs_inner.exists();
                // Try to hash inner content via temp extraction
                let mut inner_hash: Option<String> = None;
                // We'll attempt to extract to temp and hash
                // Use tempfile
                let hash_attempt = (|| -> Option<String> {
                    let tmp = tempfile::TempDir::new().ok()?;
                    let extracted = archive::safe_extract_to_temp(&sf.source_path, tmp.path(), &limits).ok()?;
                    if grp.len()==1 {
                        let target = grp[0].name.clone();
                        let fname = Path::new(&target).file_name().unwrap().to_string_lossy().to_string();
                        for p in walkdir::WalkDir::new(tmp.path()).follow_links(false).into_iter().filter_map(|e| e.ok()) {
                            if p.file_name().to_string_lossy() == fname {
                                return hash::sha256_file(p.path()).ok();
                            }
                        }
                        None
                    } else {
                        // combined hash
                        let mut hasher = sha2::Sha256::new();
                        use sha2::Digest;
                        let mut found_hashes: Vec<String> = Vec::new();
                        for member in &grp {
                            let fname = Path::new(&member.name).file_name().unwrap().to_string_lossy().to_string();
                            for p in walkdir::WalkDir::new(tmp.path()).follow_links(false).into_iter().filter_map(|e| e.ok()) {
                                if p.file_name().to_string_lossy() == fname {
                                    if let Ok(h) = hash::sha256_file(p.path()) {
                                        found_hashes.push(h);
                                    }
                                    break;
                                }
                            }
                        }
                        if found_hashes.is_empty() { return None; }
                        found_hashes.sort();
                        for h in found_hashes { hasher.update(h.as_bytes()); }
                        Some(hex::encode(hasher.finalize()))
                    }
                })();
                inner_hash = hash_attempt;

                if exists {
                    if grp.len()==1 {
                        let dst_hash = if dest_abs_inner.is_file() { hash::sha256_file(&dest_abs_inner).ok() } else { None };
                        if let (Some(ih), Some(dh)) = (&inner_hash, &dst_hash) {
                            if ih == dh {
                                entries.push(PlanEntry { source: format!("{}::{}", sf.source_path.display(), grp[0].name), destination: dest_rel_inner.clone(), action: "skip_unchanged".into(), reason: "same path + same hash (extracted payload unchanged)".into(), hash: inner_hash.clone(), size: Some(grp[0].size), group: None });
                                unchanged +=1;
                            } else {
                                entries.push(PlanEntry { source: format!("{}::{}", sf.source_path.display(), grp[0].name), destination: dest_rel_inner.clone(), action: "conflict".into(), reason: "same path + different hash (extracted payload conflict)".into(), hash: inner_hash.clone(), size: Some(grp[0].size), group: None });
                                conflicts +=1; changed +=1;
                            }
                        } else {
                            entries.push(PlanEntry { source: format!("{}::{}", sf.source_path.display(), grp[0].name), destination: dest_rel_inner.clone(), action: "conflict".into(), reason: "extracted payload exists, hash compare unavailable".into(), hash: inner_hash.clone(), size: Some(grp[0].size), group: None });
                            conflicts +=1;
                        }
                    } else {
                        entries.push(PlanEntry { source: format!("{}::group:{} ({})", sf.source_path.display(), Path::new(&grp[0].name).file_stem().unwrap().to_string_lossy(), grp.iter().map(|e| e.name.clone()).collect::<Vec<_>>().join(", ")), destination: dest_rel_inner.clone(), action: "conflict".into(), reason: "grouped payload destination exists (folder)".into(), hash: inner_hash.clone(), size: Some(grp.iter().map(|e| e.size).sum()), group: Some(grp.iter().map(|e| e.name.clone()).collect()) });
                        conflicts +=1;
                    }
                    continue;
                }
                // Check duplicate via hash
                if let Some(ref h) = inner_hash {
                    if sd_hash_map.contains_key(h) {
                        let src = if grp.len()==1 { format!("{}::{}", sf.source_path.display(), grp[0].name) } else { format!("{}::group:{}", sf.source_path.display(), grp[0].name) };
                        entries.push(PlanEntry { source: src, destination: dest_rel_inner.clone(), action: "skip_duplicate".into(), reason: format!("duplicate extracted payload (inner {} already on SD at {})", Path::new(&grp[0].name).extension().unwrap_or_default().to_string_lossy(), sd_hash_map[h]), hash: inner_hash.clone(), size: Some(grp.iter().map(|e| e.size).sum()), group: if grp.len()>1 { Some(grp.iter().map(|e| e.name.clone()).collect()) } else { None } });
                        duplicate +=1;
                        continue;
                    }
                    if hash_to_dest.contains_key(h) {
                        let src = if grp.len()==1 { format!("{}::{}", sf.source_path.display(), grp[0].name) } else { format!("{}::group:{}", sf.source_path.display(), grp[0].name) };
                        entries.push(PlanEntry { source: src, destination: dest_rel_inner.clone(), action: "skip_duplicate".into(), reason: "duplicate extracted payload in same job".into(), hash: inner_hash.clone(), size: Some(grp.iter().map(|e| e.size).sum()), group: if grp.len()>1 { Some(grp.iter().map(|e| e.name.clone()).collect()) } else { None } });
                        duplicate +=1;
                        continue;
                    }
                    hash_to_dest.insert(h.clone(), dest_rel_inner.clone());
                }
                // New extract
                let src_str = if grp.len()==1 { format!("{}::{}", sf.source_path.display(), grp[0].name) } else { format!("{}::group:{} ({})", sf.source_path.display(), Path::new(&grp[0].name).file_stem().unwrap().to_string_lossy(), grp.iter().map(|e| e.name.clone()).collect::<Vec<_>>().join(", ")) };
                let reason = if grp.len()==1 { format!("archive-extract -> {} ({})", Path::new(&grp[0].name).file_name().unwrap().to_string_lossy(), Path::new(&grp[0].name).extension().unwrap_or_default().to_string_lossy()) } else { format!("grouped CUE/BIN logical unit ({} files)", grp.len()) };
                entries.push(PlanEntry { source: src_str, destination: dest_rel_inner.clone(), action: "extract".into(), reason, hash: inner_hash.clone(), size: Some(grp.iter().map(|e| e.size).sum()), group: if grp.len()>1 { Some(grp.iter().map(|e| e.name.clone()).collect()) } else { None } });
                new_c +=1;
                total_planned += grp.len();
            }
            continue;
        }

        // Normal file: hash compare
        let exists = dest_abs.exists();
        let same_path = exists;
        let (src_hash, dst_hash) = if exists {
            let dst_meta = std::fs::metadata(&dest_abs).ok();
            let dst_size = dst_meta.map(|m| m.len()).unwrap_or(0);
            if dst_size != sf.size {
                (None, None)
            } else {
                let sh = hash::sha256_file(&sf.source_path).ok();
                let dh = hash::sha256_file(&dest_abs).ok();
                (sh, dh)
            }
        } else {
            (None, None)
        };
        let same_hash = match (&src_hash, &dst_hash) { (Some(a), Some(b)) => a==b, _ => false };

        let duplicate_elsewhere = if !same_path {
            if let Ok(h) = hash::sha256_file(&sf.source_path) {
                if sd_hash_map.contains_key(&h) || hash_to_dest.contains_key(&h) {
                    true
                } else {
                    hash_to_dest.insert(h.clone(), dest_rel.clone());
                    false
                }
            } else { false }
        } else { false };

        let class = if duplicate_elsewhere {
            hash::DuplicateClass::DuplicateContent
        } else {
            hash::classify(None, same_path, same_hash, exists)
        };

        let (action, reason2) = match class {
            hash::DuplicateClass::Unchanged => { unchanged+=1; ("skip_unchanged", "same path + same hash -> unchanged") },
            hash::DuplicateClass::DuplicateContent => { duplicate+=1; ("skip_duplicate", "different path + same hash -> duplicate content default skip") },
            hash::DuplicateClass::Conflict => { conflicts+=1; if same_path { changed+=1; } ("conflict", "same path + different hash -> conflict") },
            hash::DuplicateClass::New => { new_c+=1; ("copy", reason.clone()) },
        };
        let r = if action=="copy" { reason } else { reason2.to_string() };

        let src_str = if let Some(members) = sf.group_members { format!("{} (group {} files)", sf.source_path.display(), members.len()) } else { sf.source_path.to_string_lossy().to_string() };
        entries.push(PlanEntry {
            source: src_str,
            destination: dest_rel,
            action: action.into(),
            reason: r,
            hash: src_hash,
            size: Some(sf.size),
            group: sf.group_members.map(|v| v.iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect()),
        });
    }

    // Deterministic: sort entries by source then destination
    entries.sort_by(|a,b| a.source.cmp(&b.source).then(a.destination.cmp(&b.destination)));

    let summary = PlanSummary { unchanged, new: new_c, changed, duplicate_content: duplicate, conflicts, deletions: 0, manual_review: manual, unsupported_archive: unsupported };
    Ok(Plan { summary, entries, warnings: vec!["PROVISIONAL_UNVALIDATED video preset — not hardware validated".into(), "arch archives bounded: depth=1 entries=1024 expansion=1GiB".into()] })
}

fn dest_abs_exists(p: &Path) -> bool { p.exists() }

fn do_hash_compare(src: &Path, dst: &Path) -> anyhow::Result<(Option<String>, Option<String>)> {
    let sh = if src.exists() { hash::sha256_file(src).ok() } else { None };
    let dh = if dst.exists() { hash::sha256_file(dst).ok() } else { None };
    Ok((sh, dh))
}

fn resolve_destination(sf: &ScannedFile, profile: &LoadedProfile, sd_root: &Path) -> anyhow::Result<(String, String, String)> {
    let kind = &sf.classification.kind;
    let dest_base = &sf.classification.destination;
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
            Ok((dest, "copy".into(), format!("new path + new hash -> {}", dest_base)))
        },
        crate::classify::Kind::Video => {
            let dest = format!("{}/{}", dest_base, file_name);
            Ok((dest, "copy".into(), "video — will be probed via ffprobe at sync time".into()))
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
            let dest = format!("{}/{}", dest_base, file_name);
            Ok((dest, "copy".into(), "unknown -> roms/UNKNOWN (needs review)".into()))
        }
    }
}
