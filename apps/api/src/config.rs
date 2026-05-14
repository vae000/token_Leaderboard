#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: Option<String>,
    pub cors_allow_origin: String,
    pub web_base_url: String,
    pub refresh_model_catalog_on_startup: bool,
    pub auto_password_salt: String,
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
            database_url: std::env::var("API__DATABASE_URL").ok().or_else(|| {
                let host = std::env::var("API__DB_HOST").unwrap_or_else(|_| "127.0.0.1".into());
                let port = std::env::var("API__DB_PORT").unwrap_or_else(|_| "5432".into());
                let user = std::env::var("API__DB_USER").unwrap_or_else(|_| "postgres".into());
                let password = std::env::var("API__DB_PASSWORD").unwrap_or_else(|_| "".into());
                let name = std::env::var("API__DB_NAME").unwrap_or_else(|_| "postgres".into());
                let schema = std::env::var("API__DB_SCHEMA");
                let mut url = format!("postgres://{user}:{password}@{host}:{port}/{name}");
                if let Ok(schema) = schema {
                    if !schema.is_empty() {
                        url.push_str(&format!("?options=-csearch_path={}%2Cpublic", schema));
                    }
                }
                Some(url)
            }),
            cors_allow_origin: std::env::var("API__CORS_ALLOW_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            web_base_url: std::env::var("API__WEB_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:3000".into()),
            refresh_model_catalog_on_startup: std::env::var(
                "API__REFRESH_MODEL_CATALOG_ON_STARTUP",
            )
            .ok()
            .map(|value| !matches!(value.as_str(), "0" | "false" | "FALSE" | "no"))
            .unwrap_or(true),
            auto_password_salt: std::env::var("API__AUTO_PASSWORD_SALT")
                .unwrap_or_else(|_| "token-leaderboard-local".into()),
            web_session_ttl_hours: std::env::var("API__WEB_SESSION_TTL_HOURS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(24 * 30),
        }
    }
}
