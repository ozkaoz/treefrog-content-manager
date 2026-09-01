// ffprobe adapter — authoritative inspection, data-driven from video_presets.json
// Hardware decoder variance means we never claim compat without physical device.

use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProbeResult {
    pub container: String,
    pub format_name_raw: String,
    pub video_codec: Option<String>,
    pub video_profile: Option<String>,
    pub video_level: Option<String>,
    pub pix_fmt: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f32>,
    pub audio_codec: Option<String>,
    pub audio_sample_rate: Option<u32>,
    pub streams: usize,
    pub has_video: bool,
    pub file_size: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompatibilityStatus {
    pub status: String, // compatible, conversion_required, unsupported, inspection_error
    pub reason: String,
}

pub fn probe(path: &str) -> anyhow::Result<ProbeResult> {
    let mut cmd = std::process::Command::new("ffprobe");
    cmd.args([
        "-v",
        "quiet",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
        path,
    ]);
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000);
    }
    let out = cmd.output();
    match out {
        Ok(o) if o.status.success() => {
            let v: serde_json::Value = serde_json::from_slice(&o.stdout)?;
            let streams = v["streams"].as_array().cloned().unwrap_or_default();
            let format_info = &v["format"];
            let format_name = format_info
                .get("format_name")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string();
            let container = format_name
                .split(',')
                .next()
                .unwrap_or("unknown")
                .to_string();
            let video_stream = streams
                .iter()
                .find(|s| s.get("codec_type").and_then(|x| x.as_str()) == Some("video"));
            let audio_stream = streams
                .iter()
                .find(|s| s.get("codec_type").and_then(|x| x.as_str()) == Some("audio"));
            let width = video_stream
                .and_then(|s| s.get("width").and_then(|x| x.as_u64()))
                .map(|x| x as u32);
            let height = video_stream
                .and_then(|s| s.get("height").and_then(|x| x.as_u64()))
                .map(|x| x as u32);
            let fps = video_stream.and_then(|s| {
                s.get("avg_frame_rate")
                    .and_then(|x| x.as_str())
                    .and_then(|s| parse_fps(s))
                    .or_else(|| {
                        s.get("r_frame_rate")
                            .and_then(|x| x.as_str())
                            .and_then(|s| parse_fps(s))
                    })
            });
            let file_size = Path::new(path).metadata().ok().map(|m| m.len());
            Ok(ProbeResult {
                container,
                format_name_raw: format_name,
                video_codec: video_stream.and_then(|s| {
                    s.get("codec_name")
                        .and_then(|x| x.as_str())
                        .map(|x| x.to_string())
                }),
                video_profile: video_stream.and_then(|s| {
                    s.get("profile")
                        .and_then(|x| x.as_str())
                        .map(|x| x.to_string())
                }),
                video_level: video_stream.and_then(|s| {
                    s.get("level")
                        .and_then(|x| x.as_str())
                        .map(|x| x.to_string())
                }),
                pix_fmt: video_stream.and_then(|s| {
                    s.get("pix_fmt")
                        .and_then(|x| x.as_str())
                        .map(|x| x.to_string())
                }),
                width,
                height,
                fps,
                audio_codec: audio_stream.and_then(|s| {
                    s.get("codec_name")
                        .and_then(|x| x.as_str())
                        .map(|x| x.to_string())
                }),
                audio_sample_rate: audio_stream.and_then(|s| {
                    s.get("sample_rate")
                        .and_then(|x| x.as_str())
                        .and_then(|x| x.parse::<u32>().ok())
                }),
                streams: streams.len(),
                has_video: video_stream.is_some(),
                file_size,
            })
        }
        Ok(o) => {
            anyhow::bail!(
                "ffprobe failed: {}",
                String::from_utf8_lossy(&o.stderr)
                    .chars()
                    .take(1000)
                    .collect::<String>()
            )
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            anyhow::bail!("ffprobe not found")
        }
        Err(e) => {
            anyhow::bail!("ffprobe inspection error: {}", e)
        }
    }
}

