import { LeaderboardTable } from "@/components/leaderboard-table";
import { SectionCard } from "@/components/section-card";
import { StatCard } from "@/components/stat-card";
import { getDashboardSummary, getUserLeaderboard } from "@/lib/api";

export default async function Home() {
  const [summary, users] = await Promise.all([
    getDashboardSummary(),
    getUserLeaderboard("month"),
  ]);

  return (
    <div className="stack-xl">
      <section className="hero-panel">
        <div className="hero-copy">
          <p className="eyebrow">Dashboard</p>
          <h1>Company AI usage, refreshed for leaderboard workflows.</h1>
          <p className="hero-text">
            The backend aggregates raw CLI events into dashboard metrics and ranking views.
            This page reads the summary API directly and stays functional with demo fallback data.
          </p>
        </div>
        <div className="hero-badge">
          <span>Generated</span>
          <strong>{new Date(summary.generated_at).toLocaleString()}</strong>
        </div>
      </section>

      <section className="stats-grid">
        <StatCard
          label="All-time Tokens"
          value={summary.total_tokens.toLocaleString()}
          caption="Input + output + cached tokens"
        />
        <StatCard
          label="Weekly Active Users"
          value={summary.weekly_active_users.toString()}
          caption="Distinct users over the last 7 days"
        />
        <StatCard
          label="Monthly Cost"
          value={`$${summary.monthly_cost_usd.toFixed(2)}`}
          caption="Estimated against model price table"
        />
        <StatCard
          label="AI Penetration"
          value={`${(summary.ai_penetration_rate * 100).toFixed(1)}%`}
          caption="Users active in the last 30 days"
        />
      </section>

      <section className="two-column">
        <SectionCard
          title="Top Tools"
          subtitle="Ranked by total token volume"
          items={summary.top_tools.map((item) => ({
            label: item.label,
            value: Math.round(item.value).toLocaleString(),
          }))}
        />
        <SectionCard
          title="Top Models"
          subtitle="Ranked by total token volume"
          items={summary.top_models.map((item) => ({
            label: item.label,
            value: Math.round(item.value).toLocaleString(),
          }))}
        />
      </section>

      <section className="two-column">
        <div className="panel">
          <div className="panel-header">
            <div>
              <p className="eyebrow">30 Day Trend</p>
              <h2>Daily token usage</h2>
            </div>
          </div>
          <div className="trend-bars">
            {summary.trend.map((point) => (
              <div key={point.date} className="trend-row">
                <span>{point.date}</span>
                <div className="trend-bar-track">
                  <div
                    className="trend-bar-fill"
                    style={{
                      width: `${Math.max(
                        10,
                        (point.total_tokens /
                          Math.max(...summary.trend.map((entry) => entry.total_tokens), 1)) *
                          100,
                      )}%`,
                    }}
                  />
                </div>
                <strong>{point.total_tokens.toLocaleString()}</strong>
              </div>
            ))}
          </div>
        </div>
        <div className="panel">
          <div className="panel-header">
            <div>
              <p className="eyebrow">Leaderboard Snapshot</p>
              <h2>Top users this month</h2>
            </div>
          </div>
          <LeaderboardTable
            headers={["Rank", "User", "Team", "Tokens", "Requests"]}
            rows={users.rows.slice(0, 5).map((row) => [
              `#${row.rank}`,
              row.display_name,
              row.team_name,
              row.total_tokens.toLocaleString(),
              row.requests.toString(),
            ])}
          />
        </div>
      </section>
    </div>
  );
}
