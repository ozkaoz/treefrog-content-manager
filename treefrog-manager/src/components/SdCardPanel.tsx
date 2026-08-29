import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { pickFolder } from "../services/dialog";
import EmptyState from "./EmptyState";
import MiniScraper from "./MiniScraper";

type VolumeInfo = {
  path: string;
  label?: string | null;
  filesystem?: string | null;
  total_bytes?: number | null;
  free_bytes?: number | null;
  removable?: boolean | null;
  accessible: boolean;
  error?: string | null;
};

type TargetAnalysis = {
  path: string;
  volume: VolumeInfo;
  status: string;
  is_treefrog: boolean;
  is_incomplete: boolean;
  markers_found: string[];
  markers_missing: string[];
  lgpt_detected: boolean;
  rom_dirs: string[];
  media_dirs: string[];
  bios_dirs: string[];
  lgpt_dirs: string[];
  existing_count: number;
  total_size: number;
  free_bytes?: number | null;
  capacity_bytes?: number | null;
  filesystem?: string | null;
  label?: string | null;
  errors: string[];
};

type SpaceInfo = {
  bytes_to_copy: number;
  bytes_to_extract: number;
  bytes_to_generate: number;
  bytes_to_skip: number;
  required_bytes: number;
  available_bytes?: number | null;
  status: string;
};

function fmtBytes(n?: number | null): string {
  if (n == null) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = n;
  let u = 0;
  while (v >= 1024 && u < units.length - 1) {
    v /= 1024;
    u++;
  }
  return `${v.toFixed(u === 0 ? 0 : 1)} ${units[u]}`;
}

