use std::collections::HashMap;

use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use common::{
    AdminMutationResponse, AuthenticatedUser, BatchIngestResponse, DashboardSummary,
    DistributionItem, GrowthLeaderboardRow, LeaderboardResponse, MeDistributionResponse,
    MeOverviewResponse, MeTrendResponse, MetricBreakdown, ModelLeaderboardRow,
    RefreshTokenResponse, RejectedEvent, RewardItem, RewardsResponse, TeamLeaderboardRow,
    TeamMembershipImportRequest, ToolKind, ToolLeaderboardRow, TrendPoint, UsageEvent,
    UserLeaderboardRow, WebSessionResponse,
};
use sqlx::{PgPool, Postgres, QueryBuilder, Row};

use crate::aggregation::{QueryParams, Window, resolve_window};
use crate::pricing::{self, CatalogPriceRow};

const CURRENT_MEMBERSHIPS_CTE: &str = r#"
WITH current_memberships AS (
    SELECT DISTINCT ON (utm.user_id)
        utm.user_id,
        utm.team_id,
        t.name AS team_name
    FROM user_team_memberships utm
    JOIN teams t ON t.id = utm.team_id
    WHERE utm.effective_to IS NULL OR utm.effective_to > CURRENT_TIMESTAMP
    ORDER BY utm.user_id, utm.effective_from DESC
)
"#;

