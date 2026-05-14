use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, bail};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use common::{
    DashboardSummary, DistributionItem, GrowthLeaderboardRow, LeaderboardResponse,
    MeDistributionResponse, MeOverviewResponse, MeTrendResponse, MetricBreakdown,
    ModelLeaderboardRow, RewardsResponse, TeamLeaderboardRow, ToolLeaderboardRow, TrendPoint,
    UsageEvent, UserLeaderboardRow,
};
use serde::Deserialize;

use crate::repository::Repository;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct QueryParams {
    pub period: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub team_id: Option<String>,
    pub tool: Option<String>,
    pub model: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub label: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub filters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Stats {
    tokens: u64,
    cost: f64,
    requests: usize,
}

impl Stats {
    fn add(&mut self, event: &UsageEvent) {
        self.tokens += event.total_tokens();
        self.cost += event.estimated_cost_usd;
        self.requests += 1;
    }
}

pub fn resolve_window(query: &QueryParams, now: DateTime<Utc>) -> anyhow::Result<Window> {
    let period = query.period.as_deref().unwrap_or("month");
    let start = match period {
        "today" => now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc(),
        "week" => {
            let weekday = now.weekday().num_days_from_monday() as i64;
            (now.date_naive() - Duration::days(weekday))
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
        }
        "month" => NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc(),
        "custom" => {
            let from = query.from.as_deref().context("missing custom from")?;
            NaiveDate::parse_from_str(from, "%Y-%m-%d")
                .with_context(|| format!("invalid from date: {from}"))?
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
        }
        "all" => now - Duration::days(3650),
        other => bail!("unsupported period: {other}"),
    };

    let end = match period {
        "custom" => {
            let to = query.to.as_deref().context("missing custom to")?;
            NaiveDate::parse_from_str(to, "%Y-%m-%d")
                .with_context(|| format!("invalid to date: {to}"))?
                .succ_opt()
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
        }
        _ => now + Duration::seconds(1),
    };

    let mut filters = BTreeMap::new();
    if let Some(value) = &query.team_id {
        filters.insert("team_id".into(), value.clone());
    }
    if let Some(value) = &query.tool {
        filters.insert("tool".into(), value.clone());
    }
    if let Some(value) = &query.model {
        filters.insert("model".into(), value.clone());
    }
    if let Some(value) = &query.user_id {
        filters.insert("user_id".into(), value.clone());
    }

    Ok(Window {
        label: period.into(),
        start,
        end,
        filters,
    })
}

pub fn dashboard_summary(repo: &Repository, now: DateTime<Utc>) -> DashboardSummary {
    let monthly_window = resolve_window(
        &QueryParams {
            period: Some("month".into()),
            ..Default::default()
        },
        now,
    )
    .expect("month window should resolve");
    let weekly_window = resolve_window(
        &QueryParams {
            period: Some("week".into()),
            ..Default::default()
        },
        now,
    )
    .expect("week window should resolve");
    let last_30_days = now - Duration::days(30);

    let total_tokens = repo.events.iter().map(UsageEvent::total_tokens).sum();
    let weekly_active_users = filter_events(repo, &weekly_window)
        .into_iter()
        .map(|event| event.user_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let monthly_cost_usd = filter_events(repo, &monthly_window)
        .into_iter()
        .map(|event| event.estimated_cost_usd)
        .sum();
    let active_last_30 = repo
        .events
        .iter()
        .filter(|event| event.occurred_at >= last_30_days)
        .map(|event| event.user_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let ai_penetration_rate = if repo.users.is_empty() {
        0.0
    } else {
        active_last_30 as f64 / repo.users.len() as f64
    };

    let top_tools = summarize_metric(repo.events.iter().map(|event| {
        (
            event.tool.display_name().to_owned(),
            event.total_tokens() as f64,
        )
    }));
    let top_models = summarize_metric(
        repo.events
            .iter()
            .map(|event| (event.model.clone(), event.total_tokens() as f64)),
    );
    let trend = trend_points(
        repo.events
            .iter()
            .filter(|event| event.occurred_at >= last_30_days)
            .collect(),
    );

    DashboardSummary {
        generated_at: now,
        total_tokens,
        weekly_active_users,
        monthly_cost_usd,
        ai_penetration_rate,
        top_tools,
        top_models,
        trend,
    }
}

pub fn user_leaderboard(
    repo: &Repository,
    query: &QueryParams,
    now: DateTime<Utc>,
) -> anyhow::Result<LeaderboardResponse<UserLeaderboardRow>> {
    let window = resolve_window(query, now)?;
    let mut stats = BTreeMap::<String, Stats>::new();
    for event in filter_events(repo, &window) {
        stats.entry(event.user_id.clone()).or_default().add(event);
    }

    let mut rows = stats
        .into_iter()
        .map(|(user_id, stat)| {
            let profile = repo
                .users
                .get(&user_id)
                .cloned()
                .unwrap_or(common::UserProfile {
                    user_id: user_id.clone(),
                    display_name: user_id.clone(),
                    email: None,
                    team_id: "unknown".into(),
                });
            let team_name = repo
                .teams
                .get(&profile.team_id)
                .map(|team| team.name.clone())
                .unwrap_or_else(|| "Unknown".into());

            UserLeaderboardRow {
                rank: 0,
                user_id,
                display_name: profile.display_name,
                team_name,
                total_tokens: stat.tokens,
                total_cost_usd: stat.cost,
                requests: stat.requests,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| right.total_tokens.cmp(&left.total_tokens));
    assign_ranks(&mut rows, |row, rank| row.rank = rank);
    Ok(response(window, rows, now))
}

pub fn team_leaderboard(
    repo: &Repository,
    query: &QueryParams,
    now: DateTime<Utc>,
) -> anyhow::Result<LeaderboardResponse<TeamLeaderboardRow>> {
    let window = resolve_window(query, now)?;
    let mut stats = BTreeMap::<String, Stats>::new();
    for event in filter_events(repo, &window) {
        let team_id = repo
            .users
            .get(&event.user_id)
            .map(|user| user.team_id.clone())
            .unwrap_or_else(|| "unknown".into());
        stats.entry(team_id).or_default().add(event);
    }

    let mut rows = stats
        .into_iter()
        .map(|(team_id, stat)| {
            let team_name = repo
                .teams
                .get(&team_id)
                .map(|team| team.name.clone())
                .unwrap_or_else(|| team_id.clone());
            let member_count = repo
                .users
                .values()
                .filter(|user| user.team_id == team_id)
                .count();

            TeamLeaderboardRow {
                rank: 0,
                team_id,
                team_name,
                member_count,
                total_tokens: stat.tokens,
                avg_tokens_per_member: if member_count == 0 {
                    0.0
                } else {
                    stat.tokens as f64 / member_count as f64
                },
                total_cost_usd: stat.cost,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| right.total_tokens.cmp(&left.total_tokens));
    assign_ranks(&mut rows, |row, rank| row.rank = rank);
    Ok(response(window, rows, now))
}

pub fn tool_leaderboard(
    repo: &Repository,
    query: &QueryParams,
    now: DateTime<Utc>,
) -> anyhow::Result<LeaderboardResponse<ToolLeaderboardRow>> {
    let window = resolve_window(query, now)?;
    let mut stats = BTreeMap::<String, (Stats, BTreeSet<String>)>::new();
    for event in filter_events(repo, &window) {
        let entry = stats.entry(event.tool.to_string()).or_default();
        entry.0.add(event);
        entry.1.insert(event.user_id.clone());
    }

    let mut rows = stats
        .into_iter()
        .filter_map(|(tool, (stat, users))| {
            Some(ToolLeaderboardRow {
                rank: 0,
                tool: tool.parse().ok()?,
                total_tokens: stat.tokens,
                total_cost_usd: stat.cost,
                active_users: users.len(),
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| right.total_tokens.cmp(&left.total_tokens));
    assign_ranks(&mut rows, |row, rank| row.rank = rank);
    Ok(response(window, rows, now))
}

pub fn model_leaderboard(
    repo: &Repository,
    query: &QueryParams,
    now: DateTime<Utc>,
) -> anyhow::Result<LeaderboardResponse<ModelLeaderboardRow>> {
    let window = resolve_window(query, now)?;
    let mut stats = BTreeMap::<String, (Stats, BTreeSet<String>)>::new();
    for event in filter_events(repo, &window) {
        let entry = stats.entry(event.model.clone()).or_default();
        entry.0.add(event);
        entry.1.insert(event.user_id.clone());
    }

    let mut rows = stats
        .into_iter()
        .map(|(model, (stat, users))| ModelLeaderboardRow {
            rank: 0,
            model,
            total_tokens: stat.tokens,
            total_cost_usd: stat.cost,
            active_users: users.len(),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| right.total_tokens.cmp(&left.total_tokens));
    assign_ranks(&mut rows, |row, rank| row.rank = rank);
    Ok(response(window, rows, now))
}

pub fn growth_leaderboard(
    repo: &Repository,
    query: &QueryParams,
    now: DateTime<Utc>,
) -> anyhow::Result<LeaderboardResponse<GrowthLeaderboardRow>> {
    let current = resolve_window(query, now)?;
    let duration = current.end - current.start;
    let previous = Window {
        label: format!("{}_previous", current.label),
        start: current.start - duration,
        end: current.start,
        filters: current.filters.clone(),
    };
    let current_events = filter_events(repo, &current);
    let previous_events = filter_events(repo, &previous);

    let mut current_stats = BTreeMap::<String, u64>::new();
    let mut previous_stats = BTreeMap::<String, u64>::new();
    for event in current_events {
        *current_stats.entry(event.user_id.clone()).or_default() += event.total_tokens();
    }
    for event in previous_events {
        *previous_stats.entry(event.user_id.clone()).or_default() += event.total_tokens();
    }

    let mut rows = current_stats
        .into_iter()
        .map(|(user_id, current_total_tokens)| {
            let previous_total_tokens = previous_stats.get(&user_id).copied().unwrap_or(0);
            let growth_tokens = current_total_tokens as i64 - previous_total_tokens as i64;
            let growth_rate = if previous_total_tokens == 0 {
                if current_total_tokens == 0 { 0.0 } else { 1.0 }
            } else {
                growth_tokens as f64 / previous_total_tokens as f64
            };
            let subject = repo
                .users
                .get(&user_id)
                .map(|user| user.display_name.clone())
                .unwrap_or(user_id);

            GrowthLeaderboardRow {
                rank: 0,
                subject,
                current_total_tokens,
                previous_total_tokens,
                growth_tokens,
                growth_rate,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| right.growth_tokens.cmp(&left.growth_tokens));
    assign_ranks(&mut rows, |row, rank| row.rank = rank);
    Ok(response(current, rows, now))
}

pub fn me_overview(
    repo: &Repository,
    user_id: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<MeOverviewResponse> {
    let query = QueryParams {
        period: Some("month".into()),
        user_id: Some(user_id.into()),
        ..Default::default()
    };
    let window = resolve_window(&query, now)?;
    let events = filter_events(repo, &window);
    let total_tokens = events.iter().map(|event| event.total_tokens()).sum();
    let total_cost_usd = events.iter().map(|event| event.estimated_cost_usd).sum();
    let favorite_tool = summarize_metric(events.iter().map(|event| {
        (
            event.tool.display_name().to_owned(),
            event.total_tokens() as f64,
        )
    }))
    .first()
    .map(|item| item.label.clone());
    let favorite_model = summarize_metric(
        events
            .iter()
            .map(|event| (event.model.clone(), event.total_tokens() as f64)),
    )
    .first()
    .map(|item| item.label.clone());
    let profile = repo.users.get(user_id).cloned().context("unknown user")?;

    let team_name = repo.teams.get(&profile.team_id).map(|t| t.name.clone());

    Ok(MeOverviewResponse {
        generated_at: now,
        user_id: user_id.into(),
        display_name: profile.display_name,
        team_name,
        total_tokens,
        total_cost_usd,
        request_count: events.len(),
        favorite_tool,
        favorite_model,
    })
}

pub fn me_distribution(
    repo: &Repository,
    user_id: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<MeDistributionResponse> {
    let query = QueryParams {
        period: Some("month".into()),
        user_id: Some(user_id.into()),
        ..Default::default()
    };
    let window = resolve_window(&query, now)?;
    let events = filter_events(repo, &window);
    let total_tokens = events.iter().map(|event| event.total_tokens()).sum::<u64>();
    let tools = distribution(
        events
            .iter()
            .map(|event| (event.tool.display_name().to_owned(), event.total_tokens())),
        total_tokens,
    );
    let models = distribution(
        events
            .iter()
            .map(|event| (event.model.clone(), event.total_tokens())),
        total_tokens,
    );

    Ok(MeDistributionResponse {
        generated_at: now,
        user_id: user_id.into(),
        tools,
        models,
    })
}

pub fn me_trend(repo: &Repository, user_id: &str, now: DateTime<Utc>) -> MeTrendResponse {
    let points = trend_points(
        repo.events
            .iter()
            .filter(|event| {
                event.user_id == user_id && event.occurred_at >= now - Duration::days(30)
            })
            .collect(),
    );
    MeTrendResponse {
        generated_at: now,
        user_id: user_id.into(),
        points,
    }
}

pub fn me_rewards(repo: &Repository, user_id: &str, now: DateTime<Utc>) -> RewardsResponse {
    let rewards = repo.rewards.get(user_id).cloned().unwrap_or_default();
    RewardsResponse {
        generated_at: now,
        user_id: user_id.into(),
        rewards,
    }
}

fn response<T>(window: Window, rows: Vec<T>, now: DateTime<Utc>) -> LeaderboardResponse<T> {
    LeaderboardResponse {
        generated_at: now,
        period: window.label,
        filters: window.filters,
        rows,
    }
}

fn filter_events<'a>(repo: &'a Repository, window: &Window) -> Vec<&'a UsageEvent> {
    repo.events
        .iter()
        .filter(|event| event.occurred_at >= window.start && event.occurred_at < window.end)
        .filter(|event| {
            window.filters.get("team_id").is_none_or(|team_id| {
                repo.users
                    .get(&event.user_id)
                    .is_some_and(|user| &user.team_id == team_id)
            })
        })
        .filter(|event| {
            window
                .filters
                .get("tool")
                .is_none_or(|tool| &event.tool.to_string() == tool)
        })
        .filter(|event| {
            window
                .filters
                .get("model")
                .is_none_or(|model| &event.model == model)
        })
        .filter(|event| {
            window
                .filters
                .get("user_id")
                .is_none_or(|user_id| &event.user_id == user_id)
        })
        .collect()
}

fn trend_points(events: Vec<&UsageEvent>) -> Vec<TrendPoint> {
    let mut grouped = BTreeMap::<NaiveDate, (u64, f64)>::new();
    for event in events {
        let day = event.occurred_at.date_naive();
        let entry = grouped.entry(day).or_default();
        entry.0 += event.total_tokens();
        entry.1 += event.estimated_cost_usd;
    }

    grouped
        .into_iter()
        .map(|(date, (total_tokens, total_cost_usd))| TrendPoint {
            date,
            total_tokens,
            total_cost_usd,
        })
        .collect()
}

fn summarize_metric(pairs: impl Iterator<Item = (String, f64)>) -> Vec<MetricBreakdown> {
    let mut grouped = BTreeMap::<String, f64>::new();
    for (label, value) in pairs {
        *grouped.entry(label).or_default() += value;
    }
    let mut rows = grouped
        .into_iter()
        .map(|(label, value)| MetricBreakdown { label, value })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| right.value.total_cmp(&left.value));
    rows.truncate(5);
    rows
}

fn distribution(
    pairs: impl Iterator<Item = (String, u64)>,
    total_tokens: u64,
) -> Vec<DistributionItem> {
    let mut grouped = BTreeMap::<String, u64>::new();
    for (label, value) in pairs {
        *grouped.entry(label).or_default() += value;
    }
    let mut rows = grouped
        .into_iter()
        .map(|(label, total_tokens_for_label)| DistributionItem {
            label,
            total_tokens: total_tokens_for_label,
            share: if total_tokens == 0 {
                0.0
            } else {
                total_tokens_for_label as f64 / total_tokens as f64
            },
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| right.total_tokens.cmp(&left.total_tokens));
    rows
}

fn assign_ranks<T>(rows: &mut [T], mut assign: impl FnMut(&mut T, usize)) {
    for (index, row) in rows.iter_mut().enumerate() {
        assign(row, index + 1);
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use common::{ToolKind, UsageEvent};
    use uuid::Uuid;

    use super::{QueryParams, dashboard_summary, resolve_window};
    use crate::repository::Repository;

    #[test]
    fn resolve_custom_window() {
        let window = resolve_window(
            &QueryParams {
                period: Some("custom".into()),
                from: Some("2026-05-01".into()),
                to: Some("2026-05-03".into()),
                ..Default::default()
            },
            Utc::now(),
        )
        .expect("window");

        assert_eq!(window.start.date_naive().to_string(), "2026-05-01");
        assert_eq!(window.end.date_naive().to_string(), "2026-05-04");
    }

    #[test]
    fn dashboard_summary_uses_repository_events() {
        let mut repo = Repository::new();
        repo.events.push(UsageEvent {
            id: Uuid::new_v4(),
            event_id: "evt_test_1".into(),
            user_id: "u_demo".into(),
            tool: ToolKind::Codex,
            model: "gpt-5.5".into(),
            occurred_at: Utc::now(),
            input_tokens: 120,
            output_tokens: 80,
            cached_tokens: 20,
            estimated_cost_usd: 0.01,
            session_id: Some("sess_demo".into()),
            source_file: "/tmp/demo.jsonl".into(),
            source_offset: 1,
            raw_hash: "hash_demo".into(),
        });
        let summary = dashboard_summary(&repo, Utc::now());
        assert!(summary.total_tokens > 0);
        assert!(!summary.top_tools.is_empty());
    }
}
