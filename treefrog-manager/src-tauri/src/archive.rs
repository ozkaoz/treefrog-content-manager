use std::fs::File;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

pub struct Limits {
    pub max_entries: usize,
    pub max_expansion_bytes: u64,
    pub max_depth: u32,
    pub max_total_files_per_job: u32,
    pub max_compression_ratio: f64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entries: 1024,
            max_expansion_bytes: 1024 * 1024 * 1024,
            max_depth: 1,
            max_total_files_per_job: 10000,
            max_compression_ratio: 100.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub compressed_size: u64,
    pub crc: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("unsupported archive format: {0}")]
    Unsupported(String),
    #[error("safety violation: {0}")]
    Safety(String),
    #[error("collision: {0}")]
    Collision(String),
    #[error("nested archive bomb: {0}")]
    NestedBomb(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

fn is_windows_drive_absolute(name: &str) -> bool {
    // C:/, C:\, C:, \\?\, \\server\share
    if name.len() >= 2 && name.as_bytes()[1] == b':' && name.as_bytes()[0].is_ascii_alphabetic() {
        if name.len() == 2 {
            return true;
        }
        let c = name.as_bytes()[2];
        if c == b'/' || c == b'\\' {
            return true;
        }
        // also C:foo without slash is drive-relative but still suspicious for archive
        return true;
    }
    if name.starts_with("\\\\?\\") || name.starts_with("\\\\") {
        // UNC
        return true;
    }
    false
}

fn check_entry_safety(
    name: &str,
    entry: &zip::read::ZipFile,
    _limits: &Limits,
) -> Result<(), ArchiveError> {
    if name.starts_with('/') || name.starts_with('\\') {
        return Err(ArchiveError::Safety(format!(
            "absolute path entry: {}",
            name
        )));
    }
    if is_windows_drive_absolute(name) {
        return Err(ArchiveError::Safety(format!(
            "windows drive-letter absolute entry: {}",
            name
        )));
    }
    if Path::new(name).is_absolute() {
        return Err(ArchiveError::Safety(format!(
            "absolute path entry (pure): {}",
            name
        )));
    }
    // traversal
    let normalized = name.replace('\\', "/");
    for comp in Path::new(&normalized).components() {
        if matches!(comp, std::path::Component::ParentDir) {
            return Err(ArchiveError::Safety(format!("traversal entry: {}", name)));
        }
    }
    // symlink / hardlink hazards
    if entry.is_symlink() {
        return Err(ArchiveError::Safety(format!("symlink hazard: {}", name)));
    }
    // external_attr unix mode
    // zip extra: external_attr >>16 is unix file type
    // We can't easily get external_attr in stable zip crate? Use entry.unix_mode() if available
    // zip 2.x has entry.unix_mode() -> Option<u32>
    if let Some(mode) = entry.unix_mode() {
        let ftype = mode & 0o170000;
        if ftype == 0o120000 {
            return Err(ArchiveError::Safety(format!(
                "symlink hazard (unix_mode): {}",
                name
            )));
        }
        if ftype != 0 && ftype != 0o100000 && ftype != 0o040000 && ftype != 0o120000 {
            return Err(ArchiveError::Safety(format!(
                "hardlink/unsafe file type hazard: {} mode={:o}",
                name, ftype
            )));
        }
    }
    // ADS colon hazard (except drive letter)
    if name.contains(':') && !is_windows_drive_absolute(name) {
        // Colon in member name is suspicious (ADS)
        return Err(ArchiveError::Safety(format!(
            "hardlink/ADS hazard (colon in name): {}",
            name
        )));
    }
    Ok(())
}

pub fn inspect_zip(path: &Path, limits: &Limits) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    let file = File::open(path).map_err(|e| ArchiveError::Other(e.into()))?;
    let mut za = ZipArchive::new(file).map_err(|e| ArchiveError::Other(e.into()))?;
    if za.len() > limits.max_entries {
        return Err(ArchiveError::Safety(format!(
            "archive exceeds max entries {} > {}",
            za.len(),
            limits.max_entries
        )));
    }
    let mut total: u64 = 0;
    let mut out = Vec::new();
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for i in 0..za.len() {
        let entry = za.by_index(i).map_err(|e| ArchiveError::Other(e.into()))?;
        let name = entry.name().to_string();
        check_entry_safety(&name, &entry, limits)?;
        if entry.is_dir() {
            let norm = name
                .replace('\\', "/")
                .to_lowercase()
                .trim_end_matches('/')
                .to_string();
            if seen.contains_key(&norm) {
                return Err(ArchiveError::Collision(format!(
                    "collision: duplicate normalized path {} vs {}",
                    name, seen[&norm]
                )));
            }
            seen.insert(norm, name.clone());
            out.push(ArchiveEntry {
                name: name.clone(),
                is_dir: true,
                size: 0,
                compressed_size: entry.compressed_size() as u64,
                crc: entry.crc32(),
            });
            continue;
        }
        total = total.saturating_add(entry.size());
        if total > limits.max_expansion_bytes {
            return Err(ArchiveError::Safety(format!(
                "archive exceeds max expansion {} > {}",
                total, limits.max_expansion_bytes
            )));
        }
        if entry.compressed_size() > 0 && entry.size() > 1024 * 1024 {
            let ratio = entry.size() as f64 / entry.compressed_size() as f64;
            if ratio > limits.max_compression_ratio {
                return Err(ArchiveError::Safety(format!(
                    "excessive compression ratio {:.1} for {}",
                    ratio, name
                )));
            }
        }
        let norm = name.replace('\\', "/").to_lowercase();
        if seen.contains_key(&norm) {
            return Err(ArchiveError::Collision(format!(
                "collision: normalized path duplicate {} vs {}",
                name, seen[&norm]
            )));
        }
        seen.insert(norm, name.clone());
        out.push(ArchiveEntry {
            name: name.clone(),
            is_dir: false,
            size: entry.size(),
            compressed_size: entry.compressed_size() as u64,
            crc: entry.crc32(),
        });
    }
    Ok(out)
}

