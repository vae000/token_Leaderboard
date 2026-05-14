use std::sync::Arc;

use anyhow::Context;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post, put},
};
use chrono::{Duration, Utc};
use common::{
    AdminMutationResponse, AuthCallbackResponse, AuthStartResponse, BatchIngestRequest,
    DashboardSummary, GeneratedCredential, LeaderboardResponse, MeDistributionResponse,
    MeOverviewResponse, MeTrendResponse, ModelLeaderboardRow, PasswordLoginRequest,
    PasswordLoginResponse, RefreshTokenRequest, RefreshTokenResponse, RewardsResponse,
    TeamLeaderboardRow, TeamMembershipImportRequest, ToolLeaderboardRow, UserLeaderboardRow,
    UserProfile, WebSessionResponse,
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
    local_auth, postgres, pricing,
    repository::Repository,
    wechat::{self, WechatConfig},
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
        .route("/v1/auth/web/login", post(web_password_login))
        .route("/v1/auth/wechat/start", get(wechat_start))
        .route("/v1/auth/wechat/callback", get(wechat_callback))
        .route("/v1/auth/web/me", get(web_auth_me))
        .route("/v1/auth/web/logout", post(web_auth_logout))
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
        .route(
            "/v1/admin/model-catalog:refresh",
            post(refresh_model_catalog),
        )
        .route(
            "/v1/admin/users/{user_id}/credentials",
            get(get_generated_credentials),
        )
        .route("/v1/admin/users/{user_id}/team", put(assign_user_team))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

async fn healthz(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "storage": if state.pool.is_some() { "postgres" } else { "memory" }
    }))
}

