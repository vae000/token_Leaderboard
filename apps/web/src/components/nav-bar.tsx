import Link from "next/link";

export function NavBar() {
  return (
    <header className="nav">
      <div className="nav-brand">
        <span>Token Leaderboard</span>
        <strong>AI usage dashboard</strong>
      </div>
      <nav className="nav-links">
        <Link className="nav-link" href="/">
          Dashboard
        </Link>
        <Link className="nav-link" href="/leaderboards">
          Leaderboards
        </Link>
        <Link className="nav-link" href="/me">
          Me
        </Link>
      </nav>
    </header>
  );
}
