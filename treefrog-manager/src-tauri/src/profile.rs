use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Manifest {
    pub schema_version: String,
    pub profile_version: String,
    pub files: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Profile {
    pub schema_version: String,
    pub profile_version: String,
    pub id: String,
    #[serde(default)]
    pub archive_policy: Option<ArchivePolicy>,
    #[serde(default)]
    pub duplicate_handling: Option<serde_json::Value>,
    #[serde(default)]
    pub sd_root_layout: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ArchivePolicy {
    pub supported_extensions: Vec<String>,
    pub nested_archives: Option<NestedPolicy>,
    pub safety: Option<serde_json::Value>,
    #[serde(default)]
    pub profile_file: Option<String>,
    #[serde(default)]
    pub abstraction: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NestedPolicy {
    pub max_depth: u32,
    pub max_entries_per_archive: u32,
    pub max_expansion_bytes: u64,
    pub max_total_files_per_job: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SystemEntry {
    pub id: String,
    pub folder_aliases: Vec<String>,
    pub display_name: Option<String>,
    pub core: Option<String>,
    pub extensions: Vec<String>,
    pub archive_payload_valid: Option<Vec<String>>,
    pub multi_file: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SystemsFile {
    pub systems: Vec<SystemEntry>,
}

#[derive(Debug, Clone)]
pub struct LoadedProfile {
    pub profile_version: String,
    pub systems: Vec<SystemEntry>,
    pub ext_to_system: HashMap<String, Vec<String>>,
    pub alias_to_system: HashMap<String, String>,
    pub archive_valid_exts: Vec<String>,
    pub archive_policy: ArchivePolicy,
    pub archive_policy_full: serde_json::Value,
    pub video_presets: serde_json::Value,
    pub video_preset: serde_json::Value,
}

// Embedded fallback for portable EXE (no external profiles required)
const EMBEDDED_PROFILE_JSON: &str = include_str!("../../../profiles/treefrogui/profile.json");
const EMBEDDED_SYSTEMS_JSON: &str = include_str!("../../../profiles/treefrogui/systems.json");
const EMBEDDED_ARCHIVE_POLICY_JSON: &str = include_str!("../../../profiles/treefrogui/archive_policy.json");
const EMBEDDED_VIDEO_PRESETS_JSON: &str = include_str!("../../../profiles/treefrogui/video_presets.json");

pub fn load_profile() -> anyhow::Result<LoadedProfile> {
    let candidates = [
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../profiles/treefrogui"),
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../profiles/treefrogui"),
        Path::new("profiles/treefrogui").to_path_buf(),
        Path::new("../profiles/treefrogui").to_path_buf(),
        // Portable: exe dir + profiles (for distribution as folder)
        std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("profiles/treefrogui"))).unwrap_or_else(|| Path::new("profiles/treefrogui").to_path_buf()),
        std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("../profiles/treefrogui"))).unwrap_or_else(|| Path::new("profiles/treefrogui").to_path_buf()),
    ];
    let mut base: Option<std::path::PathBuf> = None;
    for c in &candidates {
        if c.join("profile.json").exists() && c.join("systems.json").exists() {
            base = Some(c.clone());
            break;
        }
    }
    // Use filesystem if found, otherwise embedded
    let (profile_str, systems_str, archive_policy_str, video_presets_str) = if let Some(base) = base {
        let p = fs::read_to_string(base.join("profile.json"))?;
        let s = fs::read_to_string(base.join("systems.json"))?;
        let a = fs::read_to_string(base.join("archive_policy.json")).unwrap_or_else(|_| EMBEDDED_ARCHIVE_POLICY_JSON.to_string());
        let v = fs::read_to_string(base.join("video_presets.json")).unwrap_or_else(|_| EMBEDDED_VIDEO_PRESETS_JSON.to_string());
        (p, s, a, v)
    } else {
        // Portable fallback: embedded (no external files required)
        (
            EMBEDDED_PROFILE_JSON.to_string(),
            EMBEDDED_SYSTEMS_JSON.to_string(),
            EMBEDDED_ARCHIVE_POLICY_JSON.to_string(),
            EMBEDDED_VIDEO_PRESETS_JSON.to_string(),
        )
    };
    let profile: Profile = serde_json::from_str(&profile_str)?;
    let systems: SystemsFile = serde_json::from_str(&systems_str)?;
    let archive_policy_full: serde_json::Value = serde_json::from_str::<serde_json::Value>(&archive_policy_str).unwrap_or(serde_json::Value::Object(Default::default()));
    let video_presets: serde_json::Value = serde_json::from_str::<serde_json::Value>(&video_presets_str).unwrap_or(serde_json::Value::Object(Default::default()));
    let video_preset = video_presets.get("presets").and_then(|v| v.as_array()).and_then(|arr| arr.get(0)).cloned().unwrap_or(serde_json::Value::Object(Default::default()));
    let archive_policy = profile.archive_policy.clone().unwrap_or(ArchivePolicy {
        supported_extensions: vec![".zip".into(), ".7z".into(), ".rar".into()],
        nested_archives: Some(NestedPolicy { max_depth: 1, max_entries_per_archive: 1024, max_expansion_bytes: 1024*1024*1024, max_total_files_per_job: 10000 }),
        safety: None,
        profile_file: None,
        abstraction: None,
    });

    let mut ext_to_system: HashMap<String, Vec<String>> = HashMap::new();
    let mut alias_to_system: HashMap<String, String> = HashMap::new();
    // PRIORITY: keep JSON order — first system with a given extension wins (e.g. .nes -> FC before NES, .gba -> GBA before mgba)
    for sys in &systems.systems {
        for ext in &sys.extensions {
            let k = ext.to_lowercase();
            ext_to_system.entry(k).or_default().push(sys.id.clone());
        }
        for alias in &sys.folder_aliases {
            alias_to_system.insert(alias.to_lowercase(), sys.id.clone());
        }
    }
    Ok(LoadedProfile {
        profile_version: profile.profile_version,
        systems: systems.systems,
        ext_to_system,
        alias_to_system,
        archive_valid_exts: archive_policy.supported_extensions.clone(),
        archive_policy,
        archive_policy_full,
        video_presets,
        video_preset,
    })
}
