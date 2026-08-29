import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { getSystemTheme } from "../services/theme";

type ProfileInfo = {
  profile_version: string;
  systems: number;
};

export default function SettingsPanel() {
  const [profile, setProfile] = useState<ProfileInfo | null>(null);
  const [theme, setTheme] = useState<string>("light");
  const [error, setError] = useState("");
  const [buildInfo, setBuildInfo] = useState<any>(null);
  useEffect(() => { invoke("build_info").then(setBuildInfo).catch(() => {}); }, []);

  useEffect(() => {
    setTheme(getSystemTheme());
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => setTheme(e.matches ? "dark" : "light");
    if (mq.addEventListener) mq.addEventListener("change", handler);
    else (mq as any).addListener(handler);
    return () => {
      if (mq.removeEventListener) mq.removeEventListener("change", handler);
      else (mq as any).removeListener(handler);
    };
  }, []);

  useEffect(() => {
    async function load() {
      try {
        const res = (await invoke("verify_profile")) as { profile_version: string };
        setProfile({ profile_version: res.profile_version, systems: 75 });
      } catch (e) {
        setError(String(e));
      }
    }
    load();
  }, []);

  return (
    <div className="card">
      <h3>Settings</h3>
      <p style={{ fontSize: 12, color: "var(--text-muted)" }}>
        Configuración global de TreeFrog Content Manager. Todo es perfil-impulsado; no hay forks por dispositivo.
      </p>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
        <div style={{ border: "1px solid var(--border)", borderRadius: 6, padding: 12, background: "var(--surface)" }}>
          <h4 style={{ margin: "0 0 8px 0" }}>Profile</h4>
          <div style={{ fontSize: 12 }}>
            <div><strong>Version:</strong> {profile?.profile_version || "—"}</div>
            <div><strong>Systems:</strong> {profile?.systems ?? "—"} (75 aliases, case-sensitive, profile-driven)</div>
            <div><strong>Archive policy:</strong> <code>archive_policy.json</code> 1.1.0 (handlers, safety, per_system)</div>
            <div><strong>Media:</strong> <code>media.json</code> (music, videos, images, ebooks)</div>
            <div><strong>BIOS:</strong> <code>bios.json</code> 1.1.0 formal (13 definiciones)</div>
            <div><strong>LGPT:</strong> <code>lgpt.json</code> (samples, projects)</div>
            <div><strong>Video:</strong> <code>video_presets.json</code> <code>PROVISIONAL_UNVALIDATED</code></div>
            <div><strong>SD markers:</strong> <code>sd_markers.json</code> (cubegm + roms)</div>
          </div>
          {error && <div style={{ fontSize: 11, color: "var(--danger)", marginTop: 6 }}>{error}</div>}
        </div>

        <div style={{ border: "1px solid var(--border)", borderRadius: 6, padding: 12, background: "var(--surface)" }}>
          <h4 style={{ margin: "0 0 8px 0" }}>Appearance</h4>
          <div style={{ fontSize: 12 }}>
            <div><strong>System theme:</strong> {theme} (follows Windows <code>prefers-color-scheme</code>)</div>
            <div><strong>App theme:</strong> <code>data-theme="{theme}"</code> with CSS variables</div>
            <div style={{ marginTop: 6, display: "flex", gap: 6 }}>
              <span style={{ padding: "4px 8px", borderRadius: 4, background: "var(--surface)", border: "1px solid var(--border)", color: "var(--text)" }}>Background</span>
              <span style={{ padding: "4px 8px", borderRadius: 4, background: "var(--accent)", color: "white" }}>Accent</span>
              <span style={{ padding: "4px 8px", borderRadius: 4, background: "var(--success)", color: "white" }}>Success</span>
              <span style={{ padding: "4px 8px", borderRadius: 4, background: "var(--danger)", color: "white" }}>Danger</span>
            </div>
            <p style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 6 }}>No custom dark/light toggle — la app sigue al sistema automáticamente.</p>
          </div>
        </div>

        <div style={{ border: "1px solid var(--border)", borderRadius: 6, padding: 12, background: "var(--surface)" }}>
          <h4 style={{ margin: "0 0 8px 0" }}>Filesystem</h4>
          <div style={{ fontSize: 12 }}>
            <div><strong>Target:</strong> TreeFrogUI SD (exFAT/FAT32, `cubegm/` + `roms/` markers)</div>
            <div><strong>Health:</strong> check <code>mounted && healthy && writable && not read-only</code> before culpar al runtime</div>
            <div><strong>Safety:</strong> bloquea `../`, absolutas, symlinks, colisiones, límites 1 GiB/1024, sin sobrescritura silenciosa</div>
            <div><strong>Duplicate:</strong> <code>SHA-256</code> exact, barato primero</div>
          </div>
        </div>

        <div style={{ border: "1px solid var(--border)", borderRadius: 6, padding: 12, background: "var(--surface)" }}>
          <h4 style={{ margin: "0 0 8px 0" }}>Application</h4>
          <div style={{ fontSize: 12 }}>
            <div><strong>Name:</strong> TreeFrog Content Manager</div>
            <div><strong>Version:</strong> 0.1.0 (Tauri 2 + Rust + React + TypeScript)</div>
            <div><strong>Branding:</strong> <code>frog-canonical.png</code> 314×280 (TreeFrogUI CC BY-NC-SA 4.0)</div>
            <div><strong>Portable:</strong> <code>TreeFrog-Content-Manager-0.1.0-Windows-x64.exe</code> 14 MB</div>
            <div><strong>Installer:</strong> <code>TreeFrog-Content-Manager-0.1.0-Windows-x64-Setup.exe</code> 3.5 MB</div>
            <div><strong>Profiles:</strong> embedded via <code>include_str!</code> for portable</div>
          </div>
        </div>
      </div>

      <div style={{ marginTop: 12, padding: 10, background: "var(--surface)", border: "1px solid var(--border)", borderRadius: 6 }}>
        <h4 style={{ margin: "0 0 6px 0" }}>Storage & Index</h4>
        <p style={{ fontSize: 12, margin: 0, color: "var(--text-muted)" }}>
          Índice local vía <code>SQLite</code> (o robust local) para bibliotecas, fingerprints, despliegues, versión de perfil, versión de herramienta, historial de jobs. Nunca se commitean rutas de usuario. Staging + <code>atomic rename</code> para Sync, reanudable si se interrumpe, sin eliminar en Sync normal.
        </p>
      </div>
      <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 8 }}>
        Build: {buildInfo?.commit || "dev"} — {buildInfo?.built_at || ""}
      </div>
    </div>
  );
}
