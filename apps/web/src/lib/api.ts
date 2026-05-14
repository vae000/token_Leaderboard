const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://127.0.0.1:8080";

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
  try {
    const response = await fetch(`${API_BASE_URL}${path}`, {
      next: { revalidate: 60 },
    });
    if (!response.ok) {
      throw new Error(`request failed with ${response.status}`);
    }
    return (await response.json()) as T;
  } catch {
    return fallback(path) as T;
  }
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

function fallback(path: string) {
  const generatedAt = new Date().toISOString();
  const trend = [
    { date: "2026-05-01", total_tokens: 4100, total_cost_usd: 0.12 },
    { date: "2026-05-04", total_tokens: 9600, total_cost_usd: 0.36 },
    { date: "2026-05-08", total_tokens: 7500, total_cost_usd: 0.28 },
    { date: "2026-05-12", total_tokens: 12100, total_cost_usd: 0.44 },
  ];
  const users = {
    generated_at: generatedAt,
    period: "month",
    filters: {},
    rows: [
      {
        rank: 1,
        user_id: "u_alice",
        display_name: "Alice",
        team_name: "Platform",
        total_tokens: 8040,
        total_cost_usd: 0.19,
        requests: 3,
      },
      {
        rank: 2,
        user_id: "u_cindy",
        display_name: "Cindy",
        team_name: "Platform",
        total_tokens: 5120,
        total_cost_usd: 0.13,
        requests: 1,
      },
      {
        rank: 3,
        user_id: "u_bob",
        display_name: "Bob",
        team_name: "Growth",
        total_tokens: 4300,
        total_cost_usd: 0.11,
        requests: 2,
      },
    ],
  };

  if (path.includes("/dashboard/summary")) {
    return {
      generated_at: generatedAt,
      total_tokens: 19320,
      weekly_active_users: 3,
      monthly_cost_usd: 0.43,
      ai_penetration_rate: 1,
      top_tools: [
        { label: "Codex", value: 11850 },
        { label: "Claude Code", value: 5120 },
        { label: "OpenCode", value: 2350 },
      ],
      top_models: [
        { label: "gpt-5.5", value: 12100 },
        { label: "claude-opus-4.1", value: 5120 },
        { label: "deepseek-reasoner", value: 2100 },
      ],
      trend,
    };
  }
  if (path.includes("/leaderboards/users")) {
    return users;
  }
  if (path.includes("/leaderboards/teams")) {
    return {
      generated_at: generatedAt,
      period: "month",
      filters: {},
      rows: [
        {
          rank: 1,
          team_id: "t_platform",
          team_name: "Platform",
          member_count: 2,
          total_tokens: 13160,
          avg_tokens_per_member: 6580,
          total_cost_usd: 0.32,
        },
        {
          rank: 2,
          team_id: "t_growth",
          team_name: "Growth",
          member_count: 1,
          total_tokens: 4300,
          avg_tokens_per_member: 4300,
          total_cost_usd: 0.11,
        },
      ],
    };
  }
  if (path.includes("/leaderboards/tools")) {
    return {
      generated_at: generatedAt,
      period: "month",
      filters: {},
      rows: [
        { rank: 1, tool: "codex", total_tokens: 11850, total_cost_usd: 0.27, active_users: 3 },
        { rank: 2, tool: "claude_code", total_tokens: 5120, total_cost_usd: 0.13, active_users: 1 },
        { rank: 3, tool: "opencode", total_tokens: 2350, total_cost_usd: 0.03, active_users: 1 },
      ],
    };
  }
  if (path.includes("/leaderboards/models")) {
    return {
      generated_at: generatedAt,
      period: "month",
      filters: {},
      rows: [
        { rank: 1, model: "gpt-5.5", total_tokens: 12100, total_cost_usd: 0.29, active_users: 3 },
        {
          rank: 2,
          model: "claude-opus-4.1",
          total_tokens: 5120,
          total_cost_usd: 0.13,
          active_users: 1,
        },
      ],
    };
  }
  if (path.includes("/leaderboards/growth")) {
    return {
      generated_at: generatedAt,
      period: "month",
      filters: {},
      rows: [
        {
          rank: 1,
          subject: "Alice",
          current_total_tokens: 8040,
          previous_total_tokens: 4000,
          growth_tokens: 4040,
          growth_rate: 1.01,
        },
        {
          rank: 2,
          subject: "Bob",
          current_total_tokens: 4300,
          previous_total_tokens: 2800,
          growth_tokens: 1500,
          growth_rate: 0.54,
        },
      ],
    };
  }
  if (path.includes("/me/overview")) {
    return {
      generated_at: generatedAt,
      user_id: "u_alice",
      display_name: "Alice",
      total_tokens: 8040,
      total_cost_usd: 0.19,
      request_count: 3,
      favorite_tool: "Codex",
      favorite_model: "gpt-5.5",
    };
  }
  if (path.includes("/me/distribution")) {
    return {
      generated_at: generatedAt,
      user_id: "u_alice",
      tools: [
        { label: "Codex", total_tokens: 5930, share: 0.74 },
        { label: "Cursor", total_tokens: 3200, share: 0.26 },
      ],
      models: [
        { label: "gpt-5.5", total_tokens: 4930, share: 0.61 },
        { label: "claude-opus-4.1", total_tokens: 3110, share: 0.39 },
      ],
    };
  }
  if (path.includes("/me/trend")) {
    return {
      generated_at: generatedAt,
      user_id: "u_alice",
      points: trend,
    };
  }
  return {
    generated_at: generatedAt,
    user_id: "u_alice",
    rewards: [
      {
        label: "Monthly Token TOP 1",
        description: "Highest token usage this month",
        granted_at: "2026-05-12",
      },
    ],
  };
}
