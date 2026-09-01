import json, subprocess, pathlib, tempfile, hashlib, os, shlex

# Video inspection and conversion service — data-driven from video_presets.json

def probe(path: pathlib.Path):
    """Run ffprobe as authoritative inspection. Returns dict with probe fields or raises inspection_error."""
    try:
        # Use ffprobe JSON output
        cmd = ["ffprobe", "-v", "quiet", "-print_format", "json", "-show_format", "-show_streams", str(path)]
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
        if result.returncode != 0:
            raise RuntimeError(f"ffprobe failed: {result.stderr.strip()}")
        data = json.loads(result.stdout)
        # Parse streams
        streams = data.get("streams", [])
        format_info = data.get("format", {})
        # Find first video stream
        video_stream = next((s for s in streams if s.get("codec_type") == "video"), None)
        audio_stream = next((s for s in streams if s.get("codec_type") == "audio"), None)
        format_name = format_info.get("format_name", "unknown")
        # ffprobe format_name can be comma-separated aliases
        container = format_name.split(",")[0] if format_name else "unknown"
        probe_result = {
            "container": container,
            "format_name_raw": format_name,
            "video_codec": video_stream.get("codec_name") if video_stream else None,
            "video_profile": video_stream.get("profile") if video_stream else None,
            "video_level": video_stream.get("level") if video_stream else None,
            "pix_fmt": video_stream.get("pix_fmt") if video_stream else None,
            "width": video_stream.get("width") if video_stream else None,
            "height": video_stream.get("height") if video_stream else None,
            "fps": None,
            "audio_codec": audio_stream.get("codec_name") if audio_stream else None,
            "audio_sample_rate": int(audio_stream.get("sample_rate")) if audio_stream and audio_stream.get("sample_rate") else None,
            "streams": len(streams),
            "has_video": video_stream is not None,
            "file_size": pathlib.Path(path).stat().st_size if pathlib.Path(path).exists() else None,
            "raw": data,
        }
        # Parse fps
        if video_stream:
            fps_str = video_stream.get("avg_frame_rate") or video_stream.get("r_frame_rate")
            if fps_str and fps_str != "0/0":
                try:
                    if "/" in fps_str:
                        num, den = fps_str.split("/")
                        probe_result["fps"] = float(num) / float(den) if float(den) != 0 else None
                    else:
                        probe_result["fps"] = float(fps_str)
                except:
                    probe_result["fps"] = None
        return probe_result
    except FileNotFoundError:
        raise RuntimeError("ffprobe not found")
    except Exception as e:
        raise RuntimeError(f"ffprobe inspection error: {e}")

