import { LeaderboardTable } from "@/components/leaderboard-table";
import { StatCard } from "@/components/stat-card";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  getGrowthLeaderboard,
  getModelLeaderboard,
  getTeamLeaderboard,
  getToolLeaderboard,
  getUserLeaderboard,
} from "@/lib/api";

export const dynamic = "force-dynamic";

function periodLabel(period: string) {
  switch (period) {
    case "today":
      return "今天";
    case "week":
      return "本周";
    case "month":
      return "本月";
    case "custom":
      return "自定义";
    case "all":
      return "全部";
    default:
      return period;
  }
}

export default async function LeaderboardsPage() {
  const result = await Promise.all([
      getUserLeaderboard("month"),
      getTeamLeaderboard("month"),
      getToolLeaderboard("month"),
      getModelLeaderboard("month"),
      getGrowthLeaderboard("month"),
    ])
    .then(([users, teams, tools, models, growth]) => ({
      users,
      teams,
      tools,
      models,
      growth,
      error: null as string | null,
    }))
    .catch((error: unknown) => ({
      users: null,
      teams: null,
      tools: null,
      models: null,
      growth: null,
      error: error instanceof Error ? error.message : "unknown error",
    }));

  if (
    result.error ||
    !result.users ||
    !result.teams ||
    !result.tools ||
    !result.models ||
    !result.growth
  ) {
    return (
      <div className="stack-xl">
        <section className="page-heading">
          <p className="eyebrow">排行榜</p>
          <h1>当前无法读取实时排行</h1>
          <p>前端已禁用演示数据回退。请确认 API 已启动，并且数据库里已经存在真实事件与用户关系数据。</p>
        </section>
        <Card>
          <CardHeader>
            <div>
              <CardDescription>接口状态</CardDescription>
              <CardTitle>排行数据请求失败</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <p className="text-sm leading-7 text-slate-600">{result.error ?? "unknown error"}</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  const { users, teams, tools, models, growth } = result;

  return (
    <div className="stack-xl">
        <section className="page-heading">
          <div className="flex flex-wrap gap-3">
            <Badge>排行矩阵</Badge>
            <Badge variant="outline">多维视图</Badge>
          </div>
          <p className="eyebrow">排行榜</p>
          <h1>
            多维度
            <span className="text-gradient">排名矩阵</span>
          </h1>
          <p>
            这里将用户、团队、工具、模型和增长信号并排展开，用更轻盈清爽的方式展示 AI 使用排名全景。
          </p>
        </section>

        <section className="filter-strip">
          <span className="filter-pill">统计周期：{periodLabel(users.period)}</span>
          <span className="filter-pill">数据来源：仅实时接口</span>
          <span className="filter-pill">增长窗口：与上一个等长周期对比</span>
        </section>

        <section className="stats-grid">
          <StatCard
            label="榜首用户"
            value={users.rows[0]?.display_name ?? "-"}
            caption={`${users.rows[0]?.total_tokens.toLocaleString() ?? "0"} Token`}
          />
          <StatCard
            label="榜首团队"
            value={teams.rows[0]?.team_name ?? "-"}
            caption={`${teams.rows[0]?.total_tokens.toLocaleString() ?? "0"} Token`}
          />
          <StatCard
            label="榜首工具"
            value={tools.rows[0]?.tool ?? "-"}
            caption={`${tools.rows[0]?.total_tokens.toLocaleString() ?? "0"} Token`}
          />
          <StatCard
            label="榜首模型"
            value={models.rows[0]?.model ?? "-"}
            caption={`${models.rows[0]?.total_tokens.toLocaleString() ?? "0"} Token`}
          />
        </section>

        <section className="two-column">
          <Card>
            <CardHeader>
              <div>
                <CardDescription>用户</CardDescription>
                <CardTitle>个人排行</CardTitle>
              </div>
            </CardHeader>
            <CardContent className="p-0">
              <LeaderboardTable
                headers={["排名", "用户", "团队", "Token", "成本"]}
                rows={users.rows.map((row) => [
                  `#${row.rank}`,
                  row.display_name,
                  row.team_name,
                  row.total_tokens.toLocaleString(),
                  `$${row.total_cost_usd.toFixed(2)}`,
                ])}
              />
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <div>
                <CardDescription>团队</CardDescription>
                <CardTitle>团队排行</CardTitle>
              </div>
            </CardHeader>
            <CardContent className="p-0">
              <LeaderboardTable
                headers={["排名", "团队", "成员数", "Token", "人均"]}
                rows={teams.rows.map((row) => [
                  `#${row.rank}`,
                  row.team_name,
                  row.member_count.toString(),
                  row.total_tokens.toLocaleString(),
                  row.avg_tokens_per_member.toFixed(1),
                ])}
              />
            </CardContent>
          </Card>
        </section>

        <section className="two-column">
          <Card>
            <CardHeader>
              <div>
                <CardDescription>工具与模型</CardDescription>
                <CardTitle>使用集中度</CardTitle>
              </div>
            </CardHeader>
            <CardContent className="p-0">
              <LeaderboardTable
                headers={["类型", "名称", "Token", "活跃用户数"]}
                rows={[
                  ...tools.rows.map((row) => [
                    "工具",
                    row.tool,
                    row.total_tokens.toLocaleString(),
                    row.active_users.toString(),
                  ]),
                  ...models.rows.map((row) => [
                    "模型",
                    row.model,
                    row.total_tokens.toLocaleString(),
                    row.active_users.toString(),
                  ]),
                ]}
              />
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <div>
                <CardDescription>增长</CardDescription>
                <CardTitle>增长动量排行</CardTitle>
              </div>
            </CardHeader>
            <CardContent className="p-0">
              <LeaderboardTable
                headers={["排名", "对象", "当前周期", "上一周期", "增长值"]}
                rows={growth.rows.map((row) => [
                  `#${row.rank}`,
                  row.subject,
                  row.current_total_tokens.toLocaleString(),
                  row.previous_total_tokens.toLocaleString(),
                  `${row.growth_tokens >= 0 ? "+" : ""}${row.growth_tokens.toLocaleString()}`,
                ])}
              />
            </CardContent>
          </Card>
        </section>
    </div>
  );
}
