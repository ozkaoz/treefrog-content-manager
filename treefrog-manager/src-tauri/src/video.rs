// ffprobe adapter — conservative default preset PROVISIONAL_UNVALIDATED
// Hardware decoder variance means we never claim compat without physical device.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProbeResult {
    pub container: String,
    pub video_codec: Option<String>,
    pub profile_level: Option<String>,
    pub pix_fmt: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f32>,
    pub audio_codec: Option<String>,
    pub streams: usize,
    pub compatible: bool,
    pub reason: String,
}

pub fn probe(path: &str) -> anyhow::Result<ProbeResult> {
    // Call ffprobe if available; otherwise mock for dry-run
    let out = std::process::Command::new("ffprobe")
        .args(["-v","quiet","-print_format","json","-show_format","-show_streams", path])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let v: serde_json::Value = serde_json::from_slice(&o.stdout)?;
            // Simplified parse — in real use map streams to fields and check against video_presets.json
            let streams = v["streams"].as_array().map(|a| a.len()).unwrap_or(0);
            Ok(ProbeResult {
                container: v["format"]["format_name"].as_str().unwrap_or("unknown").to_string(),
                video_codec: v["streams"][0]["codec_name"].as_str().map(|s| s.to_string()),
                profile_level: v["streams"][0]["profile"].as_str().map(|s| s.to_string()),
                pix_fmt: v["streams"][0]["pix_fmt"].as_str().map(|s| s.to_string()),
                width: v["streams"][0]["width"].as_u64().map(|w| w as u32),
                height: v["streams"][0]["height"].as_u64().map(|h| h as u32),
                fps: v["streams"][0]["avg_frame_rate"].as_str().and_then(|s| parse_fps(s)),
                audio_codec: None,
                streams,
                compatible: false, // conservative: mark incompatible unless validated
                reason: "PROVISIONAL_UNVALIDATED — conservative, requires device validation".into(),
            })
        },
        _ => {
            // ffprobe not available in test env — return provisional
            Ok(ProbeResult {
                container: "unknown".into(),
                video_codec: None,
                profile_level: None,
                pix_fmt: None,
                width: None,
                height: None,
                fps: None,
                audio_codec: None,
                streams: 0,
                compatible: false,
                reason: "ffprobe not available — staged for later probe".into(),
            })
        }
    }
}

fn parse_fps(s: &str) -> Option<f32> {
    if s.contains('/') {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len()==2 {
            let a: f32 = parts[0].parse().ok()?;
            let b: f32 = parts[1].parse().ok()?;
            if b!=0.0 { return Some(a/b); }
        }
        None
    } else {
        s.parse().ok()
    }
}
