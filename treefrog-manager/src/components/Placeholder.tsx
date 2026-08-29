import EmptyState from "./EmptyState";

export default function Placeholder({ title, label = "Coming in a future release" }: { title: string; label?: string }) {
  return (
    <div className="card">
      <h3>{title}</h3>
      <div className="placeholder">
        <EmptyState kind="not_implemented" title={label} description={`${title} will be available in a future release. No functionality is placeholder.`} />
      </div>
    </div>
  );
}