pub fn inspect_archive(path: &Path, limits: &Limits) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()))
        .unwrap_or_default();
    match ext.as_str() {
        ".zip" => inspect_zip(path, limits),
        // 7z/RAR are EXPLICITLY unsupported (no maintained safe adapter): they
        // are surfaced as unsupported_archive with a precise reason and can
        // never be extracted or copied as supported content. ZIP is the only
        // supported archive format.
        ".7z" => Err(ArchiveError::Unsupported(format!("7z archives are not supported (only ZIP is supported); file kept for manual handling: {}", path.display()))),
        ".rar" => Err(ArchiveError::Unsupported(format!("RAR archives are not supported (only ZIP is supported); file kept for manual handling: {}", path.display()))),
        _ => Err(ArchiveError::Unsupported(format!("unsupported archive format: {}", ext))),
    }
}

pub fn is_archive_runtime_payload(
    path: &Path,
    inner_entries: &[ArchiveEntry],
    profile: &crate::profile::LoadedProfile,
) -> bool {
    // Profile-driven heuristic fallback (mirrors Python)
    let mut has_known_inner = false;
    for e in inner_entries {
        if e.is_dir {
            continue;
        }
        let inner_ext = Path::new(&e.name)
            .extension()
            .and_then(|x| x.to_str())
            .map(|x| format!(".{}", x.to_lowercase()))
            .unwrap_or_default();
        if profile.ext_to_system.contains_key(&inner_ext)
            && inner_ext != ".zip"
            && inner_ext != ".7z"
            && inner_ext != ".rar"
        {
            has_known_inner = true;
            break;
        }
        if [
            ".cue", ".bin", ".chd", ".m3u", ".sfc", ".nes", ".gba", ".gb", ".gbc", ".md", ".sms",
            ".gg",
        ]
        .contains(&inner_ext.as_str())
        {
            has_known_inner = true;
            break;
        }
    }
    if has_known_inner {
        return false;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()))
        .unwrap_or_default();
    if [".zip", ".7z", ".rar"].contains(&ext.as_str()) && !has_known_inner {
        return true;
    }
    false
}

fn is_path_within(base: &Path, target: &Path) -> bool {
    // Canonicalize base if possible
    let canon_base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
    // For target that doesn't exist yet, check its parent and normalized components
    // Normalize by checking components for ParentDir
    for comp in target.components() {
        if matches!(comp, std::path::Component::ParentDir) {
            return false;
        }
    }
    // Check that the target's string representation starts with base's string after normalization
    // Use lexical check: target must start with base and not contain .. after normalization
    let base_str = canon_base
        .to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string();
    let target_str = target.to_string_lossy().replace('\\', "/").to_string();
    // Ensure target is absolute or joined correctly
    if !target_str.starts_with(&base_str) {
        // Try canonicalizing parent
        if let Some(parent) = target.parent() {
            if let Ok(canon_parent) = parent.canonicalize() {
                if !canon_parent.starts_with(&canon_base) {
                    return false;
                }
            } else {
                // Parent doesn't exist yet, check lexical
                let parent_str = parent.to_string_lossy().replace('\\', "/");
                if !parent_str.starts_with(&base_str) && !target_str.starts_with(&base_str) {
                    return false;
                }
            }
        } else {
            return false;
        }
    }
    // Also ensure no traversal after normalization
    let normalized = target_str.replace('\\', "/");
    if normalized.contains("../") || normalized.contains("..\\") || normalized.ends_with("/..") {
        return false;
    }
    true
}

