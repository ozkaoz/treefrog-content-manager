import { useEffect, useState } from "react";
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

  // Global sources per content type (only source folder, no SD per tab)
  const [gamesSource, setGamesSource] = useState("");
  const [musicSource, setMusicSource] = useState("");
  const [videosSource, setVideosSource] = useState("");

  // Global plan for Overview (aggregated)
  const [globalPlan, setGlobalPlan] = useState<Plan | null>(null);
  const [globalSpace, setGlobalSpace] = useState<any | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [activeTab, setActiveTab] = useState<Tab>("overview");

  useEffect(() => {
    const cleanup = initTheme();
    return cleanup;
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
        } catch (e) {
          console.warn("list_volumes failed", e);
        }

        if (vols.length === 0) {
          const candidates = ["G:\\", "E:\\", "F:\\", "D:\\", "H:\\", "I:\\", "J:\\", "K:\\", "L:"];
          for (const cand of candidates) {
            try {
              const a = (await invoke("analyze_target", { path: cand })) as TargetAnalysis;
              if (a.status !== "inaccessible") {
                vols.push({ 
                  path: cand, 
                  label: a.label || null, 
                  filesystem: a.filesystem || null, 
                  total_bytes: a.capacity_bytes, 
                  free_bytes: a.free_bytes, 
                  removable: null, 
                  accessible: true 
                });
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
                setError(`SD desconectada: ${sdPath} ya no está accesible`);
              }
            } catch {
              setSdAnalysis(null);
              setError(`SD desconectada: ${sdPath} ya no está accesible`);
            }
          }
        }

        let shouldAutoSelect = !sdPath || !sdAnalysis?.is_treefrog || !vols.some((v) => v.path === sdPath && v.accessible);
        if (shouldAutoSelect) {
          for (const v of vols) {
            if (!v.accessible) continue;
            try {
              const analysis = (await invoke("analyze_target", { path: v.path })) as TargetAnalysis;
              if (analysis.is_treefrog) {
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
      setError("Selecciona una SD primero (SD Card)");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const target = (await invoke("analyze_target", { path: sdPath })) as TargetAnalysis;
      setSdAnalysis(target);
      
      const sources = [gamesSource, musicSource, videosSource].filter(Boolean);
      const sourcePath = sources[0] || gamesSource || musicSource || videosSource;
      
      if (sourcePath) {
        const result = (await invoke("dry_run_with_target", { sourcePath, sdPath })) as any;
        setGlobalPlan(result);
        setGlobalSpace(result.space);
        setSdAnalysis(result.target);
      } else {
        setGlobalPlan(null);
        setGlobalSpace(null);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function handleSync() {
    if (!globalPlan || !sdPath) {
      setError("Primero Analizar");
      return;
    }
    if (globalSpace?.status === "insufficient_space") {
      setError("Espacio insuficiente");
      return;
    }
    const ok = confirm(`¿Sincronizar ${globalPlan.summary.new} nuevos a ${sdPath}?\nLos archivos se copiarán a la carpeta correcta según su extensión (perfil TreeFrogUI).`);
    if (!ok) return;
    
    setLoading(true);
    try {
      const sourcePath = [gamesSource, musicSource, videosSource].filter(Boolean)[0] || gamesSource;
      const res = (await invoke("deploy_to_sd", { sourcePath, sdPath })) as any;
      alert(`Sincronizado: ${res.deployed} copiados, ${res.skipped} omitidos`);
      await handleAnalyze();
    } catch (e) {
      setError(String(e));
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

  // Derived counts for Overview from REAL sdAnalysis and globalPlan (no fake placeholders)
  const contentCounts = (() => {
    if (!sdAnalysis) {
      return { Games: 0, Music: 0, Videos: 0, BIOS: 0, "LGPT Samples": 0, "LGPT Projects": 0 };
    }
    if (globalPlan) {
      const gamesFromPlan = globalPlan.entries.filter((e) => e.content_type?.startsWith("rom/") || e.content_type?.startsWith("grouped")).length;
      const musicFromPlan = globalPlan.entries.filter((e) => e.content_type === "music").length;
      const videosFromPlan = globalPlan.entries.filter((e) => e.content_type === "video").length;
      const biosFromPlan = globalPlan.entries.filter((e) => e.content_type === "bios").length;
      const lgptSamplesFromPlan = globalPlan.entries.filter((e) => e.content_type === "lgpt/sample").length;
      const lgptProjectsFromPlan = globalPlan.entries.filter((e) => e.content_type === "lgpt/project").length;
      return {
        Games: gamesFromPlan + (sdAnalysis.rom_dirs.length > 0 ? sdAnalysis.existing_count : 0),
        Music: musicFromPlan + (sdAnalysis.media_dirs.includes("music") ? sdAnalysis.existing_count : 0),
        Videos: videosFromPlan,
        BIOS: biosFromPlan + sdAnalysis.bios_dirs.length,
        "LGPT Samples": lgptSamplesFromPlan || (sdAnalysis.lgpt_dirs.includes("lgpt/samples") ? 1 : 0) * 100,
        "LGPT Projects": lgptProjectsFromPlan || (sdAnalysis.lgpt_dirs.includes("lgpt/projects") ? 1 : 0) * 10,
      };
    }
    // No plan yet, show real SD existing counts
    return {
      Games: sdAnalysis.rom_dirs.length,
      Music: sdAnalysis.media_dirs.filter((d) => d.toLowerCase() === "music").length > 0 ? sdAnalysis.existing_count : 0,
      Videos: sdAnalysis.media_dirs.filter((d) => d.toLowerCase() === "videos").length > 0 ? sdAnalysis.media_dirs.length : 0,
      BIOS: sdAnalysis.bios_dirs.length,
      "LGPT Samples": sdAnalysis.lgpt_dirs.includes("lgpt/samples") ? sdAnalysis.existing_count : 0,
      "LGPT Projects": sdAnalysis.lgpt_dirs.includes("lgpt/projects") ? sdAnalysis.lgpt_dirs.length : 0,
    };
  })();

  const estado = (() => {
    if (!sdAnalysis) {
      return { sync: 0, nuevos: 0, conflictos: 0, conversion: 0, biosFaltantes: 0, noSd: true as const };
    }
    if (globalPlan) {
      return {
        sync: globalPlan.summary.unchanged,
        nuevos: globalPlan.summary.new,
        conflictos: globalPlan.summary.conflicts,
        conversion: globalPlan.entries.filter((e) => e.action === "convert_then_copy").length,
        biosFaltantes: globalPlan.entries.filter((e) => e.content_type === "bios" && e.action === "manual_review").length,
        noSd: false as const,
      };
    }
    // Before analyze, show real SD existing vs no plan
    if (sdAnalysis.is_treefrog) {
      return { sync: sdAnalysis.existing_count, nuevos: 0, conflictos: 0, conversion: 0, biosFaltantes: 0, noSd: false as const };
    }
    return { sync: 0, nuevos: 0, conflictos: 0, conversion: 0, biosFaltantes: 0, noSd: false as const };
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

      {activeTab === "overview" && (
        <div>
          <div className="card">
            <h3 style={{ marginTop: 0 }}>TREEFROG CONTENT MANAGER</h3>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
              <div>
                <h4 style={{ margin: "0 0 8px 0", fontSize: 13, color: "var(--text-muted)" }}>SD CARD</h4>
                {volumes.length > 1 && (
                  <div style={{ marginBottom: 8, padding: 8, border: "1px solid var(--border)", borderRadius: 6, background: "var(--surface)", maxHeight: 120, overflowY: "auto" }}>
                    <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 6, color: "var(--text-muted)" }}>
                      Dispositivos removibles detectados ({volumes.length}):
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
                            {v.free_bytes ? ` • ${fmtBytes(v.free_bytes)} libre` : ""}
                          </span>
                          {v.removable ? (
                            <span style={{ marginLeft: 6, fontSize: 10, background: "var(--success)", color: "white", padding: "1px 4px", borderRadius: 3 }}>
                              Removible
                            </span>
                          ) : null}
                          {!v.accessible && (
                            <span style={{ marginLeft: 6, fontSize: 10, background: "var(--danger)", color: "white", padding: "1px 4px", borderRadius: 3 }}>
                              No accesible
                            </span>
                          )}
                        </span>
                      </label>
                    ))}
                  </div>
                )}
                
                {sdAnalysis ? (
                  <>
                    <div style={{ fontSize: 14, fontWeight: 600 }}>{sdAnalysis.label || "R36SX"} — {sdPath}</div>
                    <div style={{ fontSize: 13, color: sdAnalysis.is_treefrog ? "var(--success)" : "var(--danger)" }}>
                      {sdAnalysis.is_treefrog ? "✓ TreeFrogUI detectado" : "✕ TreeFrogUI no detectado"}
                    </div>
                    <div style={{ fontSize: 13, color: "var(--success)" }}>✓ {fmtBytes(sdAnalysis.free_bytes)} disponibles</div>
                    <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 4 }}>
                      {sdAnalysis.filesystem || "—"} • {fmtBytes(sdAnalysis.capacity_bytes)} total • Removible
                    </div>
                  </>
                ) : (
                  <EmptyState kind="empty" title="No SD detectada" description="Conecta una tarjeta SD o memoria USB. Solo se muestran dispositivos removibles." />
                )}
              </div>
              <div>
                <h4 style={{ margin: "0 0 8px 0", fontSize: 13, color: "var(--text-muted)" }}>CONTENIDO</h4>
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
            <h4 style={{ margin: "0 0 8px 0", fontSize: 13, color: "var(--text-muted)" }}>ESTADO</h4>
            {!sdAnalysis ? (
              <div style={{ fontSize: 13, color: "var(--warning)" }}>⚠ No hay SD detectada — conecta una SD TreeFrogUI o selecciónala en SD Card.</div>
            ) : (estado as any).noSd ? (
              <div style={{ fontSize: 13, color: "var(--text-muted)" }}>Conecta una SD para ver el estado.</div>
            ) : (
              <div style={{ fontSize: 13, display: "grid", gap: 4 }}>
                <div>✓ {estado.sync} archivos ya están sincronizados</div>
                <div>+ {estado.nuevos} archivos nuevos</div>
                {estado.conflictos > 0 && <div style={{ color: "var(--warning)" }}>⚠ {estado.conflictos} conflictos</div>}
                {estado.conversion > 0 && <div style={{ color: "var(--warning)" }}>⚠ {estado.conversion} vídeos necesitan conversión</div>}
                {estado.biosFaltantes > 0 && <div style={{ color: "var(--warning)" }}>⚠ {estado.biosFaltantes} BIOS faltantes</div>}
                {globalPlan && <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 4 }}>Plan: {globalPlan.summary.new} nuevos, {globalPlan.summary.unchanged} sin cambios, {globalPlan.summary.duplicate_content} duplicados</div>}
              </div>
            )}
          </div>

          <div className="card">
            <h4 style={{ margin: "0 0 8px 0", fontSize: 13, color: "var(--text-muted)" }}>ESPACIO</h4>
            {!sdAnalysis ? (
              <div style={{ fontSize: 13, color: "var(--text-muted)" }}>Sin SD — no hay información de espacio.</div>
            ) : (
              <>
                <div style={{ fontSize: 13, display: "grid", gridTemplateColumns: "1fr auto", gap: "4px 12px" }}>
                  <span>Necesario:</span><span style={{ textAlign: "right", fontWeight: 600 }}>{globalSpace ? fmtBytes(globalSpace.required_bytes) : sdAnalysis ? "— (analiza primero)" : "—"}</span>
                  <span>Disponible:</span><span style={{ textAlign: "right", fontWeight: 600 }}>{sdAnalysis ? fmtBytes(sdAnalysis.free_bytes) : "—"}</span>
                </div>
                {globalSpace?.status === "insufficient_space" && <div className="status-error" style={{ marginTop: 8 }}>Espacio insuficiente: libera espacio o reduce selección.</div>}
                {globalSpace && globalSpace.status === "ok" && <div style={{ fontSize: 11, color: "var(--success)", marginTop: 4 }}>✓ Espacio suficiente</div>}
              </>
            )}
          </div>

          {error && <div className="status-error" style={{ fontSize: 12, marginBottom: 8 }}>{error}</div>}

          <div className="row" style={{ marginTop: 12 }}>
            <button className="primary" onClick={handleAnalyze} disabled={loading || !sdPath}>
              {loading ? "Analizando…" : "ANALIZAR"}
            </button>
            {globalPlan ? (
              <button className="primary" onClick={() => setActiveTab("games")}>
                TRANSFERIR ARCHIVOS →
              </button>
            ) : (
              <button disabled title="Primero Analizar para ver el estado real">TRANSFERIR ARCHIVOS</button>
            )}
            <button className="primary" onClick={handleSync} disabled={!globalPlan || globalSpace?.status === "insufficient_space" || loading} style={{ display: "none" }}>
              SINCRONIZAR
            </button>
          </div>
          <p style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 6 }}>Flujo: <code>ANALIZAR</code> (estado real SD) → <code>TRANSFERIR ARCHIVOS</code> (Games → Music → Videos → SD Card) → <code>Sync to SD</code> en SD Card. La app verifica la extensión y copia automáticamente a la carpeta correcta (perfil TreeFrogUI, no eliges carpeta en SD). Análisis recursivo de subcarpetas.</p>
        </div>
      )}

      {activeTab === "games" && <GamesPanel globalSdPath={sdPath} onSourceChange={setGamesSource} onNext={() => setActiveTab("music")} />}
      {activeTab === "music" && <MusicPanel globalSdPath={sdPath} onSourceChange={setMusicSource} onNext={() => setActiveTab("videos")} />}
      {activeTab === "videos" && <VideosPanel globalSdPath={sdPath} onSourceChange={setVideosSource} onNext={() => setActiveTab("sdcard")} />}
      {activeTab === "bios" && <BiosManager />}
      {activeTab === "lgpt" && <LgptManager />}
      {activeTab === "sdcard" && (
        <SdCardPanel 
          sdPath={sdPath} 
          onChange={setSdPath} 
          volumes={volumes}
          globalPlan={globalPlan}
          globalSpace={globalSpace}
          onSync={handleSync}
        />
      )}
      {activeTab === "settings" && <SettingsPanel />}
      {activeTab === "about" && <About />}
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
