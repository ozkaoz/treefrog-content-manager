import { useState } from "react";

export default function SdPicker({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const [manual, setManual] = useState(value);

  async function pick() {
    const tauri = (window as unknown as { __TAURI__?: { dialog: { open: (opts: unknown) => Promise<string | null> } } }).__TAURI__;
    if (tauri?.dialog) {
      // @ts-expect-error tauri dialog plugin
      const sel = await window.__TAURI__.dialog.open({ directory: true, multiple: false, title: "Select TreeFrogUI SD root" });
      if (typeof sel === "string") {
        onChange(sel);
        setManual(sel);
      }
    } else {
      const v = prompt("Enter SD root path (must contain cubegm/ + roms/):", manual);
      if (v) { onChange(v); setManual(v); }
    }
  }

  return (
    <div className="row">
      <input value={manual} onChange={(e) => { setManual(e.target.value); onChange(e.target.value); }} placeholder="E:\  or  /mnt/sdcard  or  /Volumes/TF_CARD" style={{ flex: 1, padding: "6px 8px" }} />
      <button onClick={pick}>Browse…</button>
      {value && <span style={{ fontSize: 12, color: "#555" }}>Selected: {value}</span>}
    </div>
  );
}
