use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{Context, bail};
use chrono::Utc;
use common::{BatchIngestRequest, ToolKind};

use crate::{adapters::codex, config::CliState, http::LeaderboardClient};

pub async fn run_sync(
    api_base_url: &str,
    tool: ToolKind,
    log_dir: Option<PathBuf>,
) -> anyhow::Result<()> {
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

    let scan_result = match tool {
        ToolKind::Codex => codex::scan(&user_id, log_dir, &state.upload_cursors)?,
        other => bail!("{other} adapter has not been implemented yet"),
    };

    if scan_result.events.is_empty() {
        println!("no new events found in {}", scan_result.log_dir.display());
        return Ok(());
    }

    let batch_size = std::env::var("CLI_SYNC_BATCH_SIZE")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(500usize);
    for chunk in scan_result.events.chunks(batch_size) {
        let response = client
            .ingest_batch(&BatchIngestRequest {
                device_id: device_id.clone(),
                user_id: user_id.clone(),
                events: chunk.to_vec(),
            })
            .await?;
        println!(
            "uploaded accepted={} deduped={} rejected={}",
            response.accepted,
            response.deduped,
            response.rejected.len()
        );
    }

    state.api_base_url = Some(api_base_url.into());
    state.upload_cursors = merge_cursors(&state.upload_cursors, &scan_result.next_cursors);
    state.last_sync_at = Some(Utc::now());
    state.save()?;

    println!(
        "sync complete: {} events from {} file(s)",
        scan_result.events.len(),
        scan_result.discovered_files
    );
    Ok(())
}

pub async fn preview_pending(
    tool: ToolKind,
    state: &CliState,
    log_dir: Option<PathBuf>,
) -> anyhow::Result<usize> {
    let user_id = state.user_id.clone().unwrap_or_else(|| "u_preview".into());
    let result = match tool {
        ToolKind::Codex => codex::scan(&user_id, log_dir, &state.upload_cursors)?,
        _ => return Ok(0),
    };
    Ok(result.events.len())
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
