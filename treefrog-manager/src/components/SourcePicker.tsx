import { useState } from "react";
import { pickFolder } from "../services/dialog";

type Props = {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  title?: string; // dialog title
  debugAllowManual?: boolean; // hidden fallback for web dev
};

export default function SourcePicker({ label, value, onChange, placeholder = "No folder selected", title = "Select folder", debugAllowManual = false }: Props) {
  const [manual, setManual] = useState(value);
  const [error, setError] = useState("");

  async function handleBrowse() {
    setError("");
    try {
      const sel = await pickFolder({ title });
      if (sel) {
        onChange(sel);
        setManual(sel);
      }
    } catch (e) {
      setError(String(e));
    }
  }

  // Keep manual in sync when parent changes
  if (manual !== value && value !== "" && manual === "") {
    // do not auto-sync to avoid loop, but allow initial
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      <label style={{ fontSize: 13, fontWeight: 600 }}>{label}</label>
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
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
          title={value || placeholder}
        >
          {value || placeholder}
        </div>
        <button onClick={handleBrowse} style={{ minWidth: 92 }}>
          Browse
        </button>
      </div>
      {debugAllowManual && (
        <input
          value={manual}
          onChange={(e) => {
            setManual(e.target.value);
            onChange(e.target.value);
          }}
          placeholder={placeholder}
          style={{ fontSize: 12, opacity: 0.7 }}
        />
      )}
      {error && <span style={{ color: "var(--danger)", fontSize: 12 }}>{error}</span>}
    </div>
  );
}
