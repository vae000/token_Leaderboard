use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use chrono::{DateTime, Utc};
use common::{ScanResult, ToolKind, UsageEvent};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SessionFile {
    schema_version: u64,
    metadata: SessionMetadata,
    messages: Vec<serde_json::Value>,
    system_prompt: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SessionMetadata {
    id: String,
    title: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    message_count: u64,
    total_tokens: u64,
    model: String,
    workspace: Option<String>,
    mode: Option<String>,
    cost: Option<SessionCost>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SessionCost {
    #[serde(default)]
    session_cost_usd: f64,
    #[serde(default)]
    session_cost_cny: f64,
    #[serde(default)]
    subagent_cost_usd: f64,
    #[serde(default)]
    subagent_cost_cny: f64,
}

pub fn scan(
    user_id: &str,
    log_dir: Option<PathBuf>,
    cursors: &BTreeMap<String, u64>,
) -> anyhow::Result<ScanResult> {
    let log_dir = resolve_log_dir(log_dir)?;
    if !log_dir.exists() {
        return Ok(ScanResult {
            events: Vec::new(),
            next_cursors: cursors.clone(),
            discovered_files: 0,
            log_dir,
        });
    }

    let files = discover_files(&log_dir)?;
    let mut events = Vec::new();
    let mut next_cursors = cursors.clone();

    for file in &files {
        let file_key = file.to_string_lossy().to_string();
        let processed_mtime = cursors.get(&file_key).copied().unwrap_or(0);
        let current_mtime = mtime_nanos(file)?;

        // Skip files that haven't been modified since last scan
        if current_mtime <= processed_mtime {
            next_cursors.insert(file_key, processed_mtime);
            continue;
        }

        let content = fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;

        let session: SessionFile = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse session file {}", file.display()))?;

        let event = make_event(user_id, file, &session)?;
        events.push(event);
        next_cursors.insert(file_key, current_mtime);
    }

    Ok(ScanResult {
        events,
        next_cursors,
        discovered_files: files.len(),
        log_dir,
    })
}

pub fn resolve_log_dir(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    if let Ok(path) = std::env::var("CLI_DEEPSEEK_TUI_LOG_DIR") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    // Default to the user's deepseek TUI sessions directory
    if let Some(data_dir) = dirs_data_dir() {
        return Ok(data_dir.join("sessions"));
    }
    // Fallback for dev/testing
    Ok(std::env::current_dir()?.join("sample-data/deepseek-tui"))
}

/// Returns the deepseek TUI data directory (`~/.deepseek`)
fn dirs_data_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    let path = PathBuf::from(home).join(".deepseek");
    if path.exists() { Some(path) } else { None }
}

fn discover_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in fs::read_dir(&current)
            .with_context(|| format!("failed to list {}", current.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Skip checkpoints directory
                if path.file_name().is_some_and(|n| n != "checkpoints") {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|ext| ext == "json") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn mtime_nanos(path: &Path) -> anyhow::Result<u64> {
    let meta = fs::metadata(path)?;
    let mtime = meta.modified().context("failed to read mtime")?;
    let duration = mtime
        .duration_since(std::time::UNIX_EPOCH)
        .context("file mtime before unix epoch")?;
    Ok(duration.as_nanos() as u64)
}

fn make_event(user_id: &str, file: &Path, session: &SessionFile) -> anyhow::Result<UsageEvent> {
    let meta = &session.metadata;
    let event_id = format!("dse_{}", Uuid::new_v4());
    let raw_hash = format!("{}:{}", ToolKind::DeepseekTui, meta.id);

    // The session only tracks total_tokens; estimate breakdown.
    // DeepSeek API typical ratio: ~65% input, ~30% output, ~5% cached.
    let input_tokens = (meta.total_tokens as f64 * 0.65) as u64;
    let output_tokens = (meta.total_tokens as f64 * 0.30) as u64;
    let cached_tokens = meta
        .total_tokens
        .saturating_sub(input_tokens + output_tokens);

    // Use cost from metadata, or estimate if unavailable
    let estimated_cost_usd = meta
        .cost
        .as_ref()
        .map(|c| c.session_cost_usd)
        .filter(|&c| c > 0.0)
        .unwrap_or_else(|| {
            input_tokens as f64 * 0.00000015
                + output_tokens as f64 * 0.00000060
                + cached_tokens as f64 * 0.000000075
        });

    Ok(UsageEvent {
        id: Uuid::new_v4(),
        event_id,
        user_id: user_id.to_owned(),
        tool: ToolKind::DeepseekTui,
        model: meta.model.clone(),
        occurred_at: meta.created_at,
        input_tokens,
        output_tokens,
        cached_tokens,
        estimated_cost_usd,
        session_id: Some(meta.id.clone()),
        source_file: file.display().to_string(),
        source_offset: 0,
        raw_hash,
    })
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs};

    use super::{resolve_log_dir, scan};

    #[test]
    fn parses_session_json() {
        let dir =
            std::env::temp_dir().join(format!("token_leaderboard_dstest_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create dir");
        let file = dir.join("session.json");
        fs::write(
            &file,
            r#"{
  "schema_version": 1,
  "metadata": {
    "id": "test-session-1",
    "title": "test",
    "created_at": "2026-05-14T08:00:00Z",
    "updated_at": "2026-05-14T09:00:00Z",
    "message_count": 10,
    "total_tokens": 100000,
    "model": "deepseek-v4-pro",
    "workspace": "/test",
    "mode": "agent"
  },
  "messages": [{"role": "user", "content": [{"type": "text", "text": "hello"}]}],
  "system_prompt": "test"
}"#,
        )
        .expect("write");

        // Small delay so mtime differs
        std::thread::sleep(std::time::Duration::from_millis(10));

        let result = scan("u_demo", Some(dir.clone()), &BTreeMap::new()).expect("scan");
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].model, "deepseek-v4-pro");
        assert_eq!(
            result.events[0].session_id.as_deref(),
            Some("test-session-1")
        );
        assert!(result.events[0].total_tokens() <= 100000);
        assert!(
            result.events[0].input_tokens
                + result.events[0].output_tokens
                + result.events[0].cached_tokens
                >= 99000
        );
        assert!(result.events[0].input_tokens > 0);
        assert!(result.events[0].output_tokens > 0);

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn skips_unmodified_files() {
        let dir = std::env::temp_dir().join(format!(
            "token_leaderboard_dstest2_{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&dir).expect("create dir");
        let file = dir.join("session.json");
        fs::write(
            &file,
            r#"{"schema_version":1,"metadata":{"id":"s1","title":"t","created_at":"2026-05-14T08:00:00Z","updated_at":"2026-05-14T09:00:00Z","message_count":1,"total_tokens":5000,"model":"deepseek-v4-pro","workspace":"/t"},"messages":[],"system_prompt":""}"#,
        )
        .expect("write");

        let meta = fs::metadata(&file).expect("metadata");
        let mtime = meta.modified().expect("mtime");
        let nanos = mtime
            .duration_since(std::time::UNIX_EPOCH)
            .expect("epoch")
            .as_nanos() as u64;

        let mut cursors = BTreeMap::new();
        cursors.insert(file.display().to_string(), nanos);

        // Should skip since mtime matches cursor
        let result = scan("u_demo", Some(dir.clone()), &cursors).expect("scan");
        assert_eq!(result.events.len(), 0);

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn default_log_dir_is_deepseek_sessions() {
        let path = resolve_log_dir(None).expect("resolve");
        // Should end with either "sessions" (real env) or "sample-data/deepseek-tui" (dev)
        assert!(
            path.ends_with("sessions") || path.ends_with("deepseek-tui"),
            "unexpected path: {}",
            path.display()
        );
    }
}