def evaluate_compatibility(probe_result: dict, preset_or_compat: dict):
    """Evaluate probe result against preset compatibility. Returns (status, reason)."""
    # inspection_error already handled by caller if probe raised
    if not probe_result or not probe_result.get("has_video"):
        return ("unsupported", "no video stream found")
    # Accept either full preset (with 'compatibility' key) or direct compat dict
    if isinstance(preset_or_compat, dict) and "compatibility" in preset_or_compat and isinstance(preset_or_compat["compatibility"], dict) and "container" not in preset_or_compat:
        # It's a full preset, extract compatibility
        compat = preset_or_compat.get("compatibility", {})
    elif isinstance(preset_or_compat, dict) and "container" in preset_or_compat:
        # It's already the compat dict
        compat = preset_or_compat
    else:
        compat = preset_or_compat.get("compatibility", {}) if isinstance(preset_or_compat, dict) else {}
    # Check container
    container = (probe_result.get("container") or "").lower()
    allowed_containers = [c.lower() for c in compat.get("container", [])]
    # Also check aliases: if format_name_raw contains any allowed alias
    format_raw = (probe_result.get("format_name_raw") or "").lower()
    container_allowed = False
    if container in allowed_containers:
        container_allowed = True
    else:
        # Check if any allowed container's aliases match
        aliases = compat.get("container_aliases", {})
        for allowed, alias_list in aliases.items():
            if allowed.lower() in allowed_containers:
                for alias in alias_list:
                    if alias.lower() == format_raw or alias.lower() in format_raw.split(","):
                        container_allowed = True
                        break
        # Also check if raw format_name contains allowed container
        if not container_allowed:
            for ac in allowed_containers:
                if ac in format_raw:
                    container_allowed = True
                    break
    if not container_allowed:
        # If container completely unknown, check if it's in hints? For now conversion_required if not allowed, but if unknown then unsupported
        # We'll treat unknown container as conversion_required if it's a known video container type but not in allowed list
        # Otherwise unsupported
        known_video_containers = ["mp4", "mov", "matroska", "avi", "mpeg", "mpegts", "webm", "flv", "3gp"]
        if container in known_video_containers or any(k in format_raw for k in known_video_containers):
            return ("conversion_required", f"unsupported container {container} (allowed: {allowed_containers})")
        else:
            return ("unsupported", f"unsupported container {container}")

    # Check video codec
    vcodec = (probe_result.get("video_codec") or "").lower()
    allowed_vcodecs = [c.lower() for c in compat.get("video_codec", [])]
    # Handle aliases
    alias_map = compat.get("video_codec_aliases", {})
    vcodec_allowed = False
    if vcodec in allowed_vcodecs:
        vcodec_allowed = True
    else:
        for allowed, aliases in alias_map.items():
            if allowed.lower() in allowed_vcodecs and vcodec in [a.lower() for a in aliases]:
                vcodec_allowed = True
                break
    if not vcodec_allowed:
        return ("conversion_required", f"unsupported video codec {vcodec} (allowed: {allowed_vcodecs})")

    # Pixel format
    pix_fmt = (probe_result.get("pix_fmt") or "").lower()
    allowed_pix = [p.lower() for p in compat.get("pixel_format", [])]
    if allowed_pix and pix_fmt and pix_fmt not in allowed_pix:
        return ("conversion_required", f"unsupported pixel format {pix_fmt} (allowed: {allowed_pix})")

    # Resolution
    res = compat.get("resolution", {})
    max_w = res.get("max_width")
    max_h = res.get("max_height")
    max_pixels = res.get("max_pixels")
    w = probe_result.get("width")
    h = probe_result.get("height")
    if w and max_w and w > max_w:
        return ("conversion_required", f"width {w} > max {max_w}")
    if h and max_h and h > max_h:
        return ("conversion_required", f"height {h} > max {max_h}")
    if w and h and max_pixels and (w * h) > max_pixels:
        return ("conversion_required", f"resolution {w}x{h} exceeds max pixels {max_pixels}")

    # Frame rate
    fr = compat.get("frame_rate", {})
    max_fps = fr.get("max")
    min_fps = fr.get("min")
    fps = probe_result.get("fps")
    if fps is not None:
        if max_fps and fps > max_fps:
            return ("conversion_required", f"fps {fps:.2f} > max {max_fps}")
        if min_fps and fps < min_fps:
            return ("conversion_required", f"fps {fps:.2f} < min {min_fps}")

    # Audio codec
    acodec = (probe_result.get("audio_codec") or "").lower()
    allowed_acodecs = [c.lower() for c in compat.get("audio_codec", [])]
    if acodec and allowed_acodecs and acodec not in allowed_acodecs:
        return ("conversion_required", f"unsupported audio codec {acodec} (allowed: {allowed_acodecs})")

    # Audio sample rate
    asr = compat.get("audio_sample_rate", {})
    allowed_rates = asr.get("allowed", [])
    max_rate = asr.get("max")
    probe_rate = probe_result.get("audio_sample_rate")
    if probe_rate is not None:
        if allowed_rates and probe_rate not in allowed_rates and max_rate and probe_rate > max_rate:
            return ("conversion_required", f"audio sample rate {probe_rate} > max {max_rate} and not in allowed {allowed_rates}")
        if allowed_rates and probe_rate not in allowed_rates:
            # If not in allowed but <= max, still require conversion? For conservative, require conversion if not in allowed
            if probe_rate not in allowed_rates:
                return ("conversion_required", f"audio sample rate {probe_rate} not in allowed {allowed_rates}")

    # Max file size
    max_size = compat.get("max_file_size")
    fsize = probe_result.get("file_size")
    if max_size and fsize and fsize > max_size:
        return ("unsupported", f"file size {fsize} > max {max_size}")

    # Stream count
    sc = compat.get("stream_constraints", {})
    max_streams = sc.get("max_streams")
    if max_streams and probe_result.get("streams", 0) > max_streams:
        return ("conversion_required", f"stream count {probe_result.get('streams')} > max {max_streams}")

    return ("compatible", "compatible with preset")

def conversion_command(input_path: pathlib.Path, output_path: pathlib.Path, preset: dict):
    """Generate deterministic FFmpeg command from preset template. Returns list."""
    # Use preset's ffmpeg command_template if present, else default
    ffmpeg_cfg = preset.get("ffmpeg", {})
    template = ffmpeg_cfg.get("command_template", "ffmpeg -y -i <input> -c:v libx264 -profile:v baseline -level 3.0 -pix_fmt yuv420p -vf scale='min(640,iw)':-2:flags=lanczos -r 30 -c:a aac -ar 48000 -ac 2 -movflags +faststart <output>.mp4")
    # Replace placeholders
    # <input> and <output> are quoted
    cmd_str = template.replace("<input>", str(input_path)).replace("<output>", str(output_path.with_suffix('')))
    # Remove output extension handling: template already includes <output>.mp4, but we replace <output> with stem, so result is <stem>.mp4
    # If output_path already has .mp4, and template does <output>.mp4, we get double? Let's handle: if output_path is already .mp4, use it directly
    # Simplify: if "<output>" in template, we already replaced; if template contains "<output>.mp4", after replace it becomes "/tmp/input.converted.mp4.mp4"? Need to handle
    # Better: if output_path is provided, use its string directly for <output> placeholder that expects full path
    # For now, just use shlex split on the replaced string, but handle quotes
    # We need to handle the scale filter quotes: -vf scale='min(640,iw)':-2:flags=lanczos
    # Use shlex with posix=False for Windows?
    try:
        # Ensure input/output are properly quoted for shlex
        # Instead, construct command list manually for deterministic behavior
        # Use preset's ffmpeg config to build command
        output_ext = ffmpeg_cfg.get("output_extension", ".mp4")
        # Deterministic: output_path should already have correct extension
        # Build command list explicitly
        cmd = [
            "ffmpeg", "-y",
            "-i", str(input_path),
            "-c:v", "libx264",
            "-profile:v", "baseline",
            "-level", "3.0",
            "-pix_fmt", "yuv420p",
            # NOTE: the comma inside scale=min(...) must be escaped for the
            # ffmpeg filtergraph parser; a raw "," breaks parsing and the
            # conversion silently fails (mirrors the Rust fix).
            "-vf", "scale=min(640\\,iw):-2:flags=lanczos",
            "-r", "30",
            "-c:a", "aac",
            "-ar", "48000",
            "-ac", "2",
            "-movflags", "+faststart",
            str(output_path)
        ]
        return cmd
    except Exception as e:
        raise RuntimeError(f"failed to generate conversion command: {e}")

