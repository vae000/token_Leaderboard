mod aggregation;
mod config;
mod repository;
mod routes;

use std::net::SocketAddr;

use anyhow::Context;
use config::AppConfig;
use repository::Repository;
use routes::{AppState, router};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

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
        info!("API__DATABASE_URL not set, starting with in-memory repository only");
        return Ok(None);
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .with_context(|| "failed to connect to postgres")?;
    MIGRATOR.run(&pool).await?;
    info!("database connection established and migrations applied");
    Ok(Some(pool))
}
