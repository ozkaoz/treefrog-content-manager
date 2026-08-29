import { useEffect, useState } from "react";
import SourcePicker from "./components/SourcePicker";
import DryRunPreview from "./components/DryRunPreview";
import BiosManager from "./components/BiosManager";
import LgptManager from "./components/LgptManager";
import Header from "./components/Header";
import EmptyState from "./components/EmptyState";
import About from "./components/About";
import SdCardPanel from "./components/SdCardPanel";
import GamesPanel from "./components/GamesPanel";
import MusicPanel from "./components/MusicPanel";
import VideosPanel from "./components/VideosPanel";
import SettingsPanel from "./components/SettingsPanel";
import { initTheme } from "./services/theme";
import { pickFolder } from "./services/dialog";

type Tab = "overview" | "games" | "music" | "videos" | "bios" | "lgpt" | "sdcard" | "settings" | "about";

export default function App() {
  const [sourcePath, setSourcePath] = useState<string>("");
  const [sdPath, setSdPath] = useState<string>("");
  const [plan, setPlan] = useState<Plan | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>("");
  const [activeTab, setActiveTab] = useState<Tab>("overview");

  useEffect(() => {
    const cleanup = initTheme();
    return cleanup;
  }, []);

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

  async function handlePickSd() {
    try {
      const sel = await pickFolder({ title: "Select TreeFrogUI SD root (must contain cubegm/ + roms/)" });
      if (sel) setSdPath(sel);
    } catch (e) {
      setError(String(e));
    }
  }

  const tabs: { id: Tab; label: string }[] = [
    { id: "overview", label: "Overview" },
    { id: "games", label: "Games" },
    { id: "music", label: "Music" },
    { id: "videos", label: "Videos" },
    { id: "bios", label: "BIOS" },
    { id: "lgpt", label: "LGPT" },
    { id: "sdcard", label: "SD Card" },
    { id: "settings", label: "Settings" },
    { id: "about", label: "About" },
  ];

  return (
    <div className="container">
      <Header />
      <p style={{ color: "var(--text-muted)", fontSize: 13 }}>
        Global TreeFrogUI content — one schema for all handhelds. Profiles drive folder mappings, not device forks. BIOS is TreeFrogUI-global, not R36SX-specific; video preset remains <code>PROVISIONAL_UNVALIDATED</code>.
      </p>

      <nav className="nav" aria-label="Main navigation">
        {tabs.map((t) => (
          <button key={t.id} onClick={() => setActiveTab(t.id)} className={activeTab === t.id ? "active" : ""}>
            {t.label}
          </button>
        ))}
      </nav>

      {activeTab === "overview" && (
        <>
          <div className="card">
            <h3>1. Select source folder (arbitrary library, recursive)</h3>
            <SourcePicker label="Games source folder" value={sourcePath} onChange={setSourcePath} title="Select games source folder" />
            <div style={{ marginTop: 10 }}>
              <SourcePicker label="Music source folder" value={""} onChange={() => {}} title="Select music source folder (future)" />
            </div>
            <div style={{ marginTop: 10 }}>
              <SourcePicker label="Video source folder" value={""} onChange={() => {}} title="Select video source folder (future)" />
            </div>
            <p className="warning">Scanned recursively; classified by profile + extension/content hints; multi-file sets (CUE/BIN etc) preserved as groups. Archives inspected in temp workspace. BIOS scanned via same pipeline.</p>
            {!sourcePath && <EmptyState kind="empty" title="No folder selected" description="Click Browse to open the native Windows folder picker." />}
          </div>

          <div className="card">
            <h3>2. Select TreeFrogUI SD</h3>
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              <label style={{ fontSize: 13, fontWeight: 600 }}>TreeFrogUI SD root</label>
              <div className="row" style={{ alignItems: "stretch" }}>
                <div
                  style={{
                    flex: 1,
                    padding: "8px 10px",
                    border: "1px solid var(--border)",
                    borderRadius: 6,
                    background: "var(--input)",
                    color: sdPath ? "var(--text)" : "var(--text-muted)",
                    fontSize: 13,
                    minHeight: 36,
                    display: "flex",
                    alignItems: "center",
                  }}
                >
                  {sdPath || "No SD selected — must contain cubegm/ + roms/"}
                </div>
                <button onClick={handlePickSd}>Browse</button>
              </div>
            </div>
            <p className="warning">Detection via markers <code>cubegm/</code> + <code>roms/</code> (profile <code>sd_markers.json</code>). SD health checked before blame. No writes in preview.</p>
            {!sdPath && <EmptyState kind="empty" title="No SD selected" description="Click Browse to open the native Windows folder picker." />}
          </div>

          <div className="card">
            <h3>3. Preview (dry-run, no writes) — duplicate/conflict/video/BIOS</h3>
            <div className="row">
              <button className="primary" onClick={handlePreview} disabled={loading || !sourcePath || !sdPath}>
                {loading ? "Scanning…" : "Scan + Preview"}
              </button>
              <button onClick={() => setPlan(null)} disabled={!plan}>Clear</button>
            </div>
            {error && <div className="status-error" style={{ marginTop: 10 }}>{error}</div>}
            {!plan && !loading && <EmptyState kind={sourcePath && sdPath ? "empty" : "not_implemented"} title="No scan yet" description="Select folders and press Scan + Preview. Nothing will be written — this is a dry-run plan. Planner is single source of truth (BIOS, video, ROMs, archives)." />}
            {loading && <EmptyState kind="loading" title="Scanning…" description="Recursive scan, archive inspection in temp workspace, SHA-256, classification." />}
            {plan && <div style={{ marginTop: 12 }}><DryRunPreview plan={plan} onResolve={handleResolvedPlan} /></div>}
          </div>

          {plan && (
            <div className="card" style={{ background: "var(--surface)" }}>
              <h4>TreeFrogUI Health</h4>
              <div style={{ display: "flex", gap: 12, flexWrap: "wrap", fontSize: 12 }}>
                <span>Games {plan.summary.new + plan.summary.unchanged > 0 ? "✓" : "—"} ({plan.summary.new} new, {plan.summary.unchanged} unchanged)</span>
                <span>Music ✓</span>
                <span>Videos {plan.entries.some(e => e.content_type === "video" && e.action === "convert_then_copy") ? "⚠ " + plan.entries.filter(e => e.action === "convert_then_copy").length + " require conversion (provisional)" : "✓"}</span>
                <span>BIOS {(() => {
                  const biosEntries = plan.entries.filter(e => e.content_type === "bios" || e.destination.includes("cubegm/bios"));
                  const biosConflicts = biosEntries.filter(e => e.action === "conflict" || e.action === "manual_review").length;
                  const biosMissing = plan.entries.filter(e => e.content_type === "bios" && e.action === "manual_review").length;
                  if (biosEntries.length === 0) return "— (no BIOS content scanned)";
                  if (biosConflicts > 0) return `⚠ ${biosConflicts} BIOS need review`;
                  if (biosMissing > 0) return `⚠ ${biosMissing} BIOS missing`;
                  return `✓ ${biosEntries.length} BIOS`;
                })()}</span>
                <span>LGPT ✓</span>
              </div>
              <p style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 4 }}>BIOS status is profile-driven and conditional (e.g., PS1 BIOS required only when PS1 content was detected). No BIOS downloads — user provides file → manager validates → plans deployment.</p>
            </div>
          )}

          <div className="card">
            <h4>Artwork</h4>
            <p style={{ fontSize: 13 }}>Mini Scraper remains external. <a href="https://github.com/tzubertowski/mini-scraper-cfw/releases" target="_blank" rel="noreferrer" style={{ color: "var(--accent)" }}>Open Mini Scraper</a> — app can verify <code>.res</code> after scrape but must not build second backend.</p>
          </div>
        </>
      )}

      {activeTab === "games" && <GamesPanel globalSdPath={sdPath} />}
      {activeTab === "music" && <MusicPanel globalSdPath={sdPath} />}
      {activeTab === "videos" && <VideosPanel globalSdPath={sdPath} />}
      {activeTab === "bios" && <BiosManager />}
      {activeTab === "lgpt" && <LgptManager />}
      {activeTab === "sdcard" && <SdCardPanel sdPath={sdPath} onChange={setSdPath} />}
      {activeTab === "settings" && <SettingsPanel />}
      {activeTab === "about" && <About />}
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
  action: "copy" | "extract" | "skip_unchanged" | "skip_duplicate" | "conflict" | "manual_review" | "unsupported_archive" | "convert_then_copy" | "unsupported" | "conversion_error";
  reason: string;
  status?: string;
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
  preset?: string;
  probe?: Record<string, unknown> | null;
  converted_name?: string;
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
