import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
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
  const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set());

  async function loadDefinitions() {
    try {
      const res = (await invoke("bios_profile")) as { definitions: BiosDefinition[] };
      setDefinitions(res.definitions || []);
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
      const res = (await invoke("bios_scan", { biosSource })) as { results: BiosValidation[] };
      setResults(res.results);
      if (res.results.length > 0) setSelected(res.results[0]);
      setSelectedFiles(new Set(res.results.filter(r => r.file).map(r => r.file!)));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  const toggleFileSelection = (file: string) => {
    setSelectedFiles(prev => {
      const ns = new Set(prev);
      if (ns.has(file)) ns.delete(file); else ns.add(file);
      return ns;
    });
  };
  const toggleAll = (checked: boolean) => {
    if (checked && results) setSelectedFiles(new Set(results.filter(r => r.file).map(r => r.file!)));
    else setSelectedFiles(new Set());
  };

  const lastScanKey = useRef("");
  useEffect(() => {
    const key = `${biosSource}|${globalSdPath}`;
    if (visible && biosSource && globalSdPath && key !== lastScanKey.current) {
      lastScanKey.current = key;
      handleScan();
    }
  }, [visible, biosSource, globalSdPath]);

  useEffect(() => {
    if (!results || results.length === 0) {
      onPlanChange?.(null);
      return;
    }
    
    // Convert BIOS validation results to Plan format - solo seleccionados
    const filteredResults = results.filter(r => !r.file || selectedFiles.has(r.file));
    const entries: PlanEntry[] = filteredResults.map(r => {
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
  }, [results, selectedFiles, onPlanChange]);

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
        SD destination: {globalSdPath || "—"} — the app will automatically copy to cubegm/bios/ according to TreeFrogUI profile.
      </p>

      <div className="row">
        <button className="primary" onClick={() => { lastScanKey.current = `${biosSource}|${globalSdPath}`; handleScan(); }} disabled={loading || !biosSource}>
          {loading ? "Scanning…" : "Scan BIOS"}
        </button>
        <button onClick={() => { setResults(null); setSelected(null); onPlanChange?.(null); }} disabled={!results}>
          Clear
        </button>
        <button onClick={() => onNext?.()} style={{ marginLeft: "auto" }}>
          Skip → LGPT
        </button>
        <button className="primary" onClick={() => onNext?.()} disabled={!biosSource && !results}>
          Continue to LGPT →
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
                  <th><input type="checkbox" checked={results.length > 0 && results.filter(r => r.file).every(r => selectedFiles.has(r.file!))} onChange={(e) => toggleAll(e.target.checked)} /></th>
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
                    <tr key={idx} onClick={() => setSelected(r)} style={{ cursor: "pointer", background: selected?.bios_id === r.bios_id ? "var(--surface)" : undefined, opacity: r.file && !selectedFiles.has(r.file) ? 0.5 : 1 }}>
                      <td><input type="checkbox" checked={r.file ? selectedFiles.has(r.file) : false} onChange={(e) => { e.stopPropagation(); if (r.file) toggleFileSelection(r.file); }} onClick={(e) => e.stopPropagation()} /></td>
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

