use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum BiosState {
    #[serde(rename = "missing")]
    Missing,
    #[serde(rename = "found_valid")]
    FoundValid,
    #[serde(rename = "found_invalid")]
    FoundInvalid,
    #[serde(rename = "found_unknown")]
    FoundUnknown,
    #[serde(rename = "duplicate")]
    Duplicate,
    #[serde(rename = "conflict")]
    Conflict,
    #[serde(rename = "not_required")]
    NotRequired,
}

impl std::fmt::Display for BiosState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BiosState::Missing => "missing",
            BiosState::FoundValid => "found_valid",
            BiosState::FoundInvalid => "found_invalid",
            BiosState::FoundUnknown => "found_unknown",
            BiosState::Duplicate => "duplicate",
            BiosState::Conflict => "conflict",
            BiosState::NotRequired => "not_required",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiosValidation {
    pub bios_id: String,
    pub system_id: Option<String>,
    pub state: BiosState,
    pub reason: String,
    pub required: bool,
    pub file: Option<String>,
    pub hash: Option<String>,
    pub size: Option<u64>,
}

fn match_filename(
    filename: &str,
    accepted: &[String],
    aliases: &[String],
    patterns: &[String],
) -> bool {
    let lower = filename.to_lowercase();
    for name in accepted {
        if lower == name.to_lowercase() {
            return true;
        }
    }
    for alias in aliases {
        if lower == alias.to_lowercase() {
            return true;
        }
    }
    for pat in patterns {
        // Simple fnmatch: check with wildcards
        if fnmatch(&lower, &pat.to_lowercase()) {
            return true;
        }
    }
    false
}

fn fnmatch(text: &str, pattern: &str) -> bool {
    // Very simple fnmatch supporting * and ?
    // For our BIOS patterns like "scph*.bin", "tos*.img", "*.rom"
    if pattern == "*" {
        return true;
    }
    if pattern.contains('*') || pattern.contains('?') {
        // Convert to regex
        let mut regex = String::from("^");
        for c in pattern.chars() {
            match c {
                '*' => regex.push_str(".*"),
                '?' => regex.push('.'),
                '.' => regex.push_str("\\."),
                _ => regex.push(c),
            }
        }
        regex.push('$');
        if let Ok(re) = regex::Regex::new(&regex) {
            return re.is_match(text);
        }
        false
    } else {
        text == pattern
    }
}

fn is_known_filename(filename: &str, bios_def: &serde_json::Value) -> bool {
    let mut accepted = Vec::new();
    let mut aliases = Vec::new();
    let mut patterns = Vec::new();
    if let Some(arr) = bios_def
        .get("accepted_filenames")
        .and_then(|v| v.as_array())
    {
        for v in arr {
            if let Some(s) = v.as_str() {
                accepted.push(s.to_string());
            }
        }
    }
    if let Some(arr) = bios_def.get("aliases").and_then(|v| v.as_array()) {
        for v in arr {
            if let Some(s) = v.as_str() {
                aliases.push(s.to_string());
            }
        }
    }
    if let Some(arr) = bios_def.get("accepted_patterns").and_then(|v| v.as_array()) {
        for v in arr {
            if let Some(s) = v.as_str() {
                patterns.push(s.to_string());
            }
        }
    }
    for var in bios_def
        .get("variants")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
    {
        if let Some(arr) = var.get("filenames").and_then(|v| v.as_array()) {
            for v in arr {
                if let Some(s) = v.as_str() {
                    accepted.push(s.to_string());
                }
            }
        }
        if let Some(arr) = var.get("aliases").and_then(|v| v.as_array()) {
            for v in arr {
                if let Some(s) = v.as_str() {
                    aliases.push(s.to_string());
                }
            }
        }
    }
    match_filename(filename, &accepted, &aliases, &patterns)
}

