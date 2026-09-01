//! Canonical destination validation and resolution.
//!
//! SINGLE source of truth for every path that this application writes to an
//! SD card (or any target root). Every writer MUST go through
//! [`resolve_validated_destination`]; a destination that has not been
//! resolved through this function must never be written.
//!
//! Enforced properties of `relative_destination`:
//! - rejects absolute Unix paths (`/evil`)
//! - rejects absolute Windows paths (`\evil`)
//! - rejects drive-letter paths (`C:\evil`, `C:evil`, `C:/evil`)
//! - rejects UNC paths (`\\server\share`, `\\?\...`)
//! - rejects any `..` component (before AND after separator normalization)
//! - rejects empty components (`roms//x`, `roms/x/`)
//! - rejects Windows reserved device names (`CON`, `NUL`, `COM1`..)
//! - rejects illegal Windows characters (`<>:"\|?*`)
//! - rejects ADS syntax (any `:` outside a validated drive prefix — none allowed here)
//! - rejects trailing dots/spaces per component
//! - normalizes `/` and `\` to a single canonical `/` form
//! - resolves the final path against `sd_root` and verifies containment
//!
//! The returned absolute path is the exact path the writer must use. Writers
//! must not re-derive, re-sanitize, or re-join the destination themselves.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const MAX_COMPONENT_LEN: usize = 255;
pub const MAX_DEPTH: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum DestinationError {
    #[error("destination is empty")]
    Empty,
    #[error("absolute unix path not allowed: {0}")]
    AbsoluteUnix(String),
    #[error("absolute windows path not allowed: {0}")]
    AbsoluteWindows(String),
    #[error("drive-letter path not allowed: {0}")]
    DriveLetter(String),
    #[error("UNC path not allowed: {0}")]
    Unc(String),
    #[error("parent traversal (`..`) not allowed: {0}")]
    Traversal(String),
    #[error("empty path component not allowed: {0}")]
    EmptyComponent(String),
    #[error("reserved windows name not allowed: {0}")]
    ReservedName(String),
    #[error("illegal character {ch:?} in component {component:?}")]
    IllegalCharacter { ch: char, component: String },
    #[error("alternate data stream (`:`) not allowed: {0}")]
    Ads(String),
    #[error("component too long (max {MAX_COMPONENT_LEN}): {0}")]
    ComponentTooLong(String),
    #[error("path too deep (max {MAX_DEPTH} components): {0}")]
    TooDeep(String),
    #[error("resolved destination escapes target root: {resolved}")]
    EscapesRoot { resolved: String },
    #[error("destination must be relative, got absolute: {0}")]
    NotRelative(String),
}

const ILLEGAL_CHARS: [char; 7] = ['<', '>', ':', '"', '|', '?', '*'];

const RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// True when the string starts with a drive letter pattern (`C:`, `C:\`, `C:/`).
fn starts_with_drive_letter(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 2
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && (b.len() == 2 || b[2] == b'/' || b[2] == b'\\')
}

/// True for `\\server\share`, `\\?\...`, `\\.\...` style prefixes.
fn is_unc_prefix(s: &str) -> bool {
    s.starts_with("\\\\")
}

/// Canonicalize separators: mixed `\` and `/` become `/`; strip nothing else.
/// Duplicate separators produce empty components which are rejected later.
fn normalize_separators(s: &str) -> String {
    s.replace('\\', "/")
}

