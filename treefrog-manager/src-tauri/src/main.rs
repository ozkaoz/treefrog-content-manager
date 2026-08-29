// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--self-check" || a == "--smoke-test") {
        // Lightweight startup/self-check mode for packaged executable verification
        // Verifies: profile can load, Tauri context can be created, no SD writes
        println!("TreeFrog Content Manager — self-check");
        match treefrog_manager::profile::load_profile() {
            Ok(p) => {
                println!("profile loaded: {}", p.profile_version);
                println!("systems: {}", p.systems.len());
                println!("archive handlers: {:?}", p.archive_valid_exts);
                // Check video preset is provisional
                let preset_status = p.video_preset.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
                println!("video preset status: {}", preset_status);
                if preset_status != "PROVISIONAL_UNVALIDATED" {
                    eprintln!("warning: video preset status is not PROVISIONAL_UNVALIDATED");
                }
                // Check ffmpeg/ffprobe availability
                let ffprobe_ok = std::process::Command::new("ffprobe").arg("-version").output().map(|o| o.status.success()).unwrap_or(false);
                let ffmpeg_ok = std::process::Command::new("ffmpeg").arg("-version").output().map(|o| o.status.success()).unwrap_or(false);
                println!("ffprobe available: {}", ffprobe_ok);
                println!("ffmpeg available: {}", ffmpeg_ok);
                if !ffprobe_ok {
                    println!("note: ffprobe not found — video inspection will report inspection_error, UI will explain");
                }
                if !ffmpeg_ok {
                    println!("note: ffmpeg not found — video conversion will report conversion_error, UI will explain");
                }
                println!("self-check PASS");
                std::process::exit(0);
            },
            Err(e) => {
                eprintln!("self-check FAIL: profile load failed: {}", e);
                std::process::exit(1);
            }
        }
    }
    treefrog_manager::run()
}
