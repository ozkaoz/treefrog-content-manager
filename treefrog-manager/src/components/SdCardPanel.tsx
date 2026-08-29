import { useState } from "react";
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

export default function SdCardPanel({ sdPath, onChange }: { sdPath: string; onChange: (v: string) => void }) {
  const [analysis, setAnalysis] = useState<TargetAnalysis | null>(null);
  const [space, setSpace] = useState<SpaceInfo | null>(null);
  const [plan, setPlan] = useState<any | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [sourcePath, setSourcePath] = useState("");

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
      const tauri = (window as unknown as { __TAURI__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI__;
      if (tauri) {
        const res = (await tauri.invoke("analyze_target", { path: target })) as TargetAnalysis;
        setAnalysis(res);
      } else {
        // Mock for web dev
        setAnalysis({
          path: target,
          volume: { path: target, label: "TREEFROG", filesystem: "exFAT", total_bytes: 64 * 1024 ** 3, free_bytes: 42 * 1024 ** 3, removable: true, accessible: true, error: null },
          status: "valid",
          is_treefrog: true,
          is_incomplete: false,
          markers_found: ["cubegm", "roms", "lgpt"],
          markers_missing: [],
          lgpt_detected: true,
          rom_dirs: ["GBA", "SFC", "PS"],
          media_dirs: ["music", "videos"],
          bios_dirs: ["cubegm/bios"],
          lgpt_dirs: ["lgpt/samples", "lgpt/projects"],
          existing_count: 1234,
          total_size: 8 * 1024 ** 3,
          free_bytes: 42 * 1024 ** 3,
          capacity_bytes: 64 * 1024 ** 3,
          filesystem: "exFAT",
          label: "TREEFROG",
          errors: [],
        });
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function handleDryRun() {
    if (!sourcePath) {
      setError("Select a source folder first (Overview → Games source)");
      return;
    }
    if (!sdPath) {
      setError("Select an SD target first");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const tauri = (window as unknown as { __TAURI__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI__;
      if (tauri) {
        const res = (await tauri.invoke("dry_run_with_target", { sourcePath, sdPath })) as any;
        setPlan(res);
        setSpace(res.space);
        setAnalysis(res.target);
      } else {
        setPlan({ summary: { unchanged: 10, new: 5, changed: 1, duplicate_content: 2, conflicts: 1, deletions: 0 }, entries: [] });
        setSpace({ bytes_to_copy: 100 * 1024 ** 2, bytes_to_extract: 50 * 1024 ** 2, bytes_to_generate: 0, bytes_to_skip: 200 * 1024 ** 2, required_bytes: 150 * 1024 ** 2, available_bytes: 42 * 1024 ** 3, status: "ok" });
      }
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
        <label style={{ fontSize: 13, fontWeight: 600 }}>SD target</label>
        <div className="row" style={{ alignItems: "stretch" }}>
          <div
            style={{
              flex: 1,
              padding: "8px 10px",
              border: "1px solid var(--border)",
              borderRadius: 6,
              background: "var(--input)",
              color: sdPath ? "var(--text)" : "var(--text-muted)",
              fontSize: 13,
              minHeight: 36,
              display: "flex",
              alignItems: "center",
            }}
          >
            {sdPath || "No SD selected — e.g., E:\\"}
          </div>
          <button onClick={handlePick}>Select SD</button>
          <button onClick={() => handleAnalyze()} disabled={!sdPath || loading}>
            {loading ? "Analyzing…" : "Analyze"}
          </button>
        </div>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <input
            value={sourcePath}
            onChange={(e) => setSourcePath(e.target.value)}
            placeholder="Source for dry-run (e.g., C:\My ROM Library) — or set in Overview"
            style={{ flex: 1, fontSize: 12, opacity: 0.85 }}
          />
          <button onClick={handleDryRun} disabled={loading} title="Source + Target → dry-run (read-only)">
            Dry-run with target
          </button>
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
        <div style={{ display: "flex", gap: 8 }}>
          <button
            onClick={async () => {
              if (!sdPath || !sourcePath) {
                setError("Select source and SD target first, then Analyze and Dry-run");
                return;
              }
              if (!plan || !space) {
                setError("Run Dry-run with target first to validate space and collisions");
                return;
              }
              if (space.status === "insufficient_space") {
                setError("Not enough space on target — free more space or reduce source");
                return;
              }
              if (!analysis?.is_treefrog) {
                setError("Target is not a valid TreeFrogUI SD — check markers");
                return;
              }
              const confirmed = confirm(`Sync ${plan.summary.new} new + ${plan.summary.changed} changed to ${sdPath}?\n\nThis will copy files to the SD card (staging + atomic rename). Continue?`);
              if (!confirmed) return;
              setLoading(true);
              setError("");
              try {
                const tauri = (window as unknown as { __TAURI__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI__;
                if (tauri) {
                  const res = (await tauri.invoke("deploy_to_sd", { sourcePath, sdPath })) as any;
                  alert(`Sync complete: ${res.deployed} deployed, ${res.skipped} skipped, ${res.failed} failed.\n${res.warnings?.join("\n") || ""}`);
                  // Re-analyze after sync
                  await handleAnalyze();
                } else {
                  alert("Deploy not available in web preview");
                }
              } catch (e) {
                setError(String(e));
              } finally {
                setLoading(false);
              }
            }}
            disabled={loading || !plan || space?.status === "insufficient_space"}
            className="primary"
            title={plan ? `Sync ${plan.summary.new} new to ${sdPath}` : "Run Dry-run first"}
          >
            {loading ? "Syncing…" : "Sync to SD"}
          </button>
          <span style={{ fontSize: 11, color: "var(--text-muted)", alignSelf: "center" }}>
            {plan ? `Ready to sync ${plan.summary.new} new, ${plan.summary.duplicate_content} duplicate skipped` : "Run Dry-run to enable Sync"}
          </span>
        </div>
        <div style={{ fontSize: 11, color: "var(--text-muted)" }}>
          Progress: {plan ? `${plan.summary.new} new, ${plan.summary.unchanged} unchanged, ${plan.summary.conflicts} conflicts` : "—"} | Staging: copy to <code>.treefrog_staging_*.tmp</code> then atomic <code>rename</code>, resume on interrupt, no silent overwrite. Large libraries (&gt;10k files) are limited per job and will show `manual_review`.
        </div>
      </div>

      <MiniScraper sdPath={sdPath} />
    </div>
  );
}
