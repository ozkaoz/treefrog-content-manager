import { useState } from "react";
import { pickFolder } from "../services/dialog";

export default function SdPicker({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const [manual, setManual] = useState(value);

  async function pick() {
    try {
      const sel = await pickFolder({ title: "Select TreeFrogUI SD root (must contain cubegm/ + roms/)" });
      if (sel) {
        onChange(sel);
        setManual(sel);
      }
    } catch (e) {
      console.error(e);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      <label style={{ fontSize: 13, fontWeight: 600 }}>TreeFrogUI SD root (legacy)</label>
      <div className="row" style={{ alignItems: "stretch" }}>
        <div
          style={{
            flex: 1,
            padding: "8px 10px",
            border: "1px solid var(--border)",
            borderRadius: 6,
            background: "var(--input)",
            color: value ? "var(--text)" : "var(--text-muted)",
            fontSize: 13,
            minHeight: 36,
            display: "flex",
            alignItems: "center",
          }}
        >
          {value || "No SD selected"}
        </div>
        <button onClick={pick}>Browse</button>
      </div>
      {/* Debug manual input hidden unless needed */}
      <input
        value={manual}
        onChange={(e) => {
          setManual(e.target.value);
          onChange(e.target.value);
        }}
        placeholder="E:\  or  /mnt/sdcard"
        style={{ fontSize: 11, opacity: 0.6, display: "none" }}
      />
    </div>
  );
}
