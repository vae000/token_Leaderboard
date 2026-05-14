use std::path::PathBuf;

use common::ToolKind;

use crate::{adapters::codex, config::CliState, http::LeaderboardClient, sync::preview_pending};

pub async fn run_doctor(api_base_url: &str, log_dir: Option<PathBuf>) -> anyhow::Result<()> {
    let state = CliState::load()?;
    let client = LeaderboardClient::new(api_base_url);

    println!("api_base_url: {api_base_url}");
    match client.healthz().await {
        Ok(health) => println!("api_health: ok {}", health),
        Err(error) => println!("api_health: failed {error}"),
    }

    println!(
        "auth_state: {}",
        if state.user_id.is_some() {
            "logged_in"
        } else {
            "missing"
        }
    );

    let codex_dir = codex::resolve_log_dir(log_dir)?;
    println!("codex_log_dir: {}", codex_dir.display());
    println!(
        "codex_log_dir_exists: {}",
        if codex_dir.exists() { "yes" } else { "no" }
    );

    let pending = preview_pending(ToolKind::Codex, &state, Some(codex_dir)).await?;
    println!("codex_pending_events: {pending}");
    Ok(())
}
