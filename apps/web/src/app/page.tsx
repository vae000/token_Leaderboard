import { LeaderboardTable } from "@/components/leaderboard-table";
import { SectionCard } from "@/components/section-card";
import { StatCard } from "@/components/stat-card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
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
          <p className="eyebrow">仪表盘</p>
          <h1>当前暂无实时数据</h1>
          <p>前端已禁用演示数据回退。请确认 API 服务已启动、数据库可连通，并且已经有真实事件写入。</p>
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
        <section className="dashboard-grid">
          <Card className="hero-panel p-8 md:p-10">
            <div className="space-y-6">
              <div className="flex flex-wrap gap-3">
                <Badge>实时总览</Badge>
                <Badge variant="outline">玻璃拟态中枢</Badge>
              </div>
              <div>
                <p className="eyebrow">仪表盘</p>
                <h1>
                  企业 AI 使用
                  <span className="text-gradient">榜单总览</span>
                </h1>
                <p className="hero-text">
                  后端将 CLI 采集到的原始事件转化为高频指标、排行视图和趋势信号，让团队在一个未来感数据界面里把使用情况看清楚。
                </p>
              </div>
              <div className="flex flex-wrap gap-3">
                <Button>实时数据视图</Button>
                <Button variant="ghost">已连接后端接口</Button>
              </div>
            </div>
          </Card>

          <div className="widget-stack">
            <div className="hero-soft-grid">
              <div className="widget-card pulse-widget">
                <p className="widget-label">Live Signal</p>
                <h3 className="widget-title">活跃脉冲</h3>
                <p className="widget-copy">用更轻盈的动态节奏展示当前数据状态，不做重工业感的可视化堆叠。</p>
                <div className="pulse-orbit">
                  <div className="pulse-core" />
                </div>
              </div>

              <div className="widget-stack">
                <div className="widget-card float-chip">
                  <div>
                    <p className="widget-label">Generated</p>
                    <strong className="mt-2 block text-lg font-semibold">
                      {new Date(summary.generated_at).toLocaleTimeString()}
                    </strong>
                  </div>
                  <span className="rounded-full bg-sky-100 px-3 py-1 text-xs text-sky-700">已刷新</span>
                </div>

                <div className="widget-card mini-bar-card">
                  <p className="widget-label">Trend Micro View</p>
                  <strong className="mt-2 block text-lg font-semibold">连续活跃走势</strong>
                  <div className="mini-bars" aria-hidden="true">
                    <span style={{ height: "42%" }} />
                    <span style={{ height: "58%" }} />
                    <span style={{ height: "76%" }} />
                    <span style={{ height: "52%" }} />
                    <span style={{ height: "68%" }} />
                    <span style={{ height: "88%" }} />
                  </div>
                </div>
              </div>
            </div>

            <div className="kpi-grid">
              <div className="hero-metric rounded-[22px] p-4">
                <p className="widget-label">工具热点</p>
                <strong className="mt-2 block text-lg font-semibold text-slate-900">
                  {summary.top_tools[0]?.label ?? "-"}
                </strong>
              </div>
              <div className="hero-metric rounded-[22px] p-4">
                <p className="widget-label">模型热点</p>
                <strong className="mt-2 block text-lg font-semibold text-slate-900">
                  {summary.top_models[0]?.label ?? "-"}
                </strong>
              </div>
            </div>
          </div>
        </section>

        <section className="stats-grid">
          <StatCard
            label="累计 Token"
            value={summary.total_tokens.toLocaleString()}
            caption="输入、输出与缓存 Token 汇总后的全局体量。"
          />
          <StatCard
            label="周活跃用户"
            value={summary.weekly_active_users.toString()}
            caption="最近 7 天内实际产生 AI 使用事件的去重用户。"
          />
          <StatCard
            label="月度成本"
            value={`$${summary.monthly_cost_usd.toFixed(2)}`}
            caption="基于模型价格表换算的月度成本估算值。"
          />
          <StatCard
            label="AI 渗透率"
            value={`${(summary.ai_penetration_rate * 100).toFixed(1)}%`}
            caption="最近 30 天活跃用户在总用户中的占比。"
          />
        </section>

        <section className="two-column">
          <SectionCard
            title="热门工具"
            subtitle="按 Token 总量排序"
            items={summary.top_tools.map((item) => ({
              label: item.label,
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
                <CardDescription>30 天趋势</CardDescription>
                <CardTitle>每日 Token 使用量</CardTitle>
              </div>
            </CardHeader>
            <CardContent className="trend-bars">
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
                  <strong className="text-right text-slate-900">{point.total_tokens.toLocaleString()}</strong>
                </div>
              ))}
            </CardContent>
          </Card>

          <div className="space-y-5">
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

            <Card className="bg-[linear-gradient(180deg,rgba(255,255,255,0.82),rgba(239,246,255,0.92))]">
              <CardContent className="space-y-3">
                <p className="font-mono text-[11px] uppercase tracking-[0.22em] text-sky-600/70">
                  运营视角
                </p>
                <h3 className="text-2xl font-semibold tracking-[-0.04em] text-slate-900">
                  用一张屏看清工具热度、模型成本与团队活跃度。
                </h3>
                <p className="text-sm leading-7 text-slate-600">
                  这个版本强调“官网质感 + 数据驾驶舱”的体验，既可以给管理层看，也适合挂在内部大屏。
                </p>
              </CardContent>
            </Card>
          </div>
        </section>
    </div>
  );
}
