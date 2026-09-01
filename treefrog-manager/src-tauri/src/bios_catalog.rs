use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiosEntry {
    pub id: String,
    pub system_name: String,
    pub filenames: Vec<String>,
    pub pattern: Option<String>,
    pub destination: String,
    pub required: bool,
    pub sha256: Option<String>,
    pub md5: Option<String>,
    pub expected_size: Option<u64>,
    pub description: String,
}

/// BIOS catalog is derived from the DECLARATIVE profile (bios.json) — the
/// single authoritative BIOS validation model. No hardcoded BIOS lists here;
/// the catalog is a projection of the profile for the UI.
pub fn get_bios_catalog() -> Vec<BiosEntry> {
    let json = crate::bios_profile_json_public();
    let mut out: Vec<BiosEntry> = Vec::new();
    if let Some(defs) = json.get("bios_definitions").and_then(|v| v.as_array()) {
        for def in defs {
            let id = def
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let system_name = def
                .get("system_name")
                .or_else(|| def.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown system")
                .to_string();
            let mut filenames = Vec::new();
            if let Some(arr) = def.get("accepted_filenames").and_then(|v| v.as_array()) {
                for f in arr {
                    if let Some(s) = f.as_str() {
                        filenames.push(s.to_string());
                    }
                }
            }
            // variants contribute filenames too (merged projection)
            if let Some(vars) = def.get("variants").and_then(|v| v.as_array()) {
                for var in vars {
                    if let Some(arr) = var.get("filenames").and_then(|v| v.as_array()) {
                        for f in arr {
                            if let Some(s) = f.as_str() {
                                if !filenames.iter().any(|x| x.eq_ignore_ascii_case(s)) {
                                    filenames.push(s.to_string());
                                }
                            }
                        }
                    }
                }
            }
            let pattern = def
                .get("accepted_patterns")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.iter().find_map(|x| x.as_str().map(|s| s.to_string())));
            let destination = def
                .get("primary_destination")
                .or_else(|| def.get("destination"))
                .and_then(|v| v.as_str())
                .unwrap_or("cubegm/bios")
                .trim_end_matches('/')
                .to_string();
            let required_str = def
                .get("required")
                .and_then(|v| v.as_str())
                .unwrap_or("optional");
            let required = matches!(required_str, "required" | "conditional");
            // Single-hash projection: first declared hash (variants merged by bios.rs).
            let sha256 = def
                .get("hashes_sha256")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.iter().find_map(|x| x.as_str().map(|s| s.to_string())))
                .map(|s| s.to_string());
            let md5 = def
                .get("md5")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let expected_size = def.get("expected_size").and_then(|v| v.as_u64());
            let description = def
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            out.push(BiosEntry {
                id,
                system_name,
                filenames,
                pattern,
                destination,
                required,
                sha256,
                md5,
                expected_size,
                description,
            });
        }
    }
    // Deterministic order
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

#[cfg(test)]
mod bios_catalog_tests {
    use super::*;

    /// Catalog is derived from bios.json (single model): ids are stable,
    /// destinations match the declared profile, and no entry is fabricated.
    #[test]
    fn catalog_derived_from_profile() {
        let cat = get_bios_catalog();
        assert!(!cat.is_empty(), "bios.json must provide definitions");
        // Deterministic sort
        let mut sorted_ids: Vec<&String> = cat.iter().map(|b| &b.id).collect();
        sorted_ids.sort();
        let ids: Vec<&String> = cat.iter().map(|b| &b.id).collect();
        assert_eq!(sorted_ids, ids);
        // Every entry mirrors the profile's declared primary_destination
        let profile = crate::bios_profile_json_public();
        let defs = profile
            .get("bios_definitions")
            .and_then(|v| v.as_array())
            .unwrap();
        for b in &cat {
            let def = defs
                .iter()
                .find(|d| d.get("id").and_then(|x| x.as_str()) == Some(b.id.as_str()));
            let def =
                def.unwrap_or_else(|| panic!("catalog entry {} must exist in bios.json", b.id));
            let declared = def
                .get("primary_destination")
                .or_else(|| def.get("destination"))
                .and_then(|v| v.as_str())
                .unwrap_or("cubegm/bios");
            assert_eq!(
                b.destination,
                declared.trim_end_matches('/'),
                "destination must mirror profile for {}",
                b.id
            );
            assert!(
                !b.filenames.is_empty() || b.pattern.is_some(),
                "entry must declare names or pattern: {}",
                b.id
            );
        }
        // Spot-check a known definition (from bios.json, not hardcoded here)
        assert!(cat.iter().any(|b| b.id == "ps1_bios"
            && b.filenames
                .iter()
                .any(|f| f.eq_ignore_ascii_case("scph1001.bin"))));
    }
}
