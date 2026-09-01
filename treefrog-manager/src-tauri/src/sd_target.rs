use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VolumeInfo {
    pub path: String,
    pub label: Option<String>,
    pub filesystem: Option<String>,
    pub total_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub removable: Option<bool>,
    pub accessible: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub serial: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TargetAnalysis {
    pub path: String,
    pub volume: VolumeInfo,
    pub status: String, // valid, incomplete, unknown, inaccessible, stale_target
    pub is_treefrog: bool,
    pub is_incomplete: bool,
    pub markers_found: Vec<String>,
    pub markers_missing: Vec<String>,
    pub lgpt_detected: bool,
    pub rom_dirs: Vec<String>,
    pub media_dirs: Vec<String>,
    pub bios_dirs: Vec<String>,
    pub lgpt_dirs: Vec<String>,
    pub existing_count: usize,
    pub total_size: u64,
    pub free_bytes: Option<u64>,
    pub capacity_bytes: Option<u64>,
    pub filesystem: Option<String>,
    pub label: Option<String>,
    pub errors: Vec<String>,
    pub stable_id: Option<String>,
    pub physical_device: Option<PhysicalDevice>,
    pub folder_breakdown: std::collections::HashMap<String, usize>,
    // Semantic counts for Overview (single source of truth from backend)
    pub rom_count: usize,
    pub music_track_count: usize,
    pub video_count: usize,
    pub image_count: usize,
    pub ebook_count: usize,
    pub bios_count: usize,
    pub lgpt_sample_count: usize,
    pub lgpt_project_count: usize,
}

#[cfg(target_os = "windows")]
fn get_volume_info_windows(path: &str) -> VolumeInfo {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::MAX_PATH;
    use windows::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetVolumeInformationW,
    };

    let p = Path::new(path);
    let root = if path.len() >= 2 && path.chars().nth(1) == Some(':') {
        // Drive letter like E:\ or E:
        let drive = &path[0..2];
        format!("{}\\", drive)
    } else {
        // For a folder, get its root (e.g., E:\foo\bar -> E:\)
        p.ancestors()
            .find(|a| {
                a.to_string_lossy().len() == 3 && a.to_string_lossy().chars().nth(1) == Some(':')
            })
            .map(|a| a.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string())
    };

    let root_w: Vec<u16> = OsStr::new(&root)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let drive_type = unsafe { GetDriveTypeW(windows::core::PCWSTR(root_w.as_ptr())) };
    let removable = matches!(drive_type, 2); // DRIVE_REMOVABLE = 2

    let mut label_buf = [0u16; MAX_PATH as usize + 1];
    let mut fs_buf = [0u16; MAX_PATH as usize + 1];
    let mut serial = 0u32;
    let mut max_comp = 0u32;
    let mut flags = 0u32;

    let mut label: Option<String> = None;
    let mut filesystem: Option<String> = None;
    let mut serial_opt: Option<u32> = None;
    let mut accessible = Path::new(path).exists();
    let mut error: Option<String> = None;

    let res = unsafe {
        GetVolumeInformationW(
            windows::core::PCWSTR(root_w.as_ptr()),
            Some(&mut label_buf),
            Some(&mut serial),
            Some(&mut max_comp),
            Some(&mut flags),
            Some(&mut fs_buf),
        )
    };
    if res.is_ok() {
        let len = label_buf
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(label_buf.len());
        let s = String::from_utf16_lossy(&label_buf[..len]);
        if !s.is_empty() {
            label = Some(s);
        }
        let len2 = fs_buf.iter().position(|&c| c == 0).unwrap_or(fs_buf.len());
        let s2 = String::from_utf16_lossy(&fs_buf[..len2]);
        if !s2.is_empty() {
            filesystem = Some(s2);
        }
        serial_opt = Some(serial);
    } else if !accessible {
        error = Some(format!("GetVolumeInformationW failed for {}", root));
    }

    let mut total: Option<u64> = None;
    let mut free: Option<u64> = None;
    let path_w: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut free_bytes = 0u64;
    let mut total_bytes = 0u64;
    let mut avail = 0u64;
    let r2 = unsafe {
        GetDiskFreeSpaceExW(
            windows::core::PCWSTR(path_w.as_ptr()),
            Some(&mut avail),
            Some(&mut total_bytes),
            Some(&mut free_bytes),
        )
    };
    if r2.is_ok() {
        total = Some(total_bytes);
        free = Some(free_bytes);
    } else {
        // Try root
        let root_w2: Vec<u16> = OsStr::new(&root)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let r3 = unsafe {
            GetDiskFreeSpaceExW(
                windows::core::PCWSTR(root_w2.as_ptr()),
                Some(&mut avail),
                Some(&mut total_bytes),
                Some(&mut free_bytes),
            )
        };
        if r3.is_ok() {
            total = Some(total_bytes);
            free = Some(free_bytes);
        } else if accessible {
            // Not fatal for analysis, just leave None
        }
    }

    // Check accessibility by trying to read directory
    if accessible && Path::new(path).exists() {
        match std::fs::read_dir(path) {
            Ok(_) => {}
            Err(e) => {
                accessible = false;
                error = Some(format!("read_dir failed: {}", e));
            }
        }
    }

    VolumeInfo {
        path: path.to_string(),
        label,
        filesystem,
        total_bytes: total,
        free_bytes: free,
        removable: Some(removable),
        accessible,
        error,
        serial: serial_opt,
    }
}

