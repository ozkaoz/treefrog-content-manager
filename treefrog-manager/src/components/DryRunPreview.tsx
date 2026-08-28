import type { Plan, PlanEntry } from "../App";
import { useState } from "react";

const RESOLUTIONS = ["skip", "replace", "keep_both", "keep_destination", "keep_source"] as const;

function badgeClass(action: string) {
  if (action === "copy") return "badge-copy";
  if (action === "extract") return "badge-extract";
  if (action === "skip_duplicate" || action === "skip_unchanged" || action === "skip") return "badge-skip";
  if (action === "conflict") return "badge-conflict";
  if (action === "manual_review") return "badge-conflict";
  if (action === "unsupported_archive") return "badge-skip";
  if (action === "replace") return "badge-copy";
  return "badge-skip";
}

function applyResolutions(entries: PlanEntry[], decisions: Record<number, string>): PlanEntry[] {
  return entries.map((e, idx) => {
    const res = decisions[idx];
    if (!res) return e;
    const orig = e.action;
    const dest = e.destination;
    const copy = { ...e, resolution: res } as PlanEntry;
    // Mirror Python _apply_single_resolution logic (simplified)
    if (res === "skip" || res === "keep_destination") {
      copy.resolved_action = "skip";
      copy.reason = e.reason + ` [resolved: ${res}]`;
    } else if (res === "replace" || res === "keep_source") {
      copy.resolved_action = orig === "skip_duplicate" ? "copy" : "replace";
      copy.reason = e.reason + ` [resolved: ${res}]`;
    } else if (res === "keep_both") {
      const p = dest;
      const dot = p.lastIndexOf(".");
      const slash = p.lastIndexOf("/");
      let newDest: string;
      if (dot > slash) {
        newDest = p.slice(0, dot) + "_1" + p.slice(dot);
      } else {
        newDest = p + "_1";
      }
      copy.destination = newDest;
      copy.resolved_action = orig === "extract" ? "extract" : "copy";
      copy.reason = e.reason + " [resolved: keep_both -> renamed]";
      (copy as unknown as Record<string, unknown>)["original_destination"] = dest;
    }
    copy.resolution = res;
    return copy;
  });
}

export default function DryRunPreview({ plan, onResolve }: { plan: Plan; onResolve: (p: Plan) => void }) {
  const s = plan.summary;
  const [decisions, setDecisions] = useState<Record<number, string>>({});

  const entries = applyResolutions(plan.entries, decisions);

  const handleChange = (idx: number, value: string) => {
    const nd = { ...decisions, [idx]: value };
    if (value === "") delete nd[idx];
    setDecisions(nd);
    // Also produce a resolved plan for future SD writer (single source of truth)
    const resolvedEntries = applyResolutions(plan.entries, nd);
    const resolvedPlan = { ...plan, entries: resolvedEntries };
    onResolve(resolvedPlan);
  };

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
      <p style={{ fontSize: 12, color: "#555" }}>Planner is single source of truth for future SD writes. Defaults: exact duplicate → skip, same filename diff hash → conflict, alias duplicate → duplicate. All overrideable via per-entry resolution. No writes in preview.</p>

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
          {entries.map((e, i) => (
            <tr key={i} style={{ background: e.action === "conflict" || e.action === "manual_review" ? "#fff8e1" : undefined }}>
              <td style={{ fontSize: 11 }}>{i}</td>
              <td>
                <span className={`badge ${badgeClass(e.resolved_action || e.action)}`}>{e.resolved_action || e.action}</span>
                <div style={{ fontSize: 10, color: "#666" }}>{e.content_type || ""}</div>
                {e.default_action && e.default_action !== e.action && <div style={{ fontSize: 10, color: "#999" }}>default: {e.default_action}</div>}
              </td>
              <td style={{ maxWidth: 220, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 11 }} title={e.source}>{e.source}</td>
              <td style={{ maxWidth: 220, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 11 }} title={e.destination}>{e.destination}</td>
              <td style={{ fontSize: 11, maxWidth: 320 }}>
                <div>{e.reason}</div>
                {e.source_hash && <div style={{ fontSize: 10, color: "#555", wordBreak: "break-all" }}>src: {e.source_hash.slice(0, 16)}…</div>}
                {e.destination_hash && <div style={{ fontSize: 10, color: "#555", wordBreak: "break-all" }}>dst: {e.destination_hash.slice(0, 16)}…</div>}
                {e.members && e.members.length > 0 && <div style={{ fontSize: 10, color: "#1565c0" }}>members: {e.members.join(", ")}</div>}
                {e.group && !e.members && <div style={{ fontSize: 10, color: "#1565c0" }}>group: {e.group.join(", ")}</div>}
              </td>
              <td style={{ fontSize: 11 }}>
                {(e.action === "conflict" || e.action === "skip_duplicate" || e.action === "manual_review" || e.action === "skip_unchanged") ? (
                  <select value={decisions[i] || ""} onChange={(ev) => handleChange(i, ev.target.value)} style={{ fontSize: 11, padding: "2px 4px" }}>
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
          ))}
        </tbody>
      </table>

      {plan.warnings.length > 0 && (
        <ul style={{ fontSize: 12, color: "#8a6d3b", background: "#fdf6e3", padding: "8px 16px", borderRadius: 6 }}>
          {plan.warnings.map((w, i) => <li key={i}>{w}</li>)}
        </ul>
      )}

      <p style={{ fontSize: 12, color: "#2e7d32" }}>Preview complete — no files were written. Resolved plan will be executed by future SD writer (planner is single source of truth, not recomputed).</p>
    </div>
  );
}
