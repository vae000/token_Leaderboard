use std::sync::Arc;

use anyhow::Context;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use chrono::{Duration, Utc};
use common::{
    AdminMutationResponse, AuthCallbackResponse, AuthStartResponse, BatchIngestRequest,
    DashboardSummary, LeaderboardResponse, MeDistributionResponse, MeOverviewResponse,
    MeTrendResponse, ModelLeaderboardRow, RefreshTokenRequest, RefreshTokenResponse,
    RewardsResponse, TeamLeaderboardRow, TeamMembershipImportRequest, ToolLeaderboardRow,
    UserLeaderboardRow, UserProfile,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use uuid::Uuid;

use crate::{
    aggregation::{
        QueryParams, dashboard_summary, growth_leaderboard, me_distribution, me_overview,
        me_rewards, me_trend, model_leaderboard, team_leaderboard, tool_leaderboard,
        user_leaderboard,
    },
    config::AppConfig,
    repository::Repository,
};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub pool: Option<PgPool>,
    pub repository: Arc<RwLock<Repository>>,
}

impl AppState {
    pub fn new(config: AppConfig, pool: Option<PgPool>, repository: Repository) -> Self {
        Self {
            config,
            pool,
            repository: Arc::new(RwLock::new(repository)),
        }
    }
}

pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(
            state
                .config
                .cors_allow_origin
                .parse::<axum::http::HeaderValue>()
                .expect("invalid cors origin"),
        )
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
        ]);

    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/auth/cli/start", post(auth_start))
        .route("/v1/auth/cli/callback", get(auth_callback))
        .route("/v1/auth/cli/refresh", post(auth_refresh))
        .route("/v1/ingest/events:batch", post(ingest_batch))
        .route("/v1/dashboard/summary", get(get_dashboard_summary))
        .route("/v1/leaderboards/users", get(get_users_leaderboard))
        .route("/v1/leaderboards/teams", get(get_teams_leaderboard))
        .route("/v1/leaderboards/tools", get(get_tools_leaderboard))
        .route("/v1/leaderboards/models", get(get_models_leaderboard))
        .route("/v1/leaderboards/growth", get(get_growth_leaderboard))
        .route("/v1/me/overview", get(get_me_overview))
        .route("/v1/me/trend", get(get_me_trend))
        .route("/v1/me/distribution", get(get_me_distribution))
        .route("/v1/me/rewards", get(get_me_rewards))
        .route(
            "/v1/admin/team-memberships:import",
            post(import_memberships),
        )
        .route("/v1/admin/users/{user_id}/team", put(assign_user_team))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

async fn healthz(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "storage": if state.pool.is_some() { "postgres+memory" } else { "memory" }
    }))
}

async fn auth_start(State(state): State<AppState>) -> Result<Json<AuthStartResponse>, AppError> {
    let device_id = format!("dev_{}", Uuid::new_v4());
    state
        .repository
        .write()
        .await
        .register_device(device_id.clone());

    Ok(Json(AuthStartResponse {
        device_id: device_id.clone(),
        login_url: format!(
            "{}/v1/auth/cli/callback?device_id={device_id}&user_id=u_demo&display_name=Demo",
            base_url(&state.config)
        ),
        poll_after_seconds: 3,
    }))
}

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    device_id: String,
    user_id: String,
    display_name: String,
}

async fn auth_callback(
    State(state): State<AppState>,
    Query(query): Query<CallbackQuery>,
) -> Result<Json<AuthCallbackResponse>, AppError> {
    let mut repo = state.repository.write().await;
    repo.bind_device_user(query.device_id.clone(), query.user_id.clone());
    repo.upsert_user(UserProfile {
        user_id: query.user_id.clone(),
        display_name: query.display_name.clone(),
        email: None,
        team_id: "t_platform".into(),
    });

    Ok(Json(AuthCallbackResponse {
        access_token: format!("access_{}", Uuid::new_v4()),
        refresh_token: format!("refresh_{}", Uuid::new_v4()),
        user_id: query.user_id,
        display_name: query.display_name,
        expires_at: Utc::now() + Duration::hours(12),
    }))
}