def convert(input_path: pathlib.Path, temp_dir: pathlib.Path, preset: dict):
    """Convert video to temp workspace, validate output, return result dict. Never modifies original, deterministic naming, overwrite protection, captures diagnostics."""
    # Deterministic output naming: <input_basename>.converted.mp4 in temp_dir
    # Use preset's output_extension
    ffmpeg_cfg = preset.get("ffmpeg", {})
    output_ext = ffmpeg_cfg.get("output_extension", ".mp4")
    base = input_path.stem
    # Sanitize base for filesystem
    safe_base = "".join(c if c.isalnum() or c in ("-", "_") else "_" for c in base) or "output"
    output_path = temp_dir / f"{safe_base}.converted{output_ext}"
    # Overwrite protection: if exists, add suffix _1, _2, etc.
    counter = 1
    original_output = output_path
    while output_path.exists():
        output_path = temp_dir / f"{safe_base}.converted_{counter}{output_ext}"
        counter += 1
        if counter > 100:
            raise RuntimeError("overwrite protection: too many existing converted files")
    # Ensure temp_dir exists and is a temp workspace (not SD)
    if not temp_dir.exists():
        temp_dir.mkdir(parents=True, exist_ok=True)
    # Generate command
    cmd = conversion_command(input_path, output_path, preset)
    # Run FFmpeg, capture stdout/stderr
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
        stdout = result.stdout
        stderr = result.stderr
        if result.returncode != 0:
            # Clean temp file if partially created
            if output_path.exists():
                try:
                    output_path.unlink()
                except:
                    pass
            return {
                "success": False,
                "output_path": None,
                "stdout": stdout,
                "stderr": stderr,
                "error": f"FFmpeg failed with code {result.returncode}: {stderr[:2000]}",
                "command": cmd,
            }
        # Validate output exists
        if not output_path.exists():
            return {
                "success": False,
                "output_path": None,
                "stdout": stdout,
                "stderr": stderr,
                "error": "FFmpeg succeeded but output file not found",
                "command": cmd,
            }
        # Validate with ffprobe
        try:
            probe_out = probe(output_path)
            # Evaluate converted output against same preset: should be compatible
            compat = preset.get("compatibility", {})
            status2, reason2 = evaluate_compatibility(probe_out, compat)
            if status2 != "compatible":
                # If still not compatible, treat as invalid conversion
                try:
                    output_path.unlink()
                except:
                    pass
                return {
                    "success": False,
                    "output_path": None,
                    "stdout": stdout,
                    "stderr": stderr,
                    "error": f"Converted file failed validation: {reason2} (probe: {probe_out})",
                    "command": cmd,
                    "probe": probe_out,
                }
        except Exception as e:
            try:
                output_path.unlink()
            except:
                pass
            return {
                "success": False,
                "output_path": None,
                "stdout": stdout,
                "stderr": stderr,
                "error": f"ffprobe validation failed: {e}",
                "command": cmd,
            }
        # Success: keep temp file for planner to use, caller must clean up eventually
        return {
            "success": True,
            "output_path": output_path,
            "stdout": stdout,
            "stderr": stderr,
            "error": None,
            "command": cmd,
            "probe": probe_out,
        }
    except FileNotFoundError:
        return {
            "success": False,
            "output_path": None,
            "stdout": "",
            "stderr": "",
            "error": "FFmpeg not found",
            "command": cmd,
        }
    except subprocess.TimeoutExpired as e:
        if output_path.exists():
            try:
                output_path.unlink()
            except:
                pass
        return {
            "success": False,
            "output_path": None,
            "stdout": e.stdout.decode() if isinstance(e.stdout, bytes) else str(e.stdout),
            "stderr": e.stderr.decode() if isinstance(e.stderr, bytes) else str(e.stderr),
            "error": f"FFmpeg timeout: {e}",
            "command": cmd,
        }
    except Exception as e:
        if output_path.exists():
            try:
                output_path.unlink()
            except:
                pass
        return {
            "success": False,
            "output_path": None,
            "stdout": "",
            "stderr": "",
            "error": f"conversion error: {e}",
            "command": cmd,
        }
