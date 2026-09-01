import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { pickFolder } from "../services/dialog";
import { t } from "../i18n";
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
  const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState('');

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
      setError("Select source folder");
      return;
    }
    if (!globalSdPath) {
      setError("No SD selected — go to Overview");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const result = (await invoke("dry_run_with_target", {
        sourcePath: source,
        sdPath: globalSdPath,
      })) as any;
      const videoExts = [".mp4", ".mkv", ".avi", ".mov", ".wmv", ".webm", ".m4v", ".mpg", ".mpeg", ".ts"];
      const filtered = (result.entries as PlanEntry[]).filter(e => {
        const ext = '.' + (e.source.split('.').pop() || '').toLowerCase();
        return videoExts.includes(ext) || e.content_type === "video";
      });
      const filteredPlan = { ...result, entries: filtered };
      setPlan(filteredPlan);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (plan) setSelectedFiles(new Set(plan.entries.map(e => e.source)));
    else setSelectedFiles(new Set());
  }, [plan]);

  useEffect(() => {
    if (!plan) { onPlanChange?.(null); return; }
    if (selectedFiles.size === 0 || selectedFiles.size === plan.entries.length) onPlanChange?.(plan);
    else {
      const filtered = plan.entries.filter(e => selectedFiles.has(e.source));
      const newSummary = { ...plan.summary, new: filtered.filter(e => e.action === 'copy' || e.action === 'extract').length, unchanged: filtered.filter(e => e.action === 'skip_unchanged').length, duplicate_content: filtered.filter(e => e.action === 'skip_duplicate').length, conflicts: filtered.filter(e => e.action === 'conflict').length };
      onPlanChange?.({ ...plan, entries: filtered, summary: newSummary } as any);
    }
  }, [selectedFiles, plan, onPlanChange]);
  const toggleFileSelection = (id: string) => {
    setSelectedFiles(prev => {
      const ns = new Set(prev);
      if (ns.has(id)) ns.delete(id); else ns.add(id);
      return ns;
    });
  };

  const displayFiltered = (() => {
    if (!plan) return [];
    let items = plan.entries;
    if (filter === "compatible") items = items.filter((e) => e.action === "copy");
    else if (filter === "convert") items = items.filter((e) => e.action === "convert_then_copy");
    else if (filter === "error") items = items.filter((e) => e.action === "manual_review" || e.action === "unsupported");
    const q = searchQuery.trim().toLowerCase();
    if (q) items = items.filter(item => (item.source.split(/[\\/]/).pop() || item.source).toLowerCase().includes(q));
    return items;
  })();

  const lastScanKey = useRef("");
  useEffect(() => {
    const key = `${source}|${globalSdPath}`;
    if (visible && source && globalSdPath && key !== lastScanKey.current) {
      lastScanKey.current = key;
      handlePreview();
    }
  }, [visible, source, globalSdPath]);

  return (
    <div className="card">
      <h3>Videos — Hardware decoder (TreeFrogUI)</h3>
      <p style={{ fontSize: 12, color: "var(--text-muted)" }}>
        Inspection via <code>ffprobe</code>: contenedor, codec, perfil/nivel, pix_fmt, dimensiones, framerate, audio. <code>compatible → copiar</code>, <code>incompatible → FFmpeg → re-probe → desplegar</code>. Original nunca se modifica, salida en staging. Preset <code>PROVISIONAL_UNVALIDATED</code>.
      </p>

      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 12 }}>
        <label style={{ fontSize: 13, fontWeight: 600 }}>Videos source folder</label>
        <div className="row" style={{ alignItems: "stretch" }}>
          <div style={{ flex: 1, padding: "8px 10px", border: "1px solid var(--border)", borderRadius: 6, background: "var(--input)", color: source ? "var(--text)" : "var(--text-muted)", fontSize: 13, minHeight: 36, display: "flex", alignItems: "center" }}>
            {source || "No folder selected — e.g., D:\\Videos"}
          </div>
          <button onClick={handlePickSource}>Browse</button>
        </div>
        <div style={{ fontSize: 11, color: "var(--text-muted)" }}>SD destination: <strong>{globalSdPath || "none (select in SD Card)"}</strong> — the app will automatically copy to <code>roms/videos/</code> according to extension.</div>
      </div>

      <div className="row">
        <button className="primary" onClick={() => { lastScanKey.current = `${source}|${globalSdPath}`; handlePreview(); }} disabled={loading || !source || !globalSdPath}>
          {loading ? "Scanning…" : "Scan Videos"}
        </button>
        <button onClick={() => { setPlan(null); onPlanChange?.(null); }} disabled={!plan}>Clear</button>
        <button onClick={() => onNext?.()} style={{ marginLeft: "auto" }}>
          Skip → BIOS
        </button>
        <button className="primary" onClick={() => onNext?.()} disabled={!source && !plan}>
          Continue to BIOS →
        </button>
      </div>
      <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 8 }}>Analyze recursively; the app will automatically copy to <code>roms/videos/</code> according to extension, with conversion if necessary.</div>

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
          <div style={{ marginBottom: '10px', marginTop: '10px' }}>
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t.searchPlaceholder}
              style={{
                width: '100%',
                padding: '8px 12px',
                borderRadius: '4px',
                border: '1px solid var(--border-color)',
                backgroundColor: 'var(--input-bg, transparent)',
                color: 'var(--text-primary)',
              }}
            />
          </div>
          <table style={{ marginTop: 8 }}>
            <thead>
              <tr>
                <th><input type="checkbox" checked={displayFiltered.length > 0 && displayFiltered.every(e => selectedFiles.has(e.source))} onChange={(e) => {
                  if (e.target.checked) {
                    setSelectedFiles(prev => new Set([...prev, ...displayFiltered.map(x => x.source)]));
                  } else {
                    setSelectedFiles(prev => {
                      const ns = new Set(prev);
                      displayFiltered.forEach(x => ns.delete(x.source));
                      return ns;
                    });
                  }
                }} /></th>
                <th>Source</th>
                <th>Destination</th>
                <th>Codec</th>
                <th>Action</th>
              </tr>
            </thead>
            <tbody>
              {displayFiltered.map((e, idx) => (
                <tr key={idx} style={{ background: e.action === "convert_then_copy" ? "var(--warning-bg)" : e.action === "manual_review" ? "#fff8e1" : undefined, opacity: selectedFiles.has(e.source) ? 1 : 0.5 }}>
                  <td><input type="checkbox" checked={selectedFiles.has(e.source)} onChange={() => toggleFileSelection(e.source)} /></td>
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
          {displayFiltered.length === 0 && <EmptyState kind="empty" title="No Videos found" description="No video files (MP4, MKV, AVI, etc.) matched in the selected source. Check media.json and video_presets.json." />}
          <p style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 8 }}>
            Conversion: staging in <code>temp</code>, re-probe, only if valid is copied to SD. Original intact. Conservative preset - conversions are executed with ffmpeg and ffprobe-validated before deploy; not yet hardware-validated on R36SX.
          </p>
        </>
      )}
    </div>
  );
}
