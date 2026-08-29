use crate::profile::LoadedProfile;
use crate::classify;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub source_path: PathBuf,
    pub relative_hint: String,
    pub size: u64,
    pub classification: classify::Classification,
    // for multi-file groups, leader holds group members
    pub group_members: Option<Vec<PathBuf>>,
}

pub fn scan(source_root: &str, profile: &LoadedProfile) -> anyhow::Result<Vec<ScannedFile>> {
    let root = Path::new(source_root);
    if !root.exists() {
        anyhow::bail!("source path not found: {}", source_root);
    }
    let mut out = Vec::new();
    let mut seen_sources: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    let mut consumed: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    // Walk recursively, skip hidden .res artwork folders? No — we classify everything, but planner will filter
    for entry in WalkDir::new(root).follow_links(false).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        let canon = std::fs::canonicalize(entry.path()).unwrap_or_else(|_| entry.path().to_path_buf());
        if !seen_sources.insert(canon.clone()) {
            continue; // same physical file visited twice (symlink/junction) -> ignore
        }
        if consumed.contains(&canon) {
            continue; // part of a CUE/BIN group already emitted as one logical unit
        }
        if p.is_dir() {
            continue;
        }
        // skip symlink files safely: treat as not regular file
        if entry.file_type().is_symlink() {
            continue;
        }
        // Skip junk/placeholder files that are never content and cause collisions
        let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let fname_lower = file_name.to_lowercase();
        if fname_lower.starts_with('.')
            || fname_lower == "thumbs.db"
            || fname_lower == "desktop.ini"
            || fname_lower == ".keep"
            || fname_lower == ".gitkeep"
            || fname_lower == ".ds_store"
        {
            continue;
        }
        let meta = match std::fs::metadata(p) { Ok(m) => m, Err(_) => continue };
        let size = meta.len();
        // classify by profile + extension/content hints (not filename alone, but extension is primary hint)
        let class = classify::classify(p, profile);
        // multi-file group detection: if file is part of CUE/BIN etc, group with siblings
        let group = detect_group(p, &class);
        if let Some(ref members) = group {
            for m in members.iter().skip(1) {
                if let Ok(c) = std::fs::canonicalize(m) {
                    consumed.insert(c);
                } else {
                    consumed.insert(m.clone());
                }
            }
        }
        out.push(ScannedFile {
            source_path: p.to_path_buf(),
            relative_hint: p.strip_prefix(root).unwrap_or(p).to_string_lossy().to_string(),
            size,
            classification: class,
            group_members: group,
        });
    }
    // De-duplicate group leaders: if a .cue and its .bin are both scanned, keep only .cue as leader with group
    let mut deduped: Vec<ScannedFile> = Vec::new();
    let mut seen_groups: std::collections::HashSet<String> = std::collections::HashSet::new();
    for sf in out {
        if let Some(members) = &sf.group_members {
            let key = members.iter().map(|m| m.to_string_lossy().to_string()).collect::<Vec<_>>().join("|");
            if seen_groups.contains(&key) {
                continue;
            }
            seen_groups.insert(key);
        }
        deduped.push(sf);
    }
    Ok(deduped)
}

fn detect_group(path: &Path, class: &classify::Classification) -> Option<Vec<PathBuf>> {
    // For multi-file sets (CUE/BIN, CHD, m3u, etc) preserve as group
    let ext = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e.to_lowercase())).unwrap_or_default();
    if ext == ".cue" {
        // look for sibling .bin files referenced by .cue (best-effort parse)
        if let Ok(content) = std::fs::read_to_string(path) {
            let mut bins = Vec::new();
            for line in content.lines() {
                if line.to_uppercase().contains("FILE") && line.to_lowercase().contains(".bin") {
                    // crude extract quoted filename
                    if let Some(start) = line.find('"') {
                        if let Some(end) = line[start+1..].find('"') {
                            let bin_name = &line[start+1..start+1+end];
                            let sibling = path.parent().unwrap_or(Path::new(".")).join(bin_name);
                            if sibling.exists() {
                                bins.push(sibling);
                            }
                        }
                    }
                }
            }
            if !bins.is_empty() {
                let mut all = vec![path.to_path_buf()];
                all.extend(bins);
                return Some(all);
            }
        }
        // fallback: same basename .bin
        if let Some(stem) = path.file_stem() {
            let sibling = path.parent().unwrap_or(Path::new(".")).join(format!("{}.bin", stem.to_string_lossy()));
            if sibling.exists() {
                return Some(vec![path.to_path_buf(), sibling]);
            }
        }
    }
    // Chose not to group other extensions here — planner treats single files as solo unless classification.multi_file
    let _ = class;
    None
}