async fn auth_start(State(state): State<AppState>) -> Result<Json<AuthStartResponse>, AppError> {
    let device_id = format!("dev_{}", Uuid::new_v4());
    if let Some(pool) = &state.pool {
        postgres::register_device(pool, &device_id).await?;
    } else {
        state
            .repository
            .write()
            .await
            .register_device(device_id.clone());
    }

    let login_url = format!(
        "{}/login?device_id={device_id}",
        state.config.web_base_url.trim_end_matches('/'),
    );

    Ok(Json(AuthStartResponse {
        device_id: device_id.clone(),
        login_url,
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
    if let Some(pool) = &state.pool {
        postgres::bind_device_user(pool, &query.device_id, &query.user_id, &query.display_name)
            .await?;
    } else {
        let mut repo = state.repository.write().await;
        repo.bind_device_user(query.device_id.clone(), query.user_id.clone());
        repo.upsert_user(UserProfile {
            user_id: query.user_id.clone(),
            display_name: query.display_name.clone(),
            email: None,
            team_id: "t_platform".into(),
        });
    }

    Ok(Json(AuthCallbackResponse {
        access_token: format!("access_{}", Uuid::new_v4()),
        refresh_token: format!("refresh_{}", Uuid::new_v4()),
        user_id: query.user_id,
        display_name: query.display_name,
        expires_at: Utc::now() + Duration::hours(12),
    }))
}

async fn auth_refresh(
    State(state): State<AppState>,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, AppError> {
    if request.device_id.trim().is_empty() || request.refresh_token.trim().is_empty() {
        return Err(AppError::bad_request(
            "device_id and refresh_token are required",
        ));
    }

    if let Some(pool) = &state.pool {
        return Ok(Json(
            postgres::refresh_device_token(pool, &request.device_id, &request.refresh_token)
                .await?,
        ));
    }

    Ok(Json(RefreshTokenResponse {
        access_token: format!("access_{}", Uuid::new_v4()),
        refresh_token: format!("refresh_{}", Uuid::new_v4()),
        expires_at: Utc::now() + Duration::hours(12),
    }))
}

async fn web_password_login(
    State(state): State<AppState>,
    Json(request): Json<PasswordLoginRequest>,
) -> Result<Json<PasswordLoginResponse>, AppError> {
    let pool = require_pool(&state)?;
    let username = request.username.trim();
    if username.is_empty() || request.password.trim().is_empty() {
        return Err(AppError::bad_request("username and password are required"));
    }

    let user = postgres::find_user_by_login_username(pool, username)
        .await?
        .context("invalid username or password")?;
    let expected_password =
        local_auth::password_for_user_id(&user.user_id, &state.config.auto_password_salt);
    if request.password != expected_password {
        return Err(AppError::bad_request("invalid username or password"));
    }

    let expires_at = Utc::now() + Duration::hours(state.config.web_session_ttl_hours as i64);
    let session_token = postgres::create_web_session(pool, &user.user_id, expires_at).await?;
    Ok(Json(PasswordLoginResponse {
        session_token,
        user,
        expires_at,
    }))
}

#[derive(Debug, Deserialize)]
struct WechatStartQuery {
    return_to: Option<String>,
}

async fn wechat_start(
    State(state): State<AppState>,
    Query(query): Query<WechatStartQuery>,
) -> Result<Redirect, AppError> {
    let pool = require_pool(&state)?;
    let wechat = wechat_config(&state.config)?;
    let state_token = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::minutes(10);
    let return_to = normalize_return_to(query.return_to.as_deref());

    postgres::create_web_login_state(pool, &state_token, &return_to, expires_at).await?;
    let redirect_url = wechat::build_qr_connect_url(&wechat, &state_token)?;
    Ok(Redirect::temporary(&redirect_url))
}

#[derive(Debug, Deserialize)]
struct WechatCallbackQuery {
    code: Option<String>,
    state: Option<String>,
}

async fn wechat_callback(
    State(state): State<AppState>,
    Query(query): Query<WechatCallbackQuery>,
) -> Result<Redirect, AppError> {
    let pool = require_pool(&state)?;
    let wechat = wechat_config(&state.config)?;
    let state_token = query.state.context("missing wechat state")?;
    let code = query.code.context("missing wechat code")?;
    let return_to = postgres::consume_web_login_state(pool, &state_token).await?;
    let client = reqwest::Client::new();
    let wechat_user = wechat::fetch_user_info(&client, &wechat, &code).await?;
    let user = postgres::upsert_wechat_user(
        pool,
        &wechat_user.openid,
        wechat_user.unionid.as_deref(),
        &wechat_user.nickname,
    )
    .await?;
    let expires_at = Utc::now() + Duration::hours(state.config.web_session_ttl_hours as i64);
    let session_token = postgres::create_web_session(pool, &user.user_id, expires_at).await?;
    let mut redirect_url = url::Url::parse(&format!(
        "{}/auth/callback",
        state.config.web_base_url.trim_end_matches('/')
    ))
    .context("invalid API__WEB_BASE_URL")?;
    redirect_url
        .query_pairs_mut()
        .append_pair("session_token", &session_token)
        .append_pair("next", &return_to);
    Ok(Redirect::temporary(redirect_url.as_ref()))
}

async fn web_auth_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<WebSessionResponse>, AppError> {
    let pool = require_pool(&state)?;
    let session_token = session_token_from_headers(&headers)?;
    Ok(Json(postgres::get_web_session(pool, &session_token).await?))
}

async fn web_auth_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    let pool = require_pool(&state)?;
    let session_token = session_token_from_headers(&headers)?;
    postgres::revoke_web_session(pool, &session_token).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn ingest_batch(
    State(state): State<AppState>,
    Json(request): Json<BatchIngestRequest>,
) -> Result<Json<common::BatchIngestResponse>, AppError> {
    if request.events.is_empty() {
        return Err(AppError::bad_request("events must not be empty"));
    }

    info!(
        "ingesting {} events from device {} for user {}",
        request.events.len(),
        request.device_id,
        request.user_id
    );
    if let Some(pool) = &state.pool {
        return Ok(Json(
            postgres::ingest_events(pool, &request.device_id, request.events).await?,
        ));
    }

    let mut repo = state.repository.write().await;
    Ok(Json(repo.ingest_events(request.events)))
}

async fn get_dashboard_summary(
    State(state): State<AppState>,
) -> Result<Json<DashboardSummary>, AppError> {
    if let Some(pool) = &state.pool {
        return Ok(Json(postgres::dashboard_summary(pool, Utc::now()).await?));
    }
    let repo = state.repository.read().await;
    Ok(Json(dashboard_summary(&repo, Utc::now())))
}

async fn get_users_leaderboard(
    State(state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> Result<Json<LeaderboardResponse<UserLeaderboardRow>>, AppError> {
    if let Some(pool) = &state.pool {
        return Ok(Json(
            postgres::user_leaderboard(pool, &query, Utc::now()).await?,
        ));
    }
    let repo = state.repository.read().await;
    Ok(Json(user_leaderboard(&repo, &query, Utc::now())?))
}

async fn get_teams_leaderboard(
    State(state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> Result<Json<LeaderboardResponse<TeamLeaderboardRow>>, AppError> {
    if let Some(pool) = &state.pool {
        return Ok(Json(
            postgres::team_leaderboard(pool, &query, Utc::now()).await?,
        ));
    }
    let repo = state.repository.read().await;
    Ok(Json(team_leaderboard(&repo, &query, Utc::now())?))
}

async fn get_tools_leaderboard(
    State(state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> Result<Json<LeaderboardResponse<ToolLeaderboardRow>>, AppError> {
    if let Some(pool) = &state.pool {
        return Ok(Json(
            postgres::tool_leaderboard(pool, &query, Utc::now()).await?,
        ));
    }
    let repo = state.repository.read().await;
    Ok(Json(tool_leaderboard(&repo, &query, Utc::now())?))
}

async fn get_models_leaderboard(
    State(state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> Result<Json<LeaderboardResponse<ModelLeaderboardRow>>, AppError> {
    if let Some(pool) = &state.pool {
        return Ok(Json(
            postgres::model_leaderboard(pool, &query, Utc::now()).await?,
        ));
    }
    let repo = state.repository.read().await;
    Ok(Json(model_leaderboard(&repo, &query, Utc::now())?))
}

async fn get_growth_leaderboard(
    State(state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> Result<Json<LeaderboardResponse<common::GrowthLeaderboardRow>>, AppError> {
    if let Some(pool) = &state.pool {
        return Ok(Json(
            postgres::growth_leaderboard(pool, &query, Utc::now()).await?,
        ));
    }
    let repo = state.repository.read().await;
    Ok(Json(growth_leaderboard(&repo, &query, Utc::now())?))
}

async fn get_me_overview(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<QueryParams>,
) -> Result<Json<MeOverviewResponse>, AppError> {
    let user_id = requested_user_id(&headers, &query)?;
    if let Some(pool) = &state.pool {
        return Ok(Json(
            postgres::me_overview(pool, &user_id, Utc::now()).await?,
        ));
    }
    let repo = state.repository.read().await;
    Ok(Json(me_overview(&repo, &user_id, Utc::now())?))
}

async fn get_me_trend(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<QueryParams>,
) -> Result<Json<MeTrendResponse>, AppError> {
    let user_id = requested_user_id(&headers, &query)?;
    if let Some(pool) = &state.pool {
        return Ok(Json(postgres::me_trend(pool, &user_id, Utc::now()).await?));
    }
    let repo = state.repository.read().await;
    Ok(Json(me_trend(&repo, &user_id, Utc::now())))
}

async fn get_me_distribution(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<QueryParams>,
) -> Result<Json<MeDistributionResponse>, AppError> {
    let user_id = requested_user_id(&headers, &query)?;
    if let Some(pool) = &state.pool {
        return Ok(Json(
            postgres::me_distribution(pool, &user_id, Utc::now()).await?,
        ));
    }
    let repo = state.repository.read().await;
    Ok(Json(me_distribution(&repo, &user_id, Utc::now())?))
}

async fn get_me_rewards(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<QueryParams>,
) -> Result<Json<RewardsResponse>, AppError> {
    let user_id = requested_user_id(&headers, &query)?;
    if let Some(pool) = &state.pool {
        return Ok(Json(
            postgres::me_rewards(pool, &user_id, Utc::now()).await?,
        ));
    }
    let repo = state.repository.read().await;
    Ok(Json(me_rewards(&repo, &user_id, Utc::now())))
}

async fn import_memberships(
    State(state): State<AppState>,
    Json(request): Json<TeamMembershipImportRequest>,
) -> Result<Json<AdminMutationResponse>, AppError> {
    if let Some(pool) = &state.pool {
        return Ok(Json(postgres::import_memberships(pool, request).await?));
    }
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
    let updated = if let Some(pool) = &state.pool {
        postgres::assign_user_team(pool, &user_id, &request.team_id, &request.team_name).await?
    } else {
        state.repository.write().await.assign_user_team(
            &user_id,
            &request.team_id,
            &request.team_name,
        )
    };
    if !updated {
        return Err(AppError::bad_request("unknown user"));
    }

    Ok(Json(AdminMutationResponse { updated_users: 1 }))
}

async fn refresh_model_catalog(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let Some(pool) = &state.pool else {
        return Err(AppError::bad_request("postgres storage is required"));
    };

    let updated_models = pricing::refresh_model_catalog(pool).await?;
    Ok(Json(json!({
        "updated_models": updated_models,
        "refreshed_at": Utc::now(),
    })))
}

async fn get_generated_credentials(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<GeneratedCredential>, AppError> {
    let pool = require_pool(&state)?;
    let user = postgres::find_user_by_login_username(pool, &user_id)
        .await?
        .context("unknown user")?;
    Ok(Json(GeneratedCredential {
        user_id: user.user_id.clone(),
        display_name: user.display_name,
        username: local_auth::username_for_user_id(&user_id),
        password: local_auth::password_for_user_id(&user_id, &state.config.auto_password_salt),
    }))
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

fn require_pool(state: &AppState) -> Result<&PgPool, AppError> {
    state
        .pool
        .as_ref()
        .ok_or_else(|| AppError::bad_request("postgres storage is required"))
}

fn wechat_config(config: &AppConfig) -> Result<WechatConfig, AppError> {
    let app_id = config
        .wechat_app_id
        .clone()
        .context("API__WECHAT_APP_ID is required for wechat login")?;
    let app_secret = config
        .wechat_app_secret
        .clone()
        .context("API__WECHAT_APP_SECRET is required for wechat login")?;
    Ok(WechatConfig {
        app_id,
        app_secret,
        redirect_uri: format!(
            "{}/v1/auth/wechat/callback",
            config.public_base_url.trim_end_matches('/')
        ),
    })
}

fn normalize_return_to(return_to: Option<&str>) -> String {
    let Some(value) = return_to else {
        return "/me".into();
    };
    if value.starts_with('/') && !value.starts_with("//") {
        return value.into();
    }
    "/me".into()
}

fn session_token_from_headers(headers: &HeaderMap) -> Result<String, AppError> {
    let token = headers
        .get("x-session-token")
        .context("missing x-session-token header")?
        .to_str()
        .context("x-session-token header is invalid utf-8")?
        .trim()
        .to_owned();
    if token.is_empty() {
        return Err(AppError::bad_request(
            "x-session-token header must not be empty",
        ));
    }
    Ok(token)
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
        let message = error.to_string();
        let status = if message.contains("unknown user") {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::BAD_REQUEST
        };
        Self { status, message }
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
