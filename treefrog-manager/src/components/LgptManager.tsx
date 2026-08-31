import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { pickFolder } from "../services/dialog";
import EmptyState from "./EmptyState";

type LgptScanResult = {
  samples: { path: string; hash: string; size: number }[];
  projects: { path: string; members: string[]; hash: string }[];
  plan: {
    entries: { source: string; destination: string; action: string; reason: string }[];
    summary: Record<string, number>;
  } | null;
};

type PlanEntry = {
  source: string;
  destination: string;
  action: string;
  reason: string;
  content_type?: string;
  size?: number;
};

type Plan = {
  summary: { new: number; changed: number; unchanged: number; duplicate_content: number; conflicts: number; deletions: number; manual_review?: number; unsupported_archive?: number };
  entries: PlanEntry[];
  warnings: string[];
};

export default function LgptManager({ 
  globalSdPath,
  onSamplesSourceChange,
  onProjectsSourceChange,
  onPlanChange,
  onNext,
  visible
}: { 
  globalSdPath: string;
  onSamplesSourceChange?: (v: string) => void;
  onProjectsSourceChange?: (v: string) => void;
  onPlanChange?: (plan: Plan | null) => void;
  onNext?: () => void;
  visible?: boolean;
}) {
  const [samplesSource, setSamplesSource] = useState<string>("");
  const [projectsSource, setProjectsSource] = useState<string>("");
  const [activeSubTab, setActiveSubTab] = useState<"samples" | "projects">("samples");
  const [samplesResult, setSamplesResult] = useState<LgptScanResult | null>(null);
  const [projectsResult, setProjectsResult] = useState<LgptScanResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>("");
  const [filter, setFilter] = useState<"all" | "new" | "duplicate" | "conflict" | "unchanged">("all");
  const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set());

  async function handlePickSamples() {
    try {
      const sel = await pickFolder({ title: "Select LGPT Samples folder (lgpt/samples)" });
      if (sel) {
        setSamplesSource(sel);
        onSamplesSourceChange?.(sel);
      }
    } catch (e) {
      setError(String(e));
    }
  }

  async function handlePickProjects() {
    try {
      const sel = await pickFolder({ title: "Select LGPT Projects folder (lgpt/projects)" });
      if (sel) {
        setProjectsSource(sel);
        onProjectsSourceChange?.(sel);
      }
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleScanSamples() {
    if (!samplesSource) { setError("Select Samples source folder"); return; }
    if (!globalSdPath) { setError("No hay SD seleccionada — ve a Overview"); return; }
    setLoading(true); setError("");
    try {
      const res = (await invoke("lgpt_scan_samples", { samplesSource, sdPath: globalSdPath })) as LgptScanResult;
      setSamplesResult(res);
      if (res.plan?.entries) setSelectedFiles(prev => new Set([...prev, ...res.plan!.entries.map(e => e.source)]));
    } catch (e) { setError(String(e)); } finally { setLoading(false); }
  }

  async function handleScanProjects() {
    if (!projectsSource) { setError("Select Projects source folder"); return; }
    if (!globalSdPath) { setError("No hay SD seleccionada — ve a Overview"); return; }
    setLoading(true); setError("");
    try {
      const res = (await invoke("lgpt_scan_projects", { projectsSource, sdPath: globalSdPath })) as LgptScanResult;
      setProjectsResult(res);
      if (res.plan?.entries) setSelectedFiles(prev => new Set([...prev, ...res.plan!.entries.map(e => e.source)]));
    } catch (e) { setError(String(e)); } finally { setLoading(false); }
  }

  const toggleFileSelection = (id: string) => {
    setSelectedFiles(prev => {
      const ns = new Set(prev);
      if (ns.has(id)) ns.delete(id); else ns.add(id);
      return ns;
    });
  };
  const toggleAllSamples = (checked: boolean) => {
    const entries = samplesResult?.plan?.entries || [];
    setSelectedFiles(prev => {
      const ns = new Set(prev);
      if (checked) entries.forEach(e => ns.add(e.source));
      else entries.forEach(e => ns.delete(e.source));
      return ns;
    });
  };
  const toggleAllProjects = (checked: boolean) => {
    const entries = projectsResult?.plan?.entries || [];
    setSelectedFiles(prev => {
      const ns = new Set(prev);
      if (checked) entries.forEach(e => ns.add(e.source));
      else entries.forEach(e => ns.delete(e.source));
      return ns;
    });
  };

  async function handleScanBoth() {
    setError("");
    if (samplesSource) await handleScanSamples();
    if (projectsSource) await handleScanProjects();
  }

  const lastScanKey = useRef("");
  useEffect(() => {
    const key = `${samplesSource}|${projectsSource}|${globalSdPath}`;
    if (visible && (samplesSource || projectsSource) && globalSdPath && key !== lastScanKey.current) {
      lastScanKey.current = key;
      handleScanBoth();
    }
  }, [visible, samplesSource, projectsSource, globalSdPath]);

  useEffect(() => {
    const allEntries: PlanEntry[] = [];
    
    if (samplesResult?.plan?.entries) {
      const filtered = samplesResult.plan.entries.filter(e => selectedFiles.has(e.source));
      const effective = selectedFiles.size > 0 ? filtered : samplesResult.plan.entries;
      allEntries.push(...effective.map(e => ({
        ...e,
        content_type: 'lgpt/sample',
      })));
    }
    
    if (projectsResult?.plan?.entries) {
      const filtered = projectsResult.plan.entries.filter(e => selectedFiles.has(e.source));
      const effective = selectedFiles.size > 0 ? filtered : projectsResult.plan.entries;
      allEntries.push(...effective.map(e => ({
        ...e,
        content_type: 'lgpt/project',
      })));
    }
    
    if (allEntries.length === 0) {
      onPlanChange?.(null);
      return;
    }
    
    const summary = {
      new: allEntries.filter(e => e.action === 'copy' || e.action === 'extract').length,
      changed: 0,
      unchanged: allEntries.filter(e => e.action === 'skip_unchanged').length,
      duplicate_content: allEntries.filter(e => e.action === 'skip_duplicate').length,
      conflicts: allEntries.filter(e => e.action === 'conflict').length,
      deletions: 0,
      manual_review: allEntries.filter(e => e.action === 'manual_review').length,
      unsupported_archive: 0,
    };
    
    const plan: Plan = { entries: allEntries, summary, warnings: [] };
    onPlanChange?.(plan);
  }, [samplesResult, projectsResult, selectedFiles, onPlanChange]);

  const currentResult = activeSubTab === "samples" ? samplesResult : projectsResult;
  const currentPlan = currentResult?.plan;

  return (
    <div className="card">
      <h3>LGPT — Samples & Projects (Little Piggy Tracker)</h3>
      <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 8 }}>
        <label style={{ fontSize: 13, fontWeight: 600 }}>LGPT Samples source folder</label>
        <div className="row" style={{ alignItems: "stretch" }}>
          <div
            style={{
              flex: 1,
              padding: "8px 10px",
              border: "1px solid var(--border)",
              borderRadius: 6,
              background: "var(--input)",
              color: samplesSource ? "var(--text)" : "var(--text-muted)",
              fontSize: 13,
              minHeight: 36,
              display: "flex",
              alignItems: "center",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
            title={samplesSource || "No folder selected — e.g., D:\\LGPT\\samples"}
          >
            {samplesSource || "No folder selected — e.g., D:\\LGPT\\samples"}
          </div>
          <button onClick={handlePickSamples}>Browse</button>
        </div>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 8 }}>
        <label style={{ fontSize: 13, fontWeight: 600 }}>LGPT Projects source folder</label>
        <div className="row" style={{ alignItems: "stretch" }}>
          <div
            style={{
              flex: 1,
              padding: "8px 10px",
              border: "1px solid var(--border)",
              borderRadius: 6,
              background: "var(--input)",
              color: projectsSource ? "var(--text)" : "var(--text-muted)",
              fontSize: 13,
              minHeight: 36,
              display: "flex",
              alignItems: "center",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
            title={projectsSource || "No folder selected — e.g., D:\\LGPT\\projects"}
          >
            {projectsSource || "No folder selected — e.g., D:\\LGPT\\projects"}
          </div>
          <button onClick={handlePickProjects}>Browse</button>
        </div>
      </div>

      <p style={{ fontSize: 11, color: "var(--text-muted)", margin: "0 0 8px 0" }}>
        SD destino: {globalSdPath || "—"} — la app copiará automáticamente a lgpt/samples/ y lgpt/projects/ según el tipo de contenido.
      </p>

      <div className="row">
        <button className="primary" onClick={() => { lastScanKey.current = `${samplesSource}|${projectsSource}|${globalSdPath}`; handleScanBoth(); }} disabled={loading || (!samplesSource && !projectsSource)}>
          {loading ? "Scanning…" : "Scan LGPT"}
        </button>
        <button
          onClick={() => { setSamplesResult(null); setProjectsResult(null); onPlanChange?.(null); }}
          disabled={!samplesResult && !projectsResult}
        >
          Clear
        </button>
        <button onClick={() => onNext?.()} style={{ marginLeft: "auto" }}>
          Omitir → SD Card
        </button>
        <button className="primary" onClick={() => onNext?.()} disabled={!samplesSource && !projectsSource && !samplesResult && !projectsResult}>
          Continuar a SD Card →
        </button>
      </div>

      {error && <div className="status-error" style={{ fontSize: 12, marginTop: 8 }}>{error}</div>}

      <div className="nav" style={{ margin: "12px 0" }}>
        <button onClick={() => setActiveSubTab("samples")} className={activeSubTab === "samples" ? "active" : ""}>Samples</button>
        <button onClick={() => setActiveSubTab("projects")} className={activeSubTab === "projects" ? "active" : ""}>Projects</button>
      </div>

      {activeSubTab === "samples" && (
        <div>
          <h4>Samples — lgpt/samples (profile-driven)</h4>
          <p style={{ fontSize: 11, color: "var(--text-muted)" }}>Recursive scan, WAV baseline, SHA-256 duplicate, same-name/different-content conflict, unchanged, archive via Phase 2A, deterministic, dry-run, no SD writes.</p>
          {!samplesResult && !loading && <EmptyState kind="empty" title="No scan yet" description="Select a Samples folder and press Scan. Click Browse to open the native Windows folder picker." />}
          {loading && activeSubTab === "samples" && <EmptyState kind="loading" title="Scanning…" />}
          {samplesResult && (
            <>
              <div style={{ fontSize: 12, marginBottom: 6 }}>
                <strong>Counts:</strong> {samplesResult.samples.length} samples scanned — {currentPlan?.summary.new ?? 0} new, {currentPlan?.summary.duplicate ?? 0} duplicate, {currentPlan?.summary.unchanged ?? 0} unchanged, {currentPlan?.summary.conflict ?? 0} conflict
              </div>
              <div style={{ display: "flex", gap: 6, marginBottom: 6 }}>
                <span style={{ fontSize: 11, color: "var(--text-muted)" }}>Filter:</span>
                {(["all", "new", "duplicate", "conflict", "unchanged"] as const).map((f) => (
                  <button key={f} onClick={() => setFilter(f)} style={{ padding: "2px 8px", fontSize: 11 }} className={filter === f ? "active" : ""}>{f}</button>
                ))}
              </div>
              <table>
                <thead>
                  <tr><th><input type="checkbox" checked={samplesResult?.plan ? samplesResult.plan.entries.every(e => selectedFiles.has(e.source)) : false} onChange={(e) => toggleAllSamples(e.target.checked)} /></th><th>Source</th><th>Destination</th><th>Action</th><th>Reason</th></tr>
                </thead>
                <tbody>
                  {(currentPlan?.entries.filter((e) => {
                    if (filter === "all") return true;
                    if (filter === "new") return e.action === "copy" || e.action === "extract";
                    if (filter === "duplicate") return e.action === "skip_duplicate";
                    if (filter === "conflict") return e.action === "conflict";
                    if (filter === "unchanged") return e.action === "skip_unchanged";
                    return true;
                  }) || []).map((e, idx) => (
                    <tr key={idx} style={{ opacity: selectedFiles.has(e.source) ? 1 : 0.5 }}>
                      <td><input type="checkbox" checked={selectedFiles.has(e.source)} onChange={() => toggleFileSelection(e.source)} /></td>
                      <td style={{ fontSize: 11, maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis" }} title={e.source}>{e.source}</td>
                      <td style={{ fontSize: 11 }}>{e.destination}</td>
                      <td><span className={`badge badge-${e.action === "copy" ? "copy" : e.action === "skip_duplicate" ? "skip" : e.action === "conflict" ? "conflict" : "skip"}`}>{e.action}</span></td>
                      <td style={{ fontSize: 11 }}>{e.reason}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {currentPlan?.entries.length === 0 && <EmptyState kind="empty" title="No files found" description="No WAV samples found in the selected folder." />}
            </>
          )}
        </div>
      )}

      {activeSubTab === "projects" && (
        <div>
          <h4>Projects — lgpt/projects (logical units, not flattened)</h4>
          <p style={{ fontSize: 11, color: "var(--text-muted)" }}>Projects are logical units (directory + related files, not flattened). Recursive scan, project detection, duplicate/conflict/unchanged via deterministic content hash, deterministic planning, dry-run, no SD writes.</p>
          {!projectsResult && !loading && <EmptyState kind="empty" title="No scan yet" description="Select a Projects folder and press Scan. Click Browse to open the native Windows folder picker." />}
          {loading && activeSubTab === "projects" && <EmptyState kind="loading" title="Scanning…" />}
          {projectsResult && (
            <>
              <div style={{ fontSize: 12, marginBottom: 6 }}>
                <strong>Counts:</strong> {projectsResult.projects.length} projects — {currentPlan?.summary.new ?? 0} new, {currentPlan?.summary.duplicate ?? 0} duplicate, {currentPlan?.summary.conflict ?? 0} conflict
              </div>
              <table>
                <thead>
                  <tr><th><input type="checkbox" checked={projectsResult?.plan ? projectsResult.plan.entries.every(e => selectedFiles.has(e.source)) : false} onChange={(e) => toggleAllProjects(e.target.checked)} /></th><th>Project</th><th>Destination</th><th>Members</th><th>Action</th><th>Reason</th></tr>
                </thead>
                <tbody>
                  {(currentPlan?.entries || []).map((e, idx) => (
                    <tr key={idx} style={{ opacity: selectedFiles.has(e.source) ? 1 : 0.5 }}>
                      <td><input type="checkbox" checked={selectedFiles.has(e.source)} onChange={() => toggleFileSelection(e.source)} /></td>
                      <td style={{ fontSize: 11 }}>{e.source.split(/[\\/]/).pop()}</td>
                      <td style={{ fontSize: 11 }}>{e.destination}</td>
                      <td style={{ fontSize: 10, color: "var(--text-muted)" }}>{(projectsResult.projects[idx]?.members || []).join(", ") || "-"}</td>
                      <td><span className={`badge badge-${e.action === "copy" ? "copy" : e.action === "skip_duplicate" ? "skip" : "conflict"}`}>{e.action}</span></td>
                      <td style={{ fontSize: 11 }}>{e.reason}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {currentPlan?.entries.length === 0 && <EmptyState kind="empty" title="No projects found" description="No LGPT projects (directory with project.lgpt/lgptsav.dat) found." />}
            </>
          )}
        </div>
      )}

      {error && <div className="status-error" style={{ fontSize: 12, marginTop: 8 }}>{error}</div>}

      <p style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 12 }}>
        Audio waveform/preview is a future enhancement (not in this milestone). Archive handling reuses Phase 2A <code>ArchiveHandler</code> (temp workspace, traversal/symlink/collision/expansion protections).
      </p>

      <div className="row" style={{ marginTop: 16 }}>
        <button onClick={() => onNext?.()} style={{ marginLeft: "auto" }}>
          Omitir → SD Card
        </button>
        <button className="primary" onClick={() => onNext?.()} disabled={!samplesResult && !projectsResult}>
          Continuar a SD Card →
        </button>
      </div>
    </div>
  );
}
