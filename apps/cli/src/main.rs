mod adapters;
mod auth;
mod config;
mod diagnostics;
mod http;
mod sync;

use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use common::ToolKind;

use crate::{
    auth::{login, logout},
    config::CliState,
    diagnostics::run_doctor,
    sync::{preview_pending, run_sync},
};

#[derive(Debug, Parser)]
#[command(name = "leaderboard", about = "Token leaderboard local collector")]
struct Cli {
    #[arg(
        long,
        env = "CLI_API_BASE_URL",
        default_value = "http://127.0.0.1:8080"
    )]
    api_base_url: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Login {
        #[arg(long, default_value = "u_demo")]
        user_id: String,
        #[arg(long, default_value = "Demo User")]
        name: String,
    },
    Sync {
        #[arg(long, default_value = "codex")]
        tool: String,
        #[arg(long)]
        log_dir: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        log_dir: Option<PathBuf>,
    },
    Logout,
    Doctor {
        #[arg(long)]
        log_dir: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Commands::Login { user_id, name } => {
            login(&cli.api_base_url, &user_id, &name).await?;
        }
        Commands::Sync { tool, log_dir } => {
            let tool = parse_tool(&tool)?;
            run_sync(&cli.api_base_url, tool, log_dir).await?;
        }
        Commands::Status { log_dir } => {
            let state = CliState::load()?;
            let pending = preview_pending(ToolKind::Codex, &state, log_dir)
                .await
                .unwrap_or(0);
            print_status(&state, pending);
        }
        Commands::Logout => {
            logout()?;
        }
        Commands::Doctor { log_dir } => {
            run_doctor(&cli.api_base_url, log_dir).await?;
        }
    }

    Ok(())
}

fn parse_tool(raw: &str) -> anyhow::Result<ToolKind> {
    raw.parse::<ToolKind>()
        .map_err(|error| anyhow::anyhow!(error))
        .with_context(|| format!("unsupported tool `{raw}`"))
}

fn print_status(state: &CliState, pending: usize) {
    println!(
        "user: {}",
        state
            .user_id
            .clone()
            .unwrap_or_else(|| "<not logged in>".into())
    );
    println!(
        "name: {}",
        state.display_name.clone().unwrap_or_else(|| "-".into())
    );
    println!(
        "device_id: {}",
        state.device_id.clone().unwrap_or_else(|| "-".into())
    );
    println!(
        "last_sync_at: {}",
        state
            .last_sync_at
            .map(|value| value.to_rfc3339())
            .unwrap_or_else(|| "never".into())
    );
    println!("pending_events: {pending}");
    println!("recognized_tools: codex");
}
