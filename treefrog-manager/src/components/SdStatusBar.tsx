import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Shared REAL SD status bar (point 4): shown in every content panel so the
 * true SD state (TreeFrogUI detected, capacity, free, semantic counts) is
 * visible everywhere and refreshes after every sync (bump `refreshSignal`).
 */
export default function SdStatusBar({
  sdPath,
  refreshSignal = 0,
}: {
  sdPath: string;
  refreshSignal?: number;
}) {
  const [analysis, setAnalysis] = useState<any | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!sdPath) { setAnalysis(null); return; }
    let cancelled = false;
    setLoading(true);
    invoke("analyze_target", { path: sdPath })
      .then((a: any) => { if (!cancelled) setAnalysis(a); })
      .catch(() => { if (!cancelled) setAnalysis(null); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [sdPath, refreshSignal]);

  if (!sdPath) {
    return (
      <div style={{ fontSize: 11, color: "var(--text-muted)", padding: "6px 10px", border: "1px dashed var(--border)", borderRadius: 6, marginBottom: 10 }}>
        No SD selected — go to Overview
      </div>
    );
  }

  const isTreefrog = analysis?.is_treefrog === true;
  const free = analysis?.free_bytes;
  const cap = analysis?.capacity_bytes;
  const fmt = (n?: number | null) => {
    if (n == null) return "—";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let v = n, u = 0;
    while (v >= 1024 && u < units.length - 1) { v /= 1024; u++; }
    return `${v.toFixed(u === 0 ? 0 : 1)} ${units[u]}`;
  };

  return (
    <div style={{
      display: "flex", flexWrap: "wrap", gap: 12, alignItems: "center",
      fontSize: 11, padding: "6px 10px", marginBottom: 10,
      border: `1px solid ${isTreefrog ? "var(--success)" : "var(--danger)"}`,
      borderRadius: 6, background: "var(--surface)",
      color: "var(--text)",
    }}>
      <span style={{ fontWeight: 600 }}>
        SD: {sdPath}
      </span>
      <span style={{ color: isTreefrog ? "var(--success)" : "var(--danger)" }}>
        {loading ? "checking…" : isTreefrog ? "✓ TreeFrogUI" : "✕ not TreeFrogUI"}
      </span>
      {analysis && (
        <>
          <span style={{ color: "var(--text-muted)" }}>{fmt(cap)} total</span>
          <span style={{ color: "var(--success)" }}>{fmt(free)} free</span>
          <span style={{ color: "var(--text-muted)" }}>
            {analysis.rom_count ?? 0} ROMs · {analysis.music_track_count ?? 0} music · {analysis.video_count ?? 0} videos · {analysis.bios_count ?? 0} BIOS
          </span>
        </>
      )}
    </div>
  );
}