#[cfg(not(target_os = "windows"))]
fn get_volume_info_fallback(path: &str) -> VolumeInfo {
    let p = Path::new(path);
    let accessible = p.exists();
    let mut error = None;
    if !accessible {
        error = Some(format!("path not found: {}", path));
    } else if let Err(e) = std::fs::read_dir(path) {
        error = Some(format!("read_dir failed: {}", e));
    }
    // Try to get free space via statvfs on unix, but keep simple
    VolumeInfo {
        path: path.to_string(),
        label: None,
        filesystem: None,
        total_bytes: None,
        free_bytes: None,
        removable: None,
        accessible,
        error,
        serial: None,
    }
}

pub fn get_volume_info(path: &str) -> VolumeInfo {
    #[cfg(target_os = "windows")]
    {
        get_volume_info_windows(path)
    }
    #[cfg(not(target_os = "windows"))]
    {
        get_volume_info_fallback(path)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PhysicalDevice {
    pub device_path: String,
    pub friendly_name: Option<String>,
    pub bus_type: Option<String>,
    pub removable: bool,
    pub is_usb: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Volume {
    pub guid: String,
    pub mount_points: Vec<String>,
    pub label: Option<String>,
    pub filesystem: Option<String>,
    pub serial: Option<u32>,
    pub total_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub removable: bool,
    pub drive_type: u32,
    pub accessible: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TreeFrogTarget {
    pub volume: Volume,
    pub mount_point: String,
    pub analysis: TargetAnalysis,
    pub physical_device: Option<PhysicalDevice>,
    pub stable_id: String, // volume GUID + serial
}

#[cfg(target_os = "windows")]
pub fn list_volumes_findfirst() -> Vec<Volume> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::MAX_PATH;
    use windows::Win32::Storage::FileSystem::{
        FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, GetDiskFreeSpaceExW, GetDriveTypeW,
        GetVolumeInformationW, GetVolumePathNamesForVolumeNameW,
    };

    let mut volumes = Vec::new();
    let mut buffer = [0u16; MAX_PATH as usize + 1];
    let handle = unsafe { FindFirstVolumeW(&mut buffer) };
    let handle = match handle {
        Ok(h) if !h.is_invalid() => h,
        _ => return list_volumes_fallback(),
    };
    loop {
        let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        let guid = String::from_utf16_lossy(&buffer[..len]);
        // Get mount points for this volume GUID
        let mut path_names_buf = [0u16; 1024];
        let mut return_len = 0u32;
        let res = unsafe {
            GetVolumePathNamesForVolumeNameW(
                windows::core::PCWSTR(buffer.as_ptr()),
                Some(&mut path_names_buf),
                &mut return_len,
            )
        };
        let mut mount_points = Vec::new();
        if res.is_ok() {
            let mut start = 0;
            for i in 0..return_len as usize {
                if path_names_buf[i] == 0 {
                    if i > start {
                        let s = String::from_utf16_lossy(&path_names_buf[start..i]);
                        if !s.is_empty() {
                            mount_points.push(s);
                        }
                    }
                    start = i + 1;
                    if i + 1 < return_len as usize && path_names_buf[i + 1] == 0 {
                        break;
                    }
                }
            }
        }
        // Get volume information for the GUID itself
        let mut label_buf = [0u16; MAX_PATH as usize + 1];
        let mut fs_buf = [0u16; MAX_PATH as usize + 1];
        let mut serial = 0u32;
        let mut max_comp = 0u32;
        let mut flags = 0u32;
        let mut label: Option<String> = None;
        let mut filesystem: Option<String> = None;
        let mut serial_opt: Option<u32> = None;
        let res2 = unsafe {
            GetVolumeInformationW(
                windows::core::PCWSTR(buffer.as_ptr()),
                Some(&mut label_buf),
                Some(&mut serial),
                Some(&mut max_comp),
                Some(&mut flags),
                Some(&mut fs_buf),
            )
        };
        if res2.is_ok() {
            let len = label_buf
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(label_buf.len());
            let s = String::from_utf16_lossy(&label_buf[..len]);
            if !s.is_empty() {
                label = Some(s);
            }
            let len2 = fs_buf.iter().position(|&c| c == 0).unwrap_or(fs_buf.len());
            let s2 = String::from_utf16_lossy(&fs_buf[..len2]);
            if !s2.is_empty() {
                filesystem = Some(s2);
            }
            serial_opt = Some(serial);
        }
        // Get free space and drive type from first mount point or GUID
        let mut total: Option<u64> = None;
        let mut free: Option<u64> = None;
        let test_path = mount_points.first().map(|s| s.as_str()).unwrap_or(&guid);
        let w: Vec<u16> = OsStr::new(test_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let drive_type = unsafe { GetDriveTypeW(windows::core::PCWSTR(w.as_ptr())) };
        let accessible = Path::new(test_path).exists();
        let mut free_bytes = 0u64;
        let mut total_bytes = 0u64;
        let mut avail = 0u64;
        let path_w: Vec<u16> = OsStr::new(test_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let r = unsafe {
            GetDiskFreeSpaceExW(
                windows::core::PCWSTR(path_w.as_ptr()),
                Some(&mut avail),
                Some(&mut total_bytes),
                Some(&mut free_bytes),
            )
        };
        if r.is_ok() {
            total = Some(total_bytes);
            free = Some(free_bytes);
        }
        let removable = matches!(drive_type, 2);
        // Try to get physical device info via IOCTL (best effort, no admin)
        let _physical = get_physical_device_for_volume(&guid);

        // Solo incluir dispositivos removibles (SD cards, USB drives)
        // DRIVE_REMOVABLE = 2, excluye discos fijos (DRIVE_FIXED = 3)
        if removable {
            volumes.push(Volume {
                guid: guid.clone(),
                mount_points: mount_points.clone(),
                label: label.clone(),
                filesystem: filesystem.clone(),
                serial: serial_opt,
                total_bytes: total,
                free_bytes: free,
                removable,
                drive_type,
                accessible,
            });
        }

        // Next volume
        let mut next_buf = [0u16; MAX_PATH as usize + 1];
        let next_res = unsafe { FindNextVolumeW(handle, &mut next_buf) };
        if next_res.is_err() {
            break;
        }
        buffer.copy_from_slice(&next_buf);
    }
    unsafe {
        let _ = FindVolumeClose(handle);
    }
    if volumes.is_empty() {
        return list_volumes_fallback();
    }
    volumes
}

#[cfg(target_os = "windows")]
fn get_physical_device_for_volume(_guid: &str) -> Option<PhysicalDevice> {
    // Best effort: try to open volume and query STORAGE_DEVICE_NUMBER via DeviceIoControl
    // This requires no admin for read, but may fail for some volumes
    // For now, return None and let the caller handle fallback
    // A full implementation would use CreateFileW + DeviceIoControl(IOCTL_STORAGE_GET_DEVICE_NUMBER)
    None
}

#[cfg(target_os = "windows")]
fn list_volumes_fallback() -> Vec<Volume> {
    // Fallback to A-Z enumeration if FindFirstVolume fails
    let mut out = Vec::new();
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        let p = Path::new(&drive);
        if !p.exists() {
            continue;
        }
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Storage::FileSystem::GetDriveTypeW;
        let w: Vec<u16> = OsStr::new(&drive)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let dt = unsafe { GetDriveTypeW(windows::core::PCWSTR(w.as_ptr())) };
        // Solo incluir DRIVE_REMOVABLE (2), excluir DRIVE_FIXED (3), DRIVE_REMOTE (4), etc.
        if dt != 2 {
            continue;
        }
        let info = get_volume_info(&drive);
        // Convert VolumeInfo to Volume for fallback
        out.push(Volume {
            guid: drive.clone(),
            mount_points: vec![drive.clone()],
            label: info.label.clone(),
            filesystem: info.filesystem.clone(),
            serial: None,
            total_bytes: info.total_bytes,
            free_bytes: info.free_bytes,
            removable: info.removable.unwrap_or(false),
            drive_type: dt,
            accessible: info.accessible,
        });
    }
    out
}

#[cfg(target_os = "windows")]
pub fn list_volumes() -> Vec<VolumeInfo> {
    // New implementation: use FindFirstVolume for robust enumeration, then map to VolumeInfo for backward compat
    // This satisfies the requirement to not rely exclusively on A:\–Z:\ scanning
    let volumes = list_volumes_findfirst();
    // For backward compat, return VolumeInfo for each mount point
    let mut out = Vec::new();
    for vol in volumes {
        if vol.mount_points.is_empty() {
            // Volume with no mount point (e.g., hidden recovery) - still report as VolumeInfo with GUID as path
            out.push(VolumeInfo {
                path: vol.guid.clone(),
                label: vol.label.clone(),
                filesystem: vol.filesystem.clone(),
                total_bytes: vol.total_bytes,
                free_bytes: vol.free_bytes,
                removable: Some(vol.removable),
                accessible: vol.accessible,
                error: None,
                serial: vol.serial,
            });
        } else {
            for mp in vol.mount_points {
                out.push(VolumeInfo {
                    path: mp.clone(),
                    label: vol.label.clone(),
                    filesystem: vol.filesystem.clone(),
                    total_bytes: vol.total_bytes,
                    free_bytes: vol.free_bytes,
                    removable: Some(vol.removable),
                    accessible: vol.accessible,
                    error: None,
                    serial: vol.serial,
                });
            }
        }
    }
    // Filtrar volúmenes sin letra asignada o inaccesibles
    out.retain(|v| {
        if v.path.is_empty() || v.path.starts_with("\\\\?\\") {
            tracing::info!("Skipping volume without drive letter: {:?}", v);
            return false;
        }
        if !Path::new(&v.path).exists() {
            tracing::info!("Skipping inaccessible volume: {}", v.path);
            return false;
        }
        if !v.accessible {
            tracing::info!("Skipping inaccessible volume: {}", v.path);
            return false;
        }
        // Solo volúmenes con letra de unidad (ej: G:\)
        if v.path.len() < 2 || v.path.chars().nth(1) != Some(':') {
            tracing::info!("Skipping volume without drive letter: {}", v.path);
            return false;
        }
        true
    });
    // Also include fallback for any drives not found via FindFirstVolume (should be rare)
    if out.is_empty() {
        return list_volumes_fallback()
            .into_iter()
            .map(|v| VolumeInfo {
                path: v.mount_points.first().cloned().unwrap_or(v.guid),
                label: v.label,
                filesystem: v.filesystem,
                total_bytes: v.total_bytes,
                free_bytes: v.free_bytes,
                removable: Some(v.removable),
                accessible: v.accessible,
                error: None,
                serial: v.serial,
            })
            .collect();
    }
    out
}

#[cfg(not(target_os = "windows"))]
pub fn list_volumes() -> Vec<VolumeInfo> {
    vec![]
}

fn load_markers() -> (Vec<String>, Vec<String>) {
    // Try to load sd_markers.json, fallback to cubegm, roms
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles/treefrogui/sd_markers.json"),
        PathBuf::from("profiles/treefrogui/sd_markers.json"),
    ];
    for cand in candidates {
        if let Ok(s) = std::fs::read_to_string(&cand) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                if let Some(arr) = v
                    .get("detection")
                    .and_then(|d| d.get("markers"))
                    .and_then(|m| m.as_array())
                {
                    let mut req = Vec::new();
                    let mut _opt = Vec::new();
                    for ent in arr {
                        if let Some(path) = ent.get("path").and_then(|p| p.as_str()) {
                            let p = path.trim_end_matches('/').to_string();
                            let required = ent
                                .get("required")
                                .and_then(|r| r.as_bool())
                                .unwrap_or(false);
                            if required {
                                req.push(p);
                            } else {
                                _opt.push(p);
                            }
                        }
                    }
                    // For validation, required are cubegm and roms
                    return (req, _opt);
                }
            }
        }
    }
    (
        vec!["cubegm".to_string(), "roms".to_string()],
        vec![
            "frogui".to_string(),
            "lgpt".to_string(),
            "cubegm/cores".to_string(),
            "cubegm/bios".to_string(),
        ],
    )
}

pub fn analyze_target(path: &str) -> anyhow::Result<TargetAnalysis> {
    let vol = get_volume_info(path);
    let p = Path::new(path);
    if let Some(fname) = p.file_name().and_then(|n| n.to_str()) {
        if fname.to_lowercase() == "roms" && !p.join("cubegm").exists() {
            anyhow::bail!("You selected the 'roms' folder as root. Please select the SD card root (e.g. 'E:\\') so ROMs are copied correctly to 'roms/SYSTEM/'.");
        }
    }
    let mut errors = Vec::new();
    if let Some(e) = &vol.error {
        errors.push(e.clone());
    }

    if !vol.accessible {
        return Ok(TargetAnalysis {
            path: path.to_string(),
            volume: vol.clone(),
            status: "inaccessible".to_string(),
            is_treefrog: false,
            is_incomplete: false,
            markers_found: vec![],
            markers_missing: vec!["cubegm".to_string(), "roms".to_string()],
            lgpt_detected: false,
            rom_dirs: vec![],
            media_dirs: vec![],
            bios_dirs: vec![],
            lgpt_dirs: vec![],
            existing_count: 0,
            total_size: 0,
            free_bytes: vol.free_bytes,
            capacity_bytes: vol.total_bytes,
            filesystem: vol.filesystem.clone(),
            label: vol.label.clone(),
            errors,
            stable_id: None,
            physical_device: None,
            folder_breakdown: std::collections::HashMap::new(),
            rom_count: 0,
            music_track_count: 0,
            video_count: 0,
            image_count: 0,
            ebook_count: 0,
            bios_count: 0,
            lgpt_sample_count: 0,
            lgpt_project_count: 0,
        });
    }

    let (required_markers, _optional) = load_markers();
    let mut found = Vec::new();
    let mut missing = Vec::new();
    for m in &required_markers {
        if p.join(m).exists() {
            found.push(m.clone());
        } else {
            missing.push(m.clone());
        }
    }
    // Also check optional for info
    let optional_markers = ["frogui", "lgpt", "cubegm/cores", "cubegm/bios"];
    for m in optional_markers {
        if p.join(m).exists() && !found.contains(&m.to_string()) {
            found.push(m.to_string());
        }
    }

    let is_treefrog = found.contains(&"cubegm".to_string()) && found.contains(&"roms".to_string());
    let is_incomplete = !is_treefrog
        && (found.contains(&"cubegm".to_string()) || found.contains(&"roms".to_string()));

    let status = if !vol.accessible {
        "inaccessible".to_string()
    } else if is_treefrog {
        "valid".to_string()
    } else if is_incomplete {
        "incomplete".to_string()
    } else {
        "unknown".to_string()
    };

    let lgpt_detected = p.join("lgpt").exists();

    // Use scanner.rs for SD destination (single source of truth) - ensures counts exactly match planner
    let profile_for_scan =
        crate::profile::load_profile().unwrap_or_else(|_| crate::profile::load_profile().unwrap());
    let scanned_files =
        crate::scanner::scan_directory(path, &profile_for_scan, true).unwrap_or_default();

    let mut rom_dirs = Vec::new();
    let mut media_dirs = Vec::new();
    let mut bios_dirs = Vec::new();
    let mut lgpt_dirs = Vec::new();
    let mut total_size = 0u64;
    let mut folder_breakdown: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    // Derive UI badge dirs from scanned files' destinations (whitelist already applied by scanner+classify)
    let mut seen_rom_dirs = std::collections::HashSet::new();
    let mut seen_media_dirs = std::collections::HashSet::new();
    let mut seen_bios_dirs = std::collections::HashSet::new();
    let mut seen_lgpt_dirs = std::collections::HashSet::new();

    for sf in &scanned_files {
        total_size += sf.size;
        match sf.classification.kind {
            crate::classify::Kind::Rom => {
                if let Some(dir) = sf.classification.destination.strip_prefix("roms/") {
                    let top = dir.split('/').next().unwrap_or(dir);
                    if seen_rom_dirs.insert(top.to_string()) {
                        rom_dirs.push(top.to_string());
                    }
                } else if !sf.classification.destination.is_empty() {
                    if seen_rom_dirs.insert(sf.classification.destination.clone()) {
                        rom_dirs.push(sf.classification.destination.clone());
                    }
                }
            }
            crate::classify::Kind::Music => {
                if seen_media_dirs.insert("music".to_string()) {
                    media_dirs.push("music".to_string());
                }
            }
            crate::classify::Kind::Video => {
                if seen_media_dirs.insert("videos".to_string()) {
                    media_dirs.push("videos".to_string());
                }
            }
            crate::classify::Kind::Image => {
                if seen_media_dirs.insert("images".to_string()) {
                    media_dirs.push("images".to_string());
                }
            }
            crate::classify::Kind::Bios => {
                if seen_bios_dirs.insert("cubegm/bios".to_string()) {
                    bios_dirs.push("cubegm/bios".to_string());
                }
            }
            crate::classify::Kind::LgptSample => {
                if seen_lgpt_dirs.insert("lgpt/samples".to_string()) {
                    lgpt_dirs.push("lgpt/samples".to_string());
                }
            }
            crate::classify::Kind::LgptProject => {
                if seen_lgpt_dirs.insert("lgpt/projects".to_string()) {
                    lgpt_dirs.push("lgpt/projects".to_string());
                }
            }
            _ => {}
        }
        let folder = sf.classification.destination.clone();
        if !folder.is_empty() {
            *folder_breakdown.entry(folder).or_insert(0) += 1;
        }
    }

    let existing_count = scanned_files.len();
    // Semantic counts for Overview (single source of truth)
    let rom_count = scanned_files
        .iter()
        .filter(|f| f.classification.kind == crate::classify::Kind::Rom)
        .count();
    let music_track_count = scanned_files
        .iter()
        .filter(|f| f.classification.kind == crate::classify::Kind::Music)
        .count();
    let video_count = scanned_files
        .iter()
        .filter(|f| f.classification.kind == crate::classify::Kind::Video)
        .count();
    let image_count = scanned_files
        .iter()
        .filter(|f| f.classification.kind == crate::classify::Kind::Image)
        .count();
    let ebook_count = scanned_files
        .iter()
        .filter(|f| f.classification.kind == crate::classify::Kind::Ebook)
        .count();
    let bios_count = scanned_files
        .iter()
        .filter(|f| f.classification.kind == crate::classify::Kind::Bios)
        .count();
    let lgpt_sample_count = scanned_files
        .iter()
        .filter(|f| f.classification.kind == crate::classify::Kind::LgptSample)
        .count();
    let lgpt_project_count = scanned_files
        .iter()
        .filter(|f| f.classification.kind == crate::classify::Kind::LgptProject)
        .count();
    // Also ensure lgpt_dirs from filesystem if scanner didn't find them (for empty lgpt)
    if lgpt_detected {
        if p.join("lgpt/samples").exists() && !seen_lgpt_dirs.contains("lgpt/samples") {
            lgpt_dirs.push("lgpt/samples".to_string());
        }
        if p.join("lgpt/projects").exists() && !seen_lgpt_dirs.contains("lgpt/projects") {
            lgpt_dirs.push("lgpt/projects".to_string());
        }
        if p.join("lgpt").exists() && lgpt_dirs.is_empty() {
            lgpt_dirs.push("lgpt".to_string());
        }
    }

    rom_dirs.sort();
    media_dirs.sort();
    bios_dirs.sort();
    lgpt_dirs.sort();

    // Stable ID: volume GUID + serial (if available) for stale_target detection
    let stable_id = {
        #[cfg(target_os = "windows")]
        {
            // Try to get volume GUID and serial from FindFirstVolume path
            // For now, use label + serial + filesystem + capacity as proxy
            // A full implementation would use GetVolumeInformation's serial and FindFirstVolume's GUID
            let mut id = String::new();
            if let Some(label) = &vol.label {
                id.push_str(label);
                id.push('-');
            }
            if let Some(fs) = &vol.filesystem {
                id.push_str(fs);
                id.push('-');
            }
            if let Some(total) = vol.total_bytes {
                id.push_str(&total.to_string());
                id.push('-');
            }
            // Also include the volume path's canonical GUID if we can get it via GetVolumeNameForVolumeMountPointW
            // For now, use the path itself as part of ID, but also include serial if available via GetVolumeInformation
            // We already have serial in Volume (if we had it), but our VolumeInfo doesn't store serial yet
            // For this milestone, use path + label + serial fallback
            if id.is_empty() {
                Some(vol.path.clone())
            } else {
                Some(id)
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            None::<String>
        }
    };

    // Try to get physical device for this volume for more stable ID
    let physical_device = {
        #[cfg(target_os = "windows")]
        {
            // Best effort: try to get device number via IOCTL (requires no admin for read)
            // For now, return a simple PhysicalDevice based on drive type
            let bus_type = if vol.removable.unwrap_or(false) {
                Some("USB".to_string())
            } else {
                Some("Fixed".to_string())
            };
            let is_usb = vol.removable.unwrap_or(false);
            Some(PhysicalDevice {
                device_path: vol.path.clone(),
                friendly_name: vol.label.clone(),
                bus_type,
                removable: vol.removable.unwrap_or(false),
                is_usb,
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    };

    Ok(TargetAnalysis {
        path: path.to_string(),
        volume: vol.clone(),
        status,
        is_treefrog,
        is_incomplete,
        markers_found: found,
        markers_missing: missing,
        lgpt_detected,
        rom_dirs,
        media_dirs,
        bios_dirs,
        lgpt_dirs,
        existing_count,
        total_size,
        free_bytes: vol.free_bytes,
        capacity_bytes: vol.total_bytes,
        filesystem: vol.filesystem.clone(),
        label: vol.label.clone(),
        errors,
        stable_id,
        physical_device,
        folder_breakdown,
        rom_count,
        music_track_count,
        video_count,
        image_count,
        ebook_count,
        bios_count,
        lgpt_sample_count,
        lgpt_project_count,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum DestinationError {
    #[error("empty destination")]
    Empty,
    #[error("absolute path not allowed: {0}")]
    Absolute(String),
    #[error("drive injection not allowed: {0}")]
    Drive(String),
    #[error("UNC not allowed: {0}")]
    Unc(String),
    #[error("traversal detected: {0}")]
    Traversal(String),
    #[error("empty path component in {0}")]
    EmptyComponent(String),
    #[error("reserved name not allowed: {0}")]
    Reserved(String),
    #[error("trailing dot/space not allowed: {0}")]
    Trailing(String),
    #[error("illegal character '{0}' in {1}")]
    IllegalChar(char, String),
    #[error("ADS not allowed: {0}")]
    Ads(String),
    #[error("path escapes SD root: {0} not inside {1}")]
    Escape(String, String),
}

pub fn validate_destination_path(dest: &str) -> Result<(), String> {
    // Backwards compat: validate as if sd_root is a dummy, but use canonical logic
    // Use a fake sd_root to leverage the same checks without needing a real path
    let fake_root = Path::new(if cfg!(windows) {
        "C:\\fake_sd_root"
    } else {
        "/tmp/fake_sd_root"
    });
    resolve_validated_destination(fake_root, dest)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn resolve_validated_destination(
    sd_root: &Path,
    relative_destination: &str,
) -> Result<PathBuf, DestinationError> {
    let raw = relative_destination.trim();
    if raw.is_empty() {
        return Err(DestinationError::Empty);
    }
    // Normalize separators: treat both / and \ as /
    let normalized = raw.replace('\\', "/");
    // Reject absolute Unix
    if normalized.starts_with('/') {
        return Err(DestinationError::Absolute(relative_destination.to_string()));
    }
    // Reject drive-letter absolute (e.g., C:, C:\, C:/, D:foo)
    if normalized.len() >= 2
        && normalized.chars().nth(1) == Some(':')
        && normalized
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic())
            .unwrap_or(false)
    {
        return Err(DestinationError::Drive(relative_destination.to_string()));
    }
    // Reject UNC (after normalization //server/share)
    if normalized.starts_with("//") {
        return Err(DestinationError::Unc(relative_destination.to_string()));
    }
    // Reject ADS (any colon not already drive-checked)
    if normalized.contains(':') {
        return Err(DestinationError::Ads(relative_destination.to_string()));
    }
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    // Split and validate each component
    // Use split('/') after normalization; this also catches empty components from // or trailing /
    let parts: Vec<&str> = normalized.split('/').collect();
    for part in &parts {
        if part.is_empty() {
            return Err(DestinationError::EmptyComponent(
                relative_destination.to_string(),
            ));
        }
        if *part == "." || *part == ".." {
            return Err(DestinationError::Traversal(
                relative_destination.to_string(),
            ));
        }
        // Reserved names (case-insensitive, before dot)
        let base = part.split('.').next().unwrap_or(part).to_uppercase();
        if reserved.contains(&base.as_str()) {
            return Err(DestinationError::Reserved(part.to_string()));
        }
        if part.ends_with('.') || part.ends_with(' ') {
            return Err(DestinationError::Trailing(part.to_string()));
        }
        for ch in ['<', '>', '"', '|', '?', '*'] {
            if part.contains(ch) {
                return Err(DestinationError::IllegalChar(ch, part.to_string()));
            }
        }
        // Note: we already normalized \ to /, so no need to check \ separately;
        // ':' already checked for ADS
    }
    // Construct final path and verify it stays inside sd_root
    // Use PathBuf join with OS-specific separator handling
    let relative_path = PathBuf::from(normalized.clone());
    let final_path = sd_root.join(&relative_path);
    // Canonicalize sd_root if possible for comparison; otherwise use as-is
    let sd_root_canon = sd_root
        .canonicalize()
        .unwrap_or_else(|_| sd_root.to_path_buf());
    // For final_path, we cannot canonicalize if it doesn't exist, so do lexical check
    // Ensure final_path's string representation starts with sd_root_canon
    let sd_root_str = sd_root_canon
        .to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string();
    let final_str = final_path.to_string_lossy().replace('\\', "/");
    // Case-insensitive on Windows
    let is_windows = cfg!(windows);
    let sd_cmp = if is_windows {
        sd_root_str.to_lowercase()
    } else {
        sd_root_str.clone()
    };
    let final_cmp = if is_windows {
        final_str.to_lowercase()
    } else {
        final_str.clone()
    };
    if final_cmp == sd_cmp {
        return Err(DestinationError::EmptyComponent(
            relative_destination.to_string(),
        ));
    }
    if !final_cmp.starts_with(&sd_cmp) {
        return Err(DestinationError::Escape(final_str, sd_root_str));
    }
    // Ensure separator after sd_root
    let remainder = &final_cmp[sd_cmp.len()..];
    if !remainder.starts_with('/') {
        return Err(DestinationError::Escape(final_str, sd_root_str));
    }
    Ok(final_path)
}

pub fn check_case_collision(dests: &[String]) -> Vec<(String, String)> {
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut out = Vec::new();
    for d in dests {
        let norm = d.to_lowercase();
        if let Some(prev) = seen.get(&norm) {
            out.push((d.clone(), prev.clone()));
        } else {
            seen.insert(norm, d.clone());
        }
    }
    out
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpaceInfo {
    pub bytes_to_copy: u64,
    pub bytes_to_extract: u64,
    pub bytes_to_generate: u64,
    pub bytes_to_skip: u64,
    pub required_bytes: u64,
    pub available_bytes: Option<u64>,
    pub status: String, // ok, insufficient_space, unknown
}

pub fn effective_action(entry: &crate::PlanEntry) -> &str {
    entry.resolved_action.as_deref().unwrap_or(&entry.action)
}

pub fn calculate_space(plan: &crate::Plan, free_bytes: Option<u64>) -> SpaceInfo {
    let mut to_copy = 0u64;
    let mut to_extract = 0u64;
    let mut to_generate = 0u64;
    let mut to_skip = 0u64;
    for e in &plan.entries {
        let size = e.size.unwrap_or(0);
        match effective_action(e) {
            "copy" | "replace" => to_copy += size,
            "extract" => to_extract += size,
            "convert_then_copy" => to_generate += size,
            "skip" | "skip_unchanged" | "skip_duplicate" => to_skip += size,
            _ => {
                // conflict/manual_review/unsupported etc. with no effective write -> counted as skip
                // If effective is still conflict/manual, it is not required
                if matches!(
                    e.action.as_str(),
                    "conflict"
                        | "manual_review"
                        | "unsupported_archive"
                        | "unsupported"
                        | "conversion_error"
                ) {
                    // Will be counted as skip if effective remains non-write
                    to_skip += size;
                }
            }
        }
    }
    let required = to_copy + to_extract + to_generate;
    let status = if let Some(avail) = free_bytes {
        if required > avail {
            "insufficient_space".to_string()
        } else {
            "ok".to_string()
        }
    } else {
        "unknown".to_string()
    };
    SpaceInfo {
        bytes_to_copy: to_copy,
        bytes_to_extract: to_extract,
        bytes_to_generate: to_generate,
        bytes_to_skip: to_skip,
        required_bytes: required,
        available_bytes: free_bytes,
        status,
    }
}

pub fn check_stale_target(
    selected_path: &str,
    current_volumes: &[VolumeInfo],
    stored_stable_id: Option<&str>,
) -> bool {
    // If the selected path is no longer in the current volumes, it's stale
    if !current_volumes
        .iter()
        .any(|v| v.path == selected_path && v.accessible)
    {
        return true;
    }
    // If we have a stored stable_id, compare with current volume's stable_id (derived from label/filesystem/total)
    if let Some(stored) = stored_stable_id {
        if let Some(current) = current_volumes.iter().find(|v| v.path == selected_path) {
            let current_id = format!(
                "{}-{}-{}",
                current.label.clone().unwrap_or_default(),
                current.filesystem.clone().unwrap_or_default(),
                current.total_bytes.unwrap_or(0)
            );
            let current_id = if current_id.trim_matches('-').is_empty() {
                current.path.clone()
            } else {
                current_id
            };
            if current_id != stored {
                return true;
            }
        }
    }
    false
}
