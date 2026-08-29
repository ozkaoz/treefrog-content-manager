import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
  onSync?: () => Promise<any>;
}) {
  const [analysis, setAnalysis] = useState<TargetAnalysis | null>(null);
  const [space] = useState<SpaceInfo | null>(null);
  const [plan] = useState<any | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [syncResult, setSyncResult] = useState<any | null>(null);
  const [confirming, setConfirming] = useState(false);

  const [volumesState, setVolumesState] = useState<VolumeInfo[]>(propVolumes || []);
  const _volumes = propVolumes !== undefined ? propVolumes : volumesState;
  void _volumes;
  void onChange;

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

  useEffect(() => {
    if (sdPath) void handleAnalyze(sdPath);
  }, [sdPath]);

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

        <div style={{ fontSize: 11, color: "var(--text-muted)" }}>
          La SD seleccionada en Overview se muestra aquí. El análisis ya se realizó. Solo puedes ejecutar la sincronización.
        </div>

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
        {!confirming ? (
          <button
            onClick={() => setConfirming(true)}
            disabled={
              !globalPlan || 
              globalSpace?.status === "insufficient_space" || 
              loading ||
              (globalPlan.summary.new === 0 && globalPlan.summary.changed === 0)
            }
            className="primary"
            style={{ width: "100%", padding: "12px", fontSize: 14, fontWeight: 600 }}
            title={
              !globalPlan 
                ? "Ve a Overview y pulsa ANALIZAR primero"
                : globalPlan.summary.new === 0 && globalPlan.summary.changed === 0
                ? "No hay archivos nuevos o modificados para sincronizar"
                : `Sincronizar ${globalPlan.summary.new} nuevos a ${sdPath}`
            }
          >
            {loading ? "Sincronizando…" : "Sync to SD"}
          </button>
        ) : (
          <div style={{ padding: 12, border: "1px solid var(--warning)", borderRadius: 6, background: "var(--warning-bg)" }}>
            <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>¿Confirmar sincronización?</div>
            <div style={{ fontSize: 12, marginBottom: 12 }}>
              Se copiarán <strong>{globalPlan?.summary.new ?? 0} nuevos</strong> y <strong>{globalPlan?.summary.changed ?? 0} modificados</strong> a <code>{sdPath}</code>. Los archivos se copiarán a la carpeta correcta según su extensión.
            </div>
            <div style={{ display: "flex", gap: 8 }}>
              <button
                className="primary"
                onClick={async () => {
                  setConfirming(false);
                  if (!onSync) return;
                  try {
                    const result = await onSync();
                    if (result) setSyncResult(result);
                    if (result?.error) setError(result.error);
                  } catch (e) {
                    setError(String(e));
                  }
                }}
                disabled={loading}
                style={{ flex: 1 }}
              >
                Sí, sincronizar
              </button>
              <button onClick={() => setConfirming(false)} disabled={loading} style={{ flex: 1 }}>
                Cancelar
              </button>
            </div>
          </div>
        )}
        <div style={{ fontSize: 11, color: "var(--text-muted)", textAlign: "center" }}>
          {!globalPlan 
            ? "Ve a Overview y pulsa ANALIZAR para preparar la sincronización."
            : globalPlan.summary.new === 0 && globalPlan.summary.changed === 0
            ? "No hay archivos nuevos o modificados. Ve a Games/Music/Videos/BIOS/LGPT y selecciona carpetas de origen."
            : `Listo para sincronizar: ${globalPlan.summary.new} nuevos, ${globalPlan.summary.changed} modificados, ${globalPlan.summary.unchanged} sin cambios.`}
        </div>
      </div>

      {syncResult && (
        <div style={{ marginTop: 16, padding: 12, border: "1px solid var(--border)", borderRadius: 6, background: "var(--surface)" }}>
          <h4 style={{ margin: "0 0 8px 0" }}>Resultado de Sincronización</h4>
          <div style={{ fontSize: 13, marginBottom: 8 }}>
            <div><strong>Copiados:</strong> <span style={{ color: "var(--success)" }}>{syncResult.deployed}</span></div>
            <div><strong>Omitidos:</strong> <span style={{ color: "var(--warning)" }}>{syncResult.skipped}</span></div>
            <div><strong>Fallidos:</strong> <span style={{ color: "var(--danger)" }}>{syncResult.failed}</span></div>
          </div>
          
          {syncResult.breakdown && syncResult.breakdown.length > 0 && (
            <details style={{ marginTop: 8 }}>
              <summary style={{ cursor: "pointer", fontSize: 12, fontWeight: 600 }}>Ver detalle de archivos ({syncResult.breakdown.length})</summary>
              <div style={{ maxHeight: 300, overflowY: "auto", marginTop: 8 }}>
                <table style={{ width: "100%", fontSize: 11 }}>
                  <thead>
                    <tr>
                      <th style={{ textAlign: "left", padding: "4px" }}>Archivo</th>
                      <th style={{ textAlign: "left", padding: "4px" }}>Destino</th>
                      <th style={{ textAlign: "left", padding: "4px" }}>Acción</th>
                      <th style={{ textAlign: "left", padding: "4px" }}>Razón</th>
                    </tr>
                  </thead>
                  <tbody>
                    {syncResult.breakdown.map((item: any, idx: number) => (
                      <tr key={idx} style={{ borderBottom: "1px solid var(--border)" }}>
                        <td style={{ padding: "4px", maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis" }} title={item.source}>
                          {item.source.split(/[/\\]/).pop()}
                        </td>
                        <td style={{ padding: "4px", fontSize: 10 }}>{item.destination}</td>
                        <td style={{ padding: "4px" }}>
                          <span className={`badge badge-${item.action === "copy" ? "copy" : item.action.startsWith("skip") ? "skip" : "conflict"}`}>
                            {item.action}
                          </span>
                        </td>
                        <td style={{ padding: "4px", fontSize: 10 }}>{item.reason}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </details>
          )}
          
          {syncResult.warnings && syncResult.warnings.length > 0 && (
            <div style={{ marginTop: 8 }}>
              <strong style={{ fontSize: 12 }}>Advertencias:</strong>
              <ul style={{ fontSize: 11, margin: "4px 0", paddingLeft: 20 }}>
                {syncResult.warnings.map((w: string, idx: number) => (
                  <li key={idx} style={{ color: "var(--warning)" }}>{w}</li>
                ))}
              </ul>
            </div>
          )}
          
          {syncResult.errors && syncResult.errors.length > 0 && (
            <div style={{ marginTop: 8 }}>
              <strong style={{ fontSize: 12, color: "var(--danger)" }}>Errores:</strong>
              <ul style={{ fontSize: 11, margin: "4px 0", paddingLeft: 20 }}>
                {syncResult.errors.map((e: string, idx: number) => (
                  <li key={idx}>{e}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      <MiniScraper sdPath={sdPath} />
    </div>
  );
}
