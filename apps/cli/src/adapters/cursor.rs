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

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct CursorEvent {
    timestamp: DateTime<Utc>,
    model: String,
    input_tokens: u64,
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

        // Try to parse line-delimited JSON (JSONL)
        for (i, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<CursorEvent>(line) {
                let event = make_event(user_id, file, (i + 1) as u64, record);
                events.push(event);
                let offset = line.as_ptr() as u64;
                next_cursors.insert(file_key.clone(), offset);
            }
        }

        // If JSONL parsing yielded nothing, try as a single JSON array
        if events.is_empty() {
            if let Ok(records) = serde_json::from_str::<Vec<CursorEvent>>(&content) {
                for (i, record) in records.iter().enumerate() {
                    let event = make_event(user_id, file, (i + 1) as u64, record.clone());
                    events.push(event);
                }
                next_cursors.insert(file_key, current_mtime);
            }
        }
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
            PathBuf::from(&home).join(".cursor/sessions"),
            PathBuf::from(&home).join(".config/Cursor/sessions"),
        ];
        for candidate in &candidates {
            if candidate.exists() {
                return Ok(candidate.clone());
            }
        }
    }
    Ok(std::env::current_dir()?.join("sample-data/cursor"))
}

fn discover_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in
            fs::read_dir(dir).with_context(|| format!("failed to list {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "jsonl" | "json" | "log") {
                    files.push(path);
                }
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

fn make_event(user_id: &str, file: &Path, line: u64, record: CursorEvent) -> UsageEvent {
    UsageEvent {
        id: Uuid::new_v4(),
        event_id: format!("cur_{}", Uuid::new_v4()),
        user_id: user_id.to_owned(),
        tool: ToolKind::Cursor,
        model: record.model,
        occurred_at: record.timestamp,
        input_tokens: record.input_tokens,
        output_tokens: record.output_tokens,
        cached_tokens: record.cached_tokens,
        estimated_cost_usd: (record.input_tokens as f64 * 0.00000015
            + record.output_tokens as f64 * 0.00000060
            + record.cached_tokens as f64 * 0.000000075),
        session_id: record.session_id,
        source_file: file.display().to_string(),
        source_offset: line,
        raw_hash: format!("cur:{}:{}", file.display(), line),
    }
}