pub fn safe_extract_to_temp(
    archive_path: &Path,
    temp_dir: &Path,
    limits: &Limits,
) -> Result<Vec<PathBuf>, ArchiveError> {
    let entries = inspect_archive(archive_path, limits)?;
    let mut extracted = Vec::new();
    let mut za =
        ZipArchive::new(File::open(archive_path).map_err(|e| ArchiveError::Other(e.into()))?)
            .map_err(|e| ArchiveError::Other(e.into()))?;
    let canon_temp = temp_dir
        .canonicalize()
        .unwrap_or_else(|_| temp_dir.to_path_buf());
    for entry in entries.iter().filter(|e| !e.is_dir) {
        let mut file = za
            .by_name(&entry.name)
            .map_err(|e| ArchiveError::Other(e.into()))?;
        let normalized = entry.name.replace('\\', "/");
        // Reject any entry that would escape temp_dir lexically
        if normalized.contains("../")
            || normalized.starts_with("../")
            || normalized.starts_with('/')
        {
            return Err(ArchiveError::Safety(format!(
                "extraction would escape temp dir (traversal): {} -> {}",
                entry.name, normalized
            )));
        }
        if is_windows_drive_absolute(&normalized) {
            return Err(ArchiveError::Safety(format!(
                "extraction would escape temp dir (drive): {} -> {}",
                entry.name, normalized
            )));
        }
        let dest = temp_dir.join(&normalized);
        if !is_path_within(&canon_temp, &dest) {
            return Err(ArchiveError::Safety(format!(
                "extraction would escape temp dir: {} -> {}",
                entry.name,
                dest.display()
            )));
        }
        if dest.exists() {
            return Err(ArchiveError::Collision(format!(
                "output collision in temp: {} already exists",
                dest.display()
            )));
        }
        if let Ok(canon_archive) = archive_path.canonicalize() {
            if dest == canon_archive || dest.starts_with(&canon_archive) {
                return Err(ArchiveError::Safety(format!(
                    "would overwrite source archive: {}",
                    dest.display()
                )));
            }
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ArchiveError::Other(e.into()))?;
            // Re-check after parent creation that parent is still within temp
            let canon_parent = parent.canonicalize().unwrap_or(parent.to_path_buf());
            if !canon_parent.starts_with(&canon_temp) && !is_path_within(&canon_temp, &canon_parent)
            {
                return Err(ArchiveError::Safety(format!(
                    "parent escapes temp dir: {}",
                    parent.display()
                )));
            }
        }
        let mut out = File::create(&dest).map_err(|e| ArchiveError::Other(e.into()))?;
        std::io::copy(&mut file, &mut out).map_err(|e| ArchiveError::Other(e.into()))?;
        extracted.push(dest);
    }
    Ok(extracted)
}

pub fn safe_join(dest_root: &Path, dest_dir: &str, file_name: &str) -> anyhow::Result<PathBuf> {
    if file_name.contains("..")
        || Path::new(file_name).is_absolute()
        || is_windows_drive_absolute(file_name)
    {
        anyhow::bail!("unsafe file name: {}", file_name);
    }
    let dest = dest_root.join(dest_dir).join(file_name);
    let canon_root = dest_root
        .canonicalize()
        .unwrap_or_else(|_| dest_root.to_path_buf());
    if !dest.starts_with(&canon_root) {
        anyhow::bail!("destination escapes SD root: {}", dest.display());
    }
    Ok(dest)
}

/// Supported archive handler registry. Only formats with a maintained, safe
/// adapter are listed; 7z/RAR are intentionally absent (unsupported).
pub fn get_handler_for_ext(ext: &str) -> Option<&'static str> {
    match ext.to_lowercase().as_str() {
        ".zip" => Some("zip"),
        _ => None,
    }
}
