import { useEffect, useState } from "react";
import { pickFolder } from "../services/dialog";
import EmptyState from "./EmptyState";

type BiosVariant = {
  id: string;
  filenames: string[];
  aliases: string[];
  expected_size: number | null;
  hashes_sha256: string[];
};

type BiosDefinition = {
  id: string;
  system_id: string;
  system_name: string;
  name: string;
  description: string;
  required: string;
  requirement: { scope: string; mandatory_when: string; condition?: string };
  variants: BiosVariant[];
  accepted_filenames: string[];
  accepted_patterns?: string[];
  destinations: string[];
  primary_destination: string;
  expected_size: number | null;
  hashes_sha256: string[];
  aliases: string[];
};

type BiosValidation = {
  bios_id: string;
  system_id?: string;
  state: string;
  reason: string;
  required: boolean;
  file?: string | null;
  hash?: string | null;
  size?: number | null;
  variant?: string | null;
};

const STATUS_LABEL: Record<string, string> = {
  missing: "Missing",
  found_valid: "Verified",
  found_invalid: "Invalid",
  found_unknown: "Needs verification",
  duplicate: "Duplicate",
  conflict: "Conflict",
  not_required: "Not Required",
  found_valid_variant: "Verified",
};

type PlanEntry = {
  source: string;
  destination: string;
  action: string;
  reason: string;
  content_type?: string;
  size?: number;
};

type Plan = {
  summary: { new: number; changed: number; unchanged: number; duplicate_content: number; conflicts: number; deletions: number; manual_review?: number; unsupported_archive?: number };
  entries: PlanEntry[];
  warnings: string[];
};

const STATUS_COLOR: Record<string, string> = {
  missing: "var(--danger)",
  found_valid: "var(--success)",
  found_invalid: "var(--danger)",
  found_unknown: "var(--warning)",
  duplicate: "var(--accent)",
  conflict: "var(--danger)",
  not_required: "var(--text-muted)",
};

