use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::{Context, bail};
use chrono::Utc;
use common::{BatchIngestRequest, ToolKind};
use tokio::time::sleep;

use crate::{
    adapters::codex, adapters::deepseek_tui, config::{state_dir, CliState},
    http::LeaderboardClient,
};

/// 当前已实现适配器的工具列表。
const SUPPORTED_TOOLS: &[ToolKind] = &[ToolKind::Codex, ToolKind::DeepseekTui];

// ─── 守护进程管理 ───────────────────────────────────────

fn pid_path() -> anyhow::Result<PathBuf> {
    Ok(state_dir()?.join("daemon.pid"))
}

/// 启动后台守护进程。
pub async fn start_daemon(api_base_url: &str, interval_seconds: u64) -> anyhow::Result<()> {
    let pid_path = pid_path()?;

    // 检查是否已在运行
    if let Ok(Some(pid)) = read_pid(&pid_path) {
        if is_process_alive(pid) {
            println!("daemon already running (pid {})", pid);
            return Ok(());
        }
        // 进程已死，清理失效 PID 文件
        fs::remove_file(&pid_path).ok();
    }

    let exe = std::env::current_exe().context("failed to get current executable path")?;

    let child = Command::new(&exe)
        .args([
            "--api-base-url",
            api_base_url,
            "--daemon",
            "start",
            "--interval-seconds",
            &interval_seconds.to_string(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .context("failed to spawn daemon process")?;

    let pid = child.id();
    fs::write(&pid_path, pid.to_string())
        .with_context(|| format!("failed to write PID file {}", pid_path.display()))?;

    println!("daemon started (pid {})", pid);
    Ok(())
}

/// 停止后台守护进程。
pub fn stop_daemon() -> anyhow::Result<()> {
    let pid_path = pid_path()?;
    let pid = match read_pid(&pid_path)? {
        Some(pid) => pid,
        None => {
            println!("daemon is not running");
            return Ok(());
        }
    };

    if !is_process_alive(pid) {
        println!("daemon is not running (stale PID file removed)");
        fs::remove_file(&pid_path).ok();
        return Ok(());
    }

    // 发送 SIGTERM
    let status = Command::new("kill")
        .args([&pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(s) if s.success() => {
            // 等待进程退出
            for _ in 0..10 {
                if !is_process_alive(pid) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            fs::remove_file(&pid_path).ok();
            println!("daemon stopped (pid {})", pid);
        }
        _ => {
            // 如果进程已经不存在，清理 PID 文件
            if !is_process_alive(pid) {
                fs::remove_file(&pid_path).ok();
                println!("daemon is not running (stale PID file removed)");
            } else {
                eprintln!("failed to stop daemon (pid {pid}): kill command failed");
            }
        }
    }

    Ok(())
}

/// 查看守护进程状态。
pub fn daemon_status() -> anyhow::Result<()> {
    let pid_path = pid_path()?;
    let state = CliState::load().unwrap_or_default();

    println!("── Token Leaderboard 守护进程 ──");

    match read_pid(&pid_path)? {
        Some(pid) if is_process_alive(pid) => {
            println!("状态: 运行中 (pid {})", pid);
        }
        Some(_pid) => {
            println!("状态: 已停止 (残留 PID 文件)");
            fs::remove_file(&pid_path).ok();
        }
        None => {
            println!("状态: 未启动");
        }
    }

    println!(
        "用户: {}",
        state.user_id.clone().unwrap_or_else(|| "<未登录>".into())
    );
    println!(
        "上次同步: {}",
        state
            .last_sync_at
            .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "从未".into())
    );

    // 统计待处理事件
    let total_pending = preview_pending_all(&state).unwrap_or(0);
    println!("待处理事件: {}", total_pending);

    println!(
        "支持的工具: {}",
        SUPPORTED_TOOLS
            .iter()
            .map(|t| t.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(())
}

fn read_pid(path: &Path) -> anyhow::Result<Option<u32>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read PID file {}", path.display()))?;
    let pid: u32 = content.trim().parse().map_err(|e| {
        anyhow::anyhow!("invalid PID in {}: {e}", path.display())
    })?;
    Ok(Some(pid))
}

fn is_process_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()]) // -0 = null signal: 只检查不发送
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ─── 守护进程循环 ───────────────────────────────────────

/// 守护进程主循环（由 `--daemon` 隐藏参数启动的子进程运行）。
pub async fn run_daemon(api_base_url: &str) -> anyhow::Result<()> {
    println!("daemon child started (pid {})", std::process::id());

    let interval_seconds = std::env::var("CLI_SYNC_INTERVAL_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60u64);

    loop {
        match run_sync_all(api_base_url).await {
            Ok(()) => {}
            Err(e) => {
                eprintln!("[daemon] sync failed: {e:#}");
            }
        }

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("[daemon] received SIGTERM, exiting");
                break;
            }
            _ = sleep(Duration::from_secs(interval_seconds)) => {}
        }
    }

    Ok(())
}

// ─── 全工具同步逻辑 ─────────────────────────────────────

/// 扫描全部已实现适配器的工具，执行一次同步。
async fn run_sync_all(api_base_url: &str) -> anyhow::Result<()> {
    let mut state = CliState::load()?;
    let user_id = state
        .user_id
        .clone()
        .context("please run `leaderboard login` first")?;
    let device_id = state
        .device_id
        .clone()
        .context("missing device_id, please login again")?;
    let client = LeaderboardClient::new(api_base_url);

    if let Some(refresh_token) = state.refresh_token.clone() {
        let refreshed = client
            .refresh_token(&common::RefreshTokenRequest {
                device_id: device_id.clone(),
                refresh_token,
            })
            .await?;
        state.access_token = Some(refreshed.access_token);
        state.refresh_token = Some(refreshed.refresh_token);
    }

    let mut total_events = 0usize;
    let mut total_files = 0usize;

    for &tool in SUPPORTED_TOOLS {
        let result = scan_tool(tool, &user_id, &state.upload_cursors)
            .map_err(|e| eprintln!("[{tool}] scan failed: {e:#}"))
            .ok();

        let Some(result) = result else {
            continue;
        };

        if result.events.is_empty() {
            println!("[{tool}] no new events found in {}", result.log_dir.display());
            continue;
        }

        let batch_size = std::env::var("CLI_SYNC_BATCH_SIZE")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(500usize);
        for chunk in result.events.chunks(batch_size) {
            let response = client
                .ingest_batch(&BatchIngestRequest {
                    device_id: device_id.clone(),
                    user_id: user_id.clone(),
                    events: chunk.to_vec(),
                })
                .await?;
            println!(
                "[{tool}] uploaded accepted={} deduped={} rejected={}",
                response.accepted,
                response.deduped,
                response.rejected.len()
            );
        }

        state.upload_cursors = merge_cursors(&state.upload_cursors, &result.next_cursors);
        total_events += result.events.len();
        total_files += result.discovered_files;
    }

    state.api_base_url = Some(api_base_url.into());
    state.last_sync_at = Some(Utc::now());
    state.save()?;

    if total_events > 0 {
        println!(
            "sync complete: {total_events} events from {total_files} file(s) across {} tool(s)",
            SUPPORTED_TOOLS.len()
        );
    }

    Ok(())
}

/// 对单个工具执行扫描，返回扫描结果。
fn scan_tool(
    tool: ToolKind,
    user_id: &str,
    cursors: &BTreeMap<String, u64>,
) -> anyhow::Result<common::ScanResult> {
    match tool {
        ToolKind::Codex => codex::scan(user_id, None, cursors),
        ToolKind::DeepseekTui => deepseek_tui::scan(user_id, None, cursors),
        other => bail!("{other} adapter has not been implemented yet"),
    }
}

/// 预览单个工具的待处理事件数。
pub fn preview_pending(
    tool: ToolKind,
    state: &CliState,
    log_dir: Option<PathBuf>,
) -> anyhow::Result<usize> {
    let user_id = state.user_id.clone().unwrap_or_else(|| "u_preview".into());
    let result = match tool {
        ToolKind::Codex => codex::scan(&user_id, log_dir, &state.upload_cursors)?,
        ToolKind::DeepseekTui => deepseek_tui::scan(&user_id, log_dir, &state.upload_cursors)?,
        _ => return Ok(0),
    };
    Ok(result.events.len())
}

/// 预览所有工具待处理的事件数总和。
fn preview_pending_all(state: &CliState) -> anyhow::Result<usize> {
    let user_id = state.user_id.clone().unwrap_or_else(|| "u_preview".into());
    let mut total = 0usize;
    for &tool in SUPPORTED_TOOLS {
        let result = scan_tool(tool, &user_id, &state.upload_cursors)
            .map_err(|e| eprintln!("[{tool}] preview failed: {e:#}"))
            .ok();
        if let Some(r) = result {
            total += r.events.len();
        }
    }
    Ok(total)
}

fn merge_cursors(
    current: &BTreeMap<String, u64>,
    next: &BTreeMap<String, u64>,
) -> BTreeMap<String, u64> {
    let mut merged = current.clone();
    for (file, offset) in next {
        let entry = merged.entry(file.clone()).or_default();
        *entry = (*entry).max(*offset);
    }
    merged
}
