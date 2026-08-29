import { useEffect, useState } from "react";
import { pickFolder } from "../services/dialog";
import EmptyState from "./EmptyState";

type PlanEntry = {
  source: string;
  destination: string;
  action: string;
  reason: string;
  content_type?: string;
  size?: number;
};

type Plan = {
  summary: { new: number; unchanged: number; duplicate_content: number; conflicts: number };
  entries: PlanEntry[];
};

export default function MusicPanel({ 
  globalSdPath, 
  onSourceChange, 
  onPlanChange,
  onNext,
  visible
}: { 
  globalSdPath: string; 
  onSourceChange?: (v: string) => void; 
  onPlanChange?: (plan: Plan | null) => void;
  onNext?: () => void;
  visible?: boolean;
}) {
  const [source, setSource] = useState("");
  const [plan, setPlan] = useState<Plan | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function handlePickSource() {
    try {
      const sel = await pickFolder({ title: "Select Music source folder (playlists as subfolders)" });
      if (sel) {
        setSource(sel);
        onSourceChange?.(sel);
      }
    } catch (e) {
      setError(String(e));
    }
  }

  async function handlePreview() {
    if (!source) {
      setError("Selecciona la carpeta de origen de Music");
      return;
    }
    if (!globalSdPath) {
      setError("No hay SD seleccionada — ve a SD Card para seleccionar automáticamente");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const tauri = (window as unknown as { __TAURI__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI__;
      let result: Plan;
      if (tauri) {
        const raw = (await tauri.invoke("dry_run_preview", { sourcePath: source, sdPath: globalSdPath })) as any;
        // Filter for music only (perfil verifica extensión y copia automáticamente a roms/music/)
        const musicEntries = (raw.entries as PlanEntry[]).filter((e) => e.content_type === "music" || e.destination.includes("roms/music"));
        result = { summary: raw.summary, entries: musicEntries };
      } else {
        result = {
          summary: { new: 3, unchanged: 10, duplicate_content: 1, conflicts: 0 },
          entries: [
            { source: `${source}/My Playlist/song1.mp3`, destination: "roms/music/My Playlist/song1.mp3", action: "copy", reason: "new", content_type: "music", size: 5000000 },
            { source: `${source}/My Playlist/song2.flac`, destination: "roms/music/My Playlist/song2.flac", action: "copy", reason: "new", content_type: "music", size: 20000000 },
          ],
        };
      }
      setPlan(result);
      onPlanChange?.(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  // Group by playlist (subfolder under roms/music)
  const playlists = (() => {
    if (!plan) return [];
    const map = new Map<string, PlanEntry[]>();
    for (const e of plan.entries) {
      const parts = e.destination.split("/");
      // roms/music/<playlist>/...
      const playlist = parts.length >= 3 ? parts[2] : "unknown";
      if (!map.has(playlist)) map.set(playlist, []);
      map.get(playlist)!.push(e);
    }
    return Array.from(map.entries());
  })();

  // Re-scan automatically when the tab becomes visible again,
  // so the plan always reflects the current state of disk/SD.
  useEffect(() => {
    if (visible && source && globalSdPath) {
      handlePreview();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible]);

  return (
    <div className="card">
      <h3>Music — Playlists (TreeFrogUI)</h3>
      <p style={{ fontSize: 12, color: "var(--text-muted)" }}>
        Cada subcarpeta bajo <code>roms/music/</code> es una playlist en TreeFrogUI. Preserva subcarpetas (ej. <code>roms/music/My Album/</code>). Formatos soportados vía perfil <code>media.json</code>: MP3, M4A, AAC, WAV, FLAC, OGG, Opus.
      </p>

      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 12 }}>
        <label style={{ fontSize: 13, fontWeight: 600 }}>Music source folder</label>
        <div className="row" style={{ alignItems: "stretch" }}>
          <div style={{ flex: 1, padding: "8px 10px", border: "1px solid var(--border)", borderRadius: 6, background: "var(--input)", color: source ? "var(--text)" : "var(--text-muted)", fontSize: 13, minHeight: 36, display: "flex", alignItems: "center" }}>
            {source || "No folder selected — e.g., D:\\Music"}
          </div>
          <button onClick={handlePickSource}>Browse</button>
        </div>
        <div style={{ fontSize: 11, color: "var(--text-muted)" }}>SD destino: <strong>{globalSdPath || "ninguna (selecciona en SD Card)"}</strong> — la app copiará automáticamente a <code>roms/music/</code> según extensión.</div>
      </div>

      <div className="row">
        <button className="primary" onClick={handlePreview} disabled={loading || !source || !globalSdPath}>
          {loading ? "Scanning…" : "Scan Music"}
        </button>
        <button onClick={() => { setPlan(null); onPlanChange?.(null); }} disabled={!plan}>Clear</button>
        <button onClick={() => onNext?.()} style={{ marginLeft: "auto" }}>
          Omitir → Videos
        </button>
        <button className="primary" onClick={() => onNext?.()} disabled={!source && !plan}>
          Continuar a Videos →
        </button>
      </div>
      <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 8 }}>Analiza recursivamente subcarpetas; cada subcarpeta se preserva como playlist en <code>roms/music/</code>.</div>

      {error && <div className="status-error" style={{ fontSize: 12, marginTop: 8 }}>{error}</div>}

      {!plan && !loading && <EmptyState kind="empty" title="No scan yet" description="Select Music source and SD target, then Scan Music. Each subfolder becomes a playlist in roms/music/." />}

      {loading && <EmptyState kind="loading" title="Scanning Music…" description="Recursive scan, classify by media.json, preserve playlists." />}

      {plan && (
        <>
          <div style={{ marginTop: 12, display: "flex", gap: 8 }}>
            <span style={{ fontSize: 12 }}><strong>Playlists:</strong> {playlists.length}</span>
            <span style={{ fontSize: 12, background: "var(--surface)", border: "1px solid var(--border)", padding: "2px 6px", borderRadius: 4 }}>{plan.summary.new} new</span>
            <span style={{ fontSize: 12, background: "var(--surface)", border: "1px solid var(--border)", padding: "2px 6px", borderRadius: 4 }}>{plan.summary.unchanged} unchanged</span>
          </div>
          {playlists.map(([playlist, entries]) => (
            <div key={playlist} style={{ marginTop: 10, border: "1px solid var(--border)", borderRadius: 6, padding: 8, background: "var(--surface)" }}>
              <h4 style={{ margin: "0 0 6px 0", fontSize: 13 }}>Playlist: {playlist} ({entries.length} tracks)</h4>
              <table>
                <thead>
                  <tr>
                    <th>Track</th>
                    <th>Destination</th>
                    <th>Action</th>
                  </tr>
                </thead>
                <tbody>
                  {entries.map((e, idx) => (
                    <tr key={idx}>
                      <td style={{ fontSize: 11 }}>{e.source.split(/[\\/]/).pop()}</td>
                      <td style={{ fontSize: 11 }}>{e.destination}</td>
                      <td>
                        <span className={`badge badge-${e.action === "copy" ? "copy" : "skip"}`}>{e.action}</span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ))}
          {plan.entries.length === 0 && <EmptyState kind="empty" title="No Music found" description="No audio files (MP3, FLAC, etc.) matched media.json in the selected source." />}
        </>
      )}
    </div>
  );
}
