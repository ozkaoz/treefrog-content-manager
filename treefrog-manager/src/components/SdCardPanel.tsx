import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { t } from "../i18n";

import EmptyState from "./EmptyState";

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
  folder_breakdown?: Record<string, number>;
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
  onSync,
  isSyncing
}: { 
  sdPath: string; 
  onChange: (v: string) => void; 
  volumes?: VolumeInfo[];
  globalPlan?: any;
  globalSpace?: any;
  onSync?: (force: boolean) => Promise<any>;
  isSyncing?: boolean;
}) {
  const [analysis, setAnalysis] = useState<TargetAnalysis | null>(null);
  const [space] = useState<SpaceInfo | null>(null);
  const [plan] = useState<any | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [syncResult, setSyncResult] = useState<any | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [forceCopy, setForceCopy] = useState(false);
  const [selectedRoms, setSelectedRoms] = useState<Set<string>>(new Set());
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());
  const [folderContents, setFolderContents] = useState<Record<string, string[]>>({});
  const [progress, setProgress] = useState({ current: 0, total: 0, percentage: 0, message: '' });
  const [deleteProgress, setDeleteProgress] = useState({ 
    current: 0, 
    total: 0, 
    percentage: 0, 
    message: '',
    isDeleting: false 
  });

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

  useEffect(() => {
    const unlisten = listen('deploy-progress', (event) => {
      setProgress(event.payload as any);
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  useEffect(() => {
    const unlisten = listen('delete-progress', (event) => {
      setDeleteProgress(event.payload as any);
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  useEffect(() => {
    return () => {
      setAnalysis(null);
      setSelectedRoms(new Set());
    };
  }, []);

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

  const romList = Object.entries(analysis?.folder_breakdown || {}).flatMap(([folder, count]) => {
    return [{ folder, count }];
  });

  const handleToggleRom = (folder: string) => {
    setSelectedRoms(prev => {
      const newSet = new Set(prev);
      if (newSet.has(folder)) {
        newSet.delete(folder);
      } else {
        newSet.add(folder);
      }
      return newSet;
    });
  };

  const handleFolderClick = async (folder: string) => {
    if (expandedFolders.has(folder)) {
      setExpandedFolders(prev => {
        const ns = new Set(prev);
        ns.delete(folder);
        return ns;
      });
    } else {
      try {
        const files = await invoke('list_files_in_folder', { sdPath, folderRel: folder }) as string[];
        setFolderContents(prev => ({ ...prev, [folder]: files }));
        setExpandedFolders(prev => {
          const ns = new Set(prev);
          ns.add(folder);
          return ns;
        });
      } catch (e) {
        console.error(e);
      }
    }
  };

  const handleFolderToggle = (folder: string) => {
    const files = folderContents[folder] || [];
    if (files.length === 0) {
      handleToggleRom(folder);
      return;
    }
    const allSelected = files.every(f => selectedRoms.has(f));
    setSelectedRoms(prev => {
      const ns = new Set(prev);
      if (allSelected) {
        files.forEach(f => ns.delete(f));
        ns.delete(folder);
      } else {
        files.forEach(f => ns.add(f));
        ns.add(folder);
      }
      return ns;
    });
  };

  const handleDeleteSelected = async () => {
    if (!confirm(t.confirmDelete(selectedRoms.size))) {
      return;
    }
    
    setDeleteProgress(prev => ({ ...prev, isDeleting: true }));
    
    try {
      const result = await invoke('delete_roms_from_sd', {
        sdPath: sdPath,
        filesToDelete: Array.from(selectedRoms),
        deleteAll: false,
      }) as any;
      
      if (result.success) {
        alert(t.deleted(result.deleted));
        setSelectedRoms(new Set());
        await handleAnalyze();
      } else {
        alert(`Error: ${result.errors.join(', ')}`);
      }
      setSyncResult(result);
    } catch (e) {
      alert(`Error: ${e}`);
      setError(String(e));
    } finally {
      setDeleteProgress(prev => ({ ...prev, isDeleting: false }));
    }
  };

  const handleDeleteAll = async () => {
    if (!confirm(t.confirmDeleteAll)) {
      return;
    }
    
    setDeleteProgress(prev => ({ ...prev, isDeleting: true }));
    
    try {
      const result = await invoke('delete_roms_from_sd', {
        sdPath: sdPath,
        filesToDelete: [],
        deleteAll: true,
      }) as any;
      
      if (result.success) {
        alert(t.deletedAll);
        setSelectedRoms(new Set());
        await handleAnalyze();
      } else {
        alert(`Error: ${result.errors.join(', ')}`);
      }
      setSyncResult(result);
    } catch (e) {
      alert(`Error: ${e}`);
      setError(String(e));
    } finally {
      setDeleteProgress(prev => ({ ...prev, isDeleting: false }));
    }
  };


  return (
    <div className="card">
      <h3>SD Card — TreeFrogUI target (read-only)</h3>
      <p style={{ fontSize: 12, color: "var(--text-muted)" }}>
        Select the SD card root (e.g., <code>E:\</code> on Windows) via the native folder picker. The app will inspect it <strong>read-only</strong> — no files or directories will be created. Validation is profile-driven via <code>sd_markers.json</code> (global TreeFrogUI, not R36SX-specific).
      </p>

      <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 12 }}>
        <div style={{ fontSize: 13, fontWeight: 600, display: "flex", alignItems: "center", gap: 8 }}>
          <span>SD selected (global)</span>
          <span style={{ fontSize: 11, color: "var(--text-muted)" }}>— already selected in Overview, not repeated here</span>
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
          {sdPath ? `${sdPath} — ${analysis?.label || "TreeFrogUI"} ✓` : "No SD selected — go to Overview for automatic detection"}
        </div>

        <div style={{ fontSize: 11, color: "var(--text-muted)" }}>
          The SD selected in Overview is shown here. Analysis already done. You can only run synchronization.
        </div>

      </div>

      {error && (
        <div className="status-error" style={{ fontSize: 12, marginBottom: 8, padding: 10, border: "2px solid var(--danger)", background: "var(--danger-bg)", borderRadius: 6 }}>
          <div style={{ fontWeight: 600, marginBottom: 4 }}>⚠ SD Selection Error</div>
          <div>{error}</div>
          {error.includes("You selected the 'roms'") && (
            <div style={{ marginTop: 8, fontSize: 11, color: "var(--text)" }}>
              <strong>Solution:</strong> Select the <strong>root</strong> of the SD card (e.g. <code>E:\</code>) instead of the subfolder <code>roms</code>. The app will automatically create <code>roms/SYSTEM/</code>.
            </div>
          )}
        </div>
      )}

      {!analysis && !loading && !error && <EmptyState kind="empty" title="No target analyzed" description="Click Select SD (native Windows folder picker) then Analyze. No writes will occur." />}

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
          {analysis.folder_breakdown && Object.keys(analysis.folder_breakdown).length > 0 && (
            <div style={{ border: "1px solid var(--border)", borderRadius: 6, padding: 10, background: "var(--surface)", gridColumn: "span 2", marginTop: 12 }}>
              <details open>
                <summary style={{ cursor: "pointer", fontWeight: 600, fontSize: 13 }}>Folder breakdown — {Object.keys(analysis.folder_breakdown).length} folders</summary>
                <div style={{ marginTop: 8, maxHeight: 300, overflowY: "auto" }}>
                  <table style={{ width: "100%", fontSize: 11 }}>
                    <thead><tr><th style={{ textAlign: "left", padding: "4px" }}>Folder</th><th style={{ textAlign: "right", padding: "4px" }}>Files</th><th style={{ textAlign: "left", padding: "4px" }}>Type</th></tr></thead>
                    <tbody>
                      {Object.entries(analysis.folder_breakdown).sort((a,b) => (b[1] as number) - (a[1] as number)).map(([folder, count]) => {
                        const lower = folder.toLowerCase();
                        const isDemo = lower.includes("pico8") || lower.includes("treefrog_defaults") || (lower.includes("samples") && (count as number) > 100);
                        const isAsset = lower.includes("treefrog_defaults") || lower.includes("pico8") || lower.includes("bios");
                        const tag = lower.includes("bios") ? "[SISTEMA]" : isDemo ? "[DEMO]" : isAsset ? "[ASSET]" : "[Usuario]";
                        const tagColor = isDemo ? "var(--warning)" : isAsset ? "var(--accent)" : "var(--success)";
                        return (
                          <tr key={folder}>
                            <td style={{ padding: "4px" }}>{folder}/</td>
                            <td style={{ padding: "4px", textAlign: "right" }}>{count as number}</td>
                            <td style={{ padding: "4px" }}><span style={{ background: tagColor, color: "white", padding: "1px 4px", borderRadius: 3, fontSize: 10 }}>{tag}</span></td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                  <p style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 6 }}>Folders with [DEMO]/[ASSET]/[SYSTEM] are default TreeFrogUI content. [User] are your personal content. E.g.: <code>roms/FC/</code> 12 games, <code>roms/pico8/</code> 28 carts [DEMO].</p>
                </div>
              </details>
            </div>
          )}
        </div>
      )}

      {(loading || isSyncing) && (
        <div style={{ padding: 12, border: "2px solid var(--accent)", borderRadius: 6, background: "var(--accent)", color: "white", textAlign: "center", fontWeight: 600, fontSize: 14 }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: 8 }}>
            <span style={{ display: "inline-block", width: 16, height: 16, border: "2px solid white", borderTop: "2px solid transparent", borderRadius: "50%", animation: "spin 1s linear infinite" }} />
            Transferring files to SD — please wait...
          </div>
          {isSyncing && progress.total > 0 && (
            <div style={{ marginTop: '10px' }}>
              <div style={{ fontSize: '14px', marginBottom: '5px' }}>{progress.message}</div>
              <div style={{ width: '100%', height: '20px', backgroundColor: '#333', borderRadius: '10px', overflow: 'hidden' }}>
                <div style={{ width: `${progress.percentage}%`, height: '100%', backgroundColor: '#4CAF50', transition: 'width 0.3s ease' }} />
              </div>
              <div style={{ fontSize: '12px', marginTop: '5px', textAlign: 'center' }}>
                {progress.percentage}% ({progress.current}/{progress.total})
              </div>
            </div>
          )}
          <div style={{ fontSize: 11, fontWeight: 400, marginTop: 4 }}>Do not disconnect the SD card</div>
        </div>
      )}
      <div style={{ marginTop: 12, display: "flex", gap: 8, flexDirection: "column" }}>
        <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12, cursor: "pointer" }}>
          <input type="checkbox" checked={forceCopy} onChange={(e) => setForceCopy(e.target.checked)} />
          {t.forceCopy}
        </label>
        {confirming ? (
          <div style={{ padding: 12, border: "1px solid var(--accent)", borderRadius: 6, background: "var(--surface-elevated)" }}>
            <div style={{ fontSize: 13, marginBottom: 8 }}>
              {t.confirmSync(sdPath, globalPlan?.summary.new ?? 0, globalPlan?.summary.changed ?? 0, forceCopy)}
            </div>
            <div className="row">
              <button className="primary" onClick={async () => {
                setConfirming(false);
                setSyncResult(null);
                try {
                  const r = await onSync?.(forceCopy);
                  setSyncResult(r);
                } catch (e) { setError(String(e)); }
              }}>{t.yes}</button>
              <button onClick={() => setConfirming(false)}>{t.cancel}</button>
            </div>
          </div>
        ) : (
          <button
            onClick={() => { setError(""); setConfirming(true); }}
            disabled={!globalPlan || globalSpace?.status === "insufficient_space" || loading ||
              ((globalPlan.summary.new === 0 && globalPlan.summary.changed === 0) && !forceCopy)}
            className="primary"
            style={{ width: "100%", padding: "12px", fontSize: 14, fontWeight: 600 }}
            title={
              !globalPlan 
                ? "Go to Overview and press ANALYZE first"
                : globalPlan.summary.new === 0 && globalPlan.summary.changed === 0
                ? "No new or modified files to synchronize"
                : `Synchronize ${globalPlan.summary.new} new to ${sdPath}`
            }
          >
            {loading ? "Synchronizing…" : "Sync to SD"}
          </button>

        )}
        <div style={{ fontSize: 11, color: "var(--text-muted)", textAlign: "center" }}>
          {!globalPlan 
            ? "Go to Overview and press ANALYZE to prepare synchronization."
            : globalPlan.summary.new === 0 && globalPlan.summary.changed === 0
            ? "No new or modified files. Go to Games/Music/Videos/BIOS/LGPT and select source folders."
            : `Ready to synchronize: ${globalPlan.summary.new} new, ${globalPlan.summary.changed} modified, ${globalPlan.summary.unchanged} unchanged.`}
        </div>
      </div>

      {syncResult && (
        <div style={{ marginTop: 16, padding: 12, border: "1px solid var(--border)", borderRadius: 6, background: "var(--surface)" }}>
          <h4 style={{ margin: "0 0 8px 0" }}>Synchronization Result</h4>
          <div style={{ fontSize: 13, marginBottom: 8 }}>
            <div><strong>Copied:</strong> <span style={{ color: "var(--success)" }}>{syncResult.deployed}</span></div>
            <div><strong>Skipped:</strong> <span style={{ color: "var(--warning)" }}>{syncResult.skipped}</span></div>
            <div><strong>Failed:</strong> <span style={{ color: "var(--danger)" }}>{syncResult.failed}</span></div>
          </div>
          
          {syncResult.breakdown && syncResult.breakdown.length > 0 && (
            <details style={{ marginTop: 8 }}>
              <summary style={{ cursor: "pointer", fontSize: 12, fontWeight: 600 }}>View file details ({syncResult.breakdown.length})</summary>
              <div style={{ maxHeight: 300, overflowY: "auto", marginTop: 8 }}>
                <table style={{ width: "100%", fontSize: 11 }}>
                  <thead>
                    <tr>
                      <th style={{ textAlign: "left", padding: "4px" }}>File</th>
                      <th style={{ textAlign: "left", padding: "4px" }}>Destination</th>
                      <th style={{ textAlign: "left", padding: "4px" }}>Action</th>
                      <th style={{ textAlign: "left", padding: "4px" }}>Reason</th>
                    </tr>
                  </thead>
                  <tbody>
                    {syncResult.breakdown.map((item: any, idx: number) => (
                      <tr key={idx} style={{ borderBottom: "1px solid var(--border)" }}>
                        <td style={{ padding: "4px", maxWidth: 180, overflow: "hidden", textOverflow: "ellipsis" }} title={item.source}>
                          {item.source.split(/[/\\]/).pop()}
                        </td>
                        <td style={{ padding: "4px", fontSize: 10 }} title={item.dest_abs || item.destination}>
                          {item.destination}
                          <div style={{ fontSize: 9, color: "var(--text-muted)" }}>
                            {item.dest_exists ? "✓" : "✕"} {item.dest_abs || ""}
                          </div>
                        </td>
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
            <p style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 6 }}>
              Destination existence (✓/✕) is verified on disk at sync time. If a file does not exist on the SD, it is copied automatically.
            </p>
          
          {syncResult.warnings && syncResult.warnings.length > 0 && (
            <div style={{ marginTop: 8 }}>
              <strong style={{ fontSize: 12 }}>Warnings:</strong>
              <ul style={{ fontSize: 11, margin: "4px 0", paddingLeft: 20 }}>
                {syncResult.warnings.map((w: string, idx: number) => (
                  <li key={idx} style={{ color: "var(--warning)" }}>{w}</li>
                ))}
              </ul>
            </div>
          )}
          
          {syncResult.errors && syncResult.errors.length > 0 && (
            <div style={{ marginTop: 8 }}>
              <strong style={{ fontSize: 12, color: "var(--danger)" }}>Errors:</strong>
              <ul style={{ fontSize: 11, margin: "4px 0", paddingLeft: 20 }}>
                {syncResult.errors.map((e: string, idx: number) => (
                  <li key={idx}>{e}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      <div style={{ 
        marginTop: '30px', 
        padding: '20px', 
        border: '2px solid var(--danger)', 
        borderRadius: '8px',
        backgroundColor: 'var(--danger-bg, rgba(211, 47, 47, 0.1))'
      }}>
        <h3 style={{ color: 'var(--danger)', marginBottom: '15px' }}>
          ⚠️ {t.dangerZone}
        </h3>
        
        <div style={{ marginBottom: '15px', color: 'var(--text-primary)' }}>
          <h4 style={{ color: 'var(--text-primary)' }}>{t.romFoldersOnSd}:</h4>
          {romList.length === 0 ? (
            <div style={{ fontSize: 11, color: "var(--text-muted)" }}>No ROM folders detected. Analyze the SD first.</div>
          ) : (
            romList.map(({ folder, count }) => {
              const isExpanded = expandedFolders.has(folder);
              const files = folderContents[folder] || [];
              const isFolderSelected = selectedRoms.has(folder) || (files.length > 0 && files.every(f => selectedRoms.has(f)));
              return (
                <div key={folder} style={{ marginBottom: '8px' }}>
                  <label style={{ 
                    display: 'flex', 
                    alignItems: 'center',
                    color: 'var(--text-primary)',
                    cursor: 'pointer'
                  }}>
                    <input
                      type="checkbox"
                      checked={isFolderSelected}
                      onChange={() => handleFolderToggle(folder)}
                      style={{ marginRight: '8px' }}
                    />
                    <span onClick={(e) => { e.preventDefault(); handleFolderClick(folder); }} style={{ flex: 1, display: 'flex', alignItems: 'center', gap: '6px' }}>
                      <span style={{ fontSize: '10px', transform: isExpanded ? 'rotate(90deg)' : 'rotate(0deg)', transition: 'transform 0.2s', display: 'inline-block' }}>▶</span>
                      {folder} <span style={{ color: 'var(--text-secondary)' }}>({count} files)</span>
                    </span>
                  </label>
                  {isExpanded && files && (
                    <div style={{ marginLeft: '24px', marginTop: '8px', paddingLeft: '12px', borderLeft: '2px solid var(--border-color)' }}>
                      {files.map(filePath => (
                        <label key={filePath} style={{ display: 'block', marginBottom: '4px', fontSize: '13px', cursor: 'pointer' }}>
                          <input
                            type="checkbox"
                            checked={selectedRoms.has(filePath)}
                            onChange={() => handleToggleRom(filePath)}
                            style={{ marginRight: '8px' }}
                          />
                          {filePath.split('/').pop()}
                        </label>
                      ))}
                    </div>
                  )}
                </div>
              );
            })
          )}
        </div>
        
        <div style={{ display: 'flex', gap: '10px' }}>
          <button
            onClick={handleDeleteSelected}
            disabled={selectedRoms.size === 0}
            style={{
              padding: '10px 20px',
              backgroundColor: selectedRoms.size === 0 ? 'var(--button-disabled-bg)' : 'var(--danger)',
              color: 'var(--button-text)',
              border: 'none',
              borderRadius: '4px',
              cursor: selectedRoms.size === 0 ? 'not-allowed' : 'pointer',
              fontWeight: 'bold',
            }}
          >
            {t.deleteSelected} ({selectedRoms.size})
          </button>
          
          <button
            onClick={handleDeleteAll}
            style={{
              padding: '10px 20px',
              backgroundColor: 'var(--danger)',
              color: 'var(--button-text)',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              fontWeight: 'bold',
            }}
          >
            {t.deleteAll}
          </button>
        </div>
        {deleteProgress.isDeleting && (
          <div style={{ marginTop: '15px' }}>
            <div style={{ fontSize: '14px', marginBottom: '5px', color: 'var(--text-primary)' }}>
              {deleteProgress.message}
            </div>
            <div style={{ 
              width: '100%', 
              height: '20px', 
              backgroundColor: 'var(--progress-bg, #333)', 
              borderRadius: '10px',
              overflow: 'hidden',
              border: '1px solid var(--border-color, #555)'
            }}>
              <div style={{ 
                width: `${deleteProgress.percentage}%`, 
                height: '100%', 
                backgroundColor: 'var(--danger)',
                transition: 'width 0.3s ease'
              }} />
            </div>
            <div style={{ fontSize: '12px', marginTop: '5px', textAlign: 'center', color: 'var(--text-secondary)' }}>
              {deleteProgress.percentage}% ({deleteProgress.current}/{deleteProgress.total})
            </div>
          </div>
        )}
      </div>

      <div style={{ marginTop: '15px' }}>
        <h4 style={{ color: 'var(--text-primary)' }}>{t.artworkTitle}</h4>
        <p style={{ color: 'var(--text-secondary)', fontSize: '13px' }}>
          {t.artworkDesc}
        </p>
        <button
          onClick={() => {
            import('@tauri-apps/plugin-shell').then(({ open }) => {
              open('https://github.com/tzubertowski/mini-scraper-cfw/releases/latest');
            }).catch(() => {
              window.open('https://github.com/tzubertowski/mini-scraper-cfw/releases/latest', '_blank');
            });
          }}
          style={{
            padding: '8px 16px',
            backgroundColor: 'var(--accent, #1976d2)',
            color: 'var(--button-text, #fff)',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
          }}
        >
          {t.downloadMiniScraper}
        </button>
      </div>
    </div>
  );
}
