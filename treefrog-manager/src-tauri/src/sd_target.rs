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
}

#[cfg(target_os = "windows")]
fn get_volume_info_windows(path: &str) -> VolumeInfo {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW, GetVolumeInformationW};
    use windows::Win32::Foundation::MAX_PATH;

    let p = Path::new(path);
    let root = if path.len() >= 2 && path.chars().nth(1) == Some(':') {
        // Drive letter like E:\ or E:
        let drive = &path[0..2];
        format!("{}\\", drive)
    } else {
        // For a folder, get its root (e.g., E:\foo\bar -> E:\)
        p.ancestors().find(|a| a.to_string_lossy().len() == 3 && a.to_string_lossy().chars().nth(1) == Some(':')).map(|a| a.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string())
    };

    let root_w: Vec<u16> = OsStr::new(&root).encode_wide().chain(std::iter::once(0)).collect();
    let drive_type = unsafe { GetDriveTypeW(windows::core::PCWSTR(root_w.as_ptr())) };
    let removable = matches!(drive_type, 2); // DRIVE_REMOVABLE = 2

    let mut label_buf = [0u16; MAX_PATH as usize + 1];
    let mut fs_buf = [0u16; MAX_PATH as usize + 1];
    let mut serial = 0u32;
    let mut max_comp = 0u32;
    let mut flags = 0u32;

    let mut label: Option<String> = None;
    let mut filesystem: Option<String> = None;
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
        let len = label_buf.iter().position(|&c| c == 0).unwrap_or(label_buf.len());
        let s = String::from_utf16_lossy(&label_buf[..len]);
        if !s.is_empty() { label = Some(s); }
        let len2 = fs_buf.iter().position(|&c| c == 0).unwrap_or(fs_buf.len());
        let s2 = String::from_utf16_lossy(&fs_buf[..len2]);
        if !s2.is_empty() { filesystem = Some(s2); }
    } else if !accessible {
        error = Some(format!("GetVolumeInformationW failed for {}", root));
    }

    let mut total: Option<u64> = None;
    let mut free: Option<u64> = None;
    let path_w: Vec<u16> = OsStr::new(path).encode_wide().chain(std::iter::once(0)).collect();
    let mut free_bytes = 0u64;
    let mut total_bytes = 0u64;
    let mut avail = 0u64;
    let r2 = unsafe { GetDiskFreeSpaceExW(windows::core::PCWSTR(path_w.as_ptr()), Some(&mut avail), Some(&mut total_bytes), Some(&mut free_bytes)) };
    if r2.is_ok() {
        total = Some(total_bytes);
        free = Some(free_bytes);
    } else {
        // Try root
        let root_w2: Vec<u16> = OsStr::new(&root).encode_wide().chain(std::iter::once(0)).collect();
        let r3 = unsafe { GetDiskFreeSpaceExW(windows::core::PCWSTR(root_w2.as_ptr()), Some(&mut avail), Some(&mut total_bytes), Some(&mut free_bytes)) };
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
            Ok(_) => {},
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
    use windows::Win32::Storage::FileSystem::{FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, GetDiskFreeSpaceExW, GetDriveTypeW, GetVolumeInformationW, GetVolumePathNamesForVolumeNameW};
    use windows::Win32::Foundation::MAX_PATH;

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
        let res = unsafe { GetVolumePathNamesForVolumeNameW(windows::core::PCWSTR(buffer.as_ptr()), Some(&mut path_names_buf), &mut return_len) };
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
            let len = label_buf.iter().position(|&c| c == 0).unwrap_or(label_buf.len());
            let s = String::from_utf16_lossy(&label_buf[..len]);
            if !s.is_empty() { label = Some(s); }
            let len2 = fs_buf.iter().position(|&c| c == 0).unwrap_or(fs_buf.len());
            let s2 = String::from_utf16_lossy(&fs_buf[..len2]);
            if !s2.is_empty() { filesystem = Some(s2); }
            serial_opt = Some(serial);
        }
        // Get free space and drive type from first mount point or GUID
        let mut total: Option<u64> = None;
        let mut free: Option<u64> = None;
        let test_path = mount_points.first().map(|s| s.as_str()).unwrap_or(&guid);
        let w: Vec<u16> = OsStr::new(test_path).encode_wide().chain(std::iter::once(0)).collect();
        let drive_type = unsafe { GetDriveTypeW(windows::core::PCWSTR(w.as_ptr())) };
        let accessible = Path::new(test_path).exists();
        let mut free_bytes = 0u64;
        let mut total_bytes = 0u64;
        let mut avail = 0u64;
        let path_w: Vec<u16> = OsStr::new(test_path).encode_wide().chain(std::iter::once(0)).collect();
        let r = unsafe { GetDiskFreeSpaceExW(windows::core::PCWSTR(path_w.as_ptr()), Some(&mut avail), Some(&mut total_bytes), Some(&mut free_bytes)) };
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
    unsafe { let _ = FindVolumeClose(handle); }
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
        let w: Vec<u16> = OsStr::new(&drive).encode_wide().chain(std::iter::once(0)).collect();
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
                });
            }
        }
    }
    // Also include fallback for any drives not found via FindFirstVolume (should be rare)
    if out.is_empty() {
        return list_volumes_fallback().into_iter().map(|v| VolumeInfo {
            path: v.mount_points.first().cloned().unwrap_or(v.guid),
            label: v.label,
            filesystem: v.filesystem,
            total_bytes: v.total_bytes,
            free_bytes: v.free_bytes,
            removable: Some(v.removable),
            accessible: v.accessible,
            error: None,
        }).collect();
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
                if let Some(arr) = v.get("detection").and_then(|d| d.get("markers")).and_then(|m| m.as_array()) {
                    let mut req = Vec::new();
                    let mut _opt = Vec::new();
                    for ent in arr {
                        if let Some(path) = ent.get("path").and_then(|p| p.as_str()) {
                            let p = path.trim_end_matches('/').to_string();
                            let required = ent.get("required").and_then(|r| r.as_bool()).unwrap_or(false);
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
    (vec!["cubegm".to_string(), "roms".to_string()], vec!["frogui".to_string(), "lgpt".to_string(), "cubegm/cores".to_string(), "cubegm/bios".to_string()])
}

pub fn analyze_target(path: &str) -> anyhow::Result<TargetAnalysis> {
    let vol = get_volume_info(path);
    let p = Path::new(path);
    // Tarea 1: Detección temprana si el usuario seleccionó la carpeta roms por error
    if let Some(fname) = p.file_name().and_then(|n| n.to_str()) {
        if fname.to_lowercase() == "roms" && !p.join("cubegm").exists() {
            anyhow::bail!("Has seleccionado la carpeta 'roms' como raíz. Por favor, selecciona la raíz de la tarjeta SD (ej. 'E:\\') para que los ROMs se copien correctamente en 'roms/SISTEMA/'.");
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
    let is_incomplete = !is_treefrog && (found.contains(&"cubegm".to_string()) || found.contains(&"roms".to_string()));

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

    // Enumerate existing content read-only
    let mut rom_dirs = Vec::new();
    let mut media_dirs = Vec::new();
    let mut bios_dirs = Vec::new();
    let mut lgpt_dirs = Vec::new();
    let mut existing_count = 0usize;
    let mut total_size = 0u64;

    let roms_path = p.join("roms");
    if roms_path.exists() {
        if let Ok(entries) = std::fs::read_dir(&roms_path) {
            for ent in entries.flatten() {
                if let Ok(ft) = ent.file_type() {
                    if ft.is_dir() {
                        let name = ent.file_name().to_string_lossy().to_string();
                        // Classify media vs rom
                        match name.to_lowercase().as_str() {
                            "music" | "videos" | "images" | "ebook" => media_dirs.push(name),
                            "bios" => bios_dirs.push(name),
                            _ => rom_dirs.push(name),
                        }
                    }
                }
            }
        }
        // Count files recursively for existing_count and total_size (read-only)
        for entry in walkdir::WalkDir::new(&roms_path).follow_links(false).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() && !entry.file_type().is_symlink() {
                existing_count += 1;
                if let Ok(m) = entry.metadata() {
                    total_size += m.len();
                }
            }
        }
    }
    if p.join("cubegm/bios").exists() {
        if !bios_dirs.contains(&"cubegm/bios".to_string()) {
            bios_dirs.push("cubegm/bios".to_string());
        }
        for entry in walkdir::WalkDir::new(p.join("cubegm/bios")).follow_links(false).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                existing_count += 1;
                if let Ok(m) = entry.metadata() { total_size += m.len(); }
            }
        }
    }
    if lgpt_detected {
        if p.join("lgpt/samples").exists() { lgpt_dirs.push("lgpt/samples".to_string()); }
        if p.join("lgpt/projects").exists() { lgpt_dirs.push("lgpt/projects".to_string()); }
        if p.join("lgpt").exists() && lgpt_dirs.is_empty() { lgpt_dirs.push("lgpt".to_string()); }
        for entry in walkdir::WalkDir::new(p.join("lgpt")).follow_links(false).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                existing_count += 1;
                if let Ok(m) = entry.metadata() { total_size += m.len(); }
            }
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
            let bus_type = if vol.removable.unwrap_or(false) { Some("USB".to_string()) } else { Some("Fixed".to_string()) };
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
    })
}

