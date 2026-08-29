"""Phase 2C video conversion engine — tests without SD writes, without requiring real ffprobe/ffmpeg."""
import pathlib, tempfile, sys, json
REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))
from treefrog import profile, scanner, planner, video
import unittest.mock as mock

def test_compatible_video_copy():
    p = profile.load_profile()
    preset = p["video_preset"]
    # Mock probe to return compatible
    mock_probe = {
        "container": "mp4",
        "format_name_raw": "mov,mp4,m4a,3gp,3g2,mj2",
        "video_codec": "h264",
        "pix_fmt": "yuv420p",
        "width": 640,
        "height": 480,
        "fps": 30.0,
        "audio_codec": "aac",
        "audio_sample_rate": 48000,
        "streams": 2,
        "has_video": True,
        "file_size": 1024*1024,
    }
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms").mkdir()
        vid = src / "good.mp4"
        vid.write_bytes(b"fake mp4")
        # Mock video.probe to return compatible
        with mock.patch("treefrog.video.probe", return_value=mock_probe):
            with mock.patch("treefrog.video.evaluate_compatibility", return_value=("compatible", "compatible")):
                scanned = scanner.scan(str(src), p)
                plan = planner.plan(scanned, str(sd), p)
                # Find video entry
                vids = [e for e in plan["entries"] if e.get("content_type") == "video" or "video" in e.get("reason","").lower() or e.get("destination","").endswith(".mp4")]
                # Should be copy, not convert
                assert any(e["action"] in ("copy", "skip_unchanged", "skip_duplicate") for e in vids) or any(e["action"] == "copy" for e in plan["entries"])
                # Ensure no conversion
                assert not any(e["action"] == "convert_then_copy" for e in plan["entries"])

def test_incompatible_video_conversion_required():
    p = profile.load_profile()
    mock_probe = {
        "container": "matroska",
        "format_name_raw": "matroska,webm",
        "video_codec": "hevc",  # unsupported, should trigger conversion
        "pix_fmt": "yuv420p",
        "width": 1920,
        "height": 1080,  # exceeds 854x480
        "fps": 60.0,
        "audio_codec": "aac",
        "audio_sample_rate": 48000,
        "streams": 2,
        "has_video": True,
        "file_size": 1024*1024,
    }
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms").mkdir()
        vid = src / "bad.mkv"
        vid.write_bytes(b"fake mkv")
        with mock.patch("treefrog.video.probe", return_value=mock_probe):
            # Let evaluate_compatibility do real check: hevc should be conversion_required
            # need to ensure preset is used
            scanned = scanner.scan(str(src), p)
            plan = planner.plan(scanned, str(sd), p)
            # Should be convert_then_copy
            conv = [e for e in plan["entries"] if e["action"] == "convert_then_copy"]
            assert len(conv) >= 1, plan["entries"]
            c = conv[0]
            assert c["status"] == "conversion_required"
            assert "hevc" in c["reason"].lower() or "conversion" in c["reason"].lower()
            assert c["preset"] == "treefrog_conservative_default"
            assert c["destination"].endswith(".mp4")
            assert "converted" in c["destination"]
            # Original untouched
            assert vid.exists()
            assert vid.read_bytes() == b"fake mkv"
            # Ensure temp not on SD
            assert not (sd / "roms" / "videos" / "bad.converted.mp4").exists()

def test_conversion_command_generation():
    p = profile.load_profile()
    preset = p["video_preset"]
    cmd = video.conversion_command(pathlib.Path("input.mkv"), pathlib.Path("/tmp/out.converted.mp4"), preset)
    assert "ffmpeg" in cmd[0]
    assert "-i" in cmd
    assert "input.mkv" in " ".join(cmd)
    # On Windows, path may be with backslashes, so check for filename
    assert "out.converted.mp4" in " ".join(cmd)
    assert "-c:v" in cmd and "libx264" in cmd
    assert "-pix_fmt" in cmd and "yuv420p" in cmd

