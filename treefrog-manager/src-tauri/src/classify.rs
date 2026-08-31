use crate::profile::LoadedProfile;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
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
    Ambiguous,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Classification {
    pub kind: Kind,
    pub system_id: Option<String>,
    pub destination: String,
    pub archive_valid: bool,
    pub multi_file: bool,
    pub possible_destinations: Option<Vec<String>>,
}

pub fn classify(path: &Path, profile: &LoadedProfile) -> Classification {
    let ext = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e.to_lowercase())).unwrap_or_default();
    let lower_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    let lower_path = path.to_string_lossy().to_lowercase().replace('\\', "/");
    // Prioridad absoluta de BIOS: todo dentro de cubegm/bios es BIOS, sin importar extensión
    if lower_path.contains("/cubegm/bios/") {
        return Classification { kind: Kind::Bios, system_id: None, destination: "cubegm/bios".into(), archive_valid: false, multi_file: false, possible_destinations: None };
    }
    if lower_path.contains("/frogui/") || lower_path.contains("/cubegm/cores/") {
        return Classification { kind: Kind::Unknown, system_id: None, destination: "".into(), archive_valid: false, multi_file: false, possible_destinations: None };
    }

    // .nes SIEMPRE a FC (fceumm) — antes que cualquier otra lógica de ROM
    if ext == ".nes" {
        return Classification {
            kind: Kind::Rom,
            system_id: Some("nes_fceumm".into()),
            destination: "roms/FC".into(),
            archive_valid: false,
            multi_file: false,
            possible_destinations: None,
        };
    }

    // .cue/.bin con contexto de carpeta padre — no UNKNOWN, default PS
    if ext == ".cue" || ext == ".bin" {
        if let Some(parent_name) = path.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_lowercase()) {
            if let Some(sys_id) = profile.alias_to_system.get(&parent_name) {
                let sys = profile.systems.iter().find(|s| &s.id == sys_id);
                let folder = sys.and_then(|s| s.folder_aliases.first()).map(|f| format!("roms/{}", f)).unwrap_or_else(|| "roms/PS".into());
                return Classification {
                    kind: Kind::Rom,
                    system_id: Some(sys_id.clone()),
                    destination: folder,
                    archive_valid: false,
                    multi_file: true,
                    possible_destinations: None,
                };
            }
        }
        let sys = profile.systems.iter().find(|s| s.id == "ps_psx");
        let folder = sys.and_then(|s| s.folder_aliases.first()).map(|f| format!("roms/{}", f)).unwrap_or_else(|| "roms/PS".into());
        return Classification {
            kind: Kind::Rom,
            system_id: Some("ps_psx".into()),
            destination: folder,
            archive_valid: false,
            multi_file: true,
            possible_destinations: None,
        };
    }

    // Archives recognized first — but we still peek inside later via archive.rs
    if [".zip", ".7z", ".rar"].contains(&ext.as_str()) {
        // Determine target system by inspecting sibling? For now generic archive kind
        return Classification {
            kind: Kind::Archive,
            system_id: None,
            destination: String::new(), // resolved after inspection
            archive_valid: false,
            multi_file: false,
            possible_destinations: None,
        };
    }

    // LGPT - profile-driven destinations, WAV baseline (check before generic music)
    let lgpt_samples = "lgpt/samples";
    let lgpt_projects = "lgpt/projects";
    if lower_name.ends_with(".wav") || lower_name.ends_with(".flac") || lower_name.ends_with(".aiff") || lower_name.ends_with(".aif") || lower_name.ends_with(".mp3") || lower_name.ends_with(".ogg") {
        if path.to_string_lossy().to_lowercase().contains("lgpt") {
            return Classification {
                kind: Kind::LgptSample,
                system_id: None,
                destination: lgpt_samples.into(),
                archive_valid: false,
                multi_file: false,
            possible_destinations: None,
            };
        }
        if path.to_string_lossy().to_lowercase().contains("samples") && ext == ".wav" {
            return Classification {
                kind: Kind::LgptSample,
                system_id: None,
                destination: lgpt_samples.into(),
                archive_valid: false,
                multi_file: false,
            possible_destinations: None,
            };
        }
    }
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            let count = entries.count();
            if count > 1 && path.to_string_lossy().to_lowercase().contains("project") {
                return Classification {
                    kind: Kind::LgptProject,
                    system_id: None,
                    destination: lgpt_projects.into(),
                    archive_valid: false,
                    multi_file: true,
            possible_destinations: None,
                };
            }
            if path.join("lgptsav.dat").exists() || path.join("project.lgpt").exists() {
                return Classification {
                    kind: Kind::LgptProject,
                    system_id: None,
                    destination: lgpt_projects.into(),
                    archive_valid: false,
                    multi_file: true,
            possible_destinations: None,
                };
            }
        }
    }
    if ext == ".lgpt" {
        return Classification {
            kind: Kind::LgptProject,
            system_id: None,
            destination: lgpt_projects.into(),
            archive_valid: false,
            multi_file: true,
            possible_destinations: None,
        };
    }
    if path.to_string_lossy().to_lowercase().contains("projects") && path.is_dir() {
        return Classification {
            kind: Kind::LgptProject,
            system_id: None,
            destination: lgpt_projects.into(),
            archive_valid: false,
            multi_file: true,
            possible_destinations: None,
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
            possible_destinations: None,
        };
    }
    let video_exts = [".mp4", ".mkv", ".avi", ".mov", ".m4v", ".wmv", ".mpg", ".mpeg", ".ts", ".webm"];
    if video_exts.contains(&ext.as_str()) {
        let lower_path_v = path.to_string_lossy().to_lowercase().replace('\\', "/");
        // Only classify as Video if inside roms/videos/ (TreeFrogUI stock has 1 video there, not elsewhere)
        if lower_path_v.contains("roms/videos/") || lower_path_v.contains("/videos/") {
            return Classification {
                kind: Kind::Video,
                system_id: None,
                destination: "roms/videos".into(),
                archive_valid: false,
                multi_file: false,
            possible_destinations: None,
            };
        }
        // Fall through to Unknown for videos outside roms/videos/ (will go to roms/UNKNOWN)
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
            possible_destinations: None,
            };
        }
        return Classification {
            kind: Kind::Image,
            system_id: None,
            destination: "roms/images".into(),
            archive_valid: false,
            multi_file: false,
            possible_destinations: None,
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
            possible_destinations: None,
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
            possible_destinations: None,
            };
        }
    }

    // Special handling for CUE/BIN: disambiguate PS1 vs MD/SegaCD via path/content/size
    if ext == ".cue" || ext == ".bin" {
        let lower_path = path.to_string_lossy().to_lowercase();
        // Heuristic: if path contains PS indicators, force PS
        let is_ps_hint = lower_path.contains("ps") || lower_path.contains("playstation") || lower_path.contains("psx") || lower_path.contains("ps1");
        let is_segacd_hint = lower_path.contains("segacd") || lower_path.contains("mega cd") || lower_path.contains("sega cd");
        if is_ps_hint {
            if let Some(sys) = profile.systems.iter().find(|s| s.id == "ps_psx") {
                let folder = sys.folder_aliases.first().map(|f| format!("roms/{}", f)).unwrap_or_else(|| "roms/PS".into());
                return Classification { kind: Kind::Rom, system_id: Some("ps_psx".into()), destination: folder, archive_valid: false, multi_file: true, possible_destinations: None };
            }
        }
        if is_segacd_hint {
            if let Some(sys) = profile.systems.iter().find(|s| s.id == "segacd") {
                let folder = sys.folder_aliases.first().map(|f| format!("roms/{}", f)).unwrap_or_else(|| "roms/segacd".into());
                return Classification { kind: Kind::Rom, system_id: Some("segacd".into()), destination: folder, archive_valid: false, multi_file: true, possible_destinations: None };
            }
        }
        // For .cue, check content for PS-specific strings
        if ext == ".cue" {
            if let Ok(content) = std::fs::read_to_string(path) {
                let upper = content.to_uppercase();
                if upper.contains("PLAYSTATION") || upper.contains("PSX") {
                    if let Some(sys) = profile.systems.iter().find(|s| s.id == "ps_psx") {
                        let folder = sys.folder_aliases.first().map(|f| format!("roms/{}", f)).unwrap_or_else(|| "roms/PS".into());
                        return Classification { kind: Kind::Rom, system_id: Some("ps_psx".into()), destination: folder, archive_valid: false, multi_file: true, possible_destinations: None };
                    }
                }
            }
        }
        // For .bin, large file (>50MB) is likely PS/SegaCD, not MD (MD ROMs are <4MB)
        if ext == ".bin" {
            if let Ok(meta) = std::fs::metadata(path) {
                if meta.len() > 50 * 1024 * 1024 {
                    // Large BIN -> PS or SegaCD, default to PS (most common for TreeFrogUI)
                    if let Some(sys) = profile.systems.iter().find(|s| s.id == "ps_psx") {
                        let folder = sys.folder_aliases.first().map(|f| format!("roms/{}", f)).unwrap_or_else(|| "roms/PS".into());
                        return Classification { kind: Kind::Rom, system_id: Some("ps_psx".into()), destination: folder, archive_valid: false, multi_file: true, possible_destinations: None };
                    }
                }
            }
        }
        // Fallback: for CUE/BIN without clear hint, mark as Ambiguous for user to choose (PS vs SegaCD)
        return Classification {
            kind: Kind::Ambiguous,
            system_id: None,
            destination: "roms/UNKNOWN".into(),
            archive_valid: false,
            multi_file: true,
            possible_destinations: Some(vec!["ps_psx".to_string(), "segacd".to_string()]),
        };
    }

    // NOTA UX: Para extensiones compartidas (.cue, .bin, .iso) el usuario DEBE organizar sus archivos
    // en carpetas separadas en el PC de origen (ej. D:\ROMs\PS1\ y D:\ROMs\SegaCD\).
    // El gestor usa el nombre de la carpeta padre para clasificar (context-aware). Si todos
    // los .cue/.bin están en una sola carpeta sin pista del sistema, no puede adivinar
    // y usará la heurística genérica (PS por defecto para .cue, tamaño para .bin).
    // Ver systems.json para la tabla completa de alias -> sistema.

    // Forzar .nes a FC (alta compatibilidad) — TreeFrogUI usa FC para fceumm, NES para quicknes (mappers)
    if ext == ".nes" {
        return Classification { kind: Kind::Rom, system_id: Some("nes_fceumm".into()), destination: "roms/FC".into(), archive_valid: false, multi_file: false, possible_destinations: None };
    }

    // Clasificación Contextual (Context-Aware): si la carpeta padre coincide con un alias, usar ese sistema
    if let Some(parent_name) = path.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_lowercase()) {
        if let Some(sys_id) = profile.alias_to_system.get(&parent_name) {
            let sys = profile.systems.iter().find(|s| &s.id == sys_id);
            let folder = sys.and_then(|s| s.folder_aliases.first()).map(|f| format!("roms/{}", f)).unwrap_or_else(|| "roms/UNKNOWN".into());
            let multi = sys.and_then(|s| s.multi_file).unwrap_or(false);
            let archive_valid = sys.and_then(|s| s.archive_payload_valid.as_ref()).map(|v| v.iter().any(|e| e.to_lowercase()==ext)).unwrap_or(false);
            return Classification { kind: Kind::Rom, system_id: Some(sys_id.clone()), destination: folder, archive_valid, multi_file: multi, possible_destinations: None };
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
            possible_destinations: None,
        }
    }

    // Fallback: unknown -> no destination (planner will skip, evita crear roms/UNKNOWN)
    Classification {
        kind: Kind::Unknown,
        system_id: None,
        destination: "".into(),
        archive_valid: false,
        multi_file: false,
        possible_destinations: None,
    }
}
