use crate::profile::LoadedProfile;
use crate::scanner::ScannedFile;
use crate::hash;
use crate::archive;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use crate::Plan;
use crate::PlanEntry;
use crate::PlanSummary;

pub fn plan(scanned: Vec<ScannedFile>, sd_root: &str, profile: &LoadedProfile) -> anyhow::Result<Plan> {
    let sd_path = Path::new(sd_root);
    let mut entries: Vec<PlanEntry> = Vec::new();
    let mut unchanged = 0usize;
    let mut new_c = 0usize;
    let mut changed = 0usize;
    let mut duplicate = 0usize;
    let mut conflicts = 0usize;
    let mut hash_to_dest: HashMap<String, String> = HashMap::new();
    // Pre-index SD file hashes for duplicate detection (same content not same filename)
    use walkdir::WalkDir;
    let scanned_sizes: std::collections::HashSet<u64> = scanned.iter().map(|s| s.size).collect();
    let mut sd_hash_map: HashMap<String, String> = HashMap::new();
    if sd_path.exists() {
        for entry in WalkDir::new(sd_path).follow_links(false).into_iter().filter_map(|e| e.ok()) {
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

    for sf in scanned {
        // Resolve destination
        let (dest_rel, action_hint, reason) = resolve_destination(&sf, profile, sd_path)?;
        let dest_abs = sd_path.join(&dest_rel);

        // Archive handling: if archive, inspect entries
        if sf.classification.kind == crate::classify::Kind::Archive {
            // Inspect
            let limits = archive::Limits::default();
            let inspected = archive::inspect_zip(&sf.source_path, &limits);
            match inspected {
                Ok(inner) => {
                    let is_payload = archive::is_archive_runtime_payload(&sf.source_path, &inner, profile);
                    if is_payload {
                        // copy intact
                        let dest_rel2 = format!("{}/{}", sf.classification.destination.is_empty().then(|| dest_rel.clone()).unwrap_or(dest_rel.clone()), sf.source_path.file_name().unwrap().to_string_lossy());
                        // Actually if payload, keep original archive name under appropriate rom folder — for generic we use roms/<system> or preserve hint
                        let (p, e) = do_hash_compare(&sf.source_path, &sd_path.join(&dest_rel2))?;
                        let class = hash::classify(None, dest_abs_exists(&sd_path.join(&dest_rel2)), p.is_some() && e.is_some() && p==e, dest_abs_exists(&sd_path.join(&dest_rel2)));
                        // simplify: if new etc
                        let (act, rsn) = match class {
                            hash::DuplicateClass::Unchanged => { unchanged+=1; ("skip_unchanged", "same path + same hash -> unchanged") },
                            hash::DuplicateClass::DuplicateContent => { duplicate+=1; ("skip_duplicate", "different path + same hash -> duplicate skip") },
                            hash::DuplicateClass::Conflict => { conflicts+=1; ("conflict", "same path + different hash -> conflict") },
                            hash::DuplicateClass::New => { new_c+=1; ("copy", "archive payload valid → copy intact (new)") },
                        };
                        entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dest_rel2, action: act.into(), reason: rsn.into(), hash: p, size: Some(sf.size), group: None });
                    } else {
                        // extract supported inner files
                        for inner in inner.iter().filter(|e| !e.is_dir) {
                            let inner_ext = Path::new(&inner.name).extension().and_then(|e| e.to_str()).map(|x| format!(".{}", x.to_lowercase())).unwrap_or_default();
                            // Only extract if inner ext is known rom/media
                            if profile.ext_to_system.contains_key(&inner_ext) || [".sfc",".nes",".gba",".gb",".gbc",".md",".bin",".cue"].contains(&inner_ext.as_str()) {
                                let file_name = Path::new(&inner.name).file_name().unwrap().to_string_lossy().to_string();
                                let dest_rel_inner = format!("{}/{}", dest_rel, file_name);
                                // safety: prevent traversal already checked
                                let dest_abs_inner = sd_path.join(&dest_rel_inner);
                                let (p_hash, dest_exists) = (None::<String>, dest_abs_exists(&dest_abs_inner));
                                // For archive inner extract, we can't hash source without extracting; treat as new for now (bounded policy says stage then hash)
                                let act = if dest_exists { "conflict" } else { "extract" };
                                if act == "extract" { new_c+=1; } else { conflicts+=1; }
                                entries.push(PlanEntry { source: format!("{}::{}", sf.source_path.display(), inner.name), destination: dest_rel_inner, action: act.into(), reason: format!("archive extract safe (inner {})", inner_ext), hash: p_hash, size: Some(inner.size), group: None });
                            }
                        }
                    }
                },
                Err(err) => {
                    entries.push(PlanEntry { source: sf.source_path.to_string_lossy().to_string(), destination: dest_rel, action: "conflict".into(), reason: format!("archive inspection failed: {}", err), hash: None, size: Some(sf.size), group: None });
                    conflicts+=1;
                }
            }
            continue;
        }

        // Normal file: hash compare
        let exists = dest_abs.exists();
        let same_path = exists;
        let (src_hash, dst_hash) = if exists {
            // cheap metadata first: size compare
            let dst_meta = std::fs::metadata(&dest_abs).ok();
            let dst_size = dst_meta.map(|m| m.len()).unwrap_or(0);
            if dst_size != sf.size {
                // different size => different hash without hashing (conflict)
                (None, None)
            } else {
                // need SHA256
                let sh = hash::sha256_file(&sf.source_path).ok();
                let dh = hash::sha256_file(&dest_abs).ok();
                (sh, dh)
            }
        } else {
            // check for duplicate content elsewhere: if same size+hash exists at different path, default skip
            // For Phase 1 we do hash of source and compare against hash_to_dest
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

        let (action, reason) = match class {
            hash::DuplicateClass::Unchanged => { unchanged+=1; ("skip_unchanged", "same path + same hash -> unchanged") },
            hash::DuplicateClass::DuplicateContent => { duplicate+=1; ("skip_duplicate", "different path + same hash -> duplicate content default skip") },
            hash::DuplicateClass::Conflict => { conflicts+=1; if same_path { changed+=1; } ("conflict", "same path + different hash -> conflict") },
            hash::DuplicateClass::New => { new_c+=1; ("copy", reason.clone()) },
        };
        // For new, pick reason from resolve
        let r = if action=="copy" { reason } else { reason.to_string() };

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

    let summary = PlanSummary { unchanged, new: new_c, changed, duplicate_content: duplicate, conflicts, deletions: 0 };
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
            // preserve subfolders relative to source root's music substructure
            // Use relative_hint's directory
            let rel_dir = Path::new(&sf.relative_hint).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            // If source had music/My Album/song.flac and classification says roms/music, preserve My Album
            // Take last component(s) after source root? For Phase 1 we use rel_dir's last folder if any.
            let dest = if rel_dir.is_empty() { format!("{}/{}", dest_base, file_name) } else {
                // If rel_dir contains path separators, preserve tail after source root music detection
                // Simplify: take file's parent folder name as playlist
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
            let dest = if dest_base==".res" {
                // .res artwork — keep relative
                format!("{}/{}", dest_base, file_name)
            } else { format!("{}/{}", dest_base, file_name) };
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
            // projects as groups: dest is directory named after stem
            let stem = sf.source_path.file_stem().unwrap().to_string_lossy().to_string();
            let dest = format!("lgpt/projects/{}", stem);
            Ok((dest, "copy".into(), "LGPT project — preserve as directory group".into()))
        },
        crate::classify::Kind::Archive => {
            // For archive we need to decide extract vs intact — planner will handle, but dest base is roms/<system> or media
            // For now return generic dest for archive file itself
            let dest = if dest_base.is_empty() { format!("roms/UNKNOWN/{}", file_name) } else { format!("{}/{}", dest_base, file_name) };
            Ok((dest, "inspect".into(), "archive — inspect entries before copy".into()))
        },
        _ => {
            let dest = format!("{}/{}", dest_base, file_name);
            Ok((dest, "copy".into(), "unknown -> roms/UNKNOWN (needs review)".into()))
        }
    }
}
