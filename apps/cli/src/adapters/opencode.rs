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
struct OpenCodeSession {
    #[serde(default)]
    id: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    created_at: DateTime<Utc>,
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cached_tokens: u64,
    #[serde(default)]
    session_id: Option<String>,
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

        if current_mtime <= processed_mtime {
            next_cursors.insert(file_key, processed_mtime);
            continue;
        }

        let content = fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;

        let records: Vec<OpenCodeSession> =
            if let Ok(one) = serde_json::from_str::<OpenCodeSession>(&content) {
                vec![one]
            } else if let Ok(many) = serde_json::from_str::<Vec<OpenCodeSession>>(&content) {
                many
            } else {
                continue;
            };

        for (i, record) in records.iter().enumerate() {
            let event = make_event(user_id, file, (i + 1) as u64, record);
            events.push(event);
        }
        next_cursors.insert(file_key, current_mtime);
    }

    Ok(ScanResult {
        events,
        next_cursors,
        discovered_files: files.len(),
        log_dir,
    })
}

fn resolve_log_dir(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    if let Ok(home) = std::env::var("HOME") {
        let candidates = [
            PathBuf::from(&home).join(".opencode/sessions"),
            PathBuf::from(&home).join(".config/opencode/sessions"),
        ];
        for candidate in &candidates {
            if candidate.exists() {
                return Ok(candidate.clone());
            }
        }
    }
    Ok(std::env::current_dir()?.join("sample-data/opencode"))
}

fn discover_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in
            fs::read_dir(dir).with_context(|| format!("failed to list {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
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

fn make_event(user_id: &str, file: &Path, line: u64, session: &OpenCodeSession) -> UsageEvent {
    UsageEvent {
        id: Uuid::new_v4(),
        event_id: format!("oc_{}", Uuid::new_v4()),
        user_id: user_id.to_owned(),
        tool: ToolKind::OpenCode,
        model: if session.model.is_empty() {
            "opencode".into()
        } else {
            session.model.clone()
        },
        occurred_at: session.created_at,
        input_tokens: session.input_tokens,
        output_tokens: session.output_tokens,
        cached_tokens: session.cached_tokens,
        estimated_cost_usd: (session.input_tokens as f64 * 0.00000015
            + session.output_tokens as f64 * 0.00000060
            + session.cached_tokens as f64 * 0.000000075),
        session_id: Some(session.id.clone()),
        source_file: file.display().to_string(),
        source_offset: line,
        raw_hash: format!("oc:{}:{}", file.display(), line),
    }
}
