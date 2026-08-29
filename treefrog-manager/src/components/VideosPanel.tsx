import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
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

export default function VideosPanel({ 
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
      setError("Selecciona la carpeta de origen");
      return;
    }
    if (!globalSdPath) {
      setError("No hay SD seleccionada — ve a Overview");
      return;
    }
    setLoading(true);
    setError("");
    try {
      // Escaneo REAL de la carpeta seleccionada contra la SD seleccionada
      const result = (await invoke("dry_run_with_target", {
        sourcePath: source,
        sdPath: globalSdPath,
      })) as any;
      setPlan(result);
      onPlanChange?.(result);
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
        <button onClick={() => { setPlan(null); onPlanChange?.(null); }} disabled={!plan}>Clear</button>
        <button onClick={() => onNext?.()} style={{ marginLeft: "auto" }}>
          Omitir → BIOS
        </button>
        <button className="primary" onClick={() => onNext?.()} disabled={!source && !plan}>
          Continuar a BIOS →
        </button>
      </div>
      <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 8 }}>Analiza recursivamente; la app copiará automáticamente a <code>roms/videos/</code> según extensión, con conversión si es necesario.</div>

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
