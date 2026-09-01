"""Video conversion deploy pipeline — end-to-end with real ffmpeg/ffprobe.
Skipped automatically when ffmpeg/ffprobe are unavailable (CI has them).
Mirrors the Rust deploy_converted_video pipeline contract:
  - original never modified
  - staged output validated with ffprobe BEFORE the SD write
  - destination receives the CONVERTED file (smaller/different bytes)
  - failure/cancellation removes staged output
  - compatible videos are copied as-is
"""
import pathlib
import shutil
import subprocess
import sys
import tempfile

REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))

import pytest  # noqa: E402

from treefrog import deploy, profile, video  # noqa: E402


def _ffmpeg_available():
    try:
        r = subprocess.run(["ffmpeg", "-version"], capture_output=True, timeout=15)
        return r.returncode == 0
    except Exception:
        return False


def _make_video(path, size="320x240"):
    cmd = [
        "ffmpeg", "-y", "-f", "lavfi", "-i", f"testsrc=duration=1:size={size}:rate=30",
        "-c:v", "libx264", "-pix_fmt", "yuv420p",
        str(path),
    ]
    r = subprocess.run(cmd, capture_output=True, timeout=120)
    return r.returncode == 0 and path.exists()


@pytest.mark.skipif(not _ffmpeg_available(), reason="ffmpeg not available")
def test_converted_video_deploys_validated_output(tmp_path):
    src = tmp_path / "src.mp4"
    assert _make_video(src, size="1280x960"), "ffmpeg must produce test video"
    original_bytes = src.read_bytes()

    sd = tmp_path / "sd"
    (sd / "cubegm").mkdir(parents=True)
    (sd / "roms").mkdir(parents=True)
    p = profile.load_profile()

    # The 1280x960 video exceeds the conservative max_width (854) -> convert
    probe_result = video.probe(str(src))
    status, reason = video.evaluate_compatibility(probe_result, p["video_preset"])
    assert status == "conversion_required", f"test premise: 1280 wide must require conversion ({reason})"

    plan = {"entries": [{
        "source": str(src),
        "destination": "roms/videos/src.converted.mp4",
        "action": "convert_then_copy",
        "reason": "test",
        "size": src.stat().st_size,
    }]}
    result = deploy.deploy_plan(plan, str(sd), p)
    assert result["success"], f"deploy must succeed: {result['errors']}"
    assert result["deployed"] == 1

    dest = sd / "roms" / "videos" / "src.converted.mp4"
    assert dest.exists(), "converted output must be deployed"
    # Original untouched
    assert src.read_bytes() == original_bytes
    # Deployed file is the CONVERTED output (not the original): it must be a
    # valid, ffprobe-readable, COMPATIBLE video.
    probe_dest = video.probe(str(dest))
    status2, reason2 = video.evaluate_compatibility(probe_dest, p["video_preset"])
    assert status2 == "compatible", f"deployed output must be compatible: {reason2}"
    assert probe_dest["width"] <= 854, f"converted width must be scaled down: {probe_dest['width']}"
    # No staging leftovers
    assert not list(sd.rglob(".treefrog_staging*"))


@pytest.mark.skipif(not _ffmpeg_available(), reason="ffmpeg not available")
def test_compatible_video_copies_original(tmp_path):
    src = tmp_path / "ok.mp4"
    assert _make_video(src, size="320x240"), "ffmpeg must produce test video"

    sd = tmp_path / "sd"
    (sd / "cubegm").mkdir(parents=True)
    (sd / "roms").mkdir(parents=True)
    p = profile.load_profile()

    probe_result = video.probe(str(src))
    status, reason = video.evaluate_compatibility(probe_result, p["video_preset"])
    assert status == "compatible", f"test premise: 320 wide h264 must be compatible ({reason})"

    plan = {"entries": [{
        "source": str(src),
        "destination": "roms/videos/ok.mp4",
        "action": "convert_then_copy",
        "reason": "test",
        "size": src.stat().st_size,
    }]}
    result = deploy.deploy_plan(plan, str(sd), p)
    assert result["success"], f"deploy must succeed: {result['errors']}"
    dest = sd / "roms" / "videos" / "ok.mp4"
    assert dest.exists()
    # Compatible source re-probed compatible at deploy time -> ORIGINAL copied
    assert dest.read_bytes() == src.read_bytes()


@pytest.mark.skipif(not _ffmpeg_available(), reason="ffmpeg not available")
def test_conversion_failure_leaves_no_output(tmp_path):
    src = tmp_path / "garbage.mp4"
    src.write_bytes(b"not a real video at all")
    sd = tmp_path / "sd"
    (sd / "cubegm").mkdir(parents=True)
    (sd / "roms").mkdir(parents=True)
    p = profile.load_profile()
    plan = {"entries": [{
        "source": str(src),
        "destination": "roms/videos/garbage.mp4",
        "action": "convert_then_copy",
        "reason": "test",
        "size": src.stat().st_size,
    }]}
    result = deploy.deploy_plan(plan, str(sd), p)
    assert not result["success"]
    assert result["failed"] == 1
    assert not (sd / "roms" / "videos" / "garbage.mp4").exists()
    # No staging leftovers on the SD
    assert not list(sd.rglob(".treefrog_staging*"))


@pytest.mark.skipif(not _ffmpeg_available(), reason="ffmpeg not available")
def test_ffprobe_unavailable_surfaces_error(tmp_path, monkeypatch):
    """If ffprobe is unavailable, the failure must be OBSERVABLE — never a
    silent copy of the original."""
    src = tmp_path / "v.mp4"
    assert _make_video(src)
    sd = tmp_path / "sd"
    (sd / "cubegm").mkdir(parents=True)
    (sd / "roms").mkdir(parents=True)
    p = profile.load_profile()

    def broken_probe(_path):
        raise RuntimeError("ffprobe not found")

    monkeypatch.setattr(video, "probe", broken_probe)
    plan = {"entries": [{
        "source": str(src),
        "destination": "roms/videos/v.mp4",
        "action": "convert_then_copy",
        "reason": "test",
        "size": src.stat().st_size,
    }]}
    result = deploy.deploy_plan(plan, str(sd), p)
    assert not result["success"]
    assert result["failed"] == 1
    assert any("ffprobe" in e or "probe" in e for e in result["errors"]), result["errors"]
    assert not (sd / "roms" / "videos" / "v.mp4").exists()
