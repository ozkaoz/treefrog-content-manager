import type { Plan, PlanEntry } from "../App";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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

async function applyResolutionsBackend(plan: Plan, decisions: Record<number, string>): Promise<PlanEntry[]> {
  try {
    const resolved: Plan = await invoke("resolve_plan", { plan, decisions });
    return resolved.entries;
  } catch {
    // Fallback to local simplified logic if backend unavailable (e.g., vite dev without Tauri)
    return plan.entries.map((e, idx) => {
      const res = decisions[idx];
      if (!res) return e;
      const orig = e.action;
      const dest = e.destination;
      const copy = { ...e, resolution: res } as PlanEntry;
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
}

function applyResolutionsSync(entries: PlanEntry[], decisions: Record<number, string>): PlanEntry[] {
  // Synchronous fallback for initial render — frontend must not be authority, backend will re-resolve on deploy
  return entries.map((e, idx) => {
    const res = decisions[idx];
    if (!res) return e;
    return { ...e, resolution: res, resolved_action: e.resolved_action || e.action } as PlanEntry;
  });
}

export default function DryRunPreview({ plan, onResolve }: { plan: Plan; onResolve: (p: Plan) => void }) {
  const s = plan.summary;
  const [decisions, setDecisions] = useState<Record<number, string>>({});
  const [selectedDestinations, setSelectedDestinations] = useState<Record<string, string>>({});

  const handleDestinationChange = (source: string, dest: string) => {
    const newSelected = { ...selectedDestinations, [source]: dest };
    setSelectedDestinations(newSelected);
    const updatedEntries = entries.map(e => e.source === source ? { ...e, destination: `roms/${dest}` } : e);
    const updatedPlan = { ...plan, entries: updatedEntries };
    onResolve(updatedPlan);
  };

  const entries = applyResolutionsSync(plan.entries, decisions);

  const handleChange = async (idx: number, value: string) => {
    const nd = { ...decisions, [idx]: value };
    if (value === "") delete nd[idx];
    setDecisions(nd);
    // Backend is authority for resolution semantics (planner::apply_resolutions)
    const resolvedEntries = await applyResolutionsBackend(plan, nd);
    const resolvedPlan = { ...plan, entries: resolvedEntries };
    onResolve(resolvedPlan);
  };

  const [filter, setFilter] = useState<"all" | "bios" | "video" | "rom">("all");
  const filteredEntries = entries.filter((e) => {
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
      <p style={{ fontSize: 12, color: "#555" }}>Planner is single source of truth for future SD writes (including BIOS). Defaults: exact duplicate → skip, same filename diff hash → conflict, alias duplicate → duplicate. All overrideable via per-entry resolution. No writes in preview. BIOS appears in global plan as <code>source C:\BIOS\scph5501.bin → cubegm/bios/scph5501.bin action copy reason required PS1 BIOS is valid</code> (or <code>manual_review</code> if missing/invalid).</p>
      <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
        <span style={{ fontSize: 11, color: "#555" }}>Filter:</span>
        {(["all", "bios", "video", "rom"] as const).map((f) => (
          <button key={f} onClick={() => setFilter(f)} style={{ padding: "2px 8px", fontSize: 11, background: filter === f ? "#e3f2fd" : "#f5f5f5", border: "1px solid #ccc", borderRadius: 4 }}>{f}</button>
        ))}
        <span style={{ fontSize: 11, color: "#777", marginLeft: 8 }}>{filteredEntries.length} of {entries.length} entries</span>
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
                {e.kind === 'ambiguous' && (
                  <div style={{ marginTop: 4 }}>
                    <select value={selectedDestinations[e.source] || ''} onChange={(ev) => handleDestinationChange(e.source, ev.target.value)} style={{ fontSize: 11, padding: "2px 4px", width: "100%" }}>
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
                {e.status && <div style={{ fontSize: 10, color: "#b7791f" }}>status: {e.status}{e.preset ? ` / preset: ${e.preset}` : ""}{e.preset === "treefrog_conservative_default" ? " (provisional)" : ""}</div>}
                {e.source_hash && <div style={{ fontSize: 10, color: "#555", wordBreak: "break-all" }}>src: {e.source_hash.slice(0, 16)}…</div>}
                {e.destination_hash && <div style={{ fontSize: 10, color: "#555", wordBreak: "break-all" }}>dst: {e.destination_hash.slice(0, 16)}…</div>}
                {e.probe && typeof e.probe === "object" && !!(e.probe as unknown as Record<string, string>).video_codec && <div style={{ fontSize: 10, color: "#555" }}>codec: {String((e.probe as unknown as Record<string, string>).video_codec)} / {String((e.probe as unknown as Record<string, string>).container)} {(e.probe as unknown as Record<string, string>).width ? ` / ${String((e.probe as unknown as Record<string, string>).width)}x${String((e.probe as unknown as Record<string, string>).height)}` : ""}</div>}
                {e.converted_name && <div style={{ fontSize: 10, color: "#2e7d32" }}>converted: {e.converted_name} (temp, validated with ffprobe)</div>}
                {e.members && e.members.length > 0 && <div style={{ fontSize: 10, color: "#1565c0" }}>members: {e.members.join(", ")}</div>}
                {e.group && !e.members && <div style={{ fontSize: 10, color: "#1565c0" }}>group: {e.group.join(", ")}</div>}
                {e.content_type === "video" && <div style={{ fontSize: 10, color: "#666" }}>original never modified; temp output only, deterministic naming</div>}
                {e.content_type === "bios" && <div style={{ fontSize: 10, color: "#666" }}>BIOS: user-supplied only, never downloaded</div>}
              </td>
              <td style={{ fontSize: 11 }}>
                {(e.action === "conflict" || e.action === "skip_duplicate" || e.action === "manual_review" || e.action === "skip_unchanged" || e.content_type === "bios") ? (
                  <select value={decisions[origIdx] || ""} onChange={(ev) => handleChange(origIdx, ev.target.value)} style={{ fontSize: 11, padding: "2px 4px" }}>
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

      <p style={{ fontSize: 12, color: "#2e7d32" }}>Preview complete — no files were written. Resolved plan will be executed by future SD writer (planner is single source of truth, not recomputed).</p>
    </div>
  );
}
