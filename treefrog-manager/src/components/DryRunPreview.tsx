import type { Plan } from "../App";

export default function DryRunPreview({ plan }: { plan: Plan }) {
  const s = plan.summary;
  return (
    <div>
      <div className="summary" style={{ marginTop: 8 }}>
        <span>{s.unchanged} unchanged</span>
        <span>{s.new} new</span>
        <span>{s.changed} changed</span>
        <span>{s.duplicate_content} duplicate content</span>
        <span>{s.conflicts} conflicts</span>
        <span>{s.deletions} deletions</span>
      </div>
      <p style={{ fontSize: 12, color: "#555" }}>Normal Sync never deletes; deletion is explicit. Staging + atomic rename where supported.</p>

      <table>
        <thead>
          <tr>
            <th>Source</th>
            <th>Destination</th>
            <th>Action</th>
            <th>Reason</th>
          </tr>
        </thead>
        <tbody>
          {plan.entries.map((e, i) => (
            <tr key={i}>
              <td style={{ maxWidth: 260, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={e.source}>{e.source}</td>
              <td style={{ maxWidth: 260, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={e.destination}>{e.destination}</td>
              <td><span className={`badge badge-${e.action === "copy" ? "copy" : e.action === "extract" ? "extract" : e.action.startsWith("skip") ? "skip" : e.action === "conflict" ? "conflict" : "skip"}`}>{e.action}</span></td>
              <td style={{ fontSize: 11 }}>{e.reason}{e.group ? ` — group: ${e.group.join(", ")}` : ""}</td>
            </tr>
          ))}
        </tbody>
      </table>

      {plan.warnings.length > 0 && (
        <ul style={{ fontSize: 12, color: "#8a6d3b", background: "#fdf6e3", padding: "8px 16px", borderRadius: 6 }}>
          {plan.warnings.map((w, i) => <li key={i}>{w}</li>)}
        </ul>
      )}

      <p style={{ fontSize: 12, color: "#2e7d32" }}>Preview complete — no files were written. Press Sync (Phase 2) to apply.</p>
    </div>
  );
}
