import { SectionCard } from "@/components/section-card";
import { StatCard } from "@/components/stat-card";
import { getMeDistribution, getMeOverview, getMeRewards, getMeTrend } from "@/lib/api";

export default async function MePage() {
  const [overview, distribution, trend, rewards] = await Promise.all([
    getMeOverview("u_alice"),
    getMeDistribution("u_alice"),
    getMeTrend("u_alice"),
    getMeRewards("u_alice"),
  ]);

  return (
    <div className="stack-xl">
      <section className="page-heading">
        <p className="eyebrow">Me</p>
        <h1>{overview.display_name}&apos;s AI usage profile.</h1>
        <p>
          This page composes the dedicated personal endpoints for overview, trend, distribution
          and reward snapshots.
        </p>
      </section>

      <section className="stats-grid">
        <StatCard
          label="Monthly Tokens"
          value={overview.total_tokens.toLocaleString()}
          caption={`${overview.request_count} requests in the current month`}
        />
        <StatCard
          label="Estimated Cost"
          value={`$${overview.total_cost_usd.toFixed(2)}`}
          caption="Historical model pricing fallback"
        />
        <StatCard
          label="Favorite Tool"
          value={overview.favorite_tool ?? "-"}
          caption="Highest token share this month"
        />
        <StatCard
          label="Favorite Model"
          value={overview.favorite_model ?? "-"}
          caption="Highest token share this month"
        />
      </section>

      <section className="two-column">
        <SectionCard
          title="Tool Distribution"
          subtitle="Token share by tool"
          items={distribution.tools.map((item) => ({
            label: item.label,
            value: `${item.total_tokens.toLocaleString()} · ${(item.share * 100).toFixed(1)}%`,
          }))}
        />
        <SectionCard
          title="Model Distribution"
          subtitle="Token share by model"
          items={distribution.models.map((item) => ({
            label: item.label,
            value: `${item.total_tokens.toLocaleString()} · ${(item.share * 100).toFixed(1)}%`,
          }))}
        />
      </section>

      <section className="two-column">
        <div className="panel">
          <div className="panel-header">
            <div>
              <p className="eyebrow">Trend</p>
              <h2>Last 30 days</h2>
            </div>
          </div>
          <div className="trend-bars">
            {trend.points.map((point) => (
              <div key={point.date} className="trend-row">
                <span>{point.date}</span>
                <div className="trend-bar-track">
                  <div
                    className="trend-bar-fill"
                    style={{
                      width: `${Math.max(
                        10,
                        (point.total_tokens /
                          Math.max(...trend.points.map((entry) => entry.total_tokens), 1)) *
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
        <SectionCard
          title="Rewards"
          subtitle="Display-only first version"
          items={rewards.rewards.map((reward) => ({
            label: reward.label,
            value: `${reward.granted_at} · ${reward.description}`,
          }))}
        />
      </section>
    </div>
  );
}
