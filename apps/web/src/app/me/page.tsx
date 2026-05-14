import Link from "next/link";

import { getCurrentUser } from "@/lib/auth";
import { SectionCard } from "@/components/section-card";
import { StatCard } from "@/components/stat-card";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { getMeDistribution, getMeOverview, getMeRewards, getMeTrend, getUserLeaderboard } from "@/lib/api";

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
    const users = await getUserLeaderboard("month").catch(() => null);

    return (
      <div className="stack-xl">
        <section className="page-heading">
          <p className="eyebrow">我的</p>
          <h1>请先登录账号</h1>
          <p>当前页面会优先读取当前浏览器中的登录会话。如果还没登录，请先使用自动生成的账号密码登录。</p>
        </section>

        <Card>
          <CardHeader>
            <div>
              <CardDescription>登录方式</CardDescription>
              <CardTitle>使用账号密码登录后进入个人页</CardTitle>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            <Link
              href="/login"
              className="inline-flex items-center justify-center rounded-full px-4 py-2.5 text-sm font-medium tracking-[0.02em] transition duration-200 border border-sky-200 bg-[linear-gradient(180deg,rgba(255,255,255,0.96),rgba(230,244,255,0.96))] text-slate-900 shadow-[inset_0_1px_0_rgba(255,255,255,0.9),0_16px_28px_rgba(148,163,184,0.14)] hover:bg-[linear-gradient(180deg,rgba(255,255,255,1),rgba(219,234,254,0.98))]"
            >
              前往登录
            </Link>
            {users && users.rows.length > 0 ? (
              <div className="space-y-3">
                <p className="text-sm leading-7 text-slate-600">
                  系统默认使用 `user_id` 作为账号。密码由服务端自动生成，管理员可通过接口查询；如果你需要临时查看其他真实用户，也可以直接从实时榜单进入。
                </p>
                <div className="flex flex-wrap gap-3">
                {users.rows.slice(0, 8).map((user) => (
                  <Link
                    key={user.user_id}
                    href={`/me?user_id=${encodeURIComponent(user.user_id)}`}
                    className="inline-flex rounded-full border border-sky-200/80 bg-white/80 px-4 py-2 text-sm text-slate-700 transition hover:border-sky-300 hover:bg-sky-50"
                  >
                    {user.display_name}
                  </Link>
                ))}
                </div>
              </div>
            ) : (
              <p className="text-sm leading-7 text-slate-600">当前还没有可用的实时用户榜单数据。</p>
            )}
          </CardContent>
        </Card>
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
          <p className="eyebrow">我的</p>
          <h1>当前无法读取个人实时数据</h1>
          <p>前端已禁用演示数据回退。请确认目标用户已存在于数据库中，并且对应事件已经成功写入。</p>
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
          <div className="flex flex-wrap gap-3">
            <Badge>个人画像</Badge>
          </div>
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
            value={overview.favorite_tool ?? "-"}
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
              label: item.label,
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
                <CardTitle>最近 30 天</CardTitle>
              </div>
            </CardHeader>
            <CardContent className="trend-bars">
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
    </div>
  );
}
