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
  preset?: string;
  probe?: Record<string, unknown> | null;
  converted_name?: string;
};

type Plan = {
  summary: { new: number; unchanged: number; duplicate_content: number; conflicts: number; manual_review?: number };
  entries: PlanEntry[];
};

export default function VideosPanel({ globalSdPath, onSourceChange }: { globalSdPath: string; onSourceChange?: (v: string) => void }) {
  const [source, setSource] = useState("");
  const [plan, setPlan] = useState<Plan | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [filter, setFilter] = useState<"all" | "compatible" | "convert" | "error">("all");

  async function handlePickSource() {
    try {
      const sel = await pickFolder({ title: "Select Videos source folder" });
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
      setError("Selecciona la carpeta de origen de Videos");
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
        const videoEntries = (raw.entries as PlanEntry[]).filter((e) => e.content_type === "video" || e.destination.includes("roms/videos") || e.destination.includes("videos"));
        result = { summary: raw.summary, entries: videoEntries };
      } else {
        result = {
          summary: { new: 2, unchanged: 5, duplicate_content: 1, conflicts: 0, manual_review: 1 },
          entries: [
            { source: `${source}/good.mp4`, destination: "roms/videos/good.mp4", action: "copy", reason: "video compatible (h264 yuv420p 640x480 30fps aac) -> copy", content_type: "video", size: 50000000, preset: "treefrog_conservative_default", probe: { video_codec: "h264", container: "mp4", width: 640, height: 480 } as any },
            { source: `${source}/bad.mkv`, destination: "roms/videos/bad.converted.mp4", action: "convert_then_copy", reason: "video incompatible (hevc 1920x1080 60fps) -> requires conversion", content_type: "video", size: 200000000, preset: "treefrog_conservative_default", probe: { video_codec: "hevc", container: "mkv", width: 1920, height: 1080 } as any, converted_name: "bad.converted.mp4" },
            { source: `${source}/corrupt.avi`, destination: "roms/videos/corrupt.avi", action: "manual_review", reason: "video inspection error: ffprobe failed", content_type: "video", size: 10000000 },
          ],
        };
      }
      setPlan(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  const filtered = (() => {
    if (!plan) return [];
    if (filter === "compatible") return plan.entries.filter((e) => e.action === "copy");
    if (filter === "convert") return plan.entries.filter((e) => e.action === "convert_then_copy");
    if (filter === "error") return plan.entries.filter((e) => e.action === "manual_review" || e.action === "unsupported");
    return plan.entries;
  })();

  return (
    <div className="card">
      <h3>Videos — Hardware decoder (TreeFrogUI)</h3>
      <p style={{ fontSize: 12, color: "var(--text-muted)" }}>
        Inspección vía <code>ffprobe</code>: contenedor, codec, perfil/nivel, pix_fmt, dimensiones, framerate, audio. <code>compatible → copiar</code>, <code>incompatible → FFmpeg → re-probe → desplegar</code>. Original nunca se modifica, salida en staging. Preset <code>PROVISIONAL_UNVALIDATED</code>.
      </p>

      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 12 }}>
        <label style={{ fontSize: 13, fontWeight: 600 }}>Videos source folder</label>
        <div className="row" style={{ alignItems: "stretch" }}>
          <div style={{ flex: 1, padding: "8px 10px", border: "1px solid var(--border)", borderRadius: 6, background: "var(--input)", color: source ? "var(--text)" : "var(--text-muted)", fontSize: 13, minHeight: 36, display: "flex", alignItems: "center" }}>
            {source || "No folder selected — e.g., D:\\Videos"}
          </div>
          <button onClick={handlePickSource}>Browse</button>
        </div>
        <div style={{ fontSize: 11, color: "var(--text-muted)" }}>SD destino: <strong>{globalSdPath || "ninguna (selecciona en SD Card)"}</strong> — la app copiará automáticamente a <code>roms/videos/</code> según extensión.</div>
      </div>

      <div className="row">
        <button className="primary" onClick={handlePreview} disabled={loading || !source || !globalSdPath}>
          {loading ? "Scanning…" : "Scan Videos"}
        </button>
        <button onClick={() => setPlan(null)} disabled={!plan}>Clear</button>
      </div>

      {error && <div className="status-error" style={{ fontSize: 12, marginTop: 8 }}>{error}</div>}

      {!plan && !loading && <EmptyState kind="empty" title="No scan yet" description="Select Videos source and SD target, then Scan Videos. Videos will be probed via ffprobe for TreeFrogUI hardware decoder compatibility." />}

      {loading && <EmptyState kind="loading" title="Scanning Videos…" description="Recursive scan, ffprobe inspection, compatibility evaluation." />}

      {plan && (
        <>
          <div style={{ marginTop: 12, display: "flex", gap: 8, flexWrap: "wrap" }}>
            <span style={{ fontSize: 12 }}><strong>{plan.entries.length} videos</strong></span>
            <span style={{ fontSize: 12, background: "var(--surface)", border: "1px solid var(--border)", padding: "2px 6px", borderRadius: 4 }}>{plan.entries.filter((e) => e.action === "copy").length} compatible</span>
            <span style={{ fontSize: 12, background: "var(--surface)", border: "1px solid var(--border)", padding: "2px 6px", borderRadius: 4 }}>{plan.entries.filter((e) => e.action === "convert_then_copy").length} to convert</span>
            <span style={{ fontSize: 12, background: "var(--surface)", border: "1px solid var(--border)", padding: "2px 6px", borderRadius: 4 }}>{plan.entries.filter((e) => e.action === "manual_review").length} review</span>
          </div>
          <div style={{ marginTop: 8, display: "flex", gap: 6 }}>
            {(["all", "compatible", "convert", "error"] as const).map((f) => (
              <button key={f} onClick={() => setFilter(f)} style={{ padding: "2px 8px", fontSize: 11 }} className={filter === f ? "active" : ""}>
                {f}
              </button>
            ))}
          </div>
          <table style={{ marginTop: 8 }}>
            <thead>
              <tr>
                <th>Source</th>
                <th>Destination</th>
                <th>Codec</th>
                <th>Action</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((e, idx) => (
                <tr key={idx} style={{ background: e.action === "convert_then_copy" ? "var(--warning-bg)" : e.action === "manual_review" ? "#fff8e1" : undefined }}>
                  <td style={{ fontSize: 11, maxWidth: 180, overflow: "hidden", textOverflow: "ellipsis" }} title={e.source}>
                    {e.source.split(/[\\/]/).pop()}
                  </td>
                  <td style={{ fontSize: 11 }}>{e.destination}</td>
                  <td style={{ fontSize: 11 }}>
                    {e.probe ? `${String((e.probe as any).video_codec || "?")} ${(e.probe as any).width ? `${(e.probe as any).width}x${(e.probe as any).height}` : ""}` : "—"}
                    {e.preset && <div style={{ fontSize: 10, color: "var(--text-muted)" }}>{e.preset}</div>}
                  </td>
                  <td>
                    <span className={`badge badge-${e.action === "copy" ? "copy" : e.action === "convert_then_copy" ? "copy" : e.action === "manual_review" ? "conflict" : "skip"}`}>{e.action}</span>
                    {e.converted_name && <div style={{ fontSize: 10, color: "var(--success)" }}>{e.converted_name}</div>}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {filtered.length === 0 && <EmptyState kind="empty" title="No Videos found" description="No video files (MP4, MKV, AVI, etc.) matched in the selected source. Check media.json and video_presets.json." />}
          <p style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 8 }}>
            Conversión: staging en <code>temp</code>, re-probe, solo si válido se copia a SD. Original intacto. Preset conservador <code>PROVISIONAL_UNVALIDATED</code> hasta validación física R36SX.
          </p>
        </>
      )}
    </div>
  );
}