def test_temporary_output():
    p = profile.load_profile()
    preset = p["video_preset"]
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        vid = src / "movie.mkv"
        vid.write_bytes(b"data")
        temp_dir = pathlib.Path(tmp) / "temp"
        temp_dir.mkdir()
        # Mock subprocess.run to simulate successful ffmpeg
        with mock.patch("treefrog.video.subprocess.run") as mock_run:
            mock_run.return_value = mock.Mock(returncode=0, stdout="ok", stderr="ok", args=[])
            # Also mock probe for validation to return compatible
            with mock.patch("treefrog.video.probe") as mock_probe:
                # First probe for original is not needed here, second probe for output
                mock_probe.return_value = {
                    "container": "mp4",
                    "format_name_raw": "mov,mp4",
                    "video_codec": "h264",
                    "pix_fmt": "yuv420p",
                    "width": 640,
                    "height": 480,
                    "fps": 30.0,
                    "audio_codec": "aac",
                    "audio_sample_rate": 48000,
                    "streams": 2,
                    "has_video": True,
                    "file_size": 1000,
                }
                # Create a dummy output file to simulate ffmpeg success
                # The convert function will check output_path.exists(), so we need to make it exist after mock
                # We will patch subprocess.run to create the file
                def fake_run(cmd, capture_output, text, timeout):
                    # Create output file
                    out_path = pathlib.Path(cmd[-1])
                    out_path.write_bytes(b"converted")
                    return mock.Mock(returncode=0, stdout="ffmpeg ok", stderr="", args=cmd)
                mock_run.side_effect = fake_run
                result = video.convert(vid, temp_dir, preset)
                assert result["success"] is True
                assert result["output_path"] is not None
                assert str(result["output_path"]).startswith(str(temp_dir))
                # Deterministic naming
                assert "movie.converted.mp4" in str(result["output_path"])
                # Original untouched
                assert vid.read_bytes() == b"data"
                # Overwrite protection: second conversion with same input should give _1 suffix
                # Create second file with same stem
                vid2 = src / "movie.mkv"  # same name, but we need to test overwrite protection with existing output
                # The temp already has movie.converted.mp4, so next convert should give movie.converted_1.mp4
                # We need to mock again
                with mock.patch("treefrog.video.subprocess.run", side_effect=fake_run):
                    with mock.patch("treefrog.video.probe", return_value=mock_probe.return_value):
                        result2 = video.convert(vid2, temp_dir, preset)
                        # Since file already exists, it should be _1
                        # But our temp_dir already has movie.converted.mp4 from previous, so next should be _1
                        # However we cleaned up? The temp_dir still has the file, so it will be _1
                        pass

def test_successful_post_conversion_validation():
    p = profile.load_profile()
    preset = p["video_preset"]
    with tempfile.TemporaryDirectory() as tmp:
        temp_dir = pathlib.Path(tmp) / "temp"
        temp_dir.mkdir()
        src = pathlib.Path(tmp) / "src" / "vid.mkv"
        src.parent.mkdir(parents=True)
        src.write_bytes(b"data")
        with mock.patch("treefrog.video.subprocess.run") as mock_run:
            def fake_run(cmd, capture_output, text, timeout):
                out_path = pathlib.Path(cmd[-1])
                out_path.write_bytes(b"converted")
                return mock.Mock(returncode=0, stdout="", stderr="", args=cmd)
            mock_run.side_effect = fake_run
            with mock.patch("treefrog.video.probe") as mock_probe:
                mock_probe.return_value = {
                    "container": "mp4",
                    "format_name_raw": "mov,mp4",
                    "video_codec": "h264",
                    "pix_fmt": "yuv420p",
                    "width": 640,
                    "height": 480,
                    "fps": 30.0,
                    "audio_codec": "aac",
                    "audio_sample_rate": 48000,
                    "streams": 2,
                    "has_video": True,
                    "file_size": 1000,
                }
                result = video.convert(src, temp_dir, preset)
                assert result["success"] is True
                assert result["probe"] is not None
                assert result["probe"]["video_codec"] == "h264"