/// Validate a *relative* destination string (no SD root involved).
/// Returns the normalized (`/`-separated) relative path.
pub fn validate_relative_destination(
    relative_destination: &str,
) -> Result<String, DestinationError> {
    if relative_destination.trim().is_empty() {
        return Err(DestinationError::Empty);
    }
    // Reject absolute forms BEFORE normalization so we can give precise errors.
    if relative_destination.starts_with('/') {
        return Err(DestinationError::AbsoluteUnix(
            relative_destination.to_string(),
        ));
    }
    if relative_destination.starts_with('\\') {
        if is_unc_prefix(relative_destination) {
            return Err(DestinationError::Unc(relative_destination.to_string()));
        }
        return Err(DestinationError::AbsoluteWindows(
            relative_destination.to_string(),
        ));
    }
    if is_unc_prefix(relative_destination) {
        return Err(DestinationError::Unc(relative_destination.to_string()));
    }
    if starts_with_drive_letter(relative_destination) {
        return Err(DestinationError::DriveLetter(
            relative_destination.to_string(),
        ));
    }
    // Any remaining `:` is ADS syntax (a `:` after position 1 that survived the
    // drive check). `C:x` was handled above; embedded colons like `file.txt:ads`
    // are rejected here per component below, but a leading `x:`-like pattern is
    // also caught by starts_with_drive_letter only when followed correctly, so
    // do an explicit global colon check after separator normalization.
    let normalized = normalize_separators(relative_destination);
    if normalized.starts_with('/') {
        return Err(DestinationError::AbsoluteUnix(
            relative_destination.to_string(),
        ));
    }
    if normalized.contains(':') {
        return Err(DestinationError::Ads(relative_destination.to_string()));
    }
    let parts: Vec<&str> = normalized.split('/').collect();
    if parts.len() > MAX_DEPTH {
        return Err(DestinationError::TooDeep(relative_destination.to_string()));
    }
    let mut clean = Vec::with_capacity(parts.len());
    for part in parts {
        if part.is_empty() {
            return Err(DestinationError::EmptyComponent(
                relative_destination.to_string(),
            ));
        }
        if part == ".." || part == "." {
            return Err(DestinationError::Traversal(
                relative_destination.to_string(),
            ));
        }
        // Traversal smuggled via percent/dot variants is covered because the
        // component must literally be `..` to traverse; other encodings are
        // literal directory names on FAT/exFAT.
        if part.len() > MAX_COMPONENT_LEN {
            return Err(DestinationError::ComponentTooLong(part.to_string()));
        }
        // Reserved Windows device names (with or without extension).
        let stem_upper = part.split('.').next().unwrap_or(part).to_uppercase();
        if RESERVED_NAMES.contains(&stem_upper.as_str()) {
            return Err(DestinationError::ReservedName(part.to_string()));
        }
        for ch in ILLEGAL_CHARS {
            if part.contains(ch) {
                return Err(DestinationError::IllegalCharacter {
                    ch,
                    component: part.to_string(),
                });
            }
        }
        if part.ends_with('.') || part.ends_with(' ') {
            return Err(DestinationError::IllegalCharacter {
                ch: if part.ends_with('.') { '.' } else { ' ' },
                component: part.to_string(),
            });
        }
        clean.push(part);
    }
    Ok(clean.join("/"))
}

/// Canonical destination resolver: validates `relative_destination`, joins it
/// to `sd_root`, resolves the result, and verifies it remains inside
/// `sd_root`. Returns the ABSOLUTE path the caller MUST write to.
///
/// This function is safe to call directly from any writer; callers do not
/// need to pre-validate anything.
pub fn resolve_validated_destination(
    sd_root: &Path,
    relative_destination: &str,
) -> Result<PathBuf, DestinationError> {
    let normalized = validate_relative_destination(relative_destination)?;
    if sd_root.as_os_str().is_empty() {
        return Err(DestinationError::Empty);
    }
    let root_resolved = sd_root
        .canonicalize()
        .unwrap_or_else(|_| sd_root.to_path_buf());
    // The destination may not exist yet, so we cannot canonicalize it.
    // Build it from the canonical root + validated normalized relative parts.
    let mut resolved = root_resolved.clone();
    for part in normalized.split('/') {
        resolved.push(part);
    }
    // Containment: resolved must be the canonical root followed by exactly
    // our validated components (no symlink escapes possible because we never
    // follow links — but re-verify lexically for defense in depth).
    if !resolved.starts_with(&root_resolved) || resolved == root_resolved {
        return Err(DestinationError::EscapesRoot {
            resolved: resolved.to_string_lossy().to_string(),
        });
    }
    // Defense in depth: if the parent chain exists, ensure the canonicalized
    // parent is still inside the root (catches pre-existing symlink dirs).
    if let Some(parent) = resolved.parent() {
        if let Ok(canon_parent) = parent.canonicalize() {
            if !canon_parent.starts_with(&root_resolved) {
                return Err(DestinationError::EscapesRoot {
                    resolved: resolved.to_string_lossy().to_string(),
                });
            }
        }
    }
    Ok(resolved)
}

