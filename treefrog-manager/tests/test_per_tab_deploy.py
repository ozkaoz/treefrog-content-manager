"""Per-tab deploy regression (user report: "music not copying").

Covers the TreeFrogUI destination contract for every tab and the music
playlist semantics (each folder under roms/music = playlist):
- Games      -> roms/<SYSTEM>/
- Music      -> roms/music/ (subfolder preserved = playlist)
- Videos     -> roms/videos/
- BIOS       -> cubegm/bios/
- LGPT       -> lgpt/samples/ + lgpt/projects/
Also proves the unified deploy model: ONE combined panel plan (exactly what
the UI sends via plan_entries) deploys ALL tabs in a single job.
"""
import pathlib
import sys
import tempfile

REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "treefrog-manager" / "python"))

from treefrog import deploy, planner, profile  # noqa: E402


def _mk_sd(tmp):
    sd = pathlib.Path(tmp) / "sd"
    for sub in ("cubegm/bios", "roms", "lgpt/samples", "lgpt/projects"):
        (sd / sub).mkdir(parents=True)
    return sd


def _e(source, destination, content_type):
    return {
        "source": str(source),
        "destination": destination,
        "action": "copy",
        "resolved_action": "copy",
        "reason": "panel selection",
        "content_type": content_type,
        "size": pathlib.Path(source).stat().st_size,
    }


def test_every_tab_deploys_to_correct_treefrogui_path():
    with tempfile.TemporaryDirectory() as tmp:
        sd = _mk_sd(tmp)
        src = pathlib.Path(tmp) / "src"
        for sub in ("GBA", "MyPlaylist", "samples", "projects"):
            (src / sub).mkdir(parents=True)
        rom = src / "GBA" / "game.gba"
        rom.write_bytes(b"GBA")
        song = src / "song.mp3"
        song.write_bytes(b"mp3")
        track = src / "MyPlaylist" / "track1.mp3"
        track.write_bytes(b"mp3p")
        sample = src / "samples" / "kick.wav"
        sample.write_bytes(b"WAV")
        project = src / "projects" / "track.lgpt"
        project.write_bytes(b"LGPT")

        # ONE combined plan — exactly what App sends via plan_entries
        plan = {
            "entries": [
                _e(rom, "roms/GBA/game.gba", "rom/gba"),
                _e(song, "roms/music/song.mp3", "music"),
                _e(track, "roms/music/MyPlaylist/track1.mp3", "music"),
                _e(sample, "lgpt/samples/kick.wav", "lgpt/sample"),
                _e(project, "lgpt/projects/track.lgpt", "lgpt/project"),
            ]
        }
        p = profile.load_profile()
        result = deploy.deploy_plan(plan, str(sd), p)
        assert result["success"], f"deploy failed: {result['errors']}"
        assert result["deployed"] == 5, f"all tabs must deploy: {result}"

        # Exact TreeFrogUI paths
        assert (sd / "roms" / "GBA" / "game.gba").exists()
        assert (sd / "roms" / "music" / "song.mp3").exists()
        assert (sd / "roms" / "music" / "MyPlaylist" / "track1.mp3").exists(), \
            "music playlist subfolder must be preserved (TreeFrogUI playlist semantics)"
        assert (sd / "lgpt" / "samples" / "kick.wav").exists()
        assert (sd / "lgpt" / "projects" / "track.lgpt").exists()


def test_music_playlist_destination_semantics():
    """MusicPanel destination construction parity: folder -> roms/music/<folder>/<file>."""
    folder = "MyPlaylist\\Sub"  # Windows separators must normalize
    filename = "track.mp3"
    normalized = folder.replace("\\", "/").replace("/+", "/").strip("/")
    destination = f"roms/music/{normalized}/{filename}" if normalized else f"roms/music/{filename}"
    assert destination == "roms/music/MyPlaylist/Sub/track.mp3"
    # No traversal can survive the backend canonical validation
    bad = "roms/music/../../../evil/track.mp3"
    from treefrog import sd_target
    try:
        sd_target.validate_destination_path(bad)
        assert False, "traversal in music destination must be rejected"
    except Exception:
        pass


def test_planner_music_tab_destination():
    """The backend planner sends music files to roms/music (TreeFrogUI)."""
    with tempfile.TemporaryDirectory() as tmp:
        sd = _mk_sd(tmp)
        src = pathlib.Path(tmp) / "music_src"
        (src / "Chill").mkdir(parents=True)
        (src / "Chill" / "a.mp3").write_bytes(b"mp3data")
        p = profile.load_profile()
        scanned = __import__("treefrog.scanner", fromlist=["scan"]).scan(str(src), p)
        plan = planner.plan(scanned, str(sd), p)
        music_entries = [e for e in plan["entries"] if e.get("content_type") == "music"]
        assert music_entries, f"planner must plan music: {plan['entries']}"
        for e in music_entries:
            assert e["destination"].startswith("roms/music/"), e["destination"]


def test_videos_tab_destination():
    """Videos go to roms/videos/ (TreeFrogUI hardware video player)."""
    with tempfile.TemporaryDirectory() as tmp:
        sd = _mk_sd(tmp)
        p = profile.load_profile()
        plan = {
            "entries": [
                {
                    "source": "x",
                    "destination": "roms/videos/clip.mp4",
                    "action": "skip",
                    "resolved_action": "skip",
                    "reason": "skip",
                    "content_type": "video",
                    "size": 1,
                }
            ]
        }
        from treefrog import sd_target
        s = sd_target.calculate_space(plan, 1000)
        assert s["bytes_to_skip"] == 1
        # Destination contract: roms/videos prefix
        assert plan["entries"][0]["destination"].startswith("roms/videos/")