pub fn validate_bios_file(path: &Path, bios_def: &serde_json::Value) -> BiosValidation {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let bios_id = bios_def
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let system_id = bios_def
        .get("system_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    if !path.exists() {
        return BiosValidation {
            bios_id,
            system_id,
            state: BiosState::Missing,
            reason: "file not found".to_string(),
            required: false,
            file: Some(path.to_string_lossy().to_string()),
            hash: None,
            size: None,
        };
    }
    if !is_known_filename(&filename, bios_def) {
        return BiosValidation {
            bios_id,
            system_id,
            state: BiosState::FoundUnknown,
            reason: format!(
                "filename {} not in accepted list for {}",
                filename,
                bios_def
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
            ),
            required: false,
            file: Some(path.to_string_lossy().to_string()),
            hash: None,
            size: None,
        };
    }
    // Collect expected hashes/sizes
    let mut all_hashes: Vec<String> = Vec::new();
    let mut all_sizes: Vec<u64> = Vec::new();
    if let Some(arr) = bios_def.get("hashes_sha256").and_then(|v| v.as_array()) {
        for v in arr {
            if let Some(s) = v.as_str() {
                if !s.is_empty() {
                    all_hashes.push(s.to_lowercase());
                }
            }
        }
    }
    if let Some(sz) = bios_def.get("expected_size").and_then(|v| v.as_u64()) {
        all_sizes.push(sz);
    }
    for var in bios_def
        .get("variants")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
    {
        if let Some(arr) = var.get("hashes_sha256").and_then(|v| v.as_array()) {
            for v in arr {
                if let Some(s) = v.as_str() {
                    if !s.is_empty() {
                        all_hashes.push(s.to_lowercase());
                    }
                }
            }
        }
        if let Some(sz) = var.get("expected_size").and_then(|v| v.as_u64()) {
            all_sizes.push(sz);
        }
    }
    let has_known_hashes = !all_hashes.is_empty();
    let file_hash = crate::hash::sha256_file(path)
        .ok()
        .map(|s| s.to_lowercase());
    let file_size = path.metadata().ok().map(|m| m.len());
    // When the profile declares NO validation criteria (no hashes, no sizes —
    // e.g. neogeo.zip, segacd bios_CD_*, ecwolf.pk3), an exact/accepted
    // filename is a VALID selection: the user picked the file explicitly and
    // there is nothing stricter to compare against. The reason makes this
    // observable (validated by name only — profile declares no hash/size).
    // Old behavior returned FoundUnknown ("no hash/size to validate") which
    // made 9 of 13 BIOS unusable.
    if !has_known_hashes && all_sizes.is_empty() {
        let exact: Vec<String> = {
            let mut v = Vec::new();
            if let Some(arr) = bios_def
                .get("accepted_filenames")
                .and_then(|x| x.as_array())
            {
                for x in arr {
                    if let Some(s) = x.as_str() {
                        v.push(s.to_lowercase());
                    }
                }
            }
            for var in bios_def
                .get("variants")
                .and_then(|x| x.as_array())
                .unwrap_or(&vec![])
            {
                if let Some(arr) = var.get("filenames").and_then(|x| x.as_array()) {
                    for x in arr {
                        if let Some(s) = x.as_str() {
                            v.push(s.to_lowercase());
                        }
                    }
                }
            }
            v
        };
        let is_exact = exact.contains(&filename.to_lowercase());
        let reason = if is_exact {
            "exact filename accepted (profile declares no hash/size — validated by name)"
                .to_string()
        } else {
            "accepted alias/pattern (profile declares no hash/size — validated by name)".to_string()
        };
        return BiosValidation {
            bios_id,
            system_id,
            state: BiosState::FoundValid,
            reason,
            required: false,
            file: Some(path.to_string_lossy().to_string()),
            hash: file_hash,
            size: file_size,
        };
    }
    if has_known_hashes {
        if let Some(h) = &file_hash {
            if all_hashes.contains(h) {
                // Check exact vs alias
                let exact: Vec<String> = {
                    let mut v = Vec::new();
                    if let Some(arr) = bios_def
                        .get("accepted_filenames")
                        .and_then(|x| x.as_array())
                    {
                        for x in arr {
                            if let Some(s) = x.as_str() {
                                v.push(s.to_lowercase());
                            }
                        }
                    }
                    for var in bios_def
                        .get("variants")
                        .and_then(|x| x.as_array())
                        .unwrap_or(&vec![])
                    {
                        if let Some(arr) = var.get("filenames").and_then(|x| x.as_array()) {
                            for x in arr {
                                if let Some(s) = x.as_str() {
                                    v.push(s.to_lowercase());
                                }
                            }
                        }
                    }
                    v
                };
                let is_exact = exact.contains(&filename.to_lowercase());
                let reason = if is_exact {
                    "exact filename + known hash".to_string()
                } else {
                    "accepted alias + known hash".to_string()
                };
                return BiosValidation {
                    bios_id,
                    system_id,
                    state: BiosState::FoundValid,
                    reason,
                    required: false,
                    file: Some(path.to_string_lossy().to_string()),
                    hash: file_hash,
                    size: file_size,
                };
            } else {
                return BiosValidation {
                    bios_id,
                    system_id,
                    state: BiosState::FoundInvalid,
                    reason: format!(
                        "known filename {} but hash {} not in accepted",
                        filename,
                        file_hash.clone().unwrap_or_default()
                    ),
                    required: false,
                    file: Some(path.to_string_lossy().to_string()),
                    hash: file_hash,
                    size: file_size,
                };
            }
        }
    } else {
        if !all_sizes.is_empty() {
            if let Some(sz) = file_size {
                if all_sizes.contains(&sz) {
                    return BiosValidation {
                        bios_id,
                        system_id,
                        state: BiosState::FoundValid,
                        reason: "filename + expected size (no hash defined)".to_string(),
                        required: false,
                        file: Some(path.to_string_lossy().to_string()),
                        hash: file_hash,
                        size: file_size,
                    };
                } else {
                    return BiosValidation {
                        bios_id,
                        system_id,
                        state: BiosState::FoundInvalid,
                        reason: format!(
                            "filename {} size {} not in expected {:?}",
                            filename, sz, all_sizes
                        ),
                        required: false,
                        file: Some(path.to_string_lossy().to_string()),
                        hash: file_hash,
                        size: file_size,
                    };
                }
            }
        } else {
            if file_hash.is_some() {
                return BiosValidation {
                    bios_id: bios_id.clone(),
                    system_id,
                    state: BiosState::FoundUnknown,
                    reason: format!("filename {} known but no hash/size to validate", filename),
                    required: false,
                    file: Some(path.to_string_lossy().to_string()),
                    hash: file_hash,
                    size: file_size,
                };
            }
        }
    }
    BiosValidation {
        bios_id,
        system_id,
        state: BiosState::FoundUnknown,
        reason: "unknown BIOS".to_string(),
        required: false,
        file: Some(path.to_string_lossy().to_string()),
        hash: file_hash,
        size: file_size,
    }
}

