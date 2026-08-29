import frogOnly from "../assets/branding/frog-only.png";

export default function About() {
  return (
    <div className="card">
      <h3>About — TreeFrog Content Manager</h3>
      <div style={{ display: "flex", gap: 16, alignItems: "flex-start" }}>
        <img src={frogOnly} alt="TreeFrog frog pixel art" style={{ width: 96, height: 109, imageRendering: "pixelated", background: "var(--surface)", border: "1px solid var(--border)", borderRadius: 8, padding: 6 }} />
        <div style={{ flex: 1 }}>
          <h4 style={{ marginTop: 0 }}>TreeFrog Content Manager</h4>
          <p style={{ fontSize: 13, color: "var(--text-muted)" }}>
            Global TreeFrogUI SD-card content manager. One declarative profile schema for all handhelds (R36SX/SF3000/GB350…). Device-specific logic is limited to SD detection/markers.
          </p>
          <p style={{ fontSize: 13 }}>
            <strong>Branding:</strong> Frog pixel-art is from upstream <a href="https://github.com/tzubertowski/TreeFrogUI" target="_blank" rel="noreferrer">TreeFrogUI</a> (<code>xgame-logo.bmp</code>, 480×854) — see <code>src/assets/branding/README.md</code>. Frog is cropped and made transparent for the application/window/installer icon; the full frog + wordmark is shown here only as secondary About/Credits asset. TreeFrogUI is CC BY-NC-SA 4.0 (frog asset from FrogUI).
          </p>
          <p style={{ fontSize: 12, color: "var(--text-muted)" }}>
            Stack: Tauri 2 + Rust + React + TypeScript + SQLite + serde versioned JSON 1.1.0 + SHA-256 + FFmpeg/ffprobe + ZIP (7z/RAR stubs). Windows x64 first, portable filesystem layer.
          </p>
          <div className="row" style={{ marginTop: 12 }}>
            <a href="https://github.com/tzubertowski/TreeFrogUI" target="_blank" rel="noreferrer" style={{ fontSize: 13, color: "var(--accent)" }}>TreeFrogUI upstream</a>
            <a href="https://github.com/tzubertowski/FrogUI" target="_blank" rel="noreferrer" style={{ fontSize: 13, color: "var(--accent)" }}>FrogUI upstream</a>
            <a href="https://github.com/tzubertowski/mini-scraper-cfw/releases" target="_blank" rel="noreferrer" style={{ fontSize: 13, color: "var(--accent)" }}>Mini Scraper (artwork)</a>
          </div>
        </div>
      </div>
      <div style={{ marginTop: 16, padding: 12, background: "var(--surface)", border: "1px solid var(--border)", borderRadius: 6 }}>
        <h4 style={{ margin: "0 0 6px 0" }}>License & Attribution</h4>
        <p style={{ fontSize: 12, margin: 0 }}>
          TreeFrog Content Manager code: GPL-3.0-or-later (see <code>Cargo.toml</code>). Frog asset: CC BY-NC-SA 4.0 via TreeFrogUI/FrogUI — do not sell or bundle with commercial devices. No newly generated logo; original pixel-art preserved.
        </p>
      </div>
    </div>
  );
}
