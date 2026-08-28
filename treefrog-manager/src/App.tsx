import { useState } from "react";
import SourcePicker from "./components/SourcePicker";
import SdPicker from "./components/SdPicker";
import DryRunPreview from "./components/DryRunPreview";

export default function App() {
  const [sourcePath, setSourcePath] = useState<string>("");
  const [sdPath, setSdPath] = useState<string>("");
  const [plan, setPlan] = useState<Plan | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>("");

  async function handlePreview() {
    setError("");
    if (!sourcePath || !sdPath) {
      setError("Select both source folder and TreeFrogUI SD");
      return;
    }
    setLoading(true);
    try {
      const tauri = (window as unknown as { __TAURI__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI__;
      if (tauri) {
        const result = (await tauri.invoke("dry_run_preview", {
          sourcePath,
          sdPath,
        })) as Plan;
        setPlan(result);
      } else {
        setPlan(mockPreview());
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  function handleResolvedPlan(newPlan: Plan) {
    setPlan(newPlan);
  }

  return (
    <div className="container">
      <h1>TreeFrog Content Manager</h1>
      <p>Global TreeFrogUI content — one schema for all handhelds. Profiles drive folder mappings, not device forks. Phase 2B: deterministic duplicate/conflict resolution — planner is single source of truth.</p>

      <div className="card">
        <h3>1. Select source folder (arbitrary library, recursive)</h3>
        <SourcePicker value={sourcePath} onChange={setSourcePath} />
        <p className="warning">Scanned recursively; classified by profile + extension/content hints; multi-file sets (CUE/BIN etc) preserved as groups. Archives inspected in temp workspace.</p>
      </div>

      <div className="card">
        <h3>2. Select TreeFrogUI SD</h3>
        <SdPicker value={sdPath} onChange={setSdPath} />
        <p className="warning">Detection via markers <code>cubegm/</code> + <code>roms/</code> (profile <code>sd_markers.json</code>). SD health checked before blame. No writes in preview.</p>
      </div>

      <div className="card">
        <h3>3. Preview (dry-run, no writes) — duplicate/conflict resolution</h3>
        <div className="row">
          <button onClick={handlePreview} disabled={loading || !sourcePath || !sdPath}>
            {loading ? "Scanning…" : "Scan + Preview"}
          </button>
          <button onClick={() => setPlan(null)} disabled={!plan}>Clear</button>
        </div>
        {error && <p style={{ color: "crimson" }}>{error}</p>}
        {!plan && <p>Select folders and press Scan + Preview. Nothing will be written — this is a dry-run plan. Planner is single source of truth for future SD writes.</p>}
        {plan && <DryRunPreview plan={plan} onResolve={handleResolvedPlan} />}
      </div>

      <div className="card">
        <h4>Artwork</h4>
        <p>Mini Scraper remains external. <a href="https://github.com/tzubertowski/mini-scraper-cfw/releases" target="_blank" rel="noreferrer">Open Mini Scraper</a> — app can verify <code>.res</code> after scrape but must not build second backend.</p>
      </div>
    </div>
  );
}

export type PlanSummary = {
  unchanged: number;
  new: number;
  changed: number;
  duplicate_content: number;
  conflicts: number;
  deletions: number;
  manual_review?: number;
  unsupported_archive?: number;
};

export type PlanEntry = {
  source: string;
  destination: string;
  action: "copy" | "extract" | "skip_unchanged" | "skip_duplicate" | "conflict" | "manual_review" | "unsupported_archive";
  reason: string;
  hash?: string;
  source_hash?: string | null;
  destination_hash?: string | null;
  content_type?: string;
  size?: number;
  group?: string[];
  members?: string[] | null;
  default_action?: string;
  resolution?: string | null;
  resolved_action?: string | null;
  original_destination?: string;
};

export type Plan = {
  summary: PlanSummary;
  entries: PlanEntry[];
  warnings: string[];
  resolved_summary?: Record<string, number>;
};

function mockPreview(): Plan {
  return {
    summary: { unchanged: 2331, new: 34, changed: 12, duplicate_content: 7, conflicts: 3, deletions: 0, manual_review: 1, unsupported_archive: 1 },
    entries: [
      { source: "C:/lib/GBA/Advance Wars.gba", destination: "roms/GBA/Advance Wars.gba", action: "copy", reason: "new path + new hash", size: 8388608, source_hash: "aaa...111", destination_hash: null, content_type: "rom/GBA", default_action: "copy", resolution: "copy", resolved_action: "copy" },
      { source: "C:/lib/PS/Final Fantasy VII (Disc 1).cue + .bin", destination: "roms/PS/Final Fantasy VII", action: "extract", reason: "grouped CUE/BIN logical unit (2 files)", group: ["Final Fantasy VII (Disc 1).cue", "Final Fantasy VII (Disc 1).bin"], members: ["Final Fantasy VII (Disc 1).cue", "Final Fantasy VII (Disc 1).bin"], source_hash: "bbb...222", content_type: "grouped/CUE_BBIN", default_action: "extract", resolution: "copy", resolved_action: "extract" },
      { source: "C:/lib/archives/mame_pack.zip", destination: "roms/cps1/mame_pack.zip", action: "copy", reason: "archive-is-payload -> copy intact", content_type: "archive-payload", default_action: "copy", resolution: "copy", resolved_action: "copy" },
      { source: "C:/lib/archives/roms_collection.zip::game.sfc", destination: "roms/SFC/game.sfc", action: "extract", reason: "archive-extract -> game.sfc (.sfc)", content_type: "rom/SFC", default_action: "extract", resolution: "copy", resolved_action: "extract" },
      { source: "C:/lib/GBA/duplicate.gba", destination: "roms/GBA/duplicate.gba", action: "skip_duplicate", reason: "different path + same hash -> duplicate content default skip", source_hash: "ccc...333", destination_hash: "ccc...333", content_type: "rom/GBA", default_action: "skip_duplicate", resolution: "skip", resolved_action: "skip" },
      { source: "C:/lib/GBA/conflict.gba", destination: "roms/GBA/conflict.gba", action: "conflict", reason: "same path + different hash -> conflict", source_hash: "ddd...444", destination_hash: "eee...555", content_type: "rom/GBA", default_action: "conflict", resolution: "conflict", resolved_action: "conflict" },
      { source: "C:/lib/GBA/manual.zip", destination: "roms/UNKNOWN/manual.zip", action: "manual_review", reason: "archive safety violation: traversal", content_type: "archive", default_action: "manual_review", resolution: "manual_review", resolved_action: "manual_review" },
      { source: "C:/lib/game.7z", destination: "roms/UNKNOWN/game.7z", action: "unsupported_archive", reason: "archive handler not available for .7z (stub)", content_type: "archive", default_action: "unsupported_archive", resolution: "skip", resolved_action: "skip" },
      { source: "C:/lib/music/My Album/song.flac", destination: "roms/music/My Album/song.flac", action: "copy", reason: "music — preserve subfolders (each folder is playlist)", size: 12345678, content_type: "music", default_action: "copy", resolution: "copy", resolved_action: "copy" },
    ],
    warnings: ["PROVISIONAL_UNVALIDATED video preset — no hardware claim", "archives bounded: max_depth=1, 1024 entries, 1 GiB expansion"],
  };
}