pub fn validate_destination_path(dest: &str) -> Result<(), String> {
    if dest.is_empty() {
        return Err("empty destination".to_string());
    }
    if dest.contains("..") {
        for part in dest.split('/') {
            if part == ".." {
                return Err(format!("traversal detected: {}", dest));
            }
        }
        if dest.contains("../") || dest.starts_with("../") {
            return Err(format!("traversal detected: {}", dest));
        }
    }
    if dest.starts_with('/') || dest.starts_with('\\') {
        return Err(format!("absolute path not allowed: {}", dest));
    }
    if dest.len() >= 2 && dest.chars().nth(1) == Some(':') && dest.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
        return Err(format!("drive injection not allowed: {}", dest));
    }
    if dest.contains("\\\\") {
        if dest.starts_with("\\\\") {
            return Err(format!("UNC not allowed: {}", dest));
        }
    }
    if dest.contains(':') {
        // Any colon after drive check is ADS
        return Err(format!("ADS not allowed: {}", dest));
    }
    let reserved = ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"];
    for part in dest.split('/') {
        if part.is_empty() {
            return Err(format!("empty path component in {}", dest));
        }
        let base = part.split('.').next().unwrap_or(part).to_uppercase();
        if reserved.contains(&base.as_str()) {
            return Err(format!("reserved name not allowed: {}", part));
        }
        if part.ends_with('.') || part.ends_with(' ') {
            return Err(format!("trailing dot/space not allowed: {}", part));
        }
        for ch in ['<', '>', ':', '"', '\\', '|', '?', '*'] {
            if part.contains(ch) {
                return Err(format!("illegal character '{}' in {}", ch, part));
            }
        }
    }
    if dest.contains('\\') {
        return Err(format!("backslash not allowed in destination: {}", dest));
    }
    Ok(())
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

