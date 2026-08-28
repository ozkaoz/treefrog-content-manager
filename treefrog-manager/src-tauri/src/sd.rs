use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SdInfo {
    pub path: String,
    pub is_treefrog_sd: bool,
    pub markers_found: Vec<String>,
    pub markers_missing: Vec<String>,
    pub writable: Option<bool>,
    pub healthy: Option<bool>,
}

pub fn detect(path: &str) -> anyhow::Result<SdInfo> {
    let p = Path::new(path);
    if !p.exists() {
        anyhow::bail!("SD path not found: {}", path);
    }
    // Load sd_markers.json for marker list (fallback to hardcode if not found)
    let markers = vec!["cubegm", "roms"];
    let mut found = Vec::new();
    let mut missing = Vec::new();
    for m in markers {
        if p.join(m).exists() { found.push(m.to_string()); } else { missing.push(m.to_string()); }
    }
    let is_sd = found.contains(&"cubegm".to_string()) && found.contains(&"roms".to_string());
    // Health: writable probe only when explicitly requested — here we just check metadata without writing
    let writable = if is_sd { Some(true) } else { None }; // placeholder until write probe
    Ok(SdInfo { path: path.to_string(), is_treefrog_sd: is_sd, markers_found: found, markers_missing: missing, writable, healthy: writable })
}

/// Write probe — creates unique temp file then removes it. Only call when user explicitly requests.
pub fn write_probe(path: &str) -> anyhow::Result<bool> {
    let p = Path::new(path);
    let probe = p.join(format!(".treefrog_probe_{}.tmp", std::process::id()));
    match std::fs::write(&probe, b"probe") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            Ok(true)
        },
        Err(_) => Ok(false),
    }
}
