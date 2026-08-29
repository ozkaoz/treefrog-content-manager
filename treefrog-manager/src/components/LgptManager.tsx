import { useState } from "react";

type LgptScanResult = {
  samples: { path: string; hash: string; size: number }[];
  projects: { path: string; members: string[]; hash: string }[];
  plan: {
    entries: { source: string; destination: string; action: string; reason: string }[];
    summary: Record<string, number>;
  } | null;
};

export default function LgptManager() {
  const [samplesSource, setSamplesSource] = useState<string>("");
  const [projectsSource, setProjectsSource] = useState<string>("");
  const [activeSubTab, setActiveSubTab] = useState<"samples" | "projects">("samples");
  const [samplesResult, setSamplesResult] = useState<LgptScanResult | null>(null);
  const [projectsResult, setProjectsResult] = useState<LgptScanResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>("");
  const [filter, setFilter] = useState<"all" | "new" | "duplicate" | "conflict" | "unchanged">("all");

  async function pickFolder(setter: (v: string) => void) {
    const tauri = (window as unknown as { __TAURI__?: { dialog: { open: (opts: unknown) => Promise<string | null> } } }).__TAURI__;
    if (tauri?.dialog) {
      // @ts-ignore
      const sel = await window.__TAURI__.dialog.open({ directory: true, title: "Select LGPT folder" });
      if (typeof sel === "string") setter(sel);
    } else {
      const v = prompt("Enter folder path:");
      if (v) setter(v);
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
        // Mock for web dev
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
      <p style={{ fontSize: 12, color: "#555" }}>
        LGPT is a profile/integration within TreeFrog Content Manager (<code>lgpt/samples</code> + <code>lgpt/projects</code> via <code>lgpt.json</code>, R36SX is a target, not the manager identity). Reuses scanner, logical-unit model, archive inspector, SHA-256, conflict resolver, deployment planner, dry-run UI. No SD writes in this milestone. WAV is the explicit baseline for samples.
      </p>

      <div style={{ display: "flex", gap: 8, marginBottom: 12, borderBottom: "1px solid #ddd", paddingBottom: 8 }}>
        <button onClick={() => setActiveSubTab("samples")} style={{ background: activeSubTab === "samples" ? "#e3f2fd" : "#f5f5f5", fontWeight: activeSubTab === "samples" ? 600 : 400 }}>Samples</button>
        <button onClick={() => setActiveSubTab("projects")} style={{ background: activeSubTab === "projects" ? "#e3f2fd" : "#f5f5f5", fontWeight: activeSubTab === "projects" ? 600 : 400 }}>Projects</button>
      </div>

      {activeSubTab === "samples" && (
        <div>
          <h4>Samples — lgpt/samples (profile-driven)</h4>
          <div className="row" style={{ marginBottom: 8 }}>
            <input value={samplesSource} onChange={(e) => setSamplesSource(e.target.value)} placeholder="C:\LGPT\Samples or /path/to/samples" style={{ flex: 1, padding: "6px 8px" }} />
            <button onClick={() => pickFolder(setSamplesSource)}>Browse…</button>
            <button onClick={handleScanSamples} disabled={loading || !samplesSource}>{loading ? "Scanning…" : "Scan"}</button>
          </div>
          <p style={{ fontSize: 11, color: "#777" }}>Recursive scan, WAV baseline, SHA-256 duplicate, same-name/different-content conflict, unchanged, archive via Phase 2A, deterministic, dry-run, no SD writes.</p>
          {samplesResult && (
            <>
              <div style={{ fontSize: 12, marginBottom: 6 }}>
                <strong>Counts:</strong> {samplesResult.samples.length} samples scanned — {currentPlan?.summary.new ?? 0} new, {currentPlan?.summary.duplicate ?? 0} duplicate, {currentPlan?.summary.unchanged ?? 0} unchanged, {currentPlan?.summary.conflict ?? 0} conflict
              </div>
              <div style={{ display: "flex", gap: 6, marginBottom: 6 }}>
                <span style={{ fontSize: 11 }}>Filter:</span>
                {(["all", "new", "duplicate", "conflict", "unchanged"] as const).map((f) => (
                  <button key={f} onClick={() => setFilter(f)} style={{ padding: "2px 8px", fontSize: 11, background: filter === f ? "#e3f2fd" : "#f5f5f5" }}>{f}</button>
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
              <p style={{ fontSize: 11, color: "#2e7d32" }}>No SD writes — dry-run only. Originals never modified, temp workspace only, deterministic ordering.</p>
            </>
          )}
        </div>
      )}

      {activeSubTab === "projects" && (
        <div>
          <h4>Projects — lgpt/projects (logical units, not flattened)</h4>
          <div className="row" style={{ marginBottom: 8 }}>
            <input value={projectsSource} onChange={(e) => setProjectsSource(e.target.value)} placeholder="C:\LGPT\Projects or /path/to/projects" style={{ flex: 1, padding: "6px 8px" }} />
            <button onClick={() => pickFolder(setProjectsSource)}>Browse…</button>
            <button onClick={handleScanProjects} disabled={loading || !projectsSource}>{loading ? "Scanning…" : "Scan"}</button>
          </div>
          <p style={{ fontSize: 11, color: "#777" }}>Projects are logical units (directory + related files, not flattened). Recursive scan, project detection, duplicate/conflict/unchanged via deterministic content hash, deterministic planning, dry-run, no SD writes. If exact LGPT project structure is ambiguous, see docs/PLAN.md and lgpt.json notes.</p>
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
                      <td style={{ fontSize: 10, color: "#555" }}>{(projectsResult.projects[idx]?.members || []).join(", ") || "-"}</td>
                      <td><span className={`badge badge-${e.action === "copy" ? "copy" : e.action === "skip_duplicate" ? "skip" : "conflict"}`}>{e.action}</span></td>
                      <td style={{ fontSize: 11 }}>{e.reason}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              <p style={{ fontSize: 11, color: "#2e7d32" }}>Projects remain grouped directories; dry-run only.</p>
            </>
          )}
        </div>
      )}

      {error && <p style={{ color: "crimson", fontSize: 12 }}>{error}</p>}

      <p style={{ fontSize: 11, color: "#777", marginTop: 12 }}>
        Audio waveform/preview is a future enhancement (not in this milestone). Archive handling reuses Phase 2A <code>ArchiveHandler</code> (temp workspace, traversal/symlink/collision/expansion protections).
      </p>
    </div>
  );
}