pub async fn register_device(pool: &PgPool, device_id: &str) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO devices (id, created_at)
        VALUES ($1, NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(device_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// 查询设备绑定的用户，未绑定时返回 None。
pub async fn find_device_user(
    pool: &PgPool,
    device_id: &str,
) -> anyhow::Result<Option<AuthenticatedUser>> {
    let row = sqlx::query(
        r#"
        SELECT d.user_id, u.display_name
        FROM devices d
        JOIN users u ON u.id = d.user_id
        WHERE d.id = $1 AND u.status = 'active'
        "#,
    )
    .bind(device_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| AuthenticatedUser {
        user_id: r.get("user_id"),
        display_name: r.get("display_name"),
    }))
}

/// 更新设备的 refresh_token。
pub async fn update_device_tokens(
    pool: &PgPool,
    device_id: &str,
    refresh_token: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE devices
        SET refresh_token = $2,
            last_seen_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(device_id)
    .bind(refresh_token)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn bind_device_user(
    pool: &PgPool,
    device_id: &str,
    user_id: &str,
    display_name: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO users (id, display_name, status)
        VALUES ($1, $2, 'active')
        ON CONFLICT (id) DO UPDATE
        SET display_name = EXCLUDED.display_name,
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(display_name)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO devices (id, user_id, last_seen_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (id) DO UPDATE
        SET user_id = EXCLUDED.user_id,
            last_seen_at = NOW()
        "#,
    )
    .bind(device_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn refresh_device_token(
    pool: &PgPool,
    device_id: &str,
    refresh_token: &str,
) -> anyhow::Result<RefreshTokenResponse> {
    sqlx::query(
        r#"
        INSERT INTO devices (id, refresh_token, last_seen_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (id) DO UPDATE
        SET refresh_token = EXCLUDED.refresh_token,
            last_seen_at = NOW()
        "#,
    )
    .bind(device_id)
    .bind(refresh_token)
    .execute(pool)
    .await?;

    Ok(RefreshTokenResponse {
        access_token: format!("access_{}", uuid::Uuid::new_v4()),
        refresh_token: format!("refresh_{}", uuid::Uuid::new_v4()),
        expires_at: Utc::now() + Duration::hours(12),
    })
}

pub async fn create_web_session(
    pool: &PgPool,
    user_id: &str,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<String> {
    let session_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO web_sessions (id, user_id, expires_at)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(session_id.to_string())
}

pub async fn get_web_session(
    pool: &PgPool,
    session_token: &str,
) -> anyhow::Result<WebSessionResponse> {
    let session_id = uuid::Uuid::parse_str(session_token).context("invalid session token")?;
    let row = sqlx::query(
        r#"
        SELECT
            u.id AS user_id,
            u.display_name,
            ws.expires_at
        FROM web_sessions ws
        JOIN users u ON u.id = ws.user_id
        WHERE ws.id = $1
          AND ws.expires_at > NOW()
        "#,
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?
    .context("invalid or expired web session")?;

    Ok(WebSessionResponse {
        user: AuthenticatedUser {
            user_id: row.get("user_id"),
            display_name: row.get("display_name"),
        },
        expires_at: row.get("expires_at"),
    })
}

pub async fn revoke_web_session(pool: &PgPool, session_token: &str) -> anyhow::Result<()> {
    let session_id = uuid::Uuid::parse_str(session_token).context("invalid session token")?;
    sqlx::query("DELETE FROM web_sessions WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_user_by_login_username(
    pool: &PgPool,
    username: &str,
) -> anyhow::Result<Option<AuthenticatedUser>> {
    sqlx::query(
        r#"
        SELECT id AS user_id, display_name
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map(|row| {
        row.map(|row| AuthenticatedUser {
            user_id: row.get("user_id"),
            display_name: row.get("display_name"),
        })
    })
    .map_err(Into::into)
}

pub async fn ingest_events(
    pool: &PgPool,
    device_id: &str,
    events: Vec<UsageEvent>,
) -> anyhow::Result<BatchIngestResponse> {
    let mut tx = pool.begin().await?;
    let mut pricing_cache: HashMap<(String, chrono::NaiveDate, u64), Option<CatalogPriceRow>> =
        HashMap::new();

    sqlx::query(
        r#"
        INSERT INTO devices (id, last_seen_at)
        VALUES ($1, NOW())
        ON CONFLICT (id) DO UPDATE
        SET last_seen_at = NOW()
        "#,
    )
    .bind(device_id)
    .execute(&mut *tx)
    .await?;

    let mut accepted = 0usize;
    let mut deduped = 0usize;
    let mut rejected = Vec::new();

    for event in events {
        if event.user_id.trim().is_empty() {
            rejected.push(RejectedEvent {
                event_id: event.event_id,
                reason: "missing user_id".into(),
            });
            continue;
        }
        if event.model.trim().is_empty() {
            rejected.push(RejectedEvent {
                event_id: event.event_id,
                reason: "missing model".into(),
            });
            continue;
        }

        let pricing_key = (
            event.model.clone(),
            event.occurred_at.date_naive(),
            event.input_tokens,
        );
        let pricing = match pricing_cache.get(&pricing_key) {
            Some(cached) => *cached,
            None => {
                let loaded = pricing::find_effective_price(
                    pool,
                    &event.model,
                    event.occurred_at,
                    event.input_tokens,
                )
                .await?;
                pricing_cache.insert(pricing_key, loaded);
                loaded
            }
        };
        let estimated_cost_usd = pricing
            .map(|price| {
                pricing::calculate_cost(
                    event.input_tokens,
                    event.output_tokens,
                    event.cached_tokens,
                    price.input_price_per_1k_usd,
                    price.output_price_per_1k_usd,
                    price.cache_price_per_1k_usd,
                )
            })
            .unwrap_or(event.estimated_cost_usd);

        let inserted = sqlx::query(
            r#"
            INSERT INTO usage_events (
                id,
                event_id,
                user_id,
                tool,
                model,
                occurred_at,
                input_tokens,
                output_tokens,
                cached_tokens,
                estimated_cost_usd,
                session_id,
                source_file,
                source_offset,
                raw_hash,
                idempotency_key
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15
            )
            ON CONFLICT (idempotency_key) DO NOTHING
            RETURNING id
            "#,
        )
        .bind(event.id)
        .bind(&event.event_id)
        .bind(&event.user_id)
        .bind(event.tool.as_str())
        .bind(&event.model)
        .bind(event.occurred_at)
        .bind(to_i64(event.input_tokens)?)
        .bind(to_i64(event.output_tokens)?)
        .bind(to_i64(event.cached_tokens)?)
        .bind(estimated_cost_usd)
        .bind(&event.session_id)
        .bind(&event.source_file)
        .bind(to_i64(event.source_offset)?)
        .bind(&event.raw_hash)
        .bind(event.idempotency_key())
        .fetch_optional(&mut *tx)
        .await?;

        if inserted.is_some() {
            accepted += 1;
        } else {
            deduped += 1;
        }
    }

    tx.commit().await?;

    Ok(BatchIngestResponse {
        accepted,
        deduped,
        rejected,
        next_sync_after_seconds: 60,
    })
}

pub async fn import_memberships(
    pool: &PgPool,
    request: TeamMembershipImportRequest,
) -> anyhow::Result<AdminMutationResponse> {
    let updated_users = request.rows.len();
    let mut tx = pool.begin().await?;

    for row in request.rows {
        sqlx::query(
            r#"
            INSERT INTO teams (id, name)
            VALUES ($1, $2)
            ON CONFLICT (id) DO UPDATE
            SET name = EXCLUDED.name
            "#,
        )
        .bind(&row.team_id)
        .bind(&row.team_name)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO users (id, display_name, status)
            VALUES ($1, $2, 'active')
            ON CONFLICT (id) DO UPDATE
            SET display_name = EXCLUDED.display_name,
                updated_at = NOW()
            "#,
        )
        .bind(&row.user_id)
        .bind(&row.display_name)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE user_team_memberships
            SET effective_to = NOW()
            WHERE user_id = $1
              AND effective_to IS NULL
              AND team_id <> $2
            "#,
        )
        .bind(&row.user_id)
        .bind(&row.team_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO user_team_memberships (user_id, team_id, effective_from)
            SELECT $1, $2, NOW()
            WHERE NOT EXISTS (
                SELECT 1
                FROM user_team_memberships
                WHERE user_id = $1
                  AND team_id = $2
                  AND effective_to IS NULL
            )
            "#,
        )
        .bind(&row.user_id)
        .bind(&row.team_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(AdminMutationResponse { updated_users })
}

pub async fn assign_user_team(
    pool: &PgPool,
    user_id: &str,
    team_id: &str,
    team_name: &str,
) -> anyhow::Result<bool> {
    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM users WHERE id = $1)")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    if !exists {
        return Ok(false);
    }

    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        INSERT INTO teams (id, name)
        VALUES ($1, $2)
        ON CONFLICT (id) DO UPDATE
        SET name = EXCLUDED.name
        "#,
    )
    .bind(team_id)
    .bind(team_name)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE user_team_memberships
        SET effective_to = NOW()
        WHERE user_id = $1
          AND effective_to IS NULL
          AND team_id <> $2
        "#,
    )
    .bind(user_id)
    .bind(team_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO user_team_memberships (user_id, team_id, effective_from)
        SELECT $1, $2, NOW()
        WHERE NOT EXISTS (
            SELECT 1
            FROM user_team_memberships
            WHERE user_id = $1
              AND team_id = $2
              AND effective_to IS NULL
        )
        "#,
    )
    .bind(user_id)
    .bind(team_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(true)
}

pub async fn dashboard_summary(
    pool: &PgPool,
    now: DateTime<Utc>,
) -> anyhow::Result<DashboardSummary> {
    let month_window = resolve_window(
        &QueryParams {
            period: Some("month".into()),
            ..Default::default()
        },
        now,
    )?;
    let week_window = resolve_window(
        &QueryParams {
            period: Some("week".into()),
            ..Default::default()
        },
        now,
    )?;
    let last_30_days = now - Duration::days(30);

    let total_tokens = sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT COALESCE(SUM(input_tokens + output_tokens + cached_tokens), 0)::bigint
        FROM usage_events
        "#,
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    let weekly_active_users = sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT COUNT(DISTINCT user_id)::bigint
        FROM usage_events
        WHERE occurred_at >= $1 AND occurred_at < $2
        "#,
    )
    .bind(week_window.start)
    .bind(week_window.end)
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    let monthly_cost_usd = sqlx::query_scalar::<_, Option<f64>>(
        r#"
        SELECT COALESCE(SUM(estimated_cost_usd), 0)::double precision
        FROM usage_events
        WHERE occurred_at >= $1 AND occurred_at < $2
        "#,
    )
    .bind(month_window.start)
    .bind(month_window.end)
    .fetch_one(pool)
    .await?
    .unwrap_or(0.0);

    let active_last_30 = sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT COUNT(DISTINCT user_id)::bigint
        FROM usage_events
        WHERE occurred_at >= $1
        "#,
    )
    .bind(last_30_days)
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    let user_count = sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT COUNT(*)::bigint
        FROM users
        "#,
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    let top_tools = fetch_metric_breakdown(
        pool,
        r#"
        SELECT COALESCE(tt.display_name, ue.tool) AS label,
               SUM(ue.input_tokens + ue.output_tokens + ue.cached_tokens)::double precision AS value
        FROM usage_events ue
        LEFT JOIN tool_types tt ON tt.id = ue.tool
        GROUP BY 1
        ORDER BY value DESC
        LIMIT 5
        "#,
    )
    .await?;

    let top_models = fetch_metric_breakdown(
        pool,
        r#"
        SELECT ue.model AS label,
               SUM(ue.input_tokens + ue.output_tokens + ue.cached_tokens)::double precision AS value
        FROM usage_events ue
        GROUP BY 1
        ORDER BY value DESC
        LIMIT 5
        "#,
    )
    .await?;

    let trend_rows = sqlx::query(
        r#"
        SELECT occurred_at::date AS date,
               SUM(input_tokens + output_tokens + cached_tokens)::bigint AS total_tokens,
               COALESCE(SUM(estimated_cost_usd), 0)::double precision AS total_cost_usd
        FROM usage_events
        WHERE occurred_at >= $1
        GROUP BY 1
        ORDER BY 1
        "#,
    )
    .bind(last_30_days)
    .fetch_all(pool)
    .await?;

    let trend = trend_rows
        .into_iter()
        .map(|row| TrendPoint {
            date: row.get("date"),
            total_tokens: to_u64(row.get::<i64, _>("total_tokens")),
            total_cost_usd: row.get::<f64, _>("total_cost_usd"),
        })
        .collect();

    Ok(DashboardSummary {
        generated_at: now,
        total_tokens: to_u64(total_tokens),
        weekly_active_users: to_usize(weekly_active_users),
        monthly_cost_usd,
        ai_penetration_rate: if user_count == 0 {
            0.0
        } else {
            active_last_30 as f64 / user_count as f64
        },
        top_tools,
        top_models,
        trend,
    })
}

pub async fn user_leaderboard(
    pool: &PgPool,
    query: &QueryParams,
    now: DateTime<Utc>,
) -> anyhow::Result<LeaderboardResponse<UserLeaderboardRow>> {
    let window = resolve_window(query, now)?;
    let mut qb = QueryBuilder::<Postgres>::new(CURRENT_MEMBERSHIPS_CTE);
    qb.push(
        r#"
        SELECT
            u.id AS user_id,
            u.display_name,
            COALESCE(cm.team_name, '未分配') AS team_name,
            COALESCE(SUM(ue.input_tokens + ue.output_tokens + ue.cached_tokens), 0)::bigint AS total_tokens,
            COALESCE(SUM(ue.estimated_cost_usd), 0)::double precision AS total_cost_usd,
            COUNT(ue.id)::bigint AS requests
        FROM usage_events ue
        JOIN users u ON u.id = ue.user_id
        LEFT JOIN current_memberships cm ON cm.user_id = u.id
        "#,
    );
    push_usage_filters(&mut qb, query, &window, "ue", Some("cm.team_id"));
    qb.push(" GROUP BY u.id, u.display_name, cm.team_name ORDER BY total_tokens DESC, u.id ASC");

    let rows = qb.build().fetch_all(pool).await?;
    let rows = rows
        .into_iter()
        .enumerate()
        .map(|(index, row)| UserLeaderboardRow {
            rank: index + 1,
            user_id: row.get("user_id"),
            display_name: row.get("display_name"),
            team_name: row.get("team_name"),
            total_tokens: to_u64(row.get::<i64, _>("total_tokens")),
            total_cost_usd: row.get("total_cost_usd"),
            requests: to_usize(row.get::<i64, _>("requests")),
        })
        .collect();

    Ok(LeaderboardResponse {
        generated_at: now,
        period: window.label,
        filters: window.filters,
        rows,
    })
}

pub async fn team_leaderboard(
    pool: &PgPool,
    query: &QueryParams,
    now: DateTime<Utc>,
) -> anyhow::Result<LeaderboardResponse<TeamLeaderboardRow>> {
    let window = resolve_window(query, now)?;
    let mut qb = QueryBuilder::<Postgres>::new(CURRENT_MEMBERSHIPS_CTE);
    qb.push(
        r#"
        , member_counts AS (
            SELECT team_id, COUNT(*)::bigint AS member_count
            FROM current_memberships
            GROUP BY team_id
        )
        SELECT
            cm.team_id,
            cm.team_name,
            COALESCE(mc.member_count, 0)::bigint AS member_count,
            COALESCE(SUM(ue.input_tokens + ue.output_tokens + ue.cached_tokens), 0)::bigint AS total_tokens,
            COALESCE(SUM(ue.estimated_cost_usd), 0)::double precision AS total_cost_usd
        FROM usage_events ue
        JOIN current_memberships cm ON cm.user_id = ue.user_id
        LEFT JOIN member_counts mc ON mc.team_id = cm.team_id
        "#,
    );
    push_usage_filters(&mut qb, query, &window, "ue", Some("cm.team_id"));
    qb.push(" GROUP BY cm.team_id, cm.team_name, mc.member_count ORDER BY total_tokens DESC, cm.team_id ASC");

    let rows = qb.build().fetch_all(pool).await?;
    let rows = rows
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let member_count = row.get::<i64, _>("member_count");
            let total_tokens = row.get::<i64, _>("total_tokens");
            TeamLeaderboardRow {
                rank: index + 1,
                team_id: row.get("team_id"),
                team_name: row.get("team_name"),
                member_count: to_usize(member_count),
                total_tokens: to_u64(total_tokens),
                avg_tokens_per_member: if member_count <= 0 {
                    0.0
                } else {
                    total_tokens as f64 / member_count as f64
                },
                total_cost_usd: row.get("total_cost_usd"),
            }
        })
        .collect();

    Ok(LeaderboardResponse {
        generated_at: now,
        period: window.label,
        filters: window.filters,
        rows,
    })
}

pub async fn tool_leaderboard(
    pool: &PgPool,
    query: &QueryParams,
    now: DateTime<Utc>,
) -> anyhow::Result<LeaderboardResponse<ToolLeaderboardRow>> {
    let window = resolve_window(query, now)?;
    let mut qb = QueryBuilder::<Postgres>::new(CURRENT_MEMBERSHIPS_CTE);
    qb.push(
        r#"
        SELECT
            ue.tool,
            COALESCE(SUM(ue.input_tokens + ue.output_tokens + ue.cached_tokens), 0)::bigint AS total_tokens,
            COALESCE(SUM(ue.estimated_cost_usd), 0)::double precision AS total_cost_usd,
            COUNT(DISTINCT ue.user_id)::bigint AS active_users
        FROM usage_events ue
        LEFT JOIN current_memberships cm ON cm.user_id = ue.user_id
        "#,
    );
    push_usage_filters(&mut qb, query, &window, "ue", Some("cm.team_id"));
    qb.push(" GROUP BY ue.tool ORDER BY total_tokens DESC, ue.tool ASC");

    let rows = qb.build().fetch_all(pool).await?;
    let mut mapped = Vec::new();
    for (index, row) in rows.into_iter().enumerate() {
        let tool_raw: String = row.get("tool");
        if let Ok(tool) = tool_raw.parse::<ToolKind>() {
            mapped.push(ToolLeaderboardRow {
                rank: index + 1,
                tool,
                total_tokens: to_u64(row.get::<i64, _>("total_tokens")),
                total_cost_usd: row.get("total_cost_usd"),
                active_users: to_usize(row.get::<i64, _>("active_users")),
            });
        }
    }

    Ok(LeaderboardResponse {
        generated_at: now,
        period: window.label,
        filters: window.filters,
        rows: mapped,
    })
}

pub async fn model_leaderboard(
    pool: &PgPool,
    query: &QueryParams,
    now: DateTime<Utc>,
) -> anyhow::Result<LeaderboardResponse<ModelLeaderboardRow>> {
    let window = resolve_window(query, now)?;
    let mut qb = QueryBuilder::<Postgres>::new(CURRENT_MEMBERSHIPS_CTE);
    qb.push(
        r#"
        SELECT
            ue.model,
            COALESCE(SUM(ue.input_tokens + ue.output_tokens + ue.cached_tokens), 0)::bigint AS total_tokens,
            COALESCE(SUM(ue.estimated_cost_usd), 0)::double precision AS total_cost_usd,
            COUNT(DISTINCT ue.user_id)::bigint AS active_users
        FROM usage_events ue
        LEFT JOIN current_memberships cm ON cm.user_id = ue.user_id
        "#,
    );
    push_usage_filters(&mut qb, query, &window, "ue", Some("cm.team_id"));
    qb.push(" GROUP BY ue.model ORDER BY total_tokens DESC, ue.model ASC");

    let rows = qb.build().fetch_all(pool).await?;
    let rows = rows
        .into_iter()
        .enumerate()
        .map(|(index, row)| ModelLeaderboardRow {
            rank: index + 1,
            model: row.get("model"),
            total_tokens: to_u64(row.get::<i64, _>("total_tokens")),
            total_cost_usd: row.get("total_cost_usd"),
            active_users: to_usize(row.get::<i64, _>("active_users")),
        })
        .collect();

    Ok(LeaderboardResponse {
        generated_at: now,
        period: window.label,
        filters: window.filters,
        rows,
    })
}

pub async fn growth_leaderboard(
    pool: &PgPool,
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

    let current_stats = fetch_user_token_totals(pool, query, &current).await?;
    let previous_stats = fetch_user_token_totals(pool, query, &previous).await?;

    let mut rows = Vec::new();
    for (index, (user_id, current_total_tokens, display_name)) in
        current_stats.into_iter().enumerate()
    {
        let previous_total_tokens = previous_stats
            .iter()
            .find(|(candidate, _, _)| candidate == &user_id)
            .map(|(_, total, _)| *total)
            .unwrap_or(0);
        let growth_tokens = current_total_tokens as i64 - previous_total_tokens as i64;
        let growth_rate = if previous_total_tokens == 0 {
            if current_total_tokens == 0 { 0.0 } else { 1.0 }
        } else {
            growth_tokens as f64 / previous_total_tokens as f64
        };
        rows.push(GrowthLeaderboardRow {
            rank: index + 1,
            subject: display_name,
            current_total_tokens,
            previous_total_tokens,
            growth_tokens,
            growth_rate,
        });
    }

    Ok(LeaderboardResponse {
        generated_at: now,
        period: current.label,
        filters: current.filters,
        rows,
    })
}

pub async fn me_overview(
    pool: &PgPool,
    user_id: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<MeOverviewResponse> {
    let window = resolve_window(
        &QueryParams {
            period: Some("month".into()),
            user_id: Some(user_id.into()),
            ..Default::default()
        },
        now,
    )?;

    let profile = sqlx::query(
        r#"
        SELECT u.display_name, t.name AS team_name
        FROM users u
        LEFT JOIN user_team_memberships utm
          ON utm.user_id = u.id
         AND (utm.effective_to IS NULL OR utm.effective_to > CURRENT_TIMESTAMP)
        LEFT JOIN teams t ON t.id = utm.team_id
        WHERE u.id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .context("unknown user")?;

    let aggregates = sqlx::query(
        r#"
        SELECT
            COALESCE(SUM(input_tokens + output_tokens + cached_tokens), 0)::bigint AS total_tokens,
            COALESCE(SUM(estimated_cost_usd), 0)::double precision AS total_cost_usd,
            COUNT(id)::bigint AS request_count
        FROM usage_events
        WHERE user_id = $1
          AND occurred_at >= $2
          AND occurred_at < $3
        "#,
    )
    .bind(user_id)
    .bind(window.start)
    .bind(window.end)
    .fetch_one(pool)
    .await?;

    let favorite_tool = sqlx::query(
        r#"
        SELECT COALESCE(tt.display_name, ue.tool) AS label
        FROM usage_events ue
        LEFT JOIN tool_types tt ON tt.id = ue.tool
        WHERE ue.user_id = $1
          AND ue.occurred_at >= $2
          AND ue.occurred_at < $3
        GROUP BY label
        ORDER BY SUM(ue.input_tokens + ue.output_tokens + ue.cached_tokens) DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(window.start)
    .bind(window.end)
    .fetch_optional(pool)
    .await?
    .map(|row| row.get("label"));

    let favorite_model = sqlx::query(
        r#"
        SELECT ue.model
        FROM usage_events ue
        WHERE ue.user_id = $1
          AND ue.occurred_at >= $2
          AND ue.occurred_at < $3
        GROUP BY ue.model
        ORDER BY SUM(ue.input_tokens + ue.output_tokens + ue.cached_tokens) DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(window.start)
    .bind(window.end)
    .fetch_optional(pool)
    .await?
    .map(|row| row.get("model"));

    Ok(MeOverviewResponse {
        generated_at: now,
        user_id: user_id.into(),
        display_name: profile.get("display_name"),
        team_name: profile.try_get("team_name").unwrap_or(None),
        total_tokens: to_u64(aggregates.get::<i64, _>("total_tokens")),
        total_cost_usd: aggregates.get("total_cost_usd"),
        request_count: to_usize(aggregates.get::<i64, _>("request_count")),
        favorite_tool,
        favorite_model,
    })
}

pub async fn me_distribution(
    pool: &PgPool,
    user_id: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<MeDistributionResponse> {
    let window = resolve_window(
        &QueryParams {
            period: Some("month".into()),
            user_id: Some(user_id.into()),
            ..Default::default()
        },
        now,
    )?;

    let total_tokens = sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT COALESCE(SUM(input_tokens + output_tokens + cached_tokens), 0)::bigint
        FROM usage_events
        WHERE user_id = $1
          AND occurred_at >= $2
          AND occurred_at < $3
        "#,
    )
    .bind(user_id)
    .bind(window.start)
    .bind(window.end)
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    let tool_rows = sqlx::query(
        r#"
        SELECT COALESCE(tt.display_name, ue.tool) AS label,
               SUM(ue.input_tokens + ue.output_tokens + ue.cached_tokens)::bigint AS total_tokens
        FROM usage_events ue
        LEFT JOIN tool_types tt ON tt.id = ue.tool
        WHERE ue.user_id = $1
          AND ue.occurred_at >= $2
          AND ue.occurred_at < $3
        GROUP BY label
        ORDER BY total_tokens DESC
        "#,
    )
    .bind(user_id)
    .bind(window.start)
    .bind(window.end)
    .fetch_all(pool)
    .await?;

    let model_rows = sqlx::query(
        r#"
        SELECT ue.model AS label,
               SUM(ue.input_tokens + ue.output_tokens + ue.cached_tokens)::bigint AS total_tokens
        FROM usage_events ue
        WHERE ue.user_id = $1
          AND ue.occurred_at >= $2
          AND ue.occurred_at < $3
        GROUP BY label
        ORDER BY total_tokens DESC
        "#,
    )
    .bind(user_id)
    .bind(window.start)
    .bind(window.end)
    .fetch_all(pool)
    .await?;

    Ok(MeDistributionResponse {
        generated_at: now,
        user_id: user_id.into(),
        tools: distribution_from_rows(tool_rows, total_tokens),
        models: distribution_from_rows(model_rows, total_tokens),
    })
}

pub async fn me_trend(
    pool: &PgPool,
    user_id: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<MeTrendResponse> {
    let rows = sqlx::query(
        r#"
        SELECT occurred_at::date AS date,
               SUM(input_tokens + output_tokens + cached_tokens)::bigint AS total_tokens,
               COALESCE(SUM(estimated_cost_usd), 0)::double precision AS total_cost_usd
        FROM usage_events
        WHERE user_id = $1
          AND occurred_at >= $2
        GROUP BY 1
        ORDER BY 1
        "#,
    )
    .bind(user_id)
    .bind(now - Duration::days(30))
    .fetch_all(pool)
    .await?;

    Ok(MeTrendResponse {
        generated_at: now,
        user_id: user_id.into(),
        points: rows
            .into_iter()
            .map(|row| TrendPoint {
                date: row.get("date"),
                total_tokens: to_u64(row.get::<i64, _>("total_tokens")),
                total_cost_usd: row.get("total_cost_usd"),
            })
            .collect(),
    })
}

pub async fn me_rewards(
    pool: &PgPool,
    user_id: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<RewardsResponse> {
    let rows = sqlx::query(
        r#"
        SELECT
            rr.label,
            COALESCE(rr.details->>'description', rules.description) AS description,
            rr.snapshot_date
        FROM reward_results rr
        JOIN reward_rules rules ON rules.id = rr.rule_id
        WHERE rr.user_id = $1
        ORDER BY rr.snapshot_date DESC, rr.id DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(RewardsResponse {
        generated_at: now,
        user_id: user_id.into(),
        rewards: rows
            .into_iter()
            .map(|row| RewardItem {
                label: row.get("label"),
                description: row.get("description"),
                granted_at: row.get("snapshot_date"),
            })
            .collect(),
    })
}

async fn fetch_metric_breakdown(pool: &PgPool, sql: &str) -> anyhow::Result<Vec<MetricBreakdown>> {
    let rows = sqlx::query(sql).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| MetricBreakdown {
            label: row.get("label"),
            value: row.get("value"),
        })
        .collect())
}

async fn fetch_user_token_totals(
    pool: &PgPool,
    query: &QueryParams,
    window: &Window,
) -> anyhow::Result<Vec<(String, u64, String)>> {
    let mut qb = QueryBuilder::<Postgres>::new(CURRENT_MEMBERSHIPS_CTE);
    qb.push(
        r#"
        SELECT
            u.id AS user_id,
            u.display_name,
            COALESCE(SUM(ue.input_tokens + ue.output_tokens + ue.cached_tokens), 0)::bigint AS total_tokens
        FROM usage_events ue
        JOIN users u ON u.id = ue.user_id
        LEFT JOIN current_memberships cm ON cm.user_id = u.id
        "#,
    );
    push_usage_filters(&mut qb, query, window, "ue", Some("cm.team_id"));
    qb.push(" GROUP BY u.id, u.display_name ORDER BY total_tokens DESC, u.id ASC");

    let rows = qb.build().fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.get("user_id"),
                to_u64(row.get::<i64, _>("total_tokens")),
                row.get("display_name"),
            )
        })
        .collect())
}

fn distribution_from_rows(
    rows: Vec<sqlx::postgres::PgRow>,
    total_tokens: i64,
) -> Vec<DistributionItem> {
    rows.into_iter()
        .map(|row| {
            let label_tokens = row.get::<i64, _>("total_tokens");
            DistributionItem {
                label: row.get("label"),
                total_tokens: to_u64(label_tokens),
                share: if total_tokens <= 0 {
                    0.0
                } else {
                    label_tokens as f64 / total_tokens as f64
                },
            }
        })
        .collect()
}

fn push_usage_filters(
    qb: &mut QueryBuilder<'_, Postgres>,
    query: &QueryParams,
    window: &Window,
    alias: &str,
    team_field: Option<&str>,
) {
    qb.push(" WHERE ");
    qb.push(alias);
    qb.push(".occurred_at >= ");
    qb.push_bind(window.start);
    qb.push(" AND ");
    qb.push(alias);
    qb.push(".occurred_at < ");
    qb.push_bind(window.end);

    if let Some(team_id) = query.team_id.clone() {
        if let Some(team_field) = team_field {
            qb.push(" AND ");
            qb.push(team_field);
            qb.push(" = ");
            qb.push_bind(team_id);
        }
    }
    if let Some(tool) = query.tool.clone() {
        qb.push(" AND ");
        qb.push(alias);
        qb.push(".tool = ");
        qb.push_bind(tool);
    }
    if let Some(model) = query.model.clone() {
        qb.push(" AND ");
        qb.push(alias);
        qb.push(".model = ");
        qb.push_bind(model);
    }
    if let Some(user_id) = query.user_id.clone() {
        qb.push(" AND ");
        qb.push(alias);
        qb.push(".user_id = ");
        qb.push_bind(user_id);
    }
}

fn to_i64(value: u64) -> anyhow::Result<i64> {
    i64::try_from(value).context("numeric overflow while converting to i64")
}

fn to_u64(value: i64) -> u64 {
    value.max(0) as u64
}

fn to_usize(value: i64) -> usize {
    value.max(0) as usize
}