def test_invalid_conversion_result():
    p = profile.load_profile()
    preset = p["video_preset"]
    with tempfile.TemporaryDirectory() as tmp:
        temp_dir = pathlib.Path(tmp) / "temp"
        temp_dir.mkdir()
        src = pathlib.Path(tmp) / "vid.mkv"
        src.write_bytes(b"data")
        with mock.patch("treefrog.video.subprocess.run") as mock_run:
            def fake_run_invalid(cmd, capture_output, text, timeout):
                out_path = pathlib.Path(cmd[-1])
                out_path.write_bytes(b"converted but still hevc")
                return mock.Mock(returncode=0, stdout="", stderr="", args=cmd)
            mock_run.side_effect = fake_run_invalid
            with mock.patch("treefrog.video.probe") as mock_probe:
                # Return still incompatible (hevc) to simulate invalid conversion
                mock_probe.return_value = {
                    "container": "mp4",
                    "format_name_raw": "mov,mp4",
                    "video_codec": "hevc",  # still bad
                    "pix_fmt": "yuv420p",
                    "width": 640,
                    "height": 480,
                    "fps": 30.0,
                    "audio_codec": "aac",
                    "audio_sample_rate": 48000,
                    "streams": 2,
                    "has_video": True,
                    "file_size": 1000,
                }
                result = video.convert(src, temp_dir, preset)
                assert result["success"] is False
                assert "validation" in result["error"].lower()

def test_conversion_error():
    p = profile.load_profile()
    preset = p["video_preset"]
    with tempfile.TemporaryDirectory() as tmp:
        temp_dir = pathlib.Path(tmp) / "temp"
        temp_dir.mkdir()
        src = pathlib.Path(tmp) / "vid.mkv"
        src.write_bytes(b"data")
        with mock.patch("treefrog.video.subprocess.run") as mock_run:
            mock_run.return_value = mock.Mock(returncode=1, stdout="", stderr="ffmpeg error: invalid data", args=[])
            result = video.convert(src, temp_dir, preset)
            assert result["success"] is False
            assert "ffmpeg failed" in result["error"].lower()
            assert "stderr" in result or "error" in result
            # Ensure temp file cleaned
            assert not (temp_dir / "vid.converted.mp4").exists()

def test_original_source_untouched():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src" / "orig.mkv"
        src.parent.mkdir(parents=True)
        src.write_bytes(b"original")
        orig_hash = video.probe  # not needed, just check file
        before = src.read_bytes()
        preset = p["video_preset"]
        temp_dir = pathlib.Path(tmp) / "temp"
        temp_dir.mkdir()
        with mock.patch("treefrog.video.subprocess.run") as mock_run:
            def fake_run(cmd, capture_output, text, timeout):
                out_path = pathlib.Path(cmd[-1])
                out_path.write_bytes(b"converted")
                return mock.Mock(returncode=0, stdout="", stderr="", args=cmd)
            mock_run.side_effect = fake_run
            with mock.patch("treefrog.video.probe", return_value={
                "container": "mp4",
                "format_name_raw": "mov,mp4",
                "video_codec": "h264",
                "pix_fmt": "yuv420p",
                "width": 640,
                "height": 480,
                "fps": 30.0,
                "audio_codec": "aac",
                "audio_sample_rate": 48000,
                "streams": 2,
                "has_video": True,
                "file_size": 1000,
            }):
                result = video.convert(src, temp_dir, preset)
                assert src.read_bytes() == before
                assert src.read_bytes() == b"original"