export default function SdCardPanel({ 
  sdPath, 
  onChange, 
  volumes: propVolumes,
  globalPlan,
  globalSpace,
  onSync
}: { 
  sdPath: string; 
  onChange: (v: string) => void; 
  volumes?: VolumeInfo[];
  globalPlan?: any;
  globalSpace?: any;
  onSync?: () => Promise<void>;
}) {
  const [analysis, setAnalysis] = useState<TargetAnalysis | null>(null);
  const [space] = useState<SpaceInfo | null>(null);
  const [plan] = useState<any | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const [volumesState, setVolumesState] = useState<VolumeInfo[]>(propVolumes || []);
  const volumes = propVolumes !== undefined ? propVolumes : volumesState;

  useEffect(() => {
    if (propVolumes !== undefined) return;
    async function loadVolumes() {
      try {
        const vols = (await invoke("list_volumes")) as VolumeInfo[];
        setVolumesState(vols);
      } catch {}
    }
    loadVolumes();
  }, [propVolumes]);

  async function handlePick() {
    try {
      const sel = await pickFolder({ title: "Select TreeFrogUI SD target (e.g., E:\\ or /mnt/sdcard)" });
      if (sel) {
        onChange(sel);
        // auto-analyze after pick
        await handleAnalyze(sel);
      }
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleAnalyze(p?: string) {
    const target = p ?? sdPath;
    if (!target) {
      setError("Select an SD target first");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const res = (await invoke("analyze_target", { path: target })) as TargetAnalysis;
      setAnalysis(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }



  return (
    <div className="card">
      <h3>SD Card — TreeFrogUI target (read-only)</h3>
      <p style={{ fontSize: 12, color: "var(--text-muted)" }}>
        Select the SD card root (e.g., <code>E:\</code> on Windows) via the native folder picker. The app will inspect it <strong>read-only</strong> — no files or directories will be created. Validation is profile-driven via <code>sd_markers.json</code> (global TreeFrogUI, not R36SX-specific).
      </p>

      <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 12 }}>
        <div style={{ fontSize: 13, fontWeight: 600, display: "flex", alignItems: "center", gap: 8 }}>
          <span>SD seleccionada (global)</span>
          <span style={{ fontSize: 11, color: "var(--text-muted)" }}>— ya elegida en Overview, no se repite aquí</span>
        </div>
        <div
          style={{
            padding: "8px 10px",
            border: "1px solid var(--border)",
            borderRadius: 6,
            background: "var(--surface)",
            color: sdPath ? "var(--text)" : "var(--text-muted)",
            fontSize: 13,
            minHeight: 36,
            display: "flex",
            alignItems: "center",
            fontWeight: sdPath ? 600 : 400,
          }}
        >
          {sdPath ? `${sdPath} — ${analysis?.label || "TreeFrogUI"} ✓` : "No hay SD seleccionada — ve a Overview para detección automática"}
        </div>
        <div style={{ display: "flex", gap: 8, marginTop: 4 }}>
          <button onClick={handlePick} style={{ fontSize: 12 }}>Cambiar SD…</button>
          <button onClick={() => handleAnalyze()} disabled={!sdPath || loading} style={{ fontSize: 12 }}>
            {loading ? "Analizando…" : "Re-analizar"}
          </button>
          <span style={{ fontSize: 11, color: "var(--text-muted)", alignSelf: "center" }}>Analiza recursivamente subcarpetas, no crea nada.</span>
        </div>
        <div style={{ fontSize: 11, color: "var(--text-muted)" }}>
          La SD se detecta automáticamente al iniciar y se selecciona globalmente. Si no es la correcta, usa <strong>Cambiar SD…</strong> (explorador nativo) o elige abajo entre todas las conectadas.
        </div>
        {volumes.length > 0 && (
          <div style={{ border: "1px solid var(--border)", borderRadius: 6, padding: 8, background: "var(--surface)", maxHeight: 180, overflowY: "auto" }}>
            <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 6, color: "var(--text-muted)" }}>Unidades detectadas ({volumes.length}) — selecciona a cuál aplicar:</div>
            {volumes.map((v) => (
              <label key={v.path} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 6px", borderRadius: 4, background: sdPath === v.path ? "var(--surface-elevated)" : "transparent", border: sdPath === v.path ? "1px solid var(--accent)" : "1px solid transparent", cursor: "pointer", marginBottom: 4 }}>
                <input type="radio" name="sd-select" checked={sdPath === v.path} onChange={() => { onChange(v.path); handleAnalyze(v.path); }} />
                <span style={{ fontSize: 12, flex: 1 }}>
                  <strong>{v.path}</strong> {v.label ? `— ${v.label}` : ""} <span style={{ color: "var(--text-muted)" }}>{v.filesystem || ""} {v.total_bytes ? `• ${fmtBytes(v.total_bytes)}` : ""} {v.free_bytes ? `• ${fmtBytes(v.free_bytes)} libre` : ""}</span>
                  {v.removable ? <span style={{ marginLeft: 6, fontSize: 10, background: "var(--success)", color: "white", padding: "1px 4px", borderRadius: 3 }}>Removible</span> : null}
                  {!v.accessible && <span style={{ marginLeft: 6, fontSize: 10, background: "var(--danger)", color: "white", padding: "1px 4px", borderRadius: 3 }}>No accesible</span>}
                </span>
              </label>
            ))}
            {volumes.length === 0 && <div style={{ fontSize: 11, color: "var(--text-muted)" }}>No se detectaron unidades. Conecta una SD y pulsa ↻.</div>}
          </div>
        )}

      </div>

      {error && <div className="status-error" style={{ fontSize: 12, marginBottom: 8 }}>{error}</div>}

      {!analysis && !loading && <EmptyState kind="empty" title="No target analyzed" description="Click Select SD (native Windows folder picker) then Analyze. No writes will occur." />}

      {loading && <EmptyState kind="loading" title="Analyzing…" description="Reading volume info, checking markers, indexing existing content (read-only)." />}

      {analysis && (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginTop: 12 }}>
          <div style={{ border: "1px solid var(--border)", borderRadius: 6, padding: 10, background: "var(--surface)" }}>
            <h4 style={{ margin: "0 0 6px 0" }}>Volume</h4>
            <div style={{ fontSize: 12 }}>
              <div><strong>Path:</strong> {analysis.path}</div>
              <div><strong>Label:</strong> {analysis.label || "—"}</div>
              <div><strong>Filesystem:</strong> {analysis.filesystem || analysis.volume.filesystem || "—"}</div>
              <div><strong>Capacity:</strong> {fmtBytes(analysis.capacity_bytes ?? analysis.volume.total_bytes)}</div>
              <div><strong>Free:</strong> {fmtBytes(analysis.free_bytes ?? analysis.volume.free_bytes)}</div>
              <div><strong>Removable:</strong> {analysis.volume.removable == null ? "—" : analysis.volume.removable ? "Yes" : "No"}</div>
              <div><strong>Accessible:</strong> {analysis.volume.accessible ? "Yes" : "No"}</div>
              {analysis.volume.error && <div style={{ color: "var(--danger)" }}><strong>Error:</strong> {analysis.volume.error}</div>}
            </div>
          </div>
          <div style={{ border: "1px solid var(--border)", borderRadius: 6, padding: 10, background: analysis.status === "valid" ? "var(--success-bg)" : analysis.status === "incomplete" ? "var(--warning-bg)" : "var(--danger-bg)" }}>
            <h4 style={{ margin: "0 0 6px 0" }}>TreeFrogUI</h4>
            <div style={{ fontSize: 12 }}>
              <div><strong>Status:</strong> <span style={{ fontWeight: 600, textTransform: "uppercase" }}>{analysis.status}</span> {analysis.is_treefrog ? "✓ detected" : analysis.is_incomplete ? "⚠ incomplete" : "✕ not detected"}</div>
              <div><strong>Markers found:</strong> {analysis.markers_found.join(", ") || "—"}</div>
              <div><strong>Markers missing:</strong> {analysis.markers_missing.join(", ") || "—"}</div>
              <div><strong>LGPT:</strong> {analysis.lgpt_detected ? "✓ detected" : "—"}</div>
              <div><strong>ROM dirs:</strong> {analysis.rom_dirs.join(", ") || "—"}</div>
              <div><strong>Media dirs:</strong> {analysis.media_dirs.join(", ") || "—"}</div>
              <div><strong>BIOS dirs:</strong> {analysis.bios_dirs.join(", ") || "—"}</div>
              <div><strong>LGPT dirs:</strong> {analysis.lgpt_dirs.join(", ") || "—"}</div>
            </div>
          </div>
          <div style={{ border: "1px solid var(--border)", borderRadius: 6, padding: 10, background: "var(--surface)", gridColumn: "span 2" }}>
            <h4 style={{ margin: "0 0 6px 0" }}>Existing content (read-only index)</h4>
            <div style={{ fontSize: 12 }}>
              <div><strong>Files indexed:</strong> {analysis.existing_count}</div>
              <div><strong>Total size:</strong> {fmtBytes(analysis.total_size)}</div>
              {space && (
                <>
                  <div style={{ marginTop: 6, borderTop: "1px solid var(--border)", paddingTop: 6 }}>
                    <strong>Space for dry-run:</strong>
                    <div>To copy: {fmtBytes(space.bytes_to_copy)} | To extract: {fmtBytes(space.bytes_to_extract)} | To generate: {fmtBytes(space.bytes_to_generate)} | Skip: {fmtBytes(space.bytes_to_skip)}</div>
                    <div>Required: <strong>{fmtBytes(space.required_bytes)}</strong> | Available: <strong>{fmtBytes(space.available_bytes)}</strong> | Status: <span style={{ fontWeight: 600, color: space.status === "insufficient_space" ? "var(--danger)" : "var(--success)" }}>{space.status === "insufficient_space" ? "Not enough space" : space.status === "ok" ? "READY" : space.status}</span></div>
                    {space.status === "insufficient_space" && <div className="status-error" style={{ marginTop: 6 }}>Not enough space on target. Free more space or reduce source selection.</div>}
                  </div>
                </>
              )}
              {plan && plan.summary && (
                <div style={{ marginTop: 6 }}>
                  <strong>Plan:</strong> {plan.summary.new} new, {plan.summary.unchanged} unchanged, {plan.summary.duplicate_content} duplicate, {plan.summary.conflicts} conflicts, {plan.summary.manual_review ?? 0} manual review
                  {plan.collisions && plan.collisions.length > 0 && <div style={{ color: "var(--danger)" }}>Collisions (case-insensitive): {plan.collisions.map((c: any) => `${c[0]} ↔ ${c[1]}`).join(", ")}</div>}
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      <div style={{ marginTop: 12, display: "flex", gap: 8, flexDirection: "column" }}>
        <button
          onClick={onSync}
          disabled={!globalPlan || globalSpace?.status === "insufficient_space" || loading}
          className="primary"
          style={{ width: "100%", padding: "12px", fontSize: 14, fontWeight: 600 }}
          title={globalPlan ? `Sincronizar ${globalPlan.summary.new} nuevos a ${sdPath}` : "Ve a Overview y pulsa ANALIZAR primero"}
        >
          {loading ? "Sincronizando…" : "Sync to SD"}
        </button>
        <div style={{ fontSize: 11, color: "var(--text-muted)", textAlign: "center" }}>
          {globalPlan 
            ? `Listo para sincronizar: ${globalPlan.summary.new} nuevos, ${globalPlan.summary.unchanged} sin cambios.` 
            : "Ve a Overview y pulsa ANALIZAR para preparar la sincronización."}
        </div>
      </div>

      <MiniScraper sdPath={sdPath} />
    </div>
  );
}
