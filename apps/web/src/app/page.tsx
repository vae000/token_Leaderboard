import { LeaderboardTable } from "@/components/leaderboard-table";
import { SectionCard } from "@/components/section-card";
import { StatCard } from "@/components/stat-card";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { formatToolName } from "@/lib/utils";
import { getDashboardSummary, getUserLeaderboard } from "@/lib/api";

export const dynamic = "force-dynamic";

export default async function Home() {
  const result = await Promise.all([
      getDashboardSummary(),
      getUserLeaderboard("month"),
    ])
    .then(([summary, users]) => ({ summary, users, error: null as string | null }))
    .catch((error: unknown) => ({
      summary: null,
      users: null,
      error: error instanceof Error ? error.message : "unknown error",
    }));

  if (result.error || !result.summary || !result.users) {
    return (
      <div className="stack-xl">
        <section className="page-heading">
          <h1>当前暂无实时数据</h1>
          <p>请确认 API 服务已启动、数据库可连通，并且已经有真实事件写入。</p>
        </section>
        <Card>
          <CardHeader>
            <div>
              <CardDescription>接口状态</CardDescription>
              <CardTitle>数据请求失败</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <p className="text-sm leading-7 text-slate-600">{result.error ?? "unknown error"}</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  const { summary, users } = result;

  return (
    <div className="stack-xl">
      <section className="page-heading">
        <h1>
          <span className="text-gradient">总览</span>
        </h1>
        <p>
          Agent 工具热度、模型成本与团队活跃度，一屏看清。
        </p>
      </section>

      <section className="kpi-grid">
        <div className="hero-metric rounded-[22px] p-4">
          <p className="widget-label">工具热点</p>
          <strong className="mt-2 block text-lg font-semibold text-slate-900">
            {formatToolName(summary.top_tools[0]?.label ?? "-")}
          </strong>
        </div>
        <div className="hero-metric rounded-[22px] p-4">
          <p className="widget-label">模型热点</p>
          <strong className="mt-2 block text-lg font-semibold text-slate-900">
            {summary.top_models[0]?.label ?? "-"}
          </strong>
        </div>
      </section>

      <section className="stats-grid">
        <StatCard
          label="累计 Token"
          value={summary.total_tokens.toLocaleString()}
          caption="输入、输出与缓存 Token 汇总后的全局体量。"
          color="blue"
        />
        <StatCard
          label="周活跃用户"
          value={summary.weekly_active_users.toString()}
          caption="最近 7 天内实际产生 AI 使用事件的去重用户。"
          color="purple"
        />
        <StatCard
          label="月度成本"
          value={`$${summary.monthly_cost_usd.toFixed(2)}`}
          caption="基于模型价格表换算的月度成本估算值。"
          color="emerald"
        />
        <StatCard
          label="AI 渗透率"
          value={`${(summary.ai_penetration_rate * 100).toFixed(1)}%`}
          caption="最近 30 天活跃用户在总用户中的占比。"
          color="amber"
        />
      </section>

      <section className="two-column">
        <SectionCard
          title="热门工具"
          subtitle="按 Token 总量排序"
          items={summary.top_tools.map((item) => ({
            label: formatToolName(item.label),
            value: Math.round(item.value).toLocaleString(),
          }))}
        />
        <SectionCard
          title="热门模型"
          subtitle="按 Token 总量排序"
          items={summary.top_models.map((item) => ({
            label: item.label,
            value: Math.round(item.value).toLocaleString(),
          }))}
        />
      </section>

      <section className="two-column">
        <Card>
          <CardHeader>
            <div>
              <CardDescription>15 天趋势</CardDescription>
              <CardTitle>每日 Token 使用量</CardTitle>
            </div>
          </CardHeader>
          <CardContent className="trend-bars">
            {summary.trend.slice(-15).reverse().map((point) => (
              <div key={point.date} className="trend-row">
                <span>{point.date}</span>
                <div className="trend-bar-track">
                  <div
                    className="trend-bar-fill"
                    style={{
                      width: `${Math.max(
                        10,
                        (point.total_tokens /
                          Math.max(...summary.trend.slice(-15).map((entry) => entry.total_tokens), 1)) *
                          100,
                      )}%`,
                    }}
                  />
                </div>
                <strong className="text-right text-slate-900">{point.total_tokens.toLocaleString()}</strong>
              </div>
            ))}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <div>
              <CardDescription>榜单快照</CardDescription>
              <CardTitle>本月用户 Top 榜</CardTitle>
            </div>
          </CardHeader>
          <CardContent className="p-0">
            <LeaderboardTable
              headers={["排名", "用户", "团队", "Token", "请求数"]}
              rows={users.rows.slice(0, 5).map((row) => [
                `#${row.rank}`,
                row.display_name,
                row.team_name,
                row.total_tokens.toLocaleString(),
                row.requests.toString(),
              ])}
            />
          </CardContent>
        </Card>
      </section>
    </div>
  );
}
