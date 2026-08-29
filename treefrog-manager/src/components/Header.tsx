import frogSquare from "../assets/branding/frog-square.png";

export default function Header() {
  return (
    <div className="brand-header">
      <img src={frogSquare} alt="TreeFrog frog" width={32} height={32} />
      <div>
        <h1>TreeFrog Content Manager</h1>
        <div style={{ fontSize: 12, color: "var(--text-muted)" }}>Global TreeFrogUI SD content — one schema for all handhelds</div>
      </div>
    </div>
  );
}
