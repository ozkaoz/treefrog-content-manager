type Props = {
  kind?: "empty" | "loading" | "success" | "warning" | "error" | "not_implemented";
  title: string;
  description?: string;
  action?: React.ReactNode;
};

const ICON: Record<string, string> = {
  empty: "○",
  loading: "◐",
  success: "✓",
  warning: "⚠",
  error: "✕",
  not_implemented: "◈",
};

export default function EmptyState({ kind = "empty", title, description, action }: Props) {
  return (
    <div className="empty-state">
      <div style={{ fontSize: 28, marginBottom: 8 }}>{ICON[kind] ?? "○"}</div>
      <h4 style={{ margin: "0 0 6px 0" }}>{title}</h4>
      {description && <p style={{ margin: 0, fontSize: 13 }}>{description}</p>}
      {action && <div style={{ marginTop: 12 }}>{action}</div>}
    </div>
  );
}
