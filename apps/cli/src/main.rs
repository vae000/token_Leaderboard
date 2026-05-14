#![allow(dead_code)]

mod adapters;
mod auth;
mod config;
mod diagnostics;
mod http;
mod sync;

use clap::{Parser, Subcommand};

use crate::{
    sync::{daemon_status, start_daemon, stop_daemon},
};

#[derive(Debug, Parser)]
#[command(name = "leaderboard", about = "Token leaderboard 本地采集守护进程")]
struct Cli {
    #[arg(
        long,
        env = "CLI_API_BASE_URL",
        default_value = "http://127.0.0.1:8080"
    )]
    api_base_url: String,
    #[command(subcommand)]
    command: Commands,
    /// 隐藏参数：由 start 命令内部调用，启动守护进程循环
    #[arg(long, hide = true)]
    daemon: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// 启动后台守护进程，自动扫描所有工具并持续监控
    Start {
        /// 监控轮询间隔（秒）
        #[arg(long, env = "CLI_SYNC_INTERVAL_SECONDS", default_value_t = 60)]
        interval_seconds: u64,
    },
    /// 停止后台守护进程
    Stop,
    /// 重启后台守护进程
    Restart {
        /// 监控轮询间隔（秒）
        #[arg(long, env = "CLI_SYNC_INTERVAL_SECONDS", default_value_t = 60)]
        interval_seconds: u64,
    },
    /// 查看守护进程状态
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    // 隐藏的守护进程模式：由 start 命令内部启动的子进程
    if cli.daemon {
        return sync::run_daemon(&cli.api_base_url).await;
    }

    match cli.command {
        Commands::Start { interval_seconds } => {
            start_daemon(&cli.api_base_url, interval_seconds).await?;
        }
        Commands::Stop => {
            stop_daemon()?;
        }
        Commands::Restart { interval_seconds } => {
            stop_daemon().ok();
            start_daemon(&cli.api_base_url, interval_seconds).await?;
        }
        Commands::Status => {
            daemon_status()?;
        }
    }

    Ok(())
}
