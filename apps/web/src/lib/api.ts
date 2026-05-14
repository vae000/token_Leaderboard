const PUBLIC_API_BASE_URL =
  process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://127.0.0.1:8080";

const INTERNAL_API_BASE_URL =
  process.env.API_INTERNAL_BASE_URL ?? PUBLIC_API_BASE_URL;

function getApiBaseUrl(): string {
  return typeof window === "undefined" ? INTERNAL_API_BASE_URL : PUBLIC_API_BASE_URL;
}

type MetricBreakdown = {
  label: string;
  value: number;
};

type TrendPoint = {
  date: string;
  total_tokens: number;
  total_cost_usd: number;
};

type LeaderboardResponse<T> = {
  generated_at: string;
  period: string;
  filters: Record<string, string>;
  rows: T[];
};

type UserRow = {
  rank: number;
  user_id: string;
  display_name: string;
  team_name: string;
  total_tokens: number;
  total_cost_usd: number;
  requests: number;
};

type TeamRow = {
  rank: number;
  team_id: string;
  team_name: string;
  member_count: number;
  total_tokens: number;
  avg_tokens_per_member: number;
  total_cost_usd: number;
};

type ToolRow = {
  rank: number;
  tool: string;
  total_tokens: number;
  total_cost_usd: number;
  active_users: number;
};

type ModelRow = {
  rank: number;
  model: string;
  total_tokens: number;
  total_cost_usd: number;
  active_users: number;
};

type GrowthRow = {
  rank: number;
  subject: string;
  current_total_tokens: number;
  previous_total_tokens: number;
  growth_tokens: number;
  growth_rate: number;
};

type DistributionItem = {
  label: string;
  total_tokens: number;
  share: number;
};

type RewardItem = {
  label: string;
  description: string;
  granted_at: string;
};

export type DashboardSummary = {
  generated_at: string;
  total_tokens: number;
  weekly_active_users: number;
  monthly_cost_usd: number;
  ai_penetration_rate: number;
  top_tools: MetricBreakdown[];
  top_models: MetricBreakdown[];
  trend: TrendPoint[];
};

export type MeOverview = {
  generated_at: string;
  user_id: string;
  display_name: string;
  team_name: string | null;
  total_tokens: number;
  total_cost_usd: number;
  request_count: number;
  favorite_tool: string | null;
  favorite_model: string | null;
};

export type MeDistribution = {
  generated_at: string;
  user_id: string;
  tools: DistributionItem[];
  models: DistributionItem[];
};

export type MeTrend = {
  generated_at: string;
  user_id: string;
  points: TrendPoint[];
};

export type MeRewards = {
  generated_at: string;
  user_id: string;
  rewards: RewardItem[];
};

async function fetchJson<T>(path: string): Promise<T> {
  const response = await fetch(`${getApiBaseUrl()}${path}`, {
    cache: "no-store",
  });
  if (!response.ok) {
    throw new Error(`request failed with ${response.status}`);
  }
  return (await response.json()) as T;
}

export async function getDashboardSummary(): Promise<DashboardSummary> {
  return fetchJson<DashboardSummary>("/v1/dashboard/summary");
}

export async function getUserLeaderboard(
  period: string,
): Promise<LeaderboardResponse<UserRow>> {
  return fetchJson(`/v1/leaderboards/users?period=${period}`);
}

export async function getTeamLeaderboard(
  period: string,
): Promise<LeaderboardResponse<TeamRow>> {
  return fetchJson(`/v1/leaderboards/teams?period=${period}`);
}

export async function getToolLeaderboard(
  period: string,
): Promise<LeaderboardResponse<ToolRow>> {
  return fetchJson(`/v1/leaderboards/tools?period=${period}`);
}

export async function getModelLeaderboard(
  period: string,
): Promise<LeaderboardResponse<ModelRow>> {
  return fetchJson(`/v1/leaderboards/models?period=${period}`);
}

export async function getGrowthLeaderboard(
  period: string,
): Promise<LeaderboardResponse<GrowthRow>> {
  return fetchJson(`/v1/leaderboards/growth?period=${period}`);
}

export async function getMeOverview(userId: string): Promise<MeOverview> {
  return fetchJson(`/v1/me/overview?user_id=${userId}`);
}

export async function getMeDistribution(userId: string): Promise<MeDistribution> {
  return fetchJson(`/v1/me/distribution?user_id=${userId}`);
}

export async function getMeTrend(userId: string): Promise<MeTrend> {
  return fetchJson(`/v1/me/trend?user_id=${userId}`);
}

export async function getMeRewards(userId: string): Promise<MeRewards> {
  return fetchJson(`/v1/me/rewards?user_id=${userId}`);
}

export async function assignUserTeam(
  userId: string,
  teamId: string,
  teamName: string,
  sessionToken?: string,
): Promise<void> {
  const headers: Record<string, string> = {
    "content-type": "application/json",
  };
  if (sessionToken) {
    headers["x-session-token"] = sessionToken;
  }

  const apiBaseUrl = getApiBaseUrl();
  const response = await fetch(`${apiBaseUrl}/v1/admin/users/${encodeURIComponent(userId)}/team`, {
    method: "PUT",
    headers,
    body: JSON.stringify({ team_id: teamId, team_name: teamName }),
    cache: "no-store",
  });

  if (!response.ok) {
    throw new Error(`request failed with ${response.status}`);
  }
}

export type { TeamRow };