pub fn evaluate_compatibility(
    probe: &ProbeResult,
    preset: &serde_json::Value,
) -> CompatibilityStatus {
    // preset is either full preset (with compatibility key) or direct compat dict
    let compat = if preset.get("compatibility").is_some() && !preset.get("container").is_some() {
        preset.get("compatibility").unwrap()
    } else {
        preset
    };
    if !probe.has_video {
        return CompatibilityStatus {
            status: "unsupported".to_string(),
            reason: "no video stream found".to_string(),
        };
    }
    // container
    let allowed_containers: Vec<String> = compat
        .get("container")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_lowercase()))
                .collect()
        })
        .unwrap_or_default();
    let format_raw = probe.format_name_raw.to_lowercase();
    let container = probe.container.to_lowercase();
    let mut container_allowed = allowed_containers.contains(&container);
    if !container_allowed {
        if let Some(aliases) = compat.get("container_aliases").and_then(|v| v.as_object()) {
            for (allowed, alias_list) in aliases {
                if allowed_containers.contains(&allowed.to_lowercase()) {
                    if let Some(arr) = alias_list.as_array() {
                        for alias in arr {
                            if let Some(a) = alias.as_str() {
                                let al = a.to_lowercase();
                                if al == format_raw || format_raw.split(',').any(|x| x.trim() == al)
                                {
                                    container_allowed = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        if !container_allowed {
            for ac in &allowed_containers {
                if format_raw.contains(ac) {
                    container_allowed = true;
                    break;
                }
            }
        }
    }
    if !container_allowed {
        let known = [
            "mp4", "mov", "matroska", "avi", "mpeg", "mpegts", "webm", "flv", "3gp",
        ];
        if known
            .iter()
            .any(|k| container == *k || format_raw.contains(k))
        {
            return CompatibilityStatus {
                status: "conversion_required".to_string(),
                reason: format!(
                    "unsupported container {} (allowed: {:?})",
                    container, allowed_containers
                ),
            };
        } else {
            return CompatibilityStatus {
                status: "unsupported".to_string(),
                reason: format!("unsupported container {}", container),
            };
        }
    }
    // video codec
    let vcodec = probe.video_codec.clone().unwrap_or_default().to_lowercase();
    let allowed_vcodecs: Vec<String> = compat
        .get("video_codec")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_lowercase()))
                .collect()
        })
        .unwrap_or_default();
    let mut vcodec_allowed = allowed_vcodecs.contains(&vcodec);
    if !vcodec_allowed {
        if let Some(alias_map) = compat
            .get("video_codec_aliases")
            .and_then(|v| v.as_object())
        {
            for (allowed, aliases) in alias_map {
                if allowed_vcodecs.contains(&allowed.to_lowercase()) {
                    if let Some(arr) = aliases.as_array() {
                        if arr.iter().any(|a| {
                            a.as_str()
                                .map(|s| s.to_lowercase() == vcodec)
                                .unwrap_or(false)
                        }) {
                            vcodec_allowed = true;
                            break;
                        }
                    }
                }
            }
        }
    }
    if !vcodec_allowed && !vcodec.is_empty() {
        return CompatibilityStatus {
            status: "conversion_required".to_string(),
            reason: format!(
                "unsupported video codec {} (allowed: {:?})",
                vcodec, allowed_vcodecs
            ),
        };
    }
    // pixel format
    if let Some(pix_fmt) = &probe.pix_fmt {
        if let Some(allowed_pix) = compat.get("pixel_format").and_then(|v| v.as_array()) {
            let allowed: Vec<String> = allowed_pix
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_lowercase()))
                .collect();
            if !allowed.is_empty() && !allowed.contains(&pix_fmt.to_lowercase()) {
                return CompatibilityStatus {
                    status: "conversion_required".to_string(),
                    reason: format!(
                        "unsupported pixel format {} (allowed: {:?})",
                        pix_fmt, allowed
                    ),
                };
            }
        }
    }
    // resolution
    if let Some(res) = compat.get("resolution").and_then(|v| v.as_object()) {
        if let (Some(w), Some(max_w)) = (probe.width, res.get("max_width").and_then(|x| x.as_u64()))
        {
            if (w as u64) > max_w {
                return CompatibilityStatus {
                    status: "conversion_required".to_string(),
                    reason: format!("width {} > max {}", w, max_w),
                };
            }
        }
        if let (Some(h), Some(max_h)) =
            (probe.height, res.get("max_height").and_then(|x| x.as_u64()))
        {
            if (h as u64) > max_h {
                return CompatibilityStatus {
                    status: "conversion_required".to_string(),
                    reason: format!("height {} > max {}", h, max_h),
                };
            }
        }
        if let (Some(w), Some(h), Some(max_pixels)) = (
            probe.width,
            probe.height,
            res.get("max_pixels").and_then(|x| x.as_u64()),
        ) {
            if (w as u64) * (h as u64) > max_pixels {
                return CompatibilityStatus {
                    status: "conversion_required".to_string(),
                    reason: format!("resolution {}x{} exceeds max pixels {}", w, h, max_pixels),
                };
            }
        }
    }
    // frame rate
    if let Some(fr) = compat.get("frame_rate").and_then(|v| v.as_object()) {
        if let Some(fps) = probe.fps {
            if let Some(max) = fr.get("max").and_then(|x| x.as_f64()) {
                if (fps as f64) > max {
                    return CompatibilityStatus {
                        status: "conversion_required".to_string(),
                        reason: format!("fps {:.2} > max {}", fps, max),
                    };
                }
            }
            if let Some(min) = fr.get("min").and_then(|x| x.as_f64()) {
                if (fps as f64) < min {
                    return CompatibilityStatus {
                        status: "conversion_required".to_string(),
                        reason: format!("fps {:.2} < min {}", fps, min),
                    };
                }
            }
        }
    }
    // audio codec
    if let Some(acodec) = &probe.audio_codec {
        if let Some(allowed) = compat.get("audio_codec").and_then(|v| v.as_array()) {
            let allowed_str: Vec<String> = allowed
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_lowercase()))
                .collect();
            if !allowed_str.is_empty() && !allowed_str.contains(&acodec.to_lowercase()) {
                return CompatibilityStatus {
                    status: "conversion_required".to_string(),
                    reason: format!(
                        "unsupported audio codec {} (allowed: {:?})",
                        acodec, allowed_str
                    ),
                };
            }
        }
    }
    // audio sample rate
    if let Some(rate) = probe.audio_sample_rate {
        if let Some(asr) = compat.get("audio_sample_rate").and_then(|v| v.as_object()) {
            if let Some(allowed) = asr.get("allowed").and_then(|v| v.as_array()) {
                let allowed_rates: Vec<u64> = allowed.iter().filter_map(|x| x.as_u64()).collect();
                if !allowed_rates.contains(&(rate as u64)) {
                    if let Some(max) = asr.get("max").and_then(|x| x.as_u64()) {
                        if (rate as u64) > max {
                            return CompatibilityStatus {
                                status: "conversion_required".to_string(),
                                reason: format!(
                                    "audio sample rate {} > max {} and not in allowed {:?}",
                                    rate, max, allowed_rates
                                ),
                            };
                        }
                    }
                    return CompatibilityStatus {
                        status: "conversion_required".to_string(),
                        reason: format!(
                            "audio sample rate {} not in allowed {:?}",
                            rate, allowed_rates
                        ),
                    };
                }
            }
        }
    }
    // max file size
    if let Some(max_size) = compat.get("max_file_size").and_then(|x| x.as_u64()) {
        if let Some(fsize) = probe.file_size {
            if fsize > max_size {
                return CompatibilityStatus {
                    status: "unsupported".to_string(),
                    reason: format!("file size {} > max {}", fsize, max_size),
                };
            }
        }
    }
    // stream count
    if let Some(sc) = compat.get("stream_constraints").and_then(|v| v.as_object()) {
        if let Some(max) = sc.get("max_streams").and_then(|x| x.as_u64()) {
            if (probe.streams as u64) > max {
                return CompatibilityStatus {
                    status: "conversion_required".to_string(),
                    reason: format!("stream count {} > max {}", probe.streams, max),
                };
            }
        }
    }
    CompatibilityStatus {
        status: "compatible".to_string(),
        reason: "compatible with preset".to_string(),
    }
}