pub fn validate_all_bios(
    source_files: &[std::path::PathBuf],
    bios_definitions: &[serde_json::Value],
    system_content_present: &HashMap<String, bool>,
) -> HashMap<String, BiosValidation> {
    let mut results: HashMap<String, BiosValidation> = HashMap::new();
    for bios_def in bios_definitions {
        let bios_id = bios_def
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let system_id = bios_def
            .get("system_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let required_str = bios_def
            .get("required")
            .and_then(|v| v.as_str())
            .unwrap_or("optional");
        let mut is_required = false;
        if required_str == "required" {
            is_required = true;
        } else if required_str == "conditional" {
            if let Some(sid) = &system_id {
                if *system_content_present.get(sid).unwrap_or(&false) {
                    is_required = true;
                }
            }
        }
        // Find matching files
        let mut matching: Vec<std::path::PathBuf> = Vec::new();
        for f in source_files {
            if let Some(name) = f.file_name().and_then(|n| n.to_str()) {
                if is_known_filename(name, bios_def) {
                    matching.push(f.clone());
                }
            }
        }
        if matching.is_empty() {
            let state = if is_required {
                BiosState::Missing
            } else {
                BiosState::NotRequired
            };
            let reason = if is_required {
                format!("BIOS {} missing but required", bios_id)
            } else {
                format!(
                    "BIOS {} not required (no {} content)",
                    bios_id,
                    system_id.clone().unwrap_or("unknown".to_string())
                )
            };
            results.insert(
                bios_id.clone(),
                BiosValidation {
                    bios_id: bios_id.clone(),
                    system_id: system_id.clone(),
                    state,
                    reason,
                    required: is_required,
                    file: None,
                    hash: None,
                    size: None,
                },
            );
            continue;
        }
        let mut validations: Vec<BiosValidation> = Vec::new();
        for f in &matching {
            validations.push(validate_bios_file(f, bios_def));
        }
        // Check for duplicate/conflict
        let mut hashes: HashMap<String, Vec<BiosValidation>> = HashMap::new();
        for v in &validations {
            if let Some(h) = &v.hash {
                hashes.entry(h.clone()).or_default().push(v.clone());
            }
        }
        let has_duplicate = hashes.values().any(|v| v.len() > 1);
        let mut filenames: HashMap<String, Vec<BiosValidation>> = HashMap::new();
        for v in &validations {
            if let Some(f) = &v.file {
                let name = Path::new(f)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                filenames.entry(name).or_default().push(v.clone());
            }
        }
        let mut has_conflict = false;
        for vs in filenames.values() {
            if vs.len() > 1 {
                let hs: std::collections::HashSet<String> =
                    vs.iter().filter_map(|v| v.hash.clone()).collect();
                if hs.len() > 1 {
                    has_conflict = true;
                }
            }
        }
        let valid_count = validations
            .iter()
            .filter(|v| v.state == BiosState::FoundValid)
            .count();
        let invalid_count = validations
            .iter()
            .filter(|v| v.state == BiosState::FoundInvalid)
            .count();
        let result = if has_conflict {
            BiosValidation {
                bios_id: bios_id.clone(),
                system_id: system_id.clone(),
                state: BiosState::Conflict,
                reason: format!("same BIOS filename with different content for {}", bios_id),
                required: is_required,
                file: None,
                hash: None,
                size: None,
            }
        } else if has_duplicate {
            BiosValidation {
                bios_id: bios_id.clone(),
                system_id: system_id.clone(),
                state: BiosState::Duplicate,
                reason: format!("duplicate identical BIOS files for {}", bios_id),
                required: is_required,
                file: None,
                hash: None,
                size: None,
            }
        } else if valid_count > 0 {
            BiosValidation {
                bios_id: bios_id.clone(),
                system_id: system_id.clone(),
                state: BiosState::FoundValid,
                reason: format!(
                    "found valid BIOS for {} ({} variants)",
                    bios_id, valid_count
                ),
                required: is_required,
                file: validations[0].file.clone(),
                hash: validations[0].hash.clone(),
                size: validations[0].size,
            }
        } else if invalid_count > 0 {
            BiosValidation {
                bios_id: bios_id.clone(),
                system_id: system_id.clone(),
                state: BiosState::FoundInvalid,
                reason: format!("found BIOS but invalid for {}", bios_id),
                required: is_required,
                file: validations[0].file.clone(),
                hash: validations[0].hash.clone(),
                size: validations[0].size,
            }
        } else {
            BiosValidation {
                bios_id: bios_id.clone(),
                system_id: system_id.clone(),
                state: BiosState::FoundUnknown,
                reason: format!("found BIOS with unknown validity for {}", bios_id),
                required: is_required,
                file: validations[0].file.clone(),
                hash: validations[0].hash.clone(),
                size: validations[0].size,
            }
        };
        results.insert(bios_id, result);
    }
    results
}

pub fn get_valid_destinations(bios_def: &serde_json::Value) -> Vec<String> {
    let mut dests = Vec::new();
    if let Some(arr) = bios_def.get("destinations").and_then(|v| v.as_array()) {
        for v in arr {
            if let Some(s) = v.as_str() {
                dests.push(s.trim_end_matches('/').to_string());
            }
        }
    }
    if dests.is_empty() {
        if let Some(s) = bios_def.get("primary_destination").and_then(|v| v.as_str()) {
            dests.push(s.trim_end_matches('/').to_string());
        } else if let Some(s) = bios_def.get("destination").and_then(|v| v.as_str()) {
            dests.push(s.trim_end_matches('/').to_string());
        } else if let Some(s) = bios_def.get("destination_root").and_then(|v| v.as_str()) {
            dests.push(s.trim_end_matches('/').to_string());
        }
    }
    dests
}

#[cfg(test)]
mod bios_selectable_tests {
    use super::*;

    /// Regression (user report 2026-09-01): "Selected: neogeo.zip - filename
    /// known but no hash/size to validate". When a BIOS definition declares
    /// NO hash and NO size (neogeo, segacd, pcfx, amiga kickstart, ecwolf.pk3
    /// ...), the accepted filename IS the validation: the user picked the
    /// file explicitly. Old behavior made 9 of 13 BIOS unusable.
    #[test]
    fn bios_without_criteria_selectable_by_name() {
        let defs = crate::bios_profile_json()
            .get("bios_definitions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        assert!(!defs.is_empty());
        let tmp = tempfile::TempDir::new().unwrap();

        // neogeo.zip - the exact user report
        let neogeo = defs
            .iter()
            .find(|d| d.get("id").and_then(|x| x.as_str()) == Some("neogeo_bios"))
            .unwrap();
        let f = tmp.path().join("neogeo.zip");
        std::fs::write(&f, b"fake neogeo bios").unwrap();
        let res = validate_bios_file(&f, neogeo);
        assert_eq!(
            res.state,
            BiosState::FoundValid,
            "neogeo.zip must be selectable: {}",
            res.reason
        );
        assert!(
            res.reason.contains("name"),
            "reason must be observable: {}",
            res.reason
        );

        // segacd bios_CD_U.bin
        let segacd = defs
            .iter()
            .find(|d| d.get("id").and_then(|x| x.as_str()) == Some("segacd_bios"))
            .unwrap();
        let f2 = tmp.path().join("bios_CD_U.bin");
        std::fs::write(&f2, b"segacd").unwrap();
        let res2 = validate_bios_file(&f2, segacd);
        assert_eq!(res2.state, BiosState::FoundValid, "{}", res2.reason);
    }

    /// BIOS WITH a declared hash still validates by hash (the fix does not
    /// relax hash checking), and with size still validates by size.
    #[test]
    fn bios_with_criteria_still_strict() {
        let defs = crate::bios_profile_json()
            .get("bios_definitions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let tmp = tempfile::TempDir::new().unwrap();

        // gba_bios has a SHA-256 declared: wrong content -> invalid
        let gba = defs
            .iter()
            .find(|d| d.get("id").and_then(|x| x.as_str()) == Some("gba_bios"))
            .unwrap();
        let f = tmp.path().join("gba_bios.bin");
        std::fs::write(&f, b"wrong content").unwrap();
        let res = validate_bios_file(&f, gba);
        assert_eq!(
            res.state,
            BiosState::FoundInvalid,
            "hash-declared BIOS must stay strict"
        );

        // ps1_bios has expected_size 524288: right size -> valid, wrong -> invalid
        let ps1 = defs
            .iter()
            .find(|d| d.get("id").and_then(|x| x.as_str()) == Some("ps1_bios"))
            .unwrap();
        let f2 = tmp.path().join("scph1001.bin");
        std::fs::write(&f2, vec![0u8; 524288]).unwrap();
        let res2 = validate_bios_file(&f2, ps1);
        assert_eq!(
            res2.state,
            BiosState::FoundValid,
            "right size must validate"
        );
        let f3 = tmp.path().join("scph1002.bin");
        std::fs::write(&f3, b"tiny").unwrap();
        let res3 = validate_bios_file(&f3, ps1);
        assert_eq!(res3.state, BiosState::FoundInvalid, "wrong size must fail");
    }
}
