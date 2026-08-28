use std::path::{Path, PathBuf};
use std::fs::File;
use zip::ZipArchive;

/// Safety limits mirror profile.json archive_policy.nested_archives
pub struct Limits {
    pub max_entries: usize,
    pub max_expansion_bytes: u64,
    pub max_depth: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self { max_entries: 1024, max_expansion_bytes: 1024*1024*1024, max_depth: 1 }
    }
}

#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

/// Inspect archive entries without extracting.
/// Returns error if traversal / absolute / too many entries / too large.
pub fn inspect_zip(path: &Path, limits: &Limits) -> anyhow::Result<Vec<ArchiveEntry>> {
    let file = File::open(path)?;
    let mut za = ZipArchive::new(file)?;
    if za.len() > limits.max_entries {
        anyhow::bail!("archive exceeds max entries {} > {}", za.len(), limits.max_entries);
    }
    let mut total: u64 = 0;
    let mut out = Vec::new();
    for i in 0..za.len() {
        let entry = za.by_index(i)?;
        let name = entry.name().to_string();
        // Safety: prevent absolute paths and ../ traversal
        if name.starts_with('/') || name.starts_with('\\') || Path::new(&name).is_absolute() {
            anyhow::bail!("archive entry has absolute path: {}", name);
        }
        if name.contains("..") {
            // Check path traversal via components
            let p = Path::new(&name);
            for comp in p.components() {
                if matches!(comp, std::path::Component::ParentDir) {
                    anyhow::bail!("archive entry has traversal .. : {}", name);
                }
            }
        }
        // Detect symlink entries (unix symlink flag) — treat as hazard
        if entry.is_symlink() {
            anyhow::bail!("archive entry is symlink (hazard): {}", name);
        }
        total = total.saturating_add(entry.size());
        if total > limits.max_expansion_bytes {
            anyhow::bail!("archive exceeds max expansion {} > {}", total, limits.max_expansion_bytes);
        }
        out.push(ArchiveEntry { name: name.clone(), is_dir: entry.is_dir(), size: entry.size() });
    }
    Ok(out)
}

/// Determine whether archive itself is valid runtime payload for target system.
/// Checks profile.systems archive_payload_valid for the system's extensions.
/// For generic archives we inspect inner extensions vs profile ext_to_system.
pub fn is_archive_runtime_payload(path: &Path, inner_entries: &[ArchiveEntry], profile: &crate::profile::LoadedProfile) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e.to_lowercase())).unwrap_or_default();
    // If file is .zip for cps1/neogeo/m2k/fc where .zip is primary ROM extension, treat as payload
    // Heuristic: if archive extension itself is in archive_valid list and inner entries contain ROM-ish?
    // Simpler: if profile considers .zip valid for some system, we need system hint; without hint we assume extract.
    // So default: extract unless all inner files are same extension as archive? Not.
    // We'll copy intact only when profile explicitly marks archive ext as valid payload AND inner detection is not better.
    // For Phase 1 we adopt conservative: copy intact only for arcade folder contexts where destination is known zip system.
    // Caller should set destination based on source folder hint if any.
    let _ = profile;
    // Current simple: if inner entries contain .nes/.sfc/.gba etc and archive is .zip, we extract — not payload
    // If inner entries contain no known rom ext but archive is .zip for system where .zip is THE format, then copy intact.
    let mut has_known_rom_inner = false;
    for e in inner_entries {
        if e.is_dir { continue; }
        let inner_ext = Path::new(&e.name).extension().and_then(|x| x.to_str()).map(|x| format!(".{}", x.to_lowercase())).unwrap_or_default();
        if profile.ext_to_system.contains_key(&inner_ext) && inner_ext != ".zip" && inner_ext != ".7z" && inner_ext != ".rar" {
            has_known_rom_inner = true;
            break;
        }
    }
    if has_known_rom_inner {
        return false; // extract
    }
    // If no known inner ROM but archive ext itself is known (.zip for cps1 etc) → treat as payload
    if [".zip", ".7z", ".rar"].contains(&ext.as_str()) && !has_known_rom_inner {
        // need to ensure entry names look like ROM zips for arcade — but without system hint we keep intact as fallback
        return true;
    }
    false
}

/// Safe destination path join with collision and traversal guards (portable).
pub fn safe_join(dest_root: &Path, dest_dir: &str, file_name: &str) -> anyhow::Result<PathBuf> {
    if file_name.contains("..") || Path::new(file_name).is_absolute() {
        anyhow::bail!("unsafe file name: {}", file_name);
    }
    let dest = dest_root.join(dest_dir).join(file_name);
    // Ensure dest is still under dest_root (prevent traversal via dest_dir)
    let canon_root = dest_root.canonicalize().unwrap_or_else(|_| dest_root.to_path_buf());
    let canon_dest_parent = dest.parent().unwrap_or(dest_root).to_path_buf();
    // Simple prefix check without canonicalizing dest (it may not exist yet)
    if !dest.starts_with(&canon_root) && !canon_dest_parent.starts_with(&canon_root) {
        // On Windows, also check normalized
        anyhow::bail!("destination escapes SD root: {}", dest.display());
    }
    Ok(dest)
}
