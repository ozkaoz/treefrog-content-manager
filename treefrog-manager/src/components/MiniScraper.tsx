import { useState } from "react";

export default function MiniScraper({ sdPath }: { sdPath: string }) {
  const [checking, setChecking] = useState(false);
  const [result, setResult] = useState<string>("");

  async function handleOpen() {
    try {
      const tauri = (window as unknown as { __TAURI__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI__;
      // Use opener plugin to open URL
      if (tauri) {
        // Try to use opener plugin via invoke
        try {
          await (tauri as any).invoke("plugin:opener|open_url", { url: "https://github.com/tzubertowski/mini-scraper-cfw/releases" });
        } catch {
          // Fallback to window.open
          window.open("https://github.com/tzubertowski/mini-scraper-cfw/releases", "_blank");
        }
      } else {
        window.open("https://github.com/tzubertowski/mini-scraper-cfw/releases", "_blank");
      }
    } catch (e) {
      setResult(String(e));
    }
  }

  async function handleVerify() {
    if (!sdPath) {
      setResult("Select an SD target first");
      return;
    }
    setChecking(true);
    setResult("");
    try {
      const tauri = (window as unknown as { __TAURI__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI__;
      if (tauri) {
        // Use fs to check for .res folders
        // For now, just call analyze_target and check for roms with .res
        const analysis = (await tauri.invoke("analyze_target", { path: sdPath })) as any;
        // Simple check: look for roms subdirs with .res
        // This is a placeholder for real .res verification
        setResult(`SD has ${analysis.existing_count} files, ${analysis.rom_dirs.length} ROM systems. Check .res manually: look for roms/<system>/.res/<game>.png`);
      } else {
        setResult("Verification requires Tauri (desktop). In web preview, manually check for roms/<system>/.res/<game>.png, also Imgs/, images/ compat, and title suffixes -title/-screenshot.");
      }
    } catch (e) {
      setResult(String(e));
    } finally {
      setChecking(false);
    }
  }

  return (
    <div className="card">
      <h4>Artwork — Mini Scraper (external)</h4>
      <p style={{ fontSize: 12, color: "var(--text-muted)" }}>
        TreeFrogUI uses <strong>Mini Scraper</strong> for box art / provedores. The manager does <strong>not</strong> implement a second scraper. Use the button below to open the official Mini Scraper releases, run it against your SD card, then verify that <code>.res</code> folders were created.
      </p>
      <div className="row">
        <button onClick={handleOpen}>Open Mini Scraper Releases</button>
        <button onClick={handleVerify} disabled={checking || !sdPath}>
          {checking ? "Checking…" : "Verify .res"}
        </button>
      </div>
      {result && <div style={{ fontSize: 12, marginTop: 8, padding: "6px 8px", background: "var(--surface)", border: "1px solid var(--border)", borderRadius: 4 }}>{result}</div>}
      <p style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 8 }}>
        Expected after Mini Scraper: <code>roms/GBA/.res/Advance Wars.png</code> (box art), also <code>Imgs/</code>, <code>images/</code> compat, and title variants <code>-title</code>/<code>-screenshot</code> etc. See <code>theme.md</code> for sizes.
      </p>
    </div>
  );
}
