import { useState } from "react";
import SourcePicker from "./components/SourcePicker";
import SdPicker from "./components/SdPicker";
import DryRunPreview from "./components/DryRunPreview";

// First milestone: Select source folder + select TreeFrogUI SD + scan + preview exactly
// what would be copied/extracted/skipped/conflicted, without writing anything.
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
      // Tauri invoke — Rust backend does scan+classify+archive+hash+plan without writes
      // Falls back to mock during web dev without Tauri
      const tauri = (window as unknown as { __TAURI__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI__;
      if (tauri) {
        const result = (await tauri.invoke("dry_run_preview", {
          sourcePath,
          sdPath,
        })) as Plan;
        setPlan(result);
      } else {
        // dev fallback: fetch from Python mirror via mock
        setPlan(mockPreview());
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="container">
      <h1>TreeFrog Content Manager</h1>
      <p>Global TreeFrogUI content — one schema for all handhelds. Profiles drive folder mappings, not device forks.</p>

      <div className="card">
        <h3>1. Select source folder (arbitrary library, recursive)</h3>
        <SourcePicker value={sourcePath} onChange={setSourcePath} />
        <p className="warning">Scanned recursively; classified by profile + extension/content hints; multi-file sets (CUE/BIN etc) preserved as groups.</p>
      </div>

      <div className="card">
        <h3>2. Select TreeFrogUI SD</h3>
        <SdPicker value={sdPath} onChange={setSdPath} />
        <p className="warning">Detection via markers <code>cubegm/</code> + <code>roms/</code> (profile <code>sd_markers.json</code>). SD health checked before blame.</p>
      </div>

      <div className="card">
        <h3>3. Preview (dry-run, no writes)</h3>
        <div className="row">
          <button onClick={handlePreview} disabled={loading || !sourcePath || !sdPath}>
            {loading ? "Scanning…" : "Scan + Preview"}
          </button>
          <button onClick={() => setPlan(null)} disabled={!plan}>Clear</button>
        </div>
        {error && <p style={{ color: "crimson" }}>{error}</p>}
        {!plan && <p>Select folders and press Scan + Preview. Nothing will be written — this is a dry-run plan.</p>}
        {plan && <DryRunPreview plan={plan} />}
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
};

export type PlanEntry = {
  source: string;
  destination: string;
  action: "copy" | "extract" | "skip_unchanged" | "skip_duplicate" | "conflict";
  reason: string;
  hash?: string;
  size?: number;
  group?: string[];
};

export type Plan = {
  summary: PlanSummary;
  entries: PlanEntry[];
  warnings: string[];
};

function mockPreview(): Plan {
  return {
    summary: { unchanged: 2331, new: 34, changed: 12, duplicate_content: 7, conflicts: 3, deletions: 0 },
    entries: [
      { source: "C:/lib/GBA/Advance Wars.gba", destination: "roms/GBA/Advance Wars.gba", action: "copy", reason: "new path + new hash", size: 8388608 },
      { source: "C:/lib/PS/Final Fantasy VII (Disc 1).cue + .bin", destination: "roms/PS/Final Fantasy VII/", action: "copy", reason: "multi-file CUE/BIN group", group: ["Final Fantasy VII (Disc 1).cue", "Final Fantasy VII (Disc 1).bin"] },
      { source: "C:/lib/archives/mame_pack.zip", destination: "roms/cps1/mame_pack.zip", action: "copy", reason: "archive itself is valid runtime payload per profile (cps1 .zip)" },
      { source: "C:/lib/archives/roms_collection.zip", destination: "roms/SFC/...", action: "extract", reason: "archive contains supported ROMs; profile says extract" },
      { source: "C:/lib/GBA/duplicate.gba", destination: "roms/GBA/Advance Wars.gba", action: "skip_duplicate", reason: "different path + same hash → duplicate content default skip" },
      { source: "C:/lib/GBA/conflict.gba", destination: "roms/GBA/Advance Wars.gba", action: "conflict", reason: "same path + different hash → conflict" },
      { source: "C:/lib/music/My Album/song.flac", destination: "roms/music/My Album/song.flac", action: "copy", reason: "music — preserve subfolders (each folder is playlist)", size: 12345678 },
    ],
    warnings: ["PROVISIONAL_UNVALIDATED video preset — no hardware claim", "archives bounded: max_depth=1, 1024 entries, 1 GiB expansion"],
  };
}
