import Link from "next/link";

import { getCurrentUser } from "@/lib/auth";
import { SectionCard } from "@/components/section-card";
import { StatCard } from "@/components/stat-card";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { formatToolName } from "@/lib/utils";
import { TeamSettings } from "@/components/team-settings";
import { getMeDistribution, getMeOverview, getMeRewards, getMeTrend } from "@/lib/api";

export const dynamic = "force-dynamic";

type MePageProps = {
  searchParams?: Promise<{
    user_id?: string | string[];
  }>;
};

export default async function MePage({ searchParams }: MePageProps) {
  const resolvedSearchParams = (await searchParams) ?? {};
  const currentUser = await getCurrentUser();
  const selectedUserId =
    typeof resolvedSearchParams.user_id === "string" ? resolvedSearchParams.user_id : undefined;
  const effectiveUserId = selectedUserId ?? currentUser?.user_id;

  if (!effectiveUserId) {
    return (
      <div className="stack-xl">
        <section className="mx-auto w-full max-w-md pt-12">
          <Card className="text-center">
            <CardContent className="space-y-4 pt-8">
              <div className="mx-auto flex h-14 w-14 items-center justify-center rounded-2xl border border-sky-100 bg-sky-50">
                <svg
                  className="h-7 w-7 text-sky-500"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth={1.5}
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    d="M16.5 10.5V6.75a4.5 4.5 0 1 0-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 0 0 2.25-2.25v-6.75a2.25 2.25 0 0 0-2.25-2.25H6.75a2.25 2.25 0 0 0-2.25 2.25v6.75a2.25 2.25 0 0 0 2.25 2.25Z"
                  />
                </svg>
              </div>
              <div>
                <p className="text-sm font-semibold text-slate-800">登录后查看个人数据</p>
                <p className="mt-1 text-sm leading-6 text-slate-500">
                  登录后可查看 Token 用量、成本估算、工具模型偏好和趋势。
                </p>
              </div>
              <Link
                href="/login"
                className="inline-flex items-center justify-center rounded-full px-6 py-3 text-sm font-medium tracking-[0.02em] transition duration-200 border border-sky-200 bg-[linear-gradient(180deg,rgba(255,255,255,0.96),rgba(230,244,255,0.96))] text-slate-900 shadow-[inset_0_1px_0_rgba(255,255,255,0.9),0_16px_28px_rgba(148,163,184,0.14)] hover:bg-[linear-gradient(180deg,rgba(255,255,255,1),rgba(219,234,254,0.98))]"
              >
                前往登录
              </Link>
            </CardContent>
          </Card>
        </section>
      </div>
    );
  }

  const result = await Promise.all([
      getMeOverview(effectiveUserId),
      getMeDistribution(effectiveUserId),
      getMeTrend(effectiveUserId),
      getMeRewards(effectiveUserId),
    ])
    .then(([overview, distribution, trend, rewards]) => ({
      overview,
      distribution,
      trend,
      rewards,
      error: null as string | null,
    }))
    .catch((error: unknown) => ({
      overview: null,
      distribution: null,
      trend: null,
      rewards: null,
      error: error instanceof Error ? error.message : "unknown error",
    }));

  if (!result.overview || !result.distribution || !result.trend || !result.rewards || result.error) {
    return (
      <div className="stack-xl">
        <section className="page-heading">
          <h1>当前无法读取个人实时数据</h1>
          <p>请确认目标用户已存在于数据库中，并且对应事件已经成功写入。</p>
        </section>
        <Card>
          <CardHeader>
            <div>
              <CardDescription>接口状态</CardDescription>
              <CardTitle>个人数据请求失败</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <p className="text-sm leading-7 text-slate-600">
              {result.error === "request failed with 404"
                ? `未找到用户 ${effectiveUserId}。请检查 user_id 是否真实存在。`
                : result.error ?? "unknown error"}
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  const { overview, distribution, trend, rewards } = result;

  return (
    <div className="stack-xl">
      <section className="page-heading">
        <h1>
          <span className="text-gradient">概览</span>
        </h1>
      </section>

      <section className="stats-grid">
        <StatCard
          label="本月 Token"
          value={overview.total_tokens.toLocaleString()}
          caption={`本月共 ${overview.request_count} 次请求`}
        />
        <StatCard
          label="预估成本"
          value={`$${overview.total_cost_usd.toFixed(2)}`}
          caption="按历史模型价格规则估算"
        />
        <StatCard
          label="偏好工具"
          value={overview.favorite_tool ? formatToolName(overview.favorite_tool) : "-"}
          caption="本月 Token 占比最高的工具"
        />
        <StatCard
          label="偏好模型"
          value={overview.favorite_model ?? "-"}
          caption="本月 Token 占比最高的模型"
        />
      </section>

      <section className="two-column">
        <SectionCard
          title="工具分布"
          subtitle="按工具统计 Token 占比"
          items={distribution.tools.map((item) => ({
            label: formatToolName(item.label),
            value: `${item.total_tokens.toLocaleString()} · ${(item.share * 100).toFixed(1)}%`,
          }))}
        />
        <SectionCard
          title="模型分布"
          subtitle="按模型统计 Token 占比"
          items={distribution.models.map((item) => ({
            label: item.label,
            value: `${item.total_tokens.toLocaleString()} · ${(item.share * 100).toFixed(1)}%`,
          }))}
        />
      </section>

      <section className="two-column">
        <Card>
          <CardHeader>
            <div>
              <CardDescription>趋势</CardDescription>
              <CardTitle>最近 15 天</CardTitle>
            </div>
          </CardHeader>
          <CardContent className="trend-bars">
            {trend.points.slice(-15).reverse().map((point) => (
              <div key={point.date} className="trend-row">
                <span>{point.date}</span>
                <div className="trend-bar-track">
                  <div
                    className="trend-bar-fill"
                    style={{
                      width: `${Math.max(
                        10,
                        (point.total_tokens /
                          Math.max(...trend.points.slice(-15).map((entry) => entry.total_tokens), 1)) *
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
        <SectionCard
          title="奖励"
          subtitle="数据库结果"
          items={rewards.rewards.map((reward) => ({
            label: reward.label,
            value: `${reward.granted_at} · ${reward.description}`,
          }))}
        />
      </section>

      <section className="two-column">
        <div />
        <TeamSettings userId={effectiveUserId} currentTeamName={overview.team_name} />
      </section>
    </div>
  );
}
