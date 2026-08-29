import { useState } from "react";
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

export default function LgptManager({ onNext }: { onNext?: () => void }) {
  const [samplesSource, setSamplesSource] = useState<string>("");
  const [projectsSource, setProjectsSource] = useState<string>("");
  const [activeSubTab, setActiveSubTab] = useState<"samples" | "projects">("samples");
  const [samplesResult, setSamplesResult] = useState<LgptScanResult | null>(null);
  const [projectsResult, setProjectsResult] = useState<LgptScanResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>("");
  const [filter, setFilter] = useState<"all" | "new" | "duplicate" | "conflict" | "unchanged">("all");

  async function handlePickSamples() {
    try {
      const sel = await pickFolder({ title: "Select LGPT Samples folder (lgpt/samples)" });
      if (sel) setSamplesSource(sel);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handlePickProjects() {
    try {
      const sel = await pickFolder({ title: "Select LGPT Projects folder (lgpt/projects)" });
      if (sel) setProjectsSource(sel);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleScanSamples() {
    if (!samplesSource) { setError("Select Samples source folder"); return; }
    setLoading(true);
    setError("");
    try {
      const tauri = (window as unknown as { __TAURI__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI__;
      if (tauri) {
        const res = await tauri.invoke("lgpt_scan_samples", { samplesSource }) as LgptScanResult;
        setSamplesResult(res);
      } else {
        setSamplesResult({
          samples: [
            { path: `${samplesSource}/kick.wav`, hash: "aaa111", size: 10244 },
            { path: `${samplesSource}/snare.wav`, hash: "bbb222", size: 20480 },
            { path: `${samplesSource}/duplicate/kick-copy.wav`, hash: "aaa111", size: 10244 },
          ],
          projects: [],
          plan: {
            entries: [
              { source: `${samplesSource}/kick.wav`, destination: "lgpt/samples/kick.wav", action: "copy", reason: "new sample" },
              { source: `${samplesSource}/snare.wav`, destination: "lgpt/samples/snare.wav", action: "copy", reason: "new sample" },
              { source: `${samplesSource}/duplicate/kick-copy.wav`, destination: "lgpt/samples/kick-copy.wav", action: "skip_duplicate", reason: "identical SHA-256 -> duplicate" },
            ],
            summary: { new: 2, duplicate: 1, unchanged: 0, conflict: 0 },
          },
        });
      }
    } catch (e) { setError(String(e)); } finally { setLoading(false); }
  }

  async function handleScanProjects() {
    if (!projectsSource) { setError("Select Projects source folder"); return; }
    setLoading(true);
    setError("");
    try {
      const tauri = (window as unknown as { __TAURI__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI__;
      if (tauri) {
        const res = await tauri.invoke("lgpt_scan_projects", { projectsSource }) as LgptScanResult;
        setProjectsResult(res);
      } else {
        setProjectsResult({
          samples: [],
          projects: [
            { path: `${projectsSource}/ProjectA`, members: ["project.lgpt", "sample1.wav"], hash: "ccc333" },
            { path: `${projectsSource}/ProjectB`, members: ["project.lgpt"], hash: "ddd444" },
          ],
          plan: {
            entries: [
              { source: `${projectsSource}/ProjectA`, destination: "lgpt/projects/ProjectA", action: "copy", reason: "new project logical unit" },
              { source: `${projectsSource}/ProjectB`, destination: "lgpt/projects/ProjectB", action: "skip_duplicate", reason: "identical project hash" },
            ],
            summary: { new: 1, duplicate: 1 },
          },
        });
      }
    } catch (e) { setError(String(e)); } finally { setLoading(false); }
  }

  const currentResult = activeSubTab === "samples" ? samplesResult : projectsResult;
  const currentPlan = currentResult?.plan;

  return (
    <div className="card">
      <h3>LGPT — Samples and Projects</h3>
      <p style={{ fontSize: 12, color: "var(--text-muted)" }}>
        LGPT is a profile/integration within TreeFrog Content Manager (<code>lgpt/samples</code> + <code>lgpt/projects</code> via <code>lgpt.json</code>, R36SX is a target, not the manager identity). Reuses scanner, logical-unit model, archive inspector, SHA-256, conflict resolver, deployment planner, dry-run UI. No SD writes in this milestone. WAV is the explicit baseline for samples.
      </p>

      <div className="nav" style={{ marginBottom: 12 }}>
        <button onClick={() => setActiveSubTab("samples")} className={activeSubTab === "samples" ? "active" : ""}>Samples</button>
        <button onClick={() => setActiveSubTab("projects")} className={activeSubTab === "projects" ? "active" : ""}>Projects</button>
      </div>

      {activeSubTab === "samples" && (
        <div>
          <h4>Samples — lgpt/samples (profile-driven)</h4>
          <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 8 }}>
            <label style={{ fontSize: 13, fontWeight: 600 }}>LGPT Samples source folder</label>
            <div className="row" style={{ alignItems: "stretch" }}>
              <div style={{ flex: 1, padding: "8px 10px", border: "1px solid var(--border)", borderRadius: 6, background: "var(--input)", color: samplesSource ? "var(--text)" : "var(--text-muted)", fontSize: 13, minHeight: 36, display: "flex", alignItems: "center" }}>
                {samplesSource || "No folder selected"}
              </div>
              <button onClick={handlePickSamples}>Browse</button>
              <button onClick={handleScanSamples} disabled={loading || !samplesSource} className="primary">{loading ? "Scanning…" : "Scan"}</button>
            </div>
          </div>
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
                  <tr><th>Source</th><th>Destination</th><th>Action</th><th>Reason</th></tr>
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
                    <tr key={idx}>
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
          <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 8 }}>
            <label style={{ fontSize: 13, fontWeight: 600 }}>LGPT Projects source folder</label>
            <div className="row" style={{ alignItems: "stretch" }}>
              <div style={{ flex: 1, padding: "8px 10px", border: "1px solid var(--border)", borderRadius: 6, background: "var(--input)", color: projectsSource ? "var(--text)" : "var(--text-muted)", fontSize: 13, minHeight: 36, display: "flex", alignItems: "center" }}>
                {projectsSource || "No folder selected"}
              </div>
              <button onClick={handlePickProjects}>Browse</button>
              <button onClick={handleScanProjects} disabled={loading || !projectsSource} className="primary">{loading ? "Scanning…" : "Scan"}</button>
            </div>
          </div>
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
                  <tr><th>Project</th><th>Destination</th><th>Members</th><th>Action</th><th>Reason</th></tr>
                </thead>
                <tbody>
                  {(currentPlan?.entries || []).map((e, idx) => (
                    <tr key={idx}>
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