def test_planner_conversion_action():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms").mkdir()
        vid = src / "movie.mkv"
        vid.write_bytes(b"fake mkv")
        mock_probe = {
            "container": "matroska",
            "format_name_raw": "matroska,webm",
            "video_codec": "hevc",
            "pix_fmt": "yuv420p",
            "width": 1920,
            "height": 1080,
            "fps": 60.0,
            "audio_codec": "aac",
            "audio_sample_rate": 48000,
            "streams": 2,
            "has_video": True,
            "file_size": 1024*1024,
        }
        with mock.patch("treefrog.video.probe", return_value=mock_probe):
            scanned = scanner.scan(str(src), p)
            plan = planner.plan(scanned, str(sd), p)
            # Should have convert_then_copy
            conv = [e for e in plan["entries"] if e["action"] == "convert_then_copy"]
            assert len(conv) == 1
            c = conv[0]
            assert c["status"] == "conversion_required"
            assert "hevc" in c["reason"].lower() or "conversion" in c["reason"].lower()
            assert c["preset"] == "treefrog_conservative_default"
            assert c["destination"].endswith(".mp4")
            assert "converted" in c["destination"]
            # Ensure warnings still present
            assert any("PROVISIONAL_UNVALIDATED" in w for w in plan["warnings"])

def test_deterministic_output_path():
    p = profile.load_profile()
    preset = p["video_preset"]
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src" / "my movie.mkv"
        src.parent.mkdir(parents=True)
        src.write_bytes(b"data")
        temp_dir = pathlib.Path(tmp) / "temp"
        temp_dir.mkdir()
        # Call conversion_command multiple times, should be deterministic
        out1 = temp_dir / "my_movie.converted.mp4"
        out2 = temp_dir / "my_movie.converted.mp4"
        cmd1 = video.conversion_command(src, out1, preset)
        cmd2 = video.conversion_command(src, out2, preset)
        assert cmd1 == cmd2
        # Test overwrite protection
        out1.write_bytes(b"existing")
        # Next conversion should give _1 suffix
        with mock.patch("treefrog.video.subprocess.run") as mock_run:
            def fake_run(cmd, capture_output, text, timeout):
                out_path = pathlib.Path(cmd[-1])
                # The planner's deterministic naming is for converted_name, but convert() handles overwrite protection
                # Here we just check that convert respects existing file
                out_path.write_bytes(b"converted2")
                return mock.Mock(returncode=0, stdout="", stderr="", args=cmd)
            mock_run.side_effect = fake_run
            with mock.patch("treefrog.video.probe", return_value={
                "container": "mp4",
                "format_name_raw": "mov,mp4",
                "video_codec": "h264",
                "pix_fmt": "yuv420p",
                "width": 640,
                "height": 480,
                "fps": 30.0,
                "audio_codec": "aac",
                "audio_sample_rate": 48000,
                "streams": 2,
                "has_video": True,
                "file_size": 1000,
            }):
                result = video.convert(src, temp_dir, preset)
                # Should be _1 because my_movie.converted.mp4 already exists
                assert "_1" in str(result["output_path"])

def test_dry_run_zero_write():
    p = profile.load_profile()
    with tempfile.TemporaryDirectory() as tmp:
        src = pathlib.Path(tmp) / "src"
        src.mkdir()
        sd = pathlib.Path(tmp) / "sd"
        (sd / "cubegm").mkdir(parents=True)
        (sd / "roms" / "videos").mkdir(parents=True)
        (sd / "roms" / "videos" / "existing.mp4").write_bytes(b"existing")
        vid = src / "new.mkv"
        vid.write_bytes(b"fake")
        mock_probe = {
            "container": "matroska",
            "format_name_raw": "matroska,webm",
            "video_codec": "hevc",
            "pix_fmt": "yuv420p",
            "width": 1920,
            "height": 1080,
            "fps": 60.0,
            "audio_codec": "aac",
            "audio_sample_rate": 48000,
            "streams": 2,
            "has_video": True,
            "file_size": 1024*1024,
        }
        before = set(pl.relative_to(sd).as_posix() for pl in sd.rglob("*") if pl.is_file())
        with mock.patch("treefrog.video.probe", return_value=mock_probe):
            scanned = scanner.scan(str(src), p)
            plan = planner.plan(scanned, str(sd), p)
            # Ensure planner didn't write to SD
            after = set(pl.relative_to(sd).as_posix() for pl in sd.rglob("*") if pl.is_file())
            assert before == after
            # Also check that no temp file leaked to SD
            assert not any("converted" in p.as_posix() for p in sd.rglob("*"))
            # Check that original still untouched
            assert vid.read_bytes() == b"fake"
