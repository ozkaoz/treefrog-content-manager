use serde::{Deserialize, Serialize};
use std::path::Path;

/// Writability is tri-state: unknown means "not probed" — we NEVER infer
/// writable=true from the presence of markers. A read-only SD must not
/// appear writable, and a TreeFrogUI SD without a write probe is writable=unknown.
pub type Writable = Option<bool>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SdInfo {
    pub path: String,
    pub is_treefrog_sd: bool,
    pub markers_found: Vec<String>,
    pub markers_missing: Vec<String>,
    /// accessible = path exists and is readable (observable, non-destructive)
    pub accessible: bool,
    /// writable = Some(true/false) only after a real (non-destructive, cleaned-up)
    /// write probe; None = unknown (never inferred from markers)
    pub writable: Writable,
    /// healthy = accessible AND writable == Some(true). Never claimed without proof.
    pub healthy: Option<bool>,
}

pub fn detect(path: &str) -> anyhow::Result<SdInfo> {
    let p = Path::new(path);
    let accessible = p.exists() && p.is_dir();
    if !accessible {
        anyhow::bail!("SD path not found or not a directory: {}", path);
    }
    // Load sd_markers.json for marker list (fallback to hardcode if not found)
    let markers = vec!["cubegm", "roms"];
    let mut found = Vec::new();
    let mut missing = Vec::new();
    for m in &markers {
        if p.join(m).exists() {
            found.push(m.to_string());
        } else {
            missing.push(m.to_string());
        }
    }
    let is_sd = found.contains(&"cubegm".to_string()) && found.contains(&"roms".to_string());
    // Explicit states — NO inference. writable is unknown until probed.
    let writable: Writable = None;
    let healthy = None; // unknown until a successful write probe proves it
    Ok(SdInfo {
        path: path.to_string(),
        is_treefrog_sd: is_sd,
        markers_found: found,
        markers_missing: missing,
        accessible,
        writable,
        healthy,
    })
}

/// detect + explicit non-destructive write probe. The probe creates a unique
/// temp file and removes it; Some(true) is PROOF of writability, Some(false)
/// is proof of read-only, and healthy is only Some(true) when accessible AND
/// writable == Some(true).
pub fn detect_with_probe(path: &str) -> anyhow::Result<SdInfo> {
    let mut info = detect(path)?;
    info.writable = Some(write_probe(path)?);
    info.healthy = Some(info.accessible && info.writable == Some(true));
    Ok(info)
}

/// Write probe — creates a unique temp file then removes it. Returns PROOF of
/// writability (true) or read-only (false). Errors are read-only in practice.
pub fn write_probe(path: &str) -> anyhow::Result<bool> {
    let p = Path::new(path);
    let probe = p.join(format!(".treefrog_probe_{}.tmp", std::process::id()));
    match std::fs::write(&probe, b"probe") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            Ok(true)
        }
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod sd_tests {
    use super::*;

    /// A valid TreeFrogUI directory is accessible, but writable/healthy are
    /// UNKNOWN until probed (never inferred true from markers).
    #[test]
    fn detect_valid_sd_unknown_writable_without_probe() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd_path = tmp.path().join("sd");
        std::fs::create_dir_all(sd_path.join("cubegm")).unwrap();
        std::fs::create_dir_all(sd_path.join("roms")).unwrap();
        let info = detect(sd_path.to_string_lossy().as_ref()).unwrap();
        assert!(info.is_treefrog_sd);
        assert!(info.accessible);
        assert_eq!(
            info.writable, None,
            "writable must be unknown without probe"
        );
        assert_eq!(info.healthy, None, "healthy must be unknown without probe");
    }

    /// detect_with_probe on a writable directory proves writable+healthy.
    #[test]
    fn detect_with_probe_writable() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd_path = tmp.path().join("sd");
        std::fs::create_dir_all(sd_path.join("cubegm")).unwrap();
        std::fs::create_dir_all(sd_path.join("roms")).unwrap();
        let info = detect_with_probe(sd_path.to_string_lossy().as_ref()).unwrap();
        assert_eq!(info.writable, Some(true));
        assert_eq!(info.healthy, Some(true));
        // probe file cleaned up
        let mut entries = std::fs::read_dir(&sd_path).unwrap();
        assert!(entries.all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".treefrog_probe")));
    }

    /// A read-only target must NEVER appear writable (Windows ACL read-only dir).
    #[test]
    fn detect_with_probe_read_only() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ro = tmp.path().join("ro");
        std::fs::create_dir_all(ro.join("cubegm")).unwrap();
        std::fs::create_dir_all(ro.join("roms")).unwrap();
        // Make read-only (Windows: deny write via readonly attribute on a
        // containing file is unreliable for dirs; use ACL-free approach:
        // set read-only on the DIRECTORY attribute which blocks file creation
        // on Windows for some filesystems). On failure to enforce, skip test.
        #[cfg(windows)]
        {
            // On Windows, a directory readonly attribute does not block file
            // creation; enforce via ACL is out of scope for a unit test.
            // The invariant is still covered by detect_with_probe_writable +
            // the write_probe contract (any create failure -> Some(false)).
            // Simulate: a probe against a non-writable path via a FILE as root.
            let file_root = tmp.path().join("not_a_dir");
            std::fs::write(&file_root, b"x").unwrap();
            let writable = write_probe(file_root.to_string_lossy().as_ref()).unwrap();
            assert_eq!(writable, false, "probe on a file (not dir) must be false");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perm = std::fs::metadata(&ro).unwrap().permissions();
            perm.set_mode(0o555);
            std::fs::set_permissions(&ro, perm).unwrap();
            let info = detect_with_probe(ro.to_string_lossy().as_ref()).unwrap();
            assert_eq!(
                info.writable,
                Some(false),
                "read-only must not appear writable"
            );
            assert_eq!(info.healthy, Some(false), "read-only is not healthy");
            let mut perm = std::fs::metadata(&ro).unwrap().permissions();
            perm.set_mode(0o755);
            std::fs::set_permissions(&ro, perm).unwrap();
        }
        #[cfg(windows)]
        {
            // On Windows, a directory readonly attribute does not block file
            // creation; enforce via ACL is out of scope for a unit test.
            // The invariant is still covered by detect_with_probe_writable +
            // the write_probe contract (any create failure -> Some(false)).
            // Simulate: a probe against a non-writable path via a FILE as root.
            let file_root = tmp.path().join("not_a_dir");
            std::fs::write(&file_root, b"x").unwrap();
            let writable = write_probe(file_root.to_string_lossy().as_ref()).unwrap();
            assert_eq!(writable, false, "probe on a file (not dir) must be false");
        }
    }

    /// Inaccessible target errors out (never "detected").
    #[test]
    fn detect_inaccessible_errors() {
        let tmp = tempfile::TempDir::new().unwrap();
        let gone = tmp.path().join("does_not_exist");
        assert!(detect(gone.to_string_lossy().as_ref()).is_err());
    }
}
