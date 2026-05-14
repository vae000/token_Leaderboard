#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: Option<String>,
    pub cors_allow_origin: String,
    pub public_base_url: String,
    pub web_base_url: String,
    pub seed_demo_data: bool,
    pub refresh_model_catalog_on_startup: bool,
    pub auto_password_salt: String,
    pub wechat_app_id: Option<String>,
    pub wechat_app_secret: Option<String>,
    pub web_session_ttl_hours: u64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("API__HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("API__PORT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(8080),
            database_url: std::env::var("API__DATABASE_URL").ok(),
            cors_allow_origin: std::env::var("API__CORS_ALLOW_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            public_base_url: std::env::var("API__PUBLIC_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".into()),
            web_base_url: std::env::var("API__WEB_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:3000".into()),
            seed_demo_data: std::env::var("API__SEED_DEMO_DATA")
                .ok()
                .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes"))
                .unwrap_or(false),
            refresh_model_catalog_on_startup: std::env::var(
                "API__REFRESH_MODEL_CATALOG_ON_STARTUP",
            )
            .ok()
            .map(|value| !matches!(value.as_str(), "0" | "false" | "FALSE" | "no"))
            .unwrap_or(true),
            auto_password_salt: std::env::var("API__AUTO_PASSWORD_SALT")
                .unwrap_or_else(|_| "token-leaderboard-local".into()),
            wechat_app_id: std::env::var("API__WECHAT_APP_ID").ok(),
            wechat_app_secret: std::env::var("API__WECHAT_APP_SECRET").ok(),
            web_session_ttl_hours: std::env::var("API__WEB_SESSION_TTL_HOURS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(24 * 30),
        }
    }
}