async fn auth_refresh(
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, AppError> {
    if request.device_id.trim().is_empty() || request.refresh_token.trim().is_empty() {
        return Err(AppError::bad_request(
            "device_id and refresh_token are required",
        ));
    }

    Ok(Json(RefreshTokenResponse {
        access_token: format!("access_{}", Uuid::new_v4()),
        refresh_token: format!("refresh_{}", Uuid::new_v4()),
        expires_at: Utc::now() + Duration::hours(12),
    }))
}

async fn ingest_batch(
    State(state): State<AppState>,
    Json(request): Json<BatchIngestRequest>,
) -> Result<Json<common::BatchIngestResponse>, AppError> {
    if request.events.is_empty() {
        return Err(AppError::bad_request("events must not be empty"));
    }

    let mut repo = state.repository.write().await;
    info!(
        "ingesting {} events from device {} for user {}",
        request.events.len(),
        request.device_id,
        request.user_id
    );
    Ok(Json(repo.ingest_events(request.events)))
}

async fn get_dashboard_summary(
    State(state): State<AppState>,
) -> Result<Json<DashboardSummary>, AppError> {
    let repo = state.repository.read().await;
    Ok(Json(dashboard_summary(&repo, Utc::now())))
}

async fn get_users_leaderboard(
    State(state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> Result<Json<LeaderboardResponse<UserLeaderboardRow>>, AppError> {
    let repo = state.repository.read().await;
    Ok(Json(user_leaderboard(&repo, &query, Utc::now())?))
}

async fn get_teams_leaderboard(
    State(state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> Result<Json<LeaderboardResponse<TeamLeaderboardRow>>, AppError> {
    let repo = state.repository.read().await;
    Ok(Json(team_leaderboard(&repo, &query, Utc::now())?))
}

async fn get_tools_leaderboard(
    State(state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> Result<Json<LeaderboardResponse<ToolLeaderboardRow>>, AppError> {
    let repo = state.repository.read().await;
    Ok(Json(tool_leaderboard(&repo, &query, Utc::now())?))
}

async fn get_models_leaderboard(
    State(state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> Result<Json<LeaderboardResponse<ModelLeaderboardRow>>, AppError> {
    let repo = state.repository.read().await;
    Ok(Json(model_leaderboard(&repo, &query, Utc::now())?))
}

async fn get_growth_leaderboard(
    State(state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> Result<Json<LeaderboardResponse<common::GrowthLeaderboardRow>>, AppError> {
    let repo = state.repository.read().await;
    Ok(Json(growth_leaderboard(&repo, &query, Utc::now())?))
}

async fn get_me_overview(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<QueryParams>,
) -> Result<Json<MeOverviewResponse>, AppError> {
    let user_id = requested_user_id(&headers, &query)?;
    let repo = state.repository.read().await;
    Ok(Json(me_overview(&repo, &user_id, Utc::now())?))
}

async fn get_me_trend(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<QueryParams>,
) -> Result<Json<MeTrendResponse>, AppError> {
    let user_id = requested_user_id(&headers, &query)?;
    let repo = state.repository.read().await;
    Ok(Json(me_trend(&repo, &user_id, Utc::now())))
}

async fn get_me_distribution(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<QueryParams>,
) -> Result<Json<MeDistributionResponse>, AppError> {
    let user_id = requested_user_id(&headers, &query)?;
    let repo = state.repository.read().await;
    Ok(Json(me_distribution(&repo, &user_id, Utc::now())?))
}

async fn get_me_rewards(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<QueryParams>,
) -> Result<Json<RewardsResponse>, AppError> {
    let user_id = requested_user_id(&headers, &query)?;
    let repo = state.repository.read().await;
    Ok(Json(me_rewards(&repo, &user_id, Utc::now())))
}

async fn import_memberships(
    State(state): State<AppState>,
    Json(request): Json<TeamMembershipImportRequest>,
) -> Result<Json<AdminMutationResponse>, AppError> {
    let result = state.repository.write().await.import_memberships(request);
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
struct AssignTeamRequest {
    team_id: String,
    team_name: String,
}

async fn assign_user_team(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Json(request): Json<AssignTeamRequest>,
) -> Result<Json<AdminMutationResponse>, AppError> {
    let updated = state.repository.write().await.assign_user_team(
        &user_id,
        &request.team_id,
        &request.team_name,
    );
    if !updated {
        return Err(AppError::bad_request("unknown user"));
    }

    Ok(Json(AdminMutationResponse { updated_users: 1 }))
}

fn requested_user_id(headers: &HeaderMap, query: &QueryParams) -> Result<String, AppError> {
    if let Some(value) = query.user_id.clone() {
        return Ok(value);
    }
    if let Some(header) = headers.get("x-user-id") {
        return Ok(header
            .to_str()
            .context("x-user-id header is invalid utf-8")?
            .to_owned());
    }
    Ok("u_alice".into())
}

fn base_url(config: &AppConfig) -> String {
    format!(
        "http://{}:{}",
        config.host.replace("0.0.0.0", "127.0.0.1"),
        config.port
    )
}

#[derive(Debug)]
struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "error": self.message
            })),
        )
            .into_response()
    }
}