export default function BiosManager({ 
  globalSdPath,
  onSourceChange,
  onPlanChange,
  onNext,
  visible
}: { 
  globalSdPath: string;
  onSourceChange?: (v: string) => void;
  onPlanChange?: (plan: Plan | null) => void;
  onNext?: () => void;
  visible?: boolean;
}) {
  const [biosSource, setBiosSource] = useState<string>("");
  const [results, setResults] = useState<BiosValidation[] | null>(null);
  const [selected, setSelected] = useState<BiosValidation | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>("");
  const [definitions, setDefinitions] = useState<BiosDefinition[]>([]);

  async function loadDefinitions() {
    try {
      const tauri = (window as unknown as { __TAURI__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI__;
      if (tauri) {
        const res = (await tauri.invoke("bios_profile")) as { definitions: BiosDefinition[] };
        setDefinitions(res.definitions || []);
      } else {
        const mock: BiosDefinition[] = [
          {
            id: "ps1_bios",
            system_id: "psx",
            system_name: "PlayStation 1",
            name: "PlayStation BIOS (SCPH)",
            description: "Any 512 KiB PS1 BIOS",
            required: "conditional",
            requirement: { scope: "conditional", mandatory_when: "psx_content_present" },
            variants: [
              { id: "ps1_scph1001", filenames: ["scph1001.bin"], aliases: ["SCPH1001.BIN"], expected_size: 524288, hashes_sha256: [] },
              { id: "ps1_scph5501", filenames: ["scph5501.bin"], aliases: ["SCPH5501.BIN"], expected_size: 524288, hashes_sha256: [] },
            ],
            accepted_filenames: ["scph1001.bin", "scph5501.bin", "scph*.bin"],
            destinations: ["cubegm/bios"],
            primary_destination: "cubegm/bios",
            expected_size: 524288,
            hashes_sha256: [],
            aliases: ["SCPH1001.BIN"],
          },
          {
            id: "gba_bios",
            system_id: "gba",
            system_name: "Game Boy Advance",
            name: "GBA BIOS",
            description: "Official Nintendo GBA BIOS",
            required: "conditional",
            requirement: { scope: "conditional", mandatory_when: "gba_content_present" },
            variants: [{ id: "gba_bios_single", filenames: ["gba_bios.bin"], aliases: ["GBA_BIOS.BIN"], expected_size: 16384, hashes_sha256: ["a860a8c0b6d573d191e4ec7db1b33b04ccf2454a7df67b3a6de030423b6a436"] }],
            accepted_filenames: ["gba_bios.bin"],
            destinations: ["cubegm/bios"],
            primary_destination: "cubegm/bios",
            expected_size: 16384,
            hashes_sha256: ["a860a8c0b6d573d191e4ec7db1b33b04ccf2454a7df67b3a6de030423b6a436"],
            aliases: ["GBA_BIOS.BIN"],
          },
        ];
        setDefinitions(mock);
      }
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleBrowse() {
    try {
      const sel = await pickFolder({ title: "Select BIOS source folder" });
      if (sel) {
        setBiosSource(sel);
        onSourceChange?.(sel);
      }
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleScan() {
    if (!biosSource) {
      setError("Select a BIOS source directory (e.g., C:\\BIOS)");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const tauri = (window as unknown as { __TAURI__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI__;
      let res: unknown;
      if (tauri) {
        res = await tauri.invoke("bios_scan", { biosSource });
      } else {
        res = mockBiosScan(biosSource);
      }
      const data = res as { results: BiosValidation[] };
      setResults(data.results);
      if (data.results.length > 0) setSelected(data.results[0]);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  // Re-scan automatically when the tab becomes visible again,
  // so the plan always reflects the current state of disk/SD.
  useEffect(() => {
    if (visible && biosSource) {
      handleScan();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible]);

  useEffect(() => {
    if (!results || results.length === 0) {
      onPlanChange?.(null);
      return;
    }
    
    // Convert BIOS validation results to Plan format
    const entries: PlanEntry[] = results.map(r => {
      const action = r.state === 'found_valid' ? 'copy' : 
                     r.state === 'missing' ? 'manual_review' :
                     r.state === 'found_invalid' ? 'conflict' :
                     r.state === 'duplicate' ? 'skip_duplicate' : 'manual_review';
      
      return {
        source: r.file || '',
        destination: `cubegm/bios/${r.file?.split(/[/\\]/).pop() || ''}`,
        action,
        reason: r.reason,
        content_type: 'bios',
        size: r.size || 0,
      };
    });
    
    const summary = {
      new: entries.filter(e => e.action === 'copy').length,
      changed: 0,
      unchanged: 0,
      duplicate_content: entries.filter(e => e.action === 'skip_duplicate').length,
      conflicts: entries.filter(e => e.action === 'conflict').length,
      deletions: 0,
      manual_review: entries.filter(e => e.action === 'manual_review').length,
      unsupported_archive: 0,
    };
    
    const plan: Plan = { entries, summary, warnings: [] };
    onPlanChange?.(plan);
  }, [results, onPlanChange]);

  if (definitions.length === 0) {
    setTimeout(() => loadDefinitions(), 0);
  }

  return (
    <div className="card">
      <h3>BIOS — TreeFrogUI (profile-driven, sin descargas)</h3>

      <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 8 }}>
        <label style={{ fontSize: 13, fontWeight: 600 }}>BIOS source folder</label>
        <div className="row" style={{ alignItems: "stretch" }}>
          <div
            style={{
              flex: 1,
              padding: "8px 10px",
              border: "1px solid var(--border)",
              borderRadius: 6,
              background: "var(--input)",
              color: biosSource ? "var(--text)" : "var(--text-muted)",
              fontSize: 13,
              minHeight: 36,
              display: "flex",
              alignItems: "center",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
            title={biosSource || "No folder selected — e.g., D:\\BIOS"}
          >
            {biosSource || "No folder selected — e.g., D:\\BIOS"}
          </div>
          <button onClick={handleBrowse}>Browse</button>
        </div>
      </div>

      <p style={{ fontSize: 11, color: "var(--text-muted)", margin: "0 0 8px 0" }}>
        SD destino: {globalSdPath || "—"} — la app copiará automáticamente a cubegm/bios/ según el perfil TreeFrogUI.
      </p>

      <div className="row">
        <button className="primary" onClick={handleScan} disabled={loading || !biosSource}>
          {loading ? "Scanning…" : "Scan BIOS"}
        </button>
        <button onClick={() => { setResults(null); setSelected(null); onPlanChange?.(null); }} disabled={!results}>
          Clear
        </button>
        <button onClick={() => onNext?.()} style={{ marginLeft: "auto" }}>
          Omitir → LGPT
        </button>
        <button className="primary" onClick={() => onNext?.()} disabled={!biosSource && !results}>
          Continuar a LGPT →
        </button>
      </div>

      {error && <div className="status-error" style={{ fontSize: 12, marginTop: 8 }}>{error}</div>}

      <div style={{ display: "flex", gap: 16, alignItems: "flex-start", flexWrap: "wrap" }}>
        <div style={{ flex: 1, minWidth: 360 }}>
          <h4>Requirements {results && `(${results.length})`}</h4>
          {!results && !loading && <EmptyState kind="empty" title="No scan yet" description="Select a BIOS source and press Scan. The scan recursively inspects files, safely inspects archives (temp workspace), hashes where needed, matches filenames/patterns/aliases, validates hashes/size, and identifies duplicates/conflicts/unknown." />}
          {loading && <EmptyState kind="loading" title="Scanning…" description="Recursive scan, archive inspection, hashing, validation." />}
          {results && results.length === 0 && <EmptyState kind="empty" title="No BIOS found" description="No files matched BIOS patterns in the selected folder." />}
          {results && results.length > 0 && (
            <table>
              <thead>
                <tr>
                  <th>System</th>
                  <th>BIOS</th>
                  <th>Status</th>
                  <th>Variant</th>
                  <th>Source</th>
                  <th>Destination</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody>
                {results.map((r, idx) => {
                  const def = definitions.find((d) => d.id === r.bios_id);
                  const variantId = (r as unknown as Record<string, unknown>).variant as string | undefined || (r.file ? r.file.split(/[\\/]/).pop() : undefined);
                  const statusLabel = STATUS_LABEL[r.state] || r.state;
                  const action = r.state === "found_valid" ? "copy" : r.state === "missing" ? "manual_review" : r.state === "found_invalid" ? "conflict" : r.state === "duplicate" ? "skip" : "manual_review";
                  return (
                    <tr key={idx} onClick={() => setSelected(r)} style={{ cursor: "pointer", background: selected?.bios_id === r.bios_id ? "var(--surface)" : undefined }}>
                      <td style={{ fontSize: 11 }}>{def?.system_name || r.system_id || r.bios_id}</td>
                      <td style={{ fontSize: 11 }}>{def?.name || r.bios_id}</td>
                      <td><span className="badge" style={{ background: STATUS_COLOR[r.state] || "var(--text-muted)", color: "white", padding: "2px 6px", borderRadius: 4, fontSize: 11 }}>{statusLabel}</span></td>
                      <td style={{ fontSize: 11 }}>{variantId || (def?.variants[0]?.filenames[0] || "-")}</td>
                      <td style={{ fontSize: 10, maxWidth: 140, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={r.file || ""}>{r.file ? r.file.split(/[\\/]/).pop() : "-"}</td>
                      <td style={{ fontSize: 10 }}>{def?.primary_destination || def?.destinations[0] || "-"}</td>
                      <td style={{ fontSize: 10 }}><span className={`badge badge-${action === "copy" ? "copy" : action === "skip" ? "skip" : "conflict"}`}>{action}</span></td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
          {results && results.length > 0 && (
            <p style={{ fontSize: 11, color: "var(--success)", marginTop: 8 }}>
              Any one valid variant satisfies the logical requirement (e.g., PS1: scph1001.bin <em>or</em> scph5501.bin). Conditional requirements: PS1 BIOS shows <em>Missing</em> only when PS1 content was detected; otherwise <em>Not Required</em>.
            </p>
          )}
        </div>

        <div style={{ flex: 1, minWidth: 280, border: "1px solid var(--border)", borderRadius: 6, padding: 12, background: "var(--surface)" }}>
          <h4>Details {selected ? `— ${selected.bios_id}` : ""}</h4>
          {!selected && <EmptyState kind="empty" title="No selection" description="Select a BIOS requirement to see system, logical name, requirement status, accepted filenames, selected variant, source path, expected destination, expected size, SHA-256 when known, actual SHA-256, and validation reason." />}
          {selected && (() => {
            const def = definitions.find((d) => d.id === selected.bios_id);
            return (
              <div style={{ fontSize: 12 }}>
                <div><strong>System:</strong> {def?.system_name || selected.system_id} ({def?.system_id || selected.system_id})</div>
                <div><strong>Logical BIOS:</strong> {def?.name || selected.bios_id}</div>
                <div><strong>Status:</strong> <span style={{ color: STATUS_COLOR[selected.state] || "var(--text)", fontWeight: 600 }}>{STATUS_LABEL[selected.state] || selected.state}</span> — {selected.reason}</div>
                <div><strong>Required:</strong> {selected.required ? "Yes" : "No"} {def?.requirement?.mandatory_when ? `(${def.requirement.mandatory_when})` : ""}</div>
                <div><strong>Accepted filenames:</strong> {(def?.accepted_filenames || []).join(", ")}</div>
                <div><strong>Aliases/patterns:</strong> {[...(def?.aliases || []), ...(def?.accepted_patterns || [])].join(", ")}</div>
                <div><strong>Selected variant:</strong> {(selected as unknown as Record<string, unknown>).variant as string || (selected.file ? selected.file.split(/[\\/]/).pop() : "-")}</div>
                <div><strong>Source path:</strong> <span style={{ wordBreak: "break-all" }}>{selected.file || "-"}</span></div>
                <div><strong>Expected destination:</strong> {def?.primary_destination || def?.destinations[0] || "-"}</div>
                <div><strong>Valid destinations:</strong> {(def?.destinations || []).join(", ")}</div>
                <div><strong>Expected size:</strong> {def?.expected_size ? `${def.expected_size} bytes` : "any"} {selected.size ? `(actual: ${selected.size})` : ""}</div>
                <div><strong>SHA-256 when known:</strong> <span style={{ wordBreak: "break-all", fontSize: 11 }}>{(def?.hashes_sha256?.[0] || def?.variants[0]?.hashes_sha256?.[0] || "none (size-only or unknown)")}</span></div>
                <div><strong>Actual SHA-256:</strong> <span style={{ wordBreak: "break-all", fontSize: 11 }}>{selected.hash ? `${selected.hash.slice(0, 16)}… (${selected.hash})` : "-"}</span></div>
                <div><strong>Validation reason:</strong> {selected.reason}</div>
              </div>
            );
          })()}
        </div>
      </div>


    </div>
  );
}

function mockBiosScan(_source: string) {
  return {
    results: [
      { bios_id: "ps1_bios", system_id: "psx", state: "found_valid", reason: "exact filename + known hash", required: true, file: "C:\\BIOS\\scph5501.bin", hash: "abc123...", size: 524288, variant: "scph5501.bin" },
      { bios_id: "gba_bios", system_id: "gba", state: "found_valid", reason: "exact filename + known hash", required: true, file: "C:\\BIOS\\gba_bios.bin", hash: "a860a8...", size: 16384, variant: "gba_bios.bin" },
      { bios_id: "o2em_bios", system_id: "o2em", state: "missing", reason: "BIOS o2em_bios missing but required when o2em_content_present", required: true, file: null, hash: null, size: null },
    ],
  };
}
