mod aggregation;
mod config;
mod local_auth;
mod postgres;
mod pricing;
mod repository;
mod routes;

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
    let bj_offset = time::UtcOffset::from_hms(8, 0, 0).expect("+08:00");
    let bj_format = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3][offset_hour sign:mandatory]:[offset_minute]",
    ).expect("valid format");
    let beijing_timer = tracing_subscriber::fmt::time::OffsetTime::new(bj_offset, bj_format);
    tracing_subscriber::fmt()
        .with_timer(beijing_timer)
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
    let repository = Repository::new();
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

    ensure_database_exists(database_url).await?;
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

async fn ensure_database_exists(database_url: &str) -> anyhow::Result<()> {
    let Some(database_name) = extract_database_name(database_url) else {
        return Ok(());
    };
    if database_name == "postgres" {
        return Ok(());
    }

    let admin_database_url = build_admin_database_url(database_url)?;
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_database_url)
        .await
        .with_context(
            || "failed to connect to admin postgres database while ensuring target database exists",
        )?;

    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)",
    )
    .bind(&database_name)
    .fetch_optional(&admin_pool)
    .await
    .with_context(|| format!("failed to check whether database `{database_name}` exists"))?;

    if exists.unwrap_or(false) {
        return Ok(());
    }

    let statement = format!("CREATE DATABASE {}", quote_identifier(&database_name));
    admin_pool
        .execute(statement.as_str())
        .await
        .with_context(|| format!("failed to create database `{database_name}`"))?;
    info!("created database `{database_name}`");
    Ok(())
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

fn extract_database_name(database_url: &str) -> Option<String> {
    let url = Url::parse(database_url).ok()?;
    let database_name = url.path().trim_start_matches('/').trim();
    if database_name.is_empty() {
        None
    } else {
        Some(database_name.to_owned())
    }
}

fn build_admin_database_url(database_url: &str) -> anyhow::Result<String> {
    let mut url = Url::parse(database_url).with_context(|| "invalid database url")?;
    url.set_path("/postgres");
    Ok(url.to_string())
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
    use super::{
        build_admin_database_url, extract_database_name, extract_target_schema,
        parse_search_path_option, quote_identifier,
    };

    #[test]
    fn extracts_database_name_from_database_url() {
        let database_url =
            "postgres://postgres:pass@localhost:5432/token?options=-csearch_path%3Dtoken%2Cpublic";
        assert_eq!(extract_database_name(database_url), Some("token".into()));
    }

    #[test]
    fn builds_admin_database_url() {
        let database_url =
            "postgres://postgres:pass@localhost:5432/token?options=-csearch_path%3Dtoken%2Cpublic";
        assert_eq!(
            build_admin_database_url(database_url).expect("admin url"),
            "postgres://postgres:pass@localhost:5432/postgres?options=-csearch_path%3Dtoken%2Cpublic"
        );
    }

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
