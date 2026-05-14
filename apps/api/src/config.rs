#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: Option<String>,
    pub cors_allow_origin: String,
    pub seed_demo_data: bool,
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
            seed_demo_data: std::env::var("API__SEED_DEMO_DATA")
                .ok()
                .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes"))
                .unwrap_or(true),
        }
    }
}
