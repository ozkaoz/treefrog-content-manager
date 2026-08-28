use crate::profile::LoadedProfile;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Rom,
    Music,
    Video,
    Image,
    Ebook,
    Bios,
    LgptSample,
    LgptProject,
    Archive,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Classification {
    pub kind: Kind,
    pub system_id: Option<String>,
    pub destination: String,
    pub archive_valid: bool,
    pub multi_file: bool,
}

pub fn classify(path: &Path, profile: &LoadedProfile) -> Classification {
    let ext = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e.to_lowercase())).unwrap_or_default();
    let lower_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();

    // Archives recognized first — but we still peek inside later via archive.rs
    if [".zip", ".7z", ".rar"].contains(&ext.as_str()) {
        // Determine target system by inspecting sibling? For now generic archive kind
        return Classification {
            kind: Kind::Archive,
            system_id: None,
            destination: String::new(), // resolved after inspection
            archive_valid: false,
            multi_file: false,
        };
    }

    // Media by extension (music handled via media.json formats, but we mirror here)
    let music_exts = [".mp3", ".m4a", ".aac", ".wav", ".flac", ".ogg", ".opus"];
    if music_exts.contains(&ext.as_str()) {
        return Classification {
            kind: Kind::Music,
            system_id: None,
            destination: "roms/music".into(),
            archive_valid: false,
            multi_file: false,
        };
    }
    let video_exts = [".mp4", ".mkv", ".avi", ".mov", ".m4v", ".wmv", ".mpg", ".mpeg", ".ts", ".webm"];
    if video_exts.contains(&ext.as_str()) {
        return Classification {
            kind: Kind::Video,
            system_id: None,
            destination: "roms/videos".into(),
            archive_valid: false,
            multi_file: false,
        };
    }
    let image_exts = [".jpg", ".jpeg", ".png", ".bmp", ".gif", ".webp", ".tiff", ".tif", ".tga", ".ico"];
    if image_exts.contains(&ext.as_str()) {
        // Note: .res artwork folders are handled separately; still image kind but destination may be ignored if inside .res
        if path.components().any(|c| c.as_os_str() == ".res" || c.as_os_str() == "Imgs" || c.as_os_str() == "images") {
            return Classification {
                kind: Kind::Image,
                system_id: None,
                destination: ".res".into(),
                archive_valid: false,
                multi_file: false,
            };
        }
        return Classification {
            kind: Kind::Image,
            system_id: None,
            destination: "roms/images".into(),
            archive_valid: false,
            multi_file: false,
        };
    }
    let ebook_exts = [".epub", ".mobi", ".pdf", ".cbz", ".fb2", ".xps"];
    if ebook_exts.contains(&ext.as_str()) {
        return Classification {
            kind: Kind::Ebook,
            system_id: None,
            destination: "roms/Ebook".into(),
            archive_valid: false,
            multi_file: false,
        };
    }

    // LGPT hints
    if lower_name.ends_with(".wav") || lower_name.ends_with(".flac") || lower_name.ends_with(".aiff") {
        // Could be lgpt sample if path contains lgpt/samples — but generic music already handled
        // keep as music unless parent indicates lgpt
        if path.to_string_lossy().to_lowercase().contains("lgpt") {
            return Classification {
                kind: Kind::LgptSample,
                system_id: None,
                destination: "lgpt/samples".into(),
                archive_valid: false,
                multi_file: false,
            };
        }
    }
    if ext == ".lgpt" || (path.parent().map(|p| p.to_string_lossy().to_lowercase().contains("projects")).unwrap_or(false)) {
        return Classification {
            kind: Kind::LgptProject,
            system_id: None,
            destination: "lgpt/projects".into(),
            archive_valid: false,
            multi_file: true,
        };
    }

    // BIOS by name patterns (from bios.json)
    let bios_patterns = [
        "scph", "gba_bios.bin", "o2rom.bin", "disksys.rom", "neogeo.zip", "bios_cd", "kick13.rom", "kick20.rom", "pcfx.rom", "x86boot.img",
    ];
    for pat in bios_patterns {
        if lower_name.contains(pat) {
            return Classification {
                kind: Kind::Bios,
                system_id: None,
                destination: "cubegm/bios".into(),
                archive_valid: false,
                multi_file: false,
            };
        }
    }

    // ROM by profile: extension maps to system
    if let Some(ids) = profile.ext_to_system.get(&ext) {
        // pick first system for destination; keep system_id
        let sys_id = ids[0].clone();
        let sys = profile.systems.iter().find(|s| s.id == sys_id);
        let folder = sys.and_then(|s| s.folder_aliases.first()).map(|f| format!("roms/{}", f)).unwrap_or_else(|| "roms/UNKNOWN".into());
        let multi = sys.and_then(|s| s.multi_file).unwrap_or(false);
        let archive_valid = sys.and_then(|s| s.archive_payload_valid.as_ref()).map(|v| v.iter().any(|e| e.to_lowercase()==ext)).unwrap_or(false);
        return Classification {
            kind: Kind::Rom,
            system_id: Some(sys_id),
            destination: folder,
            archive_valid,
            multi_file: multi,
        };
    }

    // Fallback: unknown -> let user decide, but we still propose roms/UNKNOWN for dry-run visibility
    Classification {
        kind: Kind::Unknown,
        system_id: None,
        destination: "roms/UNKNOWN".into(),
        archive_valid: false,
        multi_file: false,
    }
}
