import { LeaderboardTable } from "@/components/leaderboard-table";
import { StatCard } from "@/components/stat-card";
import {
  getGrowthLeaderboard,
  getModelLeaderboard,
  getTeamLeaderboard,
  getToolLeaderboard,
  getUserLeaderboard,
} from "@/lib/api";

export default async function LeaderboardsPage() {
  const [users, teams, tools, models, growth] = await Promise.all([
    getUserLeaderboard("month"),
    getTeamLeaderboard("month"),
    getToolLeaderboard("month"),
    getModelLeaderboard("month"),
    getGrowthLeaderboard("month"),
  ]);

  return (
    <div className="stack-xl">
      <section className="page-heading">
        <p className="eyebrow">Leaderboards</p>
        <h1>Multi-dimensional rankings for users, teams, tools and models.</h1>
        <p>
          The backend exposes separate ranking endpoints so the page can stay server-rendered
          while still supporting period and filter expansion later.
        </p>
      </section>

      <section className="filter-strip">
        <span className="filter-pill">Period: {users.period}</span>
        <span className="filter-pill">Refresh: live API with demo fallback</span>
        <span className="filter-pill">Growth window: previous equal period</span>
      </section>

      <section className="stats-grid">
        <StatCard
          label="Top User"
          value={users.rows[0]?.display_name ?? "-"}
          caption={`${users.rows[0]?.total_tokens.toLocaleString() ?? "0"} tokens`}
        />
        <StatCard
          label="Top Team"
          value={teams.rows[0]?.team_name ?? "-"}
          caption={`${teams.rows[0]?.total_tokens.toLocaleString() ?? "0"} tokens`}
        />
        <StatCard
          label="Top Tool"
          value={tools.rows[0]?.tool ?? "-"}
          caption={`${tools.rows[0]?.total_tokens.toLocaleString() ?? "0"} tokens`}
        />
        <StatCard
          label="Top Model"
          value={models.rows[0]?.model ?? "-"}
          caption={`${models.rows[0]?.total_tokens.toLocaleString() ?? "0"} tokens`}
        />
      </section>

      <section className="two-column">
        <div className="panel">
          <div className="panel-header">
            <div>
              <p className="eyebrow">Users</p>
              <h2>Personal leaderboard</h2>
            </div>
          </div>
          <LeaderboardTable
            headers={["Rank", "User", "Team", "Tokens", "Cost"]}
            rows={users.rows.map((row) => [
              `#${row.rank}`,
              row.display_name,
              row.team_name,
              row.total_tokens.toLocaleString(),
              `$${row.total_cost_usd.toFixed(2)}`,
            ])}
          />
        </div>
        <div className="panel">
          <div className="panel-header">
            <div>
              <p className="eyebrow">Teams</p>
              <h2>Team leaderboard</h2>
            </div>
          </div>
          <LeaderboardTable
            headers={["Rank", "Team", "Members", "Tokens", "Per Capita"]}
            rows={teams.rows.map((row) => [
              `#${row.rank}`,
              row.team_name,
              row.member_count.toString(),
              row.total_tokens.toLocaleString(),
              row.avg_tokens_per_member.toFixed(1),
            ])}
          />
        </div>
      </section>

      <section className="two-column">
        <div className="panel">
          <div className="panel-header">
            <div>
              <p className="eyebrow">Tools & Models</p>
              <h2>Usage concentration</h2>
            </div>
          </div>
          <LeaderboardTable
            headers={["Type", "Label", "Tokens", "Active Users"]}
            rows={[
              ...tools.rows.map((row) => [
                "Tool",
                row.tool,
                row.total_tokens.toLocaleString(),
                row.active_users.toString(),
              ]),
              ...models.rows.map((row) => [
                "Model",
                row.model,
                row.total_tokens.toLocaleString(),
                row.active_users.toString(),
              ]),
            ]}
          />
        </div>
        <div className="panel">
          <div className="panel-header">
            <div>
              <p className="eyebrow">Growth</p>
              <h2>Momentum ranking</h2>
            </div>
          </div>
          <LeaderboardTable
            headers={["Rank", "Subject", "Current", "Previous", "Growth"]}
            rows={growth.rows.map((row) => [
              `#${row.rank}`,
              row.subject,
              row.current_total_tokens.toLocaleString(),
              row.previous_total_tokens.toLocaleString(),
              `${row.growth_tokens >= 0 ? "+" : ""}${row.growth_tokens.toLocaleString()}`,
            ])}
          />
        </div>
      </section>
    </div>
  );
}