/// Convenience wrapper for callers that already hold a normalized destination
/// (e.g. planner output) and only need re-validation after mutation.
pub fn validate_plan_destination(destination: &str) -> Result<String, DestinationError> {
    validate_relative_destination(destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(rel: &str) -> String {
        validate_relative_destination(rel).expect("should be valid")
    }
    fn err(rel: &str) -> DestinationError {
        validate_relative_destination(rel).expect_err("should be rejected")
    }

    #[test]
    fn accepts_valid_relative_destinations() {
        assert_eq!(ok("roms/FC/game.nes"), "roms/FC/game.nes");
        assert_eq!(ok("cubegm/bios/scph1001.bin"), "cubegm/bios/scph1001.bin");
        assert_eq!(ok("lgpt/samples/kick.wav"), "lgpt/samples/kick.wav");
        // Normalizes backslashes
        assert_eq!(ok("roms\\FC\\game.nes"), "roms/FC/game.nes");
        assert_eq!(ok("roms/FC/game (U) [!].nes"), "roms/FC/game (U) [!].nes");
    }

    #[test]
    fn rejects_traversal() {
        for bad in [
            "../evil.bin",
            "../../evil.bin",
            "cubegm/bios/../../evil.bin",
            "roms/..",
            "..",
            "roms/../..",
            "a/./b",
        ] {
            assert!(
                matches!(
                    err(bad),
                    DestinationError::Traversal(_) | DestinationError::AbsoluteUnix(_)
                ),
                "traversal must be rejected: {bad}"
            );
        }
    }

    #[test]
    fn rejects_absolute_and_drive_and_unc() {
        assert!(matches!(
            err("/evil.bin"),
            DestinationError::AbsoluteUnix(_)
        ));
        assert!(matches!(
            err("\\evil.bin"),
            DestinationError::AbsoluteWindows(_)
        ));
        assert!(matches!(
            err("C:\\evil.bin"),
            DestinationError::DriveLetter(_)
        ));
        assert!(matches!(
            err("C:/evil.bin"),
            DestinationError::DriveLetter(_)
        ));
        assert!(matches!(err("C:evil.bin"), DestinationError::Ads(_)));
        assert!(matches!(
            err("\\\\server\\share\\evil.bin"),
            DestinationError::Unc(_)
        ));
        assert!(matches!(
            err("\\\\?\\C:\\evil.bin"),
            DestinationError::Unc(_)
        ));
    }

    #[test]
    fn rejects_ads_and_illegal_and_reserved() {
        assert!(matches!(
            err("roms/file.txt:stream"),
            DestinationError::Ads(_)
        ));
        assert!(matches!(err("roms/CON"), DestinationError::ReservedName(_)));
        assert!(matches!(
            err("roms/con.txt"),
            DestinationError::ReservedName(_)
        ));
        assert!(matches!(
            err("roms/NUL.bin"),
            DestinationError::ReservedName(_)
        ));
        assert!(matches!(
            err("roms/com1"),
            DestinationError::ReservedName(_)
        ));
        let long = "x".repeat(300);
        assert!(matches!(err(&long), DestinationError::ComponentTooLong(_)));
        assert!(matches!(
            err("roms/<evil>"),
            DestinationError::IllegalCharacter { .. }
        ));
        assert!(matches!(
            err("roms/evil|.bin"),
            DestinationError::IllegalCharacter { .. }
        ));
        assert!(matches!(
            err("roms/what?.bin"),
            DestinationError::IllegalCharacter { .. }
        ));
        assert!(matches!(
            err("roms/evil*"),
            DestinationError::IllegalCharacter { .. }
        ));
        assert!(matches!(
            err("roms/\"quoted\""),
            DestinationError::IllegalCharacter { .. }
        ));
        assert!(matches!(
            err("roms/evil."),
            DestinationError::IllegalCharacter { .. }
        ));
        assert!(matches!(
            err("roms/evil "),
            DestinationError::IllegalCharacter { .. }
        ));
    }

    #[test]
    fn rejects_empty_components_and_deep_paths() {
        assert!(matches!(
            err("roms//x"),
            DestinationError::EmptyComponent(_)
        ));
        assert!(matches!(
            err("roms/x/"),
            DestinationError::EmptyComponent(_)
        ));
        assert!(matches!(err(""), DestinationError::Empty));
        assert!(matches!(err("   "), DestinationError::Empty));
        let deep = vec!["a"; 30].join("/");
        assert!(matches!(err(&deep), DestinationError::TooDeep(_)));
    }

    #[test]
    fn resolve_returns_contained_absolute_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let p = resolve_validated_destination(root, "roms/FC/game.nes").unwrap();
        let root_canon = root.canonicalize().unwrap();
        assert!(p.starts_with(&root_canon));
        assert!(p.ends_with("roms/FC/game.nes"));
    }

    #[test]
    fn resolve_rejects_escape() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        for bad in [
            "../evil.bin",
            "../../evil.bin",
            "cubegm/bios/../../evil.bin",
            "C:\\evil.bin",
            "\\\\server\\share\\evil.bin",
            "/evil.bin",
            "roms/../../evil",
        ] {
            assert!(
                resolve_validated_destination(root, bad).is_err(),
                "must reject escape: {bad}"
            );
        }
    }

    #[test]
    fn resolve_detects_existing_symlink_escape() {
        let outer = tempfile::TempDir::new().unwrap();
        let root = outer.path().join("sd_root");
        std::fs::create_dir_all(&root).unwrap();
        let evil = outer.path().join("evil.txt");
        std::fs::write(&evil, b"x").unwrap();
        // Existing symlink inside root pointing outside
        #[cfg(unix)]
        {
            std::os::unix::symlink(&evil, root.join("link")).unwrap();
        }
        #[cfg(windows)]
        {
            let _ = std::os::windows::fs::symlink_file(&evil, root.join("link"));
        }
        let res = resolve_validated_destination(&root, "link/evil.txt");
        // On success of symlink creation, this must be an escape error; if
        // symlink creation is not permitted on this host, skip silently.
        if root.join("link").exists() || root.join("link").symlink_metadata().is_ok() {
            assert!(res.is_err(), "symlink escape must be rejected");
        }
    }
}