pub fn conversion_command(input: &Path, output: &Path, _preset: &serde_json::Value) -> Vec<String> {
    vec![
        "ffmpeg".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        input.to_string_lossy().to_string(),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-profile:v".to_string(),
        "baseline".to_string(),
        "-level".to_string(),
        "3.0".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        // NOTE: the comma inside scale=min(...) must be escaped for the
        // ffmpeg filtergraph parser ("\,"); a raw "," breaks parsing with
        // "No option name near 'lanczos'" and the conversion silently fails.
        "-vf".to_string(),
        "scale=min(640\\,iw):-2:flags=lanczos".to_string(),
        "-r".to_string(),
        "30".to_string(),
        "-c:a".to_string(),
        "aac".to_string(),
        "-ar".to_string(),
        "48000".to_string(),
        "-ac".to_string(),
        "2".to_string(),
        "-movflags".to_string(),
        "+faststart".to_string(),
        output.to_string_lossy().to_string(),
    ]
}

pub struct ConversionResult {
    pub success: bool,
    pub output_path: Option<PathBuf>,
    pub stdout: String,
    pub stderr: String,
    pub error: Option<String>,
    pub command: Vec<String>,
    pub probe: Option<ProbeResult>,
}

pub fn convert(input: &Path, temp_dir: &Path, preset: &serde_json::Value) -> ConversionResult {
    let ffmpeg_cfg = preset.get("ffmpeg").and_then(|v| v.as_object());
    let output_ext = ffmpeg_cfg
        .and_then(|m| m.get("output_extension"))
        .and_then(|x| x.as_str())
        .unwrap_or(".mp4");
    let base = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let safe_base: String = base
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe_base = if safe_base.is_empty() {
        "output".to_string()
    } else {
        safe_base
    };
    let mut output_path = temp_dir.join(format!(
        "{}{}",
        safe_base,
        format!(".converted{}", output_ext)
    ));
    let mut counter = 1;
    while output_path.exists() {
        output_path = temp_dir.join(format!("{}.converted_{}{}", safe_base, counter, output_ext));
        counter += 1;
        if counter > 100 {
            return ConversionResult {
                success: false,
                output_path: None,
                stdout: String::new(),
                stderr: String::new(),
                error: Some("overwrite protection: too many existing converted files".to_string()),
                command: vec![],
                probe: None,
            };
        }
    }
    if !temp_dir.exists() {
        let _ = std::fs::create_dir_all(temp_dir);
    }
    let cmd = conversion_command(input, &output_path, preset);
    let mut command = std::process::Command::new(&cmd[0]);
    command.args(&cmd[1..]);
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(0x08000000);
    }
    let output = command.output();
    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            if !o.status.success() {
                if output_path.exists() {
                    let _ = std::fs::remove_file(&output_path);
                }
                return ConversionResult {
                    success: false,
                    output_path: None,
                    stdout,
                    stderr: stderr.clone(),
                    error: Some(format!(
                        "FFmpeg failed with code {:?}: {}",
                        o.status.code(),
                        stderr.chars().take(2000).collect::<String>()
                    )),
                    command: cmd,
                    probe: None,
                };
            }
            if !output_path.exists() {
                return ConversionResult {
                    success: false,
                    output_path: None,
                    stdout,
                    stderr,
                    error: Some("FFmpeg succeeded but output file not found".to_string()),
                    command: cmd,
                    probe: None,
                };
            }
            // Validate with ffprobe
            match probe(&output_path.to_string_lossy()) {
                Ok(probe_out) => {
                    let compat = preset.get("compatibility").unwrap_or(preset);
                    let status = evaluate_compatibility(&probe_out, compat);
                    if status.status != "compatible" {
                        let _ = std::fs::remove_file(&output_path);
                        return ConversionResult {
                            success: false,
                            output_path: None,
                            stdout,
                            stderr,
                            error: Some(format!(
                                "Converted file failed validation: {} (probe: {:?})",
                                status.reason, probe_out
                            )),
                            command: cmd,
                            probe: Some(probe_out),
                        };
                    }
                    ConversionResult {
                        success: true,
                        output_path: Some(output_path),
                        stdout,
                        stderr,
                        error: None,
                        command: cmd,
                        probe: Some(probe_out),
                    }
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&output_path);
                    return ConversionResult {
                        success: false,
                        output_path: None,
                        stdout,
                        stderr,
                        error: Some(format!("ffprobe validation failed: {}", e)),
                        command: cmd,
                        probe: None,
                    };
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => ConversionResult {
            success: false,
            output_path: None,
            stdout: String::new(),
            stderr: String::new(),
            error: Some("FFmpeg not found".to_string()),
            command: cmd,
            probe: None,
        },
        Err(e) => {
            if output_path.exists() {
                let _ = std::fs::remove_file(&output_path);
            }
            ConversionResult {
                success: false,
                output_path: None,
                stdout: String::new(),
                stderr: String::new(),
                error: Some(format!("conversion error: {}", e)),
                command: cmd,
                probe: None,
            }
        }
    }
}