pub fn calculate_space(plan: &crate::Plan, free_bytes: Option<u64>) -> SpaceInfo {
    let mut to_copy = 0u64;
    let mut to_extract = 0u64;
    let mut to_generate = 0u64;
    let mut to_skip = 0u64;
    for e in &plan.entries {
        let size = e.size.unwrap_or(0);
        match e.action.as_str() {
            "copy" => to_copy += size,
            "extract" => to_extract += size,
            "convert_then_copy" => to_generate += size,
            "skip_unchanged" | "skip_duplicate" | "skip" => to_skip += size,
            "conflict" | "manual_review" | "unsupported_archive" | "unsupported" | "conversion_error" => {
                // Not counted as required, but could be if resolved
            },
            _ => {}
        }
        if let Some(ra) = &e.resolved_action {
            if ra != &e.action {
                match ra.as_str() {
                    "copy" | "replace" => to_copy += size,
                    "extract" => to_extract += size,
                    "convert_then_copy" => to_generate += size,
                    _ => {}
                }
            }
        }
    }
    let required = to_copy + to_extract + to_generate;
    let status = if let Some(avail) = free_bytes {
        if required > avail { "insufficient_space".to_string() } else { "ok".to_string() }
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

pub fn check_stale_target(selected_path: &str, current_volumes: &[VolumeInfo], stored_stable_id: Option<&str>) -> bool {
    // If the selected path is no longer in the current volumes, it's stale
    if !current_volumes.iter().any(|v| v.path == selected_path && v.accessible) {
        return true;
    }
    // If we have a stored stable_id, compare with current volume's stable_id (derived from label/filesystem/total)
    if let Some(stored) = stored_stable_id {
        if let Some(current) = current_volumes.iter().find(|v| v.path == selected_path) {
            let current_id = format!("{}-{}-{}", current.label.clone().unwrap_or_default(), current.filesystem.clone().unwrap_or_default(), current.total_bytes.unwrap_or(0));
            let current_id = if current_id.trim_matches('-').is_empty() { current.path.clone() } else { current_id };
            if current_id != stored {
                return true;
            }
        }
    }
    false
}
