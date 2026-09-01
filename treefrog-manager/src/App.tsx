import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Header from "./components/Header";
import EmptyState from "./components/EmptyState";
import About from "./components/About";
import SdCardPanel from "./components/SdCardPanel";
import GamesPanel from "./components/GamesPanel";
import MusicPanel from "./components/MusicPanel";
import VideosPanel from "./components/VideosPanel";
import SettingsPanel from "./components/SettingsPanel";
import BiosManager from "./components/BiosManager";
import LgptManager from "./components/LgptManager";
import { UpdateChecker } from "./components/UpdateChecker";
import { initTheme } from "./services/theme";

type Tab = "overview" | "games" | "music" | "videos" | "bios" | "lgpt" | "sdcard" | "settings" | "about";

type VolumeInfo = {
  path: string;
  label?: string | null;
  filesystem?: string | null;
  total_bytes?: number | null;
  free_bytes?: number | null;
  removable?: boolean | null;
  accessible: boolean;
};

type TargetAnalysis = {
  path: string;
  status: string;
  is_treefrog: boolean;
  label?: string | null;
  filesystem?: string | null;
  free_bytes?: number | null;
  capacity_bytes?: number | null;
  volume: VolumeInfo;
  existing_count: number;
  rom_dirs: string[];
  media_dirs: string[];
  bios_dirs: string[];
  lgpt_dirs: string[];
  total_size: number;
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

export default function App() {
  // Global SD selection (auto-detected, single source of truth)
  const [sdPath, setSdPath] = useState<string>("");
  const [volumes, setVolumes] = useState<VolumeInfo[]>([]);
  const [sdAnalysis, setSdAnalysis] = useState<TargetAnalysis | null>(null);

  // Global sources per content type (session state; panels own their scan and
  // report here for overview display — deploy uses the panel PLANS, not these)
  const [_gamesSource, setGamesSource] = useState("");
  const [_musicSource, setMusicSource] = useState("");
  const [_videosSource, setVideosSource] = useState("");
  const [_biosSource, setBiosSource] = useState("");
  const [_lgptSamplesSource, setLgptSamplesSource] = useState("");
  const [_lgptProjectsSource, setLgptProjectsSource] = useState("");

  // Individual plans per content type
  const [gamesPlan, setGamesPlan] = useState<Plan | null>(null);
  const [musicPlan, setMusicPlan] = useState<Plan | null>(null);
  const [videosPlan, setVideosPlan] = useState<Plan | null>(null);
  const [_biosPlan, setBiosPlan] = useState<Plan | null>(null);
  const [lgptPlan, setLgptPlan] = useState<Plan | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [_syncResult, setSyncResult] = useState<any | null>(null);
  const [activeTab, setActiveTab] = useState<Tab>("overview");
  const [systemOverrides, setSystemOverrides] = useState<Record<string, string>>({});
  const [biosPlanEntries, setBiosPlanEntries] = useState<BiosPlanEntry[]>([]);

  interface BiosPlanEntry {
    source: string;
    destination: string;
    action: string;
    reason: string;
    content_type: string;
    is_bios: boolean;
  }

  // Build aggregated plan from all individual plans (BIOS separate, not via planner)
  const globalPlan = useMemo(() => {
    const allEntries = [
      ...(gamesPlan?.entries || []),
      ...(musicPlan?.entries || []),
      ...(videosPlan?.entries || []),
      ...(lgptPlan?.entries || []),
    ];
    
    if (allEntries.length === 0) return null;
    
    const summary = {
      new: allEntries.filter(e => e.action === 'copy' || e.action === 'extract').length,
      changed: allEntries.filter(e => e.action === 'convert_then_copy').length,
      unchanged: allEntries.filter(e => e.action === 'skip_unchanged').length,
      duplicate_content: allEntries.filter(e => e.action === 'skip_duplicate').length,
      conflicts: allEntries.filter(e => e.action === 'conflict').length,
      deletions: 0,
      manual_review: allEntries.filter(e => e.action === 'manual_review').length,
      unsupported_archive: allEntries.filter(e => e.action === 'unsupported_archive').length,
    };
    
    return { entries: allEntries, summary, warnings: [] };
  }, [gamesPlan, musicPlan, videosPlan, lgptPlan]);

  const combinedPlan = useMemo(() => {
    if (!globalPlan && biosPlanEntries.length === 0) return null;
    if (!globalPlan) {
      const allEntries = biosPlanEntries.map(bios => ({
        source: bios.source,
        destination: bios.destination,
        action: bios.action,
        reason: bios.reason,
        content_type: bios.content_type,
        hash: null,
        source_hash: null,
        destination_hash: null,
        size: 0,
        group: null,
        members: null,
        default_action: bios.action,
        resolution: 'copy',
        resolved_action: bios.action,
        original_destination: null,
      }));
      return {
        entries: allEntries,
        summary: {
          new: allEntries.length,
          unchanged: 0,
          changed: 0,
          duplicate_content: 0,
          conflicts: 0,
          deletions: 0,
          manual_review: 0,
          unsupported_archive: 0,
          bios_count: biosPlanEntries.length,
        },
        warnings: [],
      } as any;
    }
    const allEntries = [
      ...globalPlan.entries,
      ...biosPlanEntries.map(bios => ({
        source: bios.source,
        destination: bios.destination,
        action: bios.action,
        reason: bios.reason,
        content_type: bios.content_type,
        hash: null,
        source_hash: null,
        destination_hash: null,
        size: 0,
        group: null,
        members: null,
        default_action: bios.action,
        resolution: 'copy',
        resolved_action: bios.action,
        original_destination: null,
      })),
    ];
    return {
      ...globalPlan,
      entries: allEntries,
      summary: {
        ...globalPlan.summary,
        bios_count: biosPlanEntries.length,
      },
    } as any;
  }, [globalPlan, biosPlanEntries]);

  // Calculate global space from aggregated plan — using EFFECTIVE actions
  // (resolved_action || action), matching the backend's space model exactly.
  const globalSpace = useMemo(() => {
    const planForSpace = combinedPlan || globalPlan;
    if (!planForSpace || !sdAnalysis) return null;

    let to_copy = 0;
    let to_extract = 0;
    let to_generate = 0;
    let to_skip = 0;

    for (const e of planForSpace.entries) {
      const size = (e as any).size || 0;
      const eff = e.resolved_action || e.action; // effective action (backend parity)
      if (eff === "copy" || eff === "replace") to_copy += size;
      else if (eff === "extract") to_extract += size;
      else if (eff === "convert_then_copy") to_generate += size;
      else if (["skip", "skip_unchanged", "skip_duplicate"].includes(eff)) to_skip += size;
    }

    const required = to_copy + to_extract + to_generate;
    const available = (sdAnalysis as any).free_bytes || 0;
    const status = required > available ? "insufficient_space" : "ok";

    return {
      bytes_to_copy: to_copy,
      bytes_to_extract: to_extract,
      bytes_to_generate: to_generate,
      bytes_to_skip: to_skip,
      required_bytes: required,
      available_bytes: available,
      status,
    };
  }, [combinedPlan, globalPlan, sdAnalysis]);

  useEffect(() => {
    const cleanup = initTheme();
    return cleanup;
  }, []);

  useEffect(() => {
    // Limpiar cualquier estado persistente
    localStorage.clear();
    sessionStorage.clear();
    
    // Llamar al backend para resetear estado
    invoke('reset_app_state').catch(console.error);
    
    // Resetear estado local
    setSdPath('');
    setSdAnalysis(null);
    setGamesSource('');
    setMusicSource('');
    setVideosSource('');
    setBiosSource('');
    setLgptSamplesSource('');
    setLgptProjectsSource('');
    setGamesPlan(null);
    setMusicPlan(null);
    setVideosPlan(null);
    setBiosPlan(null);
    setLgptPlan(null);
    setBiosPlanEntries([]);
    setError('');
    setSyncResult(null);
  }, []);

  // Auto-detect SD on mount + polling for arrival/removal (Rufus-like, no A-Z assumption)
  useEffect(() => {
    let interval: number | null = null;
    let mounted = true;
    let lastVolumes: string[] = [];

    async function doDetect() {
      try {
        let vols: VolumeInfo[] = [];
        try {
          vols = (await invoke("list_volumes")) as VolumeInfo[];
          // Filter only volumes con letra de unidad y accesibles
          vols = vols.filter(v => v.path && !v.path.startsWith('\\\\?\\') && v.accessible !== false && v.path.length >= 2 && v.path[1] === ':');
        } catch (e) {
          console.warn("list_volumes failed", e);
        }

        if (vols.length === 0) {
          const candidates = ["G:\\", "E:\\", "F:\\", "D:\\", "H:\\", "I:\\", "J:\\", "K:\\", "L:"];
          for (const cand of candidates) {
            try {
              const a = (await invoke("analyze_target", { path: cand })) as TargetAnalysis;
              // NEVER accept fixed/local drives as SD candidates
              if (a.status !== "inaccessible" && a.volume?.removable === true) {
                vols.push({ path: cand, label: a.label || null, filesystem: a.filesystem || null, total_bytes: a.capacity_bytes, free_bytes: a.free_bytes, removable: true, accessible: true });
              }
            } catch {}
          }
        }

        if (!mounted) return;

        const seen = new Set<string>();
        vols = vols.filter((v) => {
          if (seen.has(v.path)) return false;
          seen.add(v.path);
          return true;
        });

        const volKey = vols.map((v) => `${v.path}:${v.accessible}:${v.label}`).join("|");
        if (volKey !== lastVolumes.join("|")) {
          lastVolumes = vols.map((v) => `${v.path}:${v.accessible}:${v.label}`);
          setVolumes(vols);
        } else {
          if (vols.length !== volumes.length) setVolumes(vols);
        }

        if (sdPath) {
          const stillPresent = vols.some((v) => v.path === sdPath && v.accessible);
          if (!stillPresent) {
            try {
              const a = (await invoke("analyze_target", { path: sdPath })) as TargetAnalysis;
              if (a.status === "inaccessible" || !a.volume.accessible) {
                setSdAnalysis({ ...a, status: "inaccessible" } as any);
                setError(`SD disconnected: ${sdPath} is no longer accessible`);
              }
            } catch {
              setSdAnalysis(null);
              setError(`SD disconnected: ${sdPath} is no longer accessible`);
            }
          }
        }

        let shouldAutoSelect = !sdPath || !sdAnalysis?.is_treefrog || !vols.some((v) => v.path === sdPath && v.accessible);
        if (shouldAutoSelect) {
          for (const v of vols) {
            if (!v.accessible) continue;
            try {
              const analysis = (await invoke("analyze_target", { path: v.path })) as TargetAnalysis;


              if (analysis.is_treefrog && analysis.volume?.removable === true) {
                const validCount = await (async () => {
                  let c = 0;
                  for (const vv of vols) {
                    if (!vv.accessible) continue;
                    try {
                      const aa = (await invoke("analyze_target", { path: vv.path })) as TargetAnalysis;
                      if (aa.is_treefrog) c++;
                    } catch {}
                  }
                  return c;
                })();

                if (validCount === 1 || !sdPath) {
                  setSdPath(v.path);
                  setSdAnalysis(analysis);
                }
                break;
              }
            } catch {}
          }
        }
      } catch (e) {
        console.warn("autoDetect failed", e);
      }
    }

    doDetect();
    interval = window.setInterval(doDetect, 2000) as unknown as number;
    const onFocus = () => doDetect();
    window.addEventListener("focus", onFocus);
    // Also listen for visibility change (when app regains focus)
    const onVisibility = () => {
      if (document.visibilityState === "visible") doDetect();
    };
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      mounted = false;
      if (interval) window.clearInterval(interval);
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, []); // No deps to avoid stale closure, use functional updates inside

  // When sdPath changes (via SD Card tab), re-analyze
  useEffect(() => {
    if (!sdPath) {
      setSdAnalysis(null);
      return;
    }
    async function analyze() {
      try {
        const a = (await invoke("analyze_target", { path: sdPath })) as TargetAnalysis;
        setSdAnalysis(a);
      } catch {}
    }
    analyze();
  }, [sdPath]);

  async function handleAnalyze() {
    if (!sdPath) {
      setError("Select an SD first (SD Card)");
      return;
    }
    setLoading(true);
    setError("");
    try {
      // Only analyze the SD, no sources required
      const target = (await invoke("analyze_target", { path: sdPath })) as TargetAnalysis;
      setSdAnalysis(target);
      
      // Do not generate plan here, se construye progresivamente en cada panel
      // globalPlan and globalSpace are calculated via useMemo
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function handleSync(force: boolean = false): Promise<any> {
    if (!sdPath) return { error: "No SD selected. Go to Overview." };
    const hasBios = biosPlanEntries.length > 0;
    if (!globalPlan && !hasBios) return { error: "No sync plan. Scan at least one source folder." };
    if (globalSpace?.status === "insufficient_space") return { error: "Insufficient space on the SD." };

    setLoading(true);
    setError("");
    const agg = { deployed: 0, skipped: 0, failed: 0, errors: [] as string[], warnings: [] as string[], breakdown: [] as any[] };
    try {
      // ONE combined deploy: the exact plan entries the user previewed in each
      // panel (Games / Music / Videos / LGPT) are sent as `planEntries`, so the
      // backend deploys EXACTLY what was previewed — no re-scan, no drift
      // between preview and deployment, no double LGPT deploy.
      const panelEntries = (globalPlan?.entries || []).filter(
        (e) => (e.resolved_action || e.action) !== "skip_unchanged"
      );
      const hasPanelEntries = panelEntries.length > 0;
      if (!hasPanelEntries && !hasBios) {
        return { error: "No source folders scanned. Go to Games/Music/Videos/BIOS/LGPT and press Scan." };
      }

      const biosEntriesForDeploy = hasBios ? biosPlanEntries : null;

      // Single deploy job: panel plan entries + BIOS entries together.
      // sourcePath stays null — the plan is authoritative (canonical plan model).
      const res = (await invoke("deploy_to_sd", {
        sourcePath: null,
        sdPath,
        force,
        selectedFiles: null,
        userDecisions: systemOverrides,
        biosEntries: biosEntriesForDeploy,
        planEntries: hasPanelEntries ? panelEntries : null,
      })) as any;
      agg.deployed += res.deployed || 0;
      agg.skipped += res.skipped || 0;
      agg.failed += res.failed || 0;
      agg.errors.push(...(res.errors || []));
      agg.warnings.push(...(res.warnings || []));
      agg.breakdown.push(...(res.breakdown || []));

      await handleAnalyze();
      return agg;
    } catch (e) {
      const msg = String(e);
      setError(msg);
      return { ...agg, error: msg };
    } finally {
      setLoading(false);
    }
  }

  const tabs: { id: Tab; label: string }[] = [
    { id: "overview", label: "Overview" },
    { id: "games", label: "Games" },
    { id: "music", label: "Music" },
    { id: "videos", label: "Videos" },
    { id: "bios", label: "BIOS" },
    { id: "lgpt", label: "LGPT" },
    { id: "sdcard", label: "SD Card" },
    { id: "settings", label: "Settings" },
    { id: "about", label: "About" },
  ];

  // Semantic content counts from the BACKEND scanner/classification pipeline
  // (get_content_counts). The UI never derives counts from unrelated values
  // like existing_count, media_dirs.length, or lgpt_dirs.length.
  const [sdCounts, setSdCounts] = useState<Record<string, number> | null>(null);
  useEffect(() => {
    if (!sdAnalysis?.is_treefrog || !sdPath) { setSdCounts(null); return; }
    let cancelled = false;
    invoke<Record<string, number>>("get_content_counts", { sdPath }).then((c) => {
      if (!cancelled) setSdCounts(c);
    }).catch(() => setSdCounts(null));
    return () => { cancelled = true; };
  }, [sdPath, sdAnalysis?.is_treefrog, sdAnalysis?.existing_count]);

  const contentCounts = (() => {
    const base = {
      Games: sdCounts?.rom_count ?? 0,
      Music: sdCounts?.music_track_count ?? 0,
      Videos: sdCounts?.video_count ?? 0,
      BIOS: sdCounts?.bios_count ?? 0,
      "LGPT Samples": sdCounts?.lgpt_sample_count ?? 0,
      "LGPT Projects": sdCounts?.lgpt_project_count ?? 0,
    };
    if (!globalPlan) return base;
    // Pending additions from the current plan are shown ALONGSIDE the real
    // SD counts (planned, not yet deployed).
    return {
      Games: base.Games + globalPlan.entries.filter((e) => e.content_type?.startsWith("rom/") || e.content_type?.startsWith("grouped")).filter((e) => (e.resolved_action || e.action) !== "skip_unchanged").length,
      Music: base.Music + globalPlan.entries.filter((e) => e.content_type === "music").filter((e) => (e.resolved_action || e.action) !== "skip_unchanged").length,
      Videos: base.Videos + globalPlan.entries.filter((e) => e.content_type === "video").filter((e) => (e.resolved_action || e.action) !== "skip_unchanged").length,
      BIOS: base.BIOS + biosPlanEntries.length,
      "LGPT Samples": base["LGPT Samples"] + globalPlan.entries.filter((e) => e.content_type === "lgpt/sample").filter((e) => (e.resolved_action || e.action) !== "skip_unchanged").length,
      "LGPT Projects": base["LGPT Projects"] + globalPlan.entries.filter((e) => e.content_type === "lgpt/project").filter((e) => (e.resolved_action || e.action) !== "skip_unchanged").length,
    };
  })();

  const estado = (() => {
    if (!sdAnalysis) {
      return { sync: 0, nuevos: 0, conflicts: 0, conversion: 0, biosFaltantes: 0, noSd: true as const };
    }
    if (globalPlan) {
      return {
        sync: globalPlan.summary.unchanged,
        nuevos: globalPlan.summary.new,
        conflicts: globalPlan.summary.conflicts,
        conversion: globalPlan.entries.filter((e) => e.action === "convert_then_copy").length,
        biosFaltantes: globalPlan.entries.filter((e) => e.content_type === "bios" && e.action === "manual_review").length,
        noSd: false as const,
      };
    }
    // Before analyze, show real SD existing vs no plan
    if (sdAnalysis.is_treefrog) {
      return { sync: sdAnalysis.existing_count, nuevos: 0, conflicts: 0, conversion: 0, biosFaltantes: 0, noSd: false as const };
    }
    return { sync: 0, nuevos: 0, conflicts: 0, conversion: 0, biosFaltantes: 0, noSd: false as const };
  })();

  return (
    <div className="container">
      <Header />
      <nav className="nav" aria-label="Main navigation">
        {tabs.map((t) => (
          <button key={t.id} onClick={() => setActiveTab(t.id)} className={activeTab === t.id ? "active" : ""}>
            {t.label}
          </button>
        ))}
      </nav>

      <div style={{ display: activeTab === "overview" ? "block" : "none" }}>
        <div>
          <div className="card">
            <h3 style={{ marginTop: 0 }}>TREEFROG CONTENT MANAGER</h3>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
              <div>
                <h4 style={{ margin: "0 0 8px 0", fontSize: 13, color: "var(--text-muted)" }}>SD CARD</h4>
                {volumes.length > 1 && (
                  <div style={{ marginBottom: 8, padding: 8, border: "1px solid var(--border)", borderRadius: 6, background: "var(--surface)", maxHeight: 120, overflowY: "auto" }}>
                    <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 6, color: "var(--text-muted)" }}>
                      Removable devices detected ({volumes.length}):
                    </div>
                    {volumes.map((v) => (
                      <label
                        key={v.path}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 8,
                          padding: "4px 6px",
                          borderRadius: 4,
                          background: sdPath === v.path ? "var(--surface-elevated)" : "transparent",
                          border: sdPath === v.path ? "1px solid var(--accent)" : "1px solid transparent",
                          cursor: "pointer",
                          marginBottom: 4,
                        }}
                      >
                        <input
                          type="radio"
                          name="sd-select"
                          checked={sdPath === v.path}
                          onChange={() => {
                            setSdPath(v.path);
                            // Trigger re-analysis
                            setTimeout(() => handleAnalyze(), 100);
                          }}
                        />
                        <span style={{ fontSize: 12, flex: 1 }}>
                          <strong>{v.path}</strong>
                          {v.label ? ` — ${v.label}` : ""}
                          <span style={{ color: "var(--text-muted)" }}>
                            {v.filesystem || ""}
                            {v.total_bytes ? ` • ${fmtBytes(v.total_bytes)}` : ""}
                            {v.free_bytes ? ` • ${fmtBytes(v.free_bytes)} free` : ""}
                          </span>
                          {v.removable ? (
                            <span style={{ marginLeft: 6, fontSize: 10, background: "var(--success)", color: "white", padding: "1px 4px", borderRadius: 3 }}>
                              Removable
                            </span>
                          ) : null}
                          {!v.accessible && (
                            <span style={{ marginLeft: 6, fontSize: 10, background: "var(--danger)", color: "white", padding: "1px 4px", borderRadius: 3 }}>
                              Not accessible
                            </span>
                          )}
                        </span>
                      </label>
                    ))}
                  </div>
                )}
                {sdAnalysis && sdAnalysis.volume?.removable !== true && (
                  <div className="status-error" style={{ marginTop: 6, fontSize: 12 }}>
                    ⚠ {sdPath} is not a removable drive. It does not look like a real SD — select the correct SD above.
                  </div>
                )}
                
                {sdAnalysis ? (
                  <>
                    <div style={{ fontSize: 14, fontWeight: 600 }}>{sdAnalysis.label || "R36SX"} — {sdPath}</div>
                    <div style={{ fontSize: 13, color: sdAnalysis.is_treefrog ? "var(--success)" : "var(--danger)" }}>
                      {sdAnalysis.is_treefrog ? "✓ TreeFrogUI detected" : "✕ TreeFrogUI not detected"}
                    </div>
                    <div style={{ fontSize: 13, color: "var(--success)" }}>✓ {fmtBytes(sdAnalysis.free_bytes)} available</div>
                    <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 4 }}>
                      {sdAnalysis.filesystem || "—"} • {fmtBytes(sdAnalysis.capacity_bytes)} total • Removable
                    </div>
                  </>
                ) : (
                  <EmptyState kind="empty" title="No SD detected" description="Connect an SD card or USB drive. Only removable devices are shown." />
                )}
              </div>
              <div>
                <h4 style={{ margin: "0 0 8px 0", fontSize: 13, color: "var(--text-muted)" }}>CONTENT</h4>
                <div style={{ fontSize: 13, display: "grid", gridTemplateColumns: "1fr auto", gap: "4px 12px" }}>
                  <span>Games</span><span style={{ textAlign: "right", fontWeight: 600 }}>{contentCounts.Games}</span>
                  <span>Music</span><span style={{ textAlign: "right", fontWeight: 600 }}>{contentCounts.Music}</span>
                  <span>Videos</span><span style={{ textAlign: "right", fontWeight: 600 }}>{contentCounts.Videos}</span>
                  <span>BIOS</span><span style={{ textAlign: "right", fontWeight: 600 }}>{contentCounts.BIOS}</span>
                  <span>LGPT Samples</span><span style={{ textAlign: "right", fontWeight: 600 }}>{contentCounts["LGPT Samples"]}</span>
                  <span>LGPT Projects</span><span style={{ textAlign: "right", fontWeight: 600 }}>{contentCounts["LGPT Projects"]}</span>
                </div>
              </div>
            </div>
          </div>

          <div className="card">
            <h4 style={{ margin: "0 0 8px 0", fontSize: 13, color: "var(--text-muted)" }}>STATUS</h4>
            {!sdAnalysis ? (
              <div style={{ fontSize: 13, color: "var(--warning)" }}>⚠ No SD detected — connect a TreeFrogUI SD or select one in SD Card.</div>
            ) : (estado as any).noSd ? (
              <div style={{ fontSize: 13, color: "var(--text-muted)" }}>Connect an SD to see status.</div>
            ) : (
              <div style={{ fontSize: 13, display: "grid", gap: 4 }}>
                <div>✓ {estado.sync} files already synchronized</div>
                <div>+ {estado.nuevos} new files</div>
                {estado.conflicts > 0 && <div style={{ color: "var(--warning)" }}>⚠ {estado.conflicts} conflicts</div>}
                {estado.conversion > 0 && <div style={{ color: "var(--warning)" }}>⚠ {estado.conversion} videos need conversion</div>}
                {estado.biosFaltantes > 0 && <div style={{ color: "var(--warning)" }}>⚠ {estado.biosFaltantes} missing BIOS</div>}
                {globalPlan && <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 4 }}>Plan: {globalPlan.summary.new} new, {globalPlan.summary.unchanged} unchanged, {globalPlan.summary.duplicate_content} duplicates</div>}
              </div>
            )}
          </div>

          <div className="card">
            <h4 style={{ margin: "0 0 8px 0", fontSize: 13, color: "var(--text-muted)" }}>SPACE</h4>
            {!sdAnalysis ? (
              <div style={{ fontSize: 13, color: "var(--text-muted)" }}>No SD — no space information.</div>
            ) : (
              <>
                <div style={{ fontSize: 13, display: "grid", gridTemplateColumns: "1fr auto", gap: "4px 12px" }}>
                  <span>Required:</span><span style={{ textAlign: "right", fontWeight: 600 }}>{globalSpace ? fmtBytes(globalSpace.required_bytes) : sdAnalysis ? "— (analyze first)" : "—"}</span>
                  <span>Available:</span><span style={{ textAlign: "right", fontWeight: 600 }}>{sdAnalysis ? fmtBytes(sdAnalysis.free_bytes) : "—"}</span>
                </div>
                {globalSpace?.status === "insufficient_space" && <div className="status-error" style={{ marginTop: 8 }}>Insufficient space: free up space or reduce selection.</div>}
                {globalSpace && globalSpace.status === "ok" && <div style={{ fontSize: 11, color: "var(--success)", marginTop: 4 }}>✓ Enough space</div>}
              </>
            )}
          </div>

          {error && (
            <div className="status-error" style={{ fontSize: 12, marginBottom: 8, padding: 10, border: "2px solid var(--danger)", background: "var(--danger-bg)", borderRadius: 6 }}>
              <div style={{ fontWeight: 600, marginBottom: 4 }}>⚠ Error</div>
              <div>{error}</div>
              {error.includes("You selected the 'roms' folder") && (
                <div style={{ marginTop: 8, fontSize: 11, color: "var(--text)" }}>
                  <strong>Solution:</strong> Select the <strong>root</strong> of the SD card (e.g. <code>E:\</code>) instead of the <code>roms</code> subfolder.
                </div>
              )}
            </div>
          )}

          <div className="row" style={{ marginTop: 12 }}>
            <button className="primary" onClick={handleAnalyze} disabled={loading || !sdPath}>
              {loading ? "Analyzing..." : "ANALYZE"}
            </button>
            {sdAnalysis ? (
              <button className="primary" onClick={() => setActiveTab("games")}>
                TRANSFER FILES →
              </button>
            ) : (
              <button disabled title="First Analyze the SD">TRANSFER FILES</button>
            )}
            <button className="primary" onClick={() => handleSync()} disabled={!globalPlan || globalSpace?.status === "insufficient_space" || loading} style={{ display: "none" }}>
              SYNC
            </button>
          </div>
          <p style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 6 }}>Flow: <code>ANALYZE</code> (real SD state) → <code>TRANSFER FILES</code> (Games → Music → Videos → BIOS → LGPT → SD Card) → <code>Sync to SD</code> in SD Card. The app verifies the extension and automatically copies to the correct folder (TreeFrogUI profile, you don't choose the folder on SD). Recursive analysis of subfolders.</p>
        </div>
      </div>
      <div style={{ display: activeTab === "games" ? "block" : "none" }}>
        <GamesPanel visible={activeTab === "games"} 
          globalSdPath={sdPath} 
          onSourceChange={setGamesSource} 
          onPlanChange={setGamesPlan as any}
          onOverridesChange={setSystemOverrides}
          onNext={() => setActiveTab("music")} 
        />
      </div>
      <div style={{ display: activeTab === "music" ? "block" : "none" }}>
        <MusicPanel
          visible={activeTab === "music"}
          onSourceChange={setMusicSource}
          onPlanChange={setMusicPlan as any}
          onNext={() => setActiveTab("videos")}
          onSkip={() => setActiveTab("videos")}
        />
      </div>
      <div style={{ display: activeTab === "videos" ? "block" : "none" }}>
        <VideosPanel visible={activeTab === "videos"} 
          globalSdPath={sdPath} 
          onSourceChange={setVideosSource} 
          onPlanChange={setVideosPlan as any}
          onNext={() => setActiveTab("bios")} 
        />
      </div>
      <div style={{ display: activeTab === "bios" ? "block" : "none" }}>
        <BiosManager visible={activeTab === "bios"} 
          globalSdPath={sdPath}
          onSourceChange={setBiosSource}
          onPlanChange={(plan: any) => {
            if (plan && plan.entries) {
              const entries = plan.entries.map((e: any) => ({
                source: e.source,
                destination: e.destination,
                action: e.action || 'copy',
                reason: e.reason || 'BIOS file selected by user',
                content_type: e.content_type || 'bios',
                is_bios: true,
              }));
              setBiosPlanEntries(entries);
            } else {
              setBiosPlanEntries([]);
            }
            setBiosPlan(plan);
          }}
          onNext={() => setActiveTab("lgpt")} 
        />
      </div>
      <div style={{ display: activeTab === "lgpt" ? "block" : "none" }}>
        <LgptManager visible={activeTab === "lgpt"} 
          globalSdPath={sdPath}
          onSamplesSourceChange={setLgptSamplesSource}
          onProjectsSourceChange={setLgptProjectsSource}
          onPlanChange={setLgptPlan as any}
          onNext={() => setActiveTab("sdcard")} 
        />
      </div>
      <div style={{ display: activeTab === "sdcard" ? "block" : "none" }}>
        <SdCardPanel 
          sdPath={sdPath} 
          onChange={setSdPath} 
          volumes={volumes}
          globalPlan={combinedPlan}
          globalSpace={globalSpace}
          isSyncing={loading}
          biosPlanEntries={biosPlanEntries}
          onSync={async (force) => {
            const result = await handleSync(force);
            setSyncResult(result);
            return result;
          }}
        />
      </div>
      <div style={{ display: activeTab === "settings" ? "block" : "none" }}><SettingsPanel /><UpdateChecker /></div>
      <div style={{ display: activeTab === "about" ? "block" : "none" }}><About /></div>
    </div>
  );
}

export type PlanSummary = {
  unchanged: number;
  new: number;
  changed: number;
  duplicate_content: number;
  conflicts: number;
  deletions: number;
  manual_review?: number;
  unsupported_archive?: number;
};

export type PlanEntry = {
  source: string;
  destination: string;
  action: "copy" | "extract" | "skip_unchanged" | "skip_duplicate" | "conflict" | "manual_review" | "unsupported_archive" | "convert_then_copy" | "unsupported" | "conversion_error";
  reason: string;
  status?: string;
  hash?: string;
  source_hash?: string | null;
  destination_hash?: string | null;
  content_type?: string;
  kind?: string;
  possible_destinations?: string[] | null;
  size?: number;
  group?: string[];
  members?: string[] | null;
  default_action?: string;
  resolution?: string | null;
  resolved_action?: string | null;
  original_destination?: string;
  preset?: string;
  probe?: Record<string, unknown> | null;
  converted_name?: string;
};

export type Plan = {
  summary: PlanSummary;
  entries: PlanEntry[];
  warnings: string[];
  resolved_summary?: Record<string, number>;
};
