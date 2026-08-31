import { useEffect, useMemo, useRef, useState } from "react";
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
  group?: string[];
  members?: string[] | null;
};

type Plan = {
  summary: { new: number; unchanged: number; changed: number; duplicate_content: number; conflicts: number; deletions: number; manual_review?: number; unsupported_archive?: number };
  entries: PlanEntry[];
  warnings: string[];
};

type SystemOption = {
  id: string;
  folder: string;
  display_name: string;
  core: string;
};

export default function GamesPanel({ 
  globalSdPath, 
  onSourceChange, 
  onPlanChange,
  onNext,
  visible,
  onOverridesChange
}: { 
  globalSdPath: string; 
  onSourceChange?: (v: string) => void; 
  onPlanChange?: (plan: Plan | null) => void;
  onNext?: () => void;
  visible?: boolean;
  onOverridesChange?: (overrides: Record<string, string>) => void;
}) {
  const [source, setSource] = useState("");
  const [plan, setPlan] = useState<Plan | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [filterSystem, setFilterSystem] = useState<string>("all");
  const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set());
  const [editingSystem, setEditingSystem] = useState<string | null>(null);
  const [systemOptions, setSystemOptions] = useState<SystemOption[]>([]);
  const [systemOverrides, setSystemOverrides] = useState<Record<string, string>>({});
  const [searchQuery, setSearchQuery] = useState('');

  async function handlePickSource() {
    try {
      const sel = await pickFolder({ title: "Select Games source folder (ROM library)" });
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
      // Filtrado correcto: solo ROMs
      const filteredEntries = (result.entries as PlanEntry[]).filter(e => e.content_type?.startsWith("rom/") || e.content_type?.startsWith("grouped"));
      const filteredPlan = { ...result, entries: filteredEntries };
      setPlan(filteredPlan);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (plan) {
      setSelectedFiles(new Set(plan.entries.map(e => e.source)));
    } else {
      setSelectedFiles(new Set());
    }
  }, [plan]);

  useEffect(() => {
    if (!plan) {
      onPlanChange?.(null);
      return;
    }
    if (selectedFiles.size === 0 || selectedFiles.size === plan.entries.length) {
      onPlanChange?.(plan);
    } else {
      const filtered = plan.entries.filter(e => selectedFiles.has(e.source));
      const newSummary = {
        ...plan.summary,
        new: filtered.filter(e => e.action === 'copy' || e.action === 'extract').length,
        unchanged: filtered.filter(e => e.action === 'skip_unchanged').length,
        duplicate_content: filtered.filter(e => e.action === 'skip_duplicate').length,
        conflicts: filtered.filter(e => e.action === 'conflict').length,
      };
      onPlanChange?.({ ...plan, entries: filtered, summary: newSummary } as any);
    }
  }, [selectedFiles, plan, onPlanChange]);

  const toggleFileSelection = (fileId: string) => {
    setSelectedFiles(prev => {
      const newSet = new Set(prev);
      if (newSet.has(fileId)) newSet.delete(fileId);
      else newSet.add(fileId);
      return newSet;
    });
  };

  const handleSystemClick = async (romId: string, sourcePath: string) => {
    setEditingSystem(romId);
    const ext = sourcePath.split('.').pop() ? '.' + sourcePath.split('.').pop()!.toLowerCase() : '';
    try {
      const options = await invoke('get_valid_systems_for_extension', { ext }) as SystemOption[];
      setSystemOptions(options);
      if (options.length === 0) {
        // fallback: show all systems if extension not found
        setSystemOptions([]);
      }
    } catch {
      setSystemOptions([]);
    }
  };

  const handleSystemChange = (romId: string, newFolder: string) => {
    if (!plan) return;
    const newEntries = plan.entries.map(e => {
      if (e.source === romId) {
        const fileName = e.source.split(/[\\/]/).pop() || e.source;
        const newDest = `roms/${newFolder}/${fileName}`;
        return { ...e, destination: newDest, content_type: `rom/${newFolder}` };
      }
      return e;
    });
    const newPlan = { ...plan, entries: newEntries };
    setPlan(newPlan);
    const newOverrides = { ...systemOverrides, [romId]: `roms/${newFolder}` };
    setSystemOverrides(newOverrides);
    onOverridesChange?.(newOverrides);
    setEditingSystem(null);
  };

  const lastScanKey = useRef("");
  useEffect(() => {
    const key = `${source}|${globalSdPath}`;
    if (visible && source && globalSdPath && key !== lastScanKey.current) {
      lastScanKey.current = key;
      handlePreview();
    }
  }, [visible, source, globalSdPath]);

  const systems = Array.from(new Set(plan?.entries.map((e) => e.content_type?.replace("rom/", "") || "unknown") || []));

  const filtered = plan?.entries.filter((e) => filterSystem === "all" || e.content_type?.includes(filterSystem)) || [];

  const visibleItems = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return filtered;
    return filtered.filter(item => (item.source.split(/[\\/]/).pop() || item.source).toLowerCase().includes(q));
  }, [filtered, searchQuery]);

  // Expose selected files for parent if needed
  (GamesPanel as any).getSelectedFiles = () => selectedFiles;

  return (
    <div className="card">
      <h3>Games — ROM library (profile-driven)</h3>
      <p style={{ fontSize: 12, color: "var(--text-muted)" }}>
        Manage ROMs by system. Cada carpeta bajo <code>roms/</code> selecciona el core (ej. <code>GBA</code> para GBA, <code>PS</code> para PlayStation). Preserve multi-file logical units (CUE/BIN) y respeta <code>archive_policy.json</code> (arcade <code>cps1/neogeo/m2k</code> como <code>payload</code>, no extraído). Duplicates by <code>SHA-256</code>, no por nombre.
      </p>

      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 12 }}>
        <label style={{ fontSize: 13, fontWeight: 600 }}>Games source folder</label>
        <div className="row" style={{ alignItems: "stretch" }}>
          <div style={{ flex: 1, padding: "8px 10px", border: "1px solid var(--border)", borderRadius: 6, background: "var(--input)", color: source ? "var(--text)" : "var(--text-muted)", fontSize: 13, minHeight: 36, display: "flex", alignItems: "center" }}>
            {source || "No folder selected — e.g., D:\\ROMs"}
          </div>
          <button onClick={handlePickSource}>Browse</button>
        </div>
        <div style={{ fontSize: 11, color: "var(--text-muted)" }}>SD destination: <strong>{globalSdPath || "none (select in SD Card)"}</strong> — the app will automatically copy a la carpeta correcta según extensión (perfil TreeFrogUI).</div>
      </div>

      <div className="row">
        <button className="primary" onClick={() => { lastScanKey.current = `${source}|${globalSdPath}`; handlePreview(); }} disabled={loading || !source || !globalSdPath}>
          {loading ? "Scanning…" : "Scan Games"}
        </button>
        <button onClick={() => { setPlan(null); onPlanChange?.(null); }} disabled={!plan}>Clear</button>
        <button onClick={() => onNext?.()} style={{ marginLeft: "auto" }}>
          Omitir → Music
        </button>
        <button className="primary" onClick={() => onNext?.()} disabled={!source && !plan}>
          Continuar a Music →
        </button>
      </div>
      <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 8 }}>Analiza <strong>recursivamente</strong> subcarpetas (ej. <code>{source || "D:\\ROMs"}\\GBA\game.gba</code> y subcarpetas). Omitir si no tienes Games.</div>

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
                <th>
                  <input 
                    type="checkbox" 
                    checked={visibleItems.length > 0 && visibleItems.every(e => selectedFiles.has(e.source))}
                    onChange={(e) => {
                      if (e.target.checked) {
                        setSelectedFiles(prev => new Set([...prev, ...visibleItems.map(x => x.source)]));
                      } else {
                        setSelectedFiles(prev => {
                          const ns = new Set(prev);
                          visibleItems.forEach(x => ns.delete(x.source));
                          return ns;
                        });
                      }
                    }}
                  />
                </th>
                <th>Source</th>
                <th>Destination</th>
                <th>System</th>
                <th>Action</th>
              </tr>
            </thead>
            <tbody>
              {visibleItems.map((e, idx) => (
                <tr key={idx} style={{ opacity: selectedFiles.has(e.source) ? 1 : 0.5 }}>
                  <td>
                    <input 
                      type="checkbox" 
                      checked={selectedFiles.has(e.source)}
                      onChange={() => toggleFileSelection(e.source)}
                    />
                  </td>
                  <td style={{ fontSize: 11, maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis" }} title={e.source}>
                    {e.source.split(/[\\/]/).pop()}
                  </td>
                  <td style={{ fontSize: 11 }}>{e.destination}</td>
                  <td onClick={() => handleSystemClick(e.source, e.source)} style={{ cursor: 'pointer', fontSize: 11, textDecoration: 'underline', color: 'var(--accent)' }}>
                    {editingSystem === e.source ? (
                      <select 
                        value={e.destination.split('/')[1] || e.content_type?.replace("rom/", "") || ""}
                        onChange={(ev) => handleSystemChange(e.source, ev.target.value)}
                        onBlur={() => setEditingSystem(null)}
                        autoFocus
                        onClick={(ev) => ev.stopPropagation()}
                      >
                        {systemOptions.length > 0 ? systemOptions.map(opt => (
                          <option key={opt.id} value={opt.folder}>
                            {opt.folder} ({opt.core})
                          </option>
                        )) : (
                          <option value={e.destination.split('/')[1]}>{e.destination.split('/')[1]}</option>
                        )}
                      </select>
                    ) : (
                      e.content_type?.replace("rom/", "") || e.destination.split('/')[1] || "—"
                    )}
                  </td>
                  <td>
                    <span className={`badge badge-${e.action === "copy" ? "copy" : e.action === "extract" ? "extract" : e.action === "conflict" ? "conflict" : "skip"}`}>
                      {e.action}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {visibleItems.length === 0 && <EmptyState kind="empty" title="No Games found" description="No ROMs matched profile extensions in the selected source (check roms/ subfolders and archive_policy.json)." />}
        </>
      )}
    </div>
  );
}
