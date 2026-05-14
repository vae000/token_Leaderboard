mod aggregation;
mod config;
mod local_auth;
mod postgres;
mod pricing;
mod repository;
mod routes;
mod wechat;

use std::net::SocketAddr;

use anyhow::Context;
use config::AppConfig;
use repository::Repository;
use routes::{AppState, router};
use sqlx::Executor;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use url::Url;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../db/migrations");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("api=info,tower_http=info")),
        )
        .init();

    let config = AppConfig::from_env();
    let pool = connect_database(config.database_url.as_deref()).await?;
    if let Some(pool) = &pool
        && config.refresh_model_catalog_on_startup
    {
        match pricing::refresh_model_catalog(pool).await {
            Ok(updated_models) => info!(
                "refreshed model catalog from official pricing sources: {} models",
                updated_models
            ),
            Err(error) => warn!("failed to refresh model catalog from official sources: {error}"),
        }
    }
    let repository = Repository::new(config.seed_demo_data);
    let state = AppState::new(config.clone(), pool, repository);
    let app = router(state);
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .with_context(|| "invalid API__HOST or API__PORT")?;
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind to {}", config.port))?;

    info!(
        "token leaderboard api listening on {}",
        listener.local_addr()?
    );
    axum::serve(listener, app).await?;
    Ok(())
}

async fn connect_database(database_url: Option<&str>) -> anyhow::Result<Option<PgPool>> {
    let Some(database_url) = database_url else {
        anyhow::bail!("API__DATABASE_URL is required; in-memory mode has been disabled");
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .with_context(|| "failed to connect to postgres")?;
    ensure_schema(&pool, database_url).await?;
    MIGRATOR.run(&pool).await?;
    info!("database connection established and migrations applied");
    Ok(Some(pool))
}

async fn ensure_schema(pool: &PgPool, database_url: &str) -> anyhow::Result<()> {
    let Some(schema) = extract_target_schema(database_url) else {
        return Ok(());
    };

    let statement = format!("CREATE SCHEMA IF NOT EXISTS {}", quote_identifier(&schema));
    pool.execute(statement.as_str())
        .await
        .with_context(|| format!("failed to ensure schema `{schema}`"))?;
    info!("ensured schema `{schema}` exists");
    Ok(())
}

fn extract_target_schema(database_url: &str) -> Option<String> {
    let url = Url::parse(database_url).ok()?;

    for (key, value) in url.query_pairs() {
        if key == "options" {
            if let Some(schema) = parse_search_path_option(&value) {
                return Some(schema);
            }
        }
    }

    None
}

fn parse_search_path_option(options: &str) -> Option<String> {
    for part in options.split_whitespace() {
        let Some(search_path) = part.strip_prefix("-csearch_path=") else {
            continue;
        };

        for raw_schema in search_path.split(',') {
            let schema = raw_schema.trim().trim_matches('"');
            if schema.is_empty() || matches!(schema, "public" | "pg_catalog") {
                continue;
            }
            return Some(schema.to_owned());
        }
    }

    None
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::{extract_target_schema, parse_search_path_option, quote_identifier};

    #[test]
    fn extracts_schema_from_search_path_option() {
        assert_eq!(
            parse_search_path_option("-csearch_path=token_leaderboard,public"),
            Some("token_leaderboard".into())
        );
    }

    #[test]
    fn extracts_quoted_schema_from_database_url() {
        let database_url = "postgres://postgres:pass@localhost:5432/db?options=-csearch_path%3D%22Token%22%2Cpublic";
        assert_eq!(extract_target_schema(database_url), Some("Token".into()));
    }

    #[test]
    fn quotes_identifier_safely() {
        assert_eq!(
            quote_identifier("token_leaderboard"),
            "\"token_leaderboard\""
        );
        assert_eq!(quote_identifier("bad\"name"), "\"bad\"\"name\"");
    }
}
