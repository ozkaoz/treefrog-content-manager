import { useState } from "react";
import { pickFolder } from "../services/dialog";
import EmptyState from "./EmptyState";

type PlanEntry = {
  source: string;
  destination: string;
  action: string;
  reason: string;
  content_type?: string;
  size?: number;
  group?: string[];
  members?: string[] | null;
};

type Plan = {
  summary: { new: number; unchanged: number; changed: number; duplicate_content: number; conflicts: number; deletions: number; manual_review?: number; unsupported_archive?: number };
  entries: PlanEntry[];
  warnings: string[];
};

export default function GamesPanel({ globalSdPath }: { globalSdPath: string }) {
  const [source, setSource] = useState("");
  const [sdPath, setSdPath] = useState(globalSdPath);
  const [plan, setPlan] = useState<Plan | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [filterSystem, setFilterSystem] = useState<string>("all");

  async function handlePickSource() {
    try {
      const sel = await pickFolder({ title: "Select Games source folder (ROM library)" });
      if (sel) setSource(sel);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handlePickSd() {
    try {
      const sel = await pickFolder({ title: "Select TreeFrogUI SD root (must contain cubegm/ + roms/)" });
      if (sel) setSdPath(sel);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handlePreview() {
    if (!source || !sdPath) {
      setError("Select both Games source and SD target");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const tauri = (window as unknown as { __TAURI__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI__;
      let result: Plan;
      if (tauri) {
        result = (await tauri.invoke("dry_run_preview", { sourcePath: source, sdPath })) as Plan;
      } else {
        // Mock for web
        result = {
          summary: { new: 12, unchanged: 45, changed: 2, duplicate_content: 3, conflicts: 1, deletions: 0, manual_review: 1, unsupported_archive: 0 },
          entries: [
            { source: `${source}/GBA/game1.gba`, destination: "roms/GBA/game1.gba", action: "copy", reason: "new", content_type: "rom/GBA", size: 8000000 },
            { source: `${source}/SFC/game2.smc`, destination: "roms/SFC/game2.smc", action: "copy", reason: "new", content_type: "rom/SFC", size: 4000000 },
            { source: `${source}/PS/game.cue`, destination: "roms/PS/game", action: "extract", reason: "grouped CUE/BIN", content_type: "grouped/CUE_BBIN", group: ["game.cue", "game.bin"] },
          ],
          warnings: [],
        };
      }
      // Filter for ROMs only (rom/ and grouped)
      const romEntries = result.entries.filter((e) => e.content_type?.startsWith("rom/") || e.content_type?.startsWith("grouped") || e.content_type === "archive-payload");
      setPlan({ ...result, entries: romEntries });
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  const systems = Array.from(new Set(plan?.entries.map((e) => e.content_type?.replace("rom/", "") || "unknown") || []));

  const filtered = plan?.entries.filter((e) => filterSystem === "all" || e.content_type?.includes(filterSystem)) || [];

  return (
    <div className="card">
      <h3>Games — ROM library (profile-driven)</h3>
      <p style={{ fontSize: 12, color: "var(--text-muted)" }}>
        Gestiona ROMs por sistema. Cada carpeta bajo <code>roms/</code> selecciona el core (ej. <code>GBA</code> para GBA, <code>PS</code> para PlayStation). Preserva unidades lógicas multiarchivo (CUE/BIN) y respeta <code>archive_policy.json</code> (arcade <code>cps1/neogeo/m2k</code> como <code>payload</code>, no extraído). Duplicados por <code>SHA-256</code>, no por nombre.
      </p>

      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 12 }}>
        <label style={{ fontSize: 13, fontWeight: 600 }}>Games source folder</label>
        <div className="row" style={{ alignItems: "stretch" }}>
          <div style={{ flex: 1, padding: "8px 10px", border: "1px solid var(--border)", borderRadius: 6, background: "var(--input)", color: source ? "var(--text)" : "var(--text-muted)", fontSize: 13, minHeight: 36, display: "flex", alignItems: "center" }}>
            {source || "No folder selected — e.g., D:\\ROMs"}
          </div>
          <button onClick={handlePickSource}>Browse</button>
        </div>
        <label style={{ fontSize: 13, fontWeight: 600, marginTop: 6 }}>SD target</label>
        <div className="row" style={{ alignItems: "stretch" }}>
          <div style={{ flex: 1, padding: "8px 10px", border: "1px solid var(--border)", borderRadius: 6, background: "var(--input)", color: sdPath ? "var(--text)" : "var(--text-muted)", fontSize: 13, minHeight: 36, display: "flex", alignItems: "center" }}>
            {sdPath || "No SD selected — must contain cubegm/ + roms/"}
          </div>
          <button onClick={handlePickSd}>Browse</button>
        </div>
      </div>

      <div className="row">
        <button className="primary" onClick={handlePreview} disabled={loading || !source || !sdPath}>
          {loading ? "Scanning…" : "Scan Games"}
        </button>
        <button onClick={() => setPlan(null)} disabled={!plan}>Clear</button>
      </div>

      {error && <div className="status-error" style={{ fontSize: 12, marginTop: 8 }}>{error}</div>}

      {!plan && !loading && <EmptyState kind="empty" title="No scan yet" description="Select Games source and SD target, then Scan Games. Archives will be inspected in temp workspace, duplicates via SHA-256." />}

      {loading && <EmptyState kind="loading" title="Scanning Games…" description="Recursive scan, classify by profile + extension, archive inspection, hash." />}

      {plan && (
        <>
          <div style={{ marginTop: 12, display: "flex", gap: 6, flexWrap: "wrap" }}>
            <span style={{ fontSize: 12 }}><strong>Systems:</strong> {systems.join(", ") || "—"}</span>
            <span style={{ fontSize: 12, background: "var(--surface)", border: "1px solid var(--border)", padding: "2px 6px", borderRadius: 4 }}>{plan.summary.new} new</span>
            <span style={{ fontSize: 12, background: "var(--surface)", border: "1px solid var(--border)", padding: "2px 6px", borderRadius: 4 }}>{plan.summary.unchanged} unchanged</span>
            <span style={{ fontSize: 12, background: "var(--surface)", border: "1px solid var(--border)", padding: "2px 6px", borderRadius: 4 }}>{plan.summary.duplicate_content} duplicate</span>
            <span style={{ fontSize: 12, background: "var(--surface)", border: "1px solid var(--border)", padding: "2px 6px", borderRadius: 4 }}>{plan.summary.conflicts} conflicts</span>
          </div>
          <div style={{ marginTop: 8, display: "flex", gap: 6 }}>
            <span style={{ fontSize: 11, color: "var(--text-muted)" }}>Filter:</span>
            {["all", ...systems].map((s) => (
              <button key={s} onClick={() => setFilterSystem(s)} style={{ padding: "2px 8px", fontSize: 11 }} className={filterSystem === s ? "active" : ""}>
                {s}
              </button>
            ))}
          </div>
          <table style={{ marginTop: 8 }}>
            <thead>
              <tr>
                <th>Source</th>
                <th>Destination</th>
                <th>System</th>
                <th>Action</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((e, idx) => (
                <tr key={idx}>
                  <td style={{ fontSize: 11, maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis" }} title={e.source}>
                    {e.source.split(/[\\/]/).pop()}
                  </td>
                  <td style={{ fontSize: 11 }}>{e.destination}</td>
                  <td style={{ fontSize: 11 }}>{e.content_type?.replace("rom/", "") || "—"}</td>
                  <td>
                    <span className={`badge badge-${e.action === "copy" ? "copy" : e.action === "extract" ? "extract" : e.action === "conflict" ? "conflict" : "skip"}`}>
                      {e.action}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {filtered.length === 0 && <EmptyState kind="empty" title="No Games found" description="No ROMs matched profile extensions in the selected source (check roms/ subfolders and archive_policy.json)." />}
        </>
      )}
    </div>
  );
}
