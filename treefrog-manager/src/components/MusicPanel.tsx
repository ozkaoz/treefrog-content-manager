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
  const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState('');

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
      const audioExts = [".mp3", ".flac", ".ogg", ".wav", ".m4a", ".aac", ".opus"];
      const filtered = (result.entries as PlanEntry[]).filter(e => {
        const ext = '.' + (e.source.split('.').pop() || '').toLowerCase();
        return audioExts.includes(ext) || e.content_type === "music";
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
      const newSummary = { new: filtered.filter(e => e.action === 'copy' || e.action === 'extract').length, unchanged: filtered.filter(e => e.action === 'skip_unchanged').length, duplicate_content: filtered.filter(e => e.action === 'skip_duplicate').length, conflicts: filtered.filter(e => e.action === 'conflict').length };
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

  const visibleTracks = useMemo(() => {
    if (!plan) return [];
    const q = searchQuery.trim().toLowerCase();
    let items = plan.entries;
    if (q) items = items.filter(item => (item.source.split(/[\\/]/).pop() || item.source).toLowerCase().includes(q));
    return items;
  }, [plan, searchQuery]);

  const playlists = (() => {
    if (!plan) return [];
    const map = new Map<string, PlanEntry[]>();
    for (const e of visibleTracks) {
      const parts = e.destination.split("/");
      const playlist = parts.length >= 3 ? parts[2] : "unknown";
      if (!map.has(playlist)) map.set(playlist, []);
      map.get(playlist)!.push(e);
    }
    return Array.from(map.entries());
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
      <h3>Music — Playlists (TreeFrogUI)</h3>
      <p style={{ fontSize: 12, color: "var(--text-muted)" }}>
        Each subfolder under <code>roms/music/</code> es una playlist en TreeFrogUI. Preserves subfolders (ej. <code>roms/music/My Album/</code>). Supported formats via profile <code>media.json</code>: MP3, M4A, AAC, WAV, FLAC, OGG, Opus.
      </p>

      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 12 }}>
        <label style={{ fontSize: 13, fontWeight: 600 }}>Music source folder</label>
        <div className="row" style={{ alignItems: "stretch" }}>
          <div style={{ flex: 1, padding: "8px 10px", border: "1px solid var(--border)", borderRadius: 6, background: "var(--input)", color: source ? "var(--text)" : "var(--text-muted)", fontSize: 13, minHeight: 36, display: "flex", alignItems: "center" }}>
            {source || "No folder selected — e.g., D:\\Music"}
          </div>
          <button onClick={handlePickSource}>Browse</button>
        </div>
        <div style={{ fontSize: 11, color: "var(--text-muted)" }}>SD destination: <strong>{globalSdPath || "none (select in SD Card)"}</strong> — the app will automatically copy a <code>roms/music/</code> según extensión.</div>
      </div>

      <div className="row">
        <button className="primary" onClick={() => { lastScanKey.current = `${source}|${globalSdPath}`; handlePreview(); }} disabled={loading || !source || !globalSdPath}>
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
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '8px' }}>
            <label style={{ display: 'flex', alignItems: 'center', gap: '6px', cursor: 'pointer' }}>
              <input
                type="checkbox"
                checked={visibleTracks.length > 0 && visibleTracks.every(t => selectedFiles.has(t.source))}
                onChange={(e) => {
                  if (e.target.checked) {
                    setSelectedFiles(prev => new Set([...prev, ...visibleTracks.map(t => t.source)]));
                  } else {
                    setSelectedFiles(prev => {
                      const ns = new Set(prev);
                      visibleTracks.forEach(t => ns.delete(t.source));
                      return ns;
                    });
                  }
                }}
              />
              Track
            </label>
          </div>
          {playlists.map(([playlist, entries]) => (
            <div key={playlist} style={{ marginTop: 10, border: "1px solid var(--border)", borderRadius: 6, padding: 8, background: "var(--surface)" }}>
              <h4 style={{ margin: "0 0 6px 0", fontSize: 13 }}>Playlist: {playlist} ({entries.length} tracks)</h4>
              <table>
                <thead>
                  <tr>
                    <th></th>
                    <th>Track</th>
                    <th>Destination</th>
                    <th>Action</th>
                  </tr>
                </thead>
                <tbody>
                  {entries.map((e, idx) => (
                    <tr key={idx} style={{ opacity: selectedFiles.has(e.source) ? 1 : 0.5 }}>
                      <td><input type="checkbox" checked={selectedFiles.has(e.source)} onChange={() => toggleFileSelection(e.source)} /></td>
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
          {visibleTracks.length === 0 && <EmptyState kind="empty" title="No Music found" description="No audio files (MP3, FLAC, etc.) matched in the selected source or search." />}
        </>
      )}
    </div>
  );
}