fn parse_fps(s: &str) -> Option<f32> {
    if s.contains('/') {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() == 2 {
            let a: f32 = parts[0].parse().ok()?;
            let b: f32 = parts[1].parse().ok()?;
            if b != 0.0 {
                return Some(a / b);
            }
        }
        None
    } else {
        s.parse().ok()
    }
}

#[cfg(test)]
mod conversion_tests {
    use super::*;

    /// Generate a small test video with ffmpeg (skipped if ffmpeg missing).
    fn make_test_video(dir: &Path, name: &str, extra_args: &[&str]) -> Option<PathBuf> {
        let out = dir.join(name);
        let mut cmd = std::process::Command::new("ffmpeg");
        cmd.args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1:size=320x240:rate=30",
        ]);
        cmd.args(extra_args);
        cmd.arg(out.to_string_lossy().to_string());
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let status = cmd.output().ok()?;
        if status.status.success() && out.exists() {
            Some(out)
        } else {
            None
        }
    }

    fn ffmpeg_available() -> bool {
        std::process::Command::new("ffmpeg")
            .arg("-version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn preset() -> serde_json::Value {
        crate::profile::load_profile()
            .map(|p| p.video_preset)
            .unwrap_or(serde_json::json!({}))
    }

    /// Compatible video (h264/yuv420p/mp4) must NOT require conversion.
    #[test]
    fn compatible_video_evaluates_compatible() {
        if !ffmpeg_available() {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        let vid = make_test_video(
            tmp.path(),
            "good.mp4",
            &[
                "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-ar", "48000",
            ],
        )
        .expect("ffmpeg should produce test video");
        let probe_result = probe(vid.to_string_lossy().as_ref()).expect("ffprobe must work");
        let eval = evaluate_compatibility(&probe_result, &preset());
        assert_eq!(
            eval.status, "compatible",
            "test video must be compatible: {} ({})",
            eval.status, eval.reason
        );
    }

    /// Incompatible codec (mpeg4 or high-res) must require conversion.
    #[test]
    fn incompatible_video_requires_conversion() {
        if !ffmpeg_available() {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        // 4K exceeds the conservative preset max width
        let vid = make_test_video(
            tmp.path(),
            "big.mp4",
            &["-c:v", "libx264", "-pix_fmt", "yuv420p", "-s", "3840x2160"],
        )
        .expect("ffmpeg should produce test video");
        let probe_result = probe(vid.to_string_lossy().as_ref()).expect("ffprobe must work");
        let eval = evaluate_compatibility(&probe_result, &preset());
        assert_eq!(
            eval.status, "conversion_required",
            "4K video must require conversion: {}",
            eval.reason
        );
    }

    /// Incompatible container (flv — not in the preset's allowed list) must
    /// require conversion.
    #[test]
    fn incompatible_container_requires_conversion() {
        if !ffmpeg_available() {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        let vid = make_test_video(
            tmp.path(),
            "video.flv",
            &["-c:v", "libx264", "-pix_fmt", "yuv420p"],
        )
        .expect("ffmpeg should produce flv");
        let probe_result = probe(vid.to_string_lossy().as_ref()).expect("ffprobe must work");
        let eval = evaluate_compatibility(&probe_result, &preset());
        assert_eq!(
            eval.status, "conversion_required",
            "flv must require conversion: {}",
            eval.reason
        );
    }

    /// REAL conversion: staged in temp, ffprobe-validated, output compatible.
    /// The ORIGINAL file must remain untouched (size + mtime).
    #[test]
    fn conversion_produces_validated_output_and_original_untouched() {
        if !ffmpeg_available() {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        let vid = make_test_video(
            tmp.path(),
            "src.mp4",
            &["-c:v", "libx264", "-pix_fmt", "yuv420p", "-s", "3840x2160"],
        )
        .expect("ffmpeg should produce test video");
        let original_size = std::fs::metadata(&vid).unwrap().len();
        let stage = tempfile::TempDir::new().unwrap();
        let result = convert(&vid, stage.path(), &preset());
        assert!(
            result.success,
            "conversion must succeed: {:?}",
            result.error
        );
        let out = result.output_path.expect("output path must be set");
        assert!(out.exists(), "staged output must exist");
        assert!(
            out.starts_with(stage.path()),
            "output must be staged in temp dir"
        );
        // Converted output is ffprobe-validated and compatible
        let probe_out =
            probe(out.to_string_lossy().as_ref()).expect("converted output must be probeable");
        let eval = evaluate_compatibility(&probe_out, &preset());
        assert_eq!(
            eval.status, "compatible",
            "converted output must pass compatibility: {}",
            eval.reason
        );
        // Original untouched
        assert_eq!(std::fs::metadata(&vid).unwrap().len(), original_size);
    }

    /// Conversion of a NON-video input must fail and leave no staged output.
    #[test]
    fn conversion_failure_removes_staged_output() {
        if !ffmpeg_available() {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        let not_video = tmp.path().join("not_a_video.bin");
        std::fs::write(&not_video, b"garbage bytes").unwrap();
        let stage = tempfile::TempDir::new().unwrap();
        let result = convert(&not_video, stage.path(), &preset());
        assert!(!result.success, "garbage input must fail to convert");
        // No staged output remains
        let leftovers: Vec<_> = walkdir::WalkDir::new(stage.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();
        assert!(
            leftovers.is_empty(),
            "failed conversion must leave no staged files: {:?}",
            leftovers
        );
    }
}
