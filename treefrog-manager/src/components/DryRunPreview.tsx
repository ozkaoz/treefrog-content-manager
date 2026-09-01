import type { Plan } from "../App";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";

const RESOLUTIONS = ["skip", "replace", "keep_both", "keep_destination", "keep_source"] as const;

function badgeClass(action: string) {
  if (action === "copy") return "badge-copy";
  if (action === "extract") return "badge-extract";
  if (action === "convert_then_copy") return "badge-copy";
  if (action === "skip_duplicate" || action === "skip_unchanged" || action === "skip") return "badge-skip";
  if (action === "conflict") return "badge-conflict";
  if (action === "manual_review") return "badge-conflict";
  if (action === "unsupported_archive" || action === "unsupported") return "badge-skip";
  if (action === "conversion_error") return "badge-conflict";
  if (action === "replace") return "badge-copy";
  return "badge-skip";
}

/**
 * THIN frontend: presents the plan, collects the user's resolution choice,
 * sends it to the BACKEND (resolve_plan), and displays the backend result.
 * ALL business rules (resolve, rename, effective action, collisions) live in
 * Rust — this component never reimplements resolution semantics.
 */
export default function DryRunPreview({
  plan,
  sdPath,
  onResolve,
}: {
  plan: Plan;
  sdPath: string;
  onResolve: (p: Plan) => void;
}) {
  const s = plan.summary;
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const handleChange = async (idx: number, value: string) => {
    const decisions: Record<string, string> = {};
    if (value !== "") decisions[String(idx)] = value;
    setBusy(true);
    setError("");
    try {
      // Backend is the authority: it resolves, renames (collision-safe
      // keep_both), recomputes effective actions, and re-validates.
      const resolved = (await invoke("resolve_plan", {
        plan,
        sdPath,
        decisions,
      })) as Plan;
      onResolve(resolved);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleDestinationChange = async (source: string, dest: string) => {
    // Ambiguous-destination choice is a plain destination update applied by
    // the backend planner at deploy time via user_decisions (validated there).
    const updatedEntries = plan.entries.map((e) =>
      e.source === source ? { ...e, destination: `roms/${dest}` } : e
    );
    onResolve({ ...plan, entries: updatedEntries });
  };

  const [filter, setFilter] = useState<"all" | "bios" | "video" | "rom">("all");
  const filteredEntries = plan.entries.filter((e) => {
    if (filter === "bios") return e.content_type === "bios" || e.destination.includes("cubegm/bios");
    if (filter === "video") return e.content_type === "video";
    if (filter === "rom") return e.content_type?.startsWith("rom/") || e.content_type?.startsWith("grouped");
    return true;
  });

  return (
    <div>
      <div className="summary" style={{ marginTop: 8 }}>
        <span>{s.unchanged} unchanged</span>
        <span>{s.new} new</span>
        <span>{s.changed} changed</span>
        <span>{s.duplicate_content} duplicate</span>
        <span>{s.conflicts} conflicts</span>
        <span>{s.manual_review ?? 0} manual review</span>
        <span>{s.unsupported_archive ?? 0} unsupported</span>
        <span>{s.deletions} deletions</span>
      </div>
      <p style={{ fontSize: 12, color: "#555" }}>
        Planner is the single source of truth. Resolutions are computed by the backend
        (collision-safe keep_both, canonical path validation); this preview never writes files.
      </p>
      {error && <div className="status-error" style={{ fontSize: 12, marginBottom: 8 }}>{error}</div>}
      <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
        <span style={{ fontSize: 11, color: "#555" }}>Filter:</span>
        {(["all", "bios", "video", "rom"] as const).map((f) => (
          <button key={f} onClick={() => setFilter(f)} style={{ padding: "2px 8px", fontSize: 11, background: filter === f ? "#e3f2fd" : "#f5f5f5", border: "1px solid #ccc", borderRadius: 4 }}>{f}</button>
        ))}
        <span style={{ fontSize: 11, color: "#777", marginLeft: 8 }}>{filteredEntries.length} of {plan.entries.length} entries</span>
        {busy && <span style={{ fontSize: 11, color: "#777" }}>resolving…</span>}
      </div>

      <table>
        <thead>
          <tr>
            <th>#</th>
            <th>Status / Action</th>
            <th>Source</th>
            <th>Destination</th>
            <th>Reason / Hashes / Members</th>
            <th>Resolution</th>
          </tr>
        </thead>
        <tbody>
          {filteredEntries.map((e) => {
            const origIdx = plan.entries.indexOf(e);
            return (
            <tr key={origIdx} style={{ background: e.action === "conflict" || e.action === "manual_review" ? "#fff8e1" : undefined }}>
              <td style={{ fontSize: 11 }}>{origIdx}</td>
              <td>
                <span className={`badge ${badgeClass(e.resolved_action || e.action)}`}>{e.resolved_action || e.action}</span>
                <div style={{ fontSize: 10, color: "#666" }}>{e.content_type || ""}</div>
                {e.default_action && e.default_action !== e.action && <div style={{ fontSize: 10, color: "#999" }}>default: {e.default_action}</div>}
              </td>
              <td style={{ maxWidth: 220, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 11 }} title={e.source}>{e.source}</td>
              <td style={{ maxWidth: 220, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 11 }} title={e.destination}>
                {e.destination}
                {e.original_destination && (
                  <div style={{ fontSize: 10, color: "#999" }}>was: {e.original_destination}</div>
                )}
                {e.kind === 'ambiguous' && (
                  <div style={{ marginTop: 4 }}>
                    <select value="" onChange={(ev) => handleDestinationChange(e.source, ev.target.value)} style={{ fontSize: 11, padding: "2px 4px", width: "100%" }}>
                      <option value="">Select destination...</option>
                      {e.possible_destinations?.map(dest => (
                        <option key={dest} value={dest}>{dest}</option>
                      ))}
                    </select>
                  </div>
                )}
              </td>
              <td style={{ fontSize: 11, maxWidth: 320 }}>
                <div>{e.reason}</div>
                {e.source_hash && <div style={{ fontSize: 10, color: "#555", wordBreak: "break-all" }}>src: {e.source_hash.slice(0, 16)}…</div>}
                {e.destination_hash && <div style={{ fontSize: 10, color: "#555", wordBreak: "break-all" }}>dst: {e.destination_hash.slice(0, 16)}…</div>}
                {e.converted_name && <div style={{ fontSize: 10, color: "#2e7d32" }}>converted: {e.converted_name} (staged, validated with ffprobe before deploy)</div>}
                {e.members && e.members.length > 0 && <div style={{ fontSize: 10, color: "#1565c0" }}>members: {e.members.join(", ")}</div>}
                {e.content_type === "video" && <div style={{ fontSize: 10, color: "#666" }}>original never modified; staged output only, ffprobe-validated</div>}
                {e.content_type === "bios" && <div style={{ fontSize: 10, color: "#666" }}>BIOS: user-supplied only, never downloaded</div>}
              </td>
              <td style={{ fontSize: 11 }}>
                {(e.action === "conflict" || e.action === "skip_duplicate" || e.action === "manual_review" || e.action === "skip_unchanged" || e.content_type === "bios") ? (
                  <select
                    value=""
                    disabled={busy}
                    onChange={(ev) => handleChange(origIdx, ev.target.value)}
                    style={{ fontSize: 11, padding: "2px 4px" }}
                  >
                    <option value="">{e.resolution || e.action} (default)</option>
                    {RESOLUTIONS.map((r) => (
                      <option key={r} value={r}>{r}</option>
                    ))}
                  </select>
                ) : (
                  <span style={{ fontSize: 10, color: "#999" }}>{e.resolution || e.action}</span>
                )}
              </td>
            </tr>
          )})}
        </tbody>
      </table>

      {plan.warnings.length > 0 && (
        <ul style={{ fontSize: 12, color: "#8a6d3b", background: "#fdf6e3", padding: "8px 16px", borderRadius: 6 }}>
          {plan.warnings.map((w, i) => <li key={i}>{w}</li>)}
        </ul>
      )}

      <p style={{ fontSize: 12, color: "#2e7d32" }}>Preview complete — no files were written. Resolutions are executed by the backend writer with canonical path validation.</p>
    </div>
  );
}
