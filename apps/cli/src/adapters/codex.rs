use std::{
    collections::{BTreeMap, hash_map::DefaultHasher},
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use anyhow::Context;
use chrono::{DateTime, Utc};
use common::{ToolKind, UsageEvent};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub events: Vec<UsageEvent>,
    pub next_cursors: BTreeMap<String, u64>,
    pub discovered_files: usize,
    pub log_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
struct RawCodexRecord {
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
        let current_cursor = cursors.get(&file_key).copied().unwrap_or(0);
        let content = fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        for (index, line) in content.lines().enumerate() {
            let line_number = (index + 1) as u64;
            if line_number <= current_cursor || line.trim().is_empty() {
                continue;
            }
            let record: RawCodexRecord = serde_json::from_str(line).with_context(|| {
                format!("failed to parse line {} in {}", line_number, file.display())
            })?;
            events.push(to_usage_event(user_id, file, line_number, record));
            next_cursors.insert(file_key.clone(), line_number);
        }
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
    if let Ok(path) = std::env::var("CLI_CODEX_LOG_DIR") {
        return Ok(PathBuf::from(path));
    }
    Ok(std::env::current_dir()?.join("sample-data/codex"))
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
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "jsonl") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn to_usage_event(
    user_id: &str,
    file: &Path,
    line_number: u64,
    record: RawCodexRecord,
) -> UsageEvent {
    let raw_hash = make_hash(&(
        file.display().to_string(),
        line_number,
        &record.model,
        record.timestamp,
    ));
    UsageEvent {
        id: Uuid::new_v4(),
        event_id: format!("evt_{}", Uuid::new_v4()),
        user_id: user_id.into(),
        tool: ToolKind::Codex,
        model: record.model.clone(),
        occurred_at: record.timestamp,
        input_tokens: record.input_tokens,
        output_tokens: record.output_tokens,
        cached_tokens: record.cached_tokens,
        estimated_cost_usd: estimate_cost(&record.model, record.input_tokens, record.output_tokens),
        session_id: record.session_id,
        source_file: file.display().to_string(),
        source_offset: line_number,
        raw_hash,
    }
}

fn estimate_cost(model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
    match model {
        "gpt-5.5" => input_tokens as f64 * 0.00001 + output_tokens as f64 * 0.00003,
        "claude-opus-4.1" => input_tokens as f64 * 0.000015 + output_tokens as f64 * 0.000075,
        _ => input_tokens as f64 * 0.000002 + output_tokens as f64 * 0.000008,
    }
}

fn make_hash(value: &impl Hash) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs};

    use super::scan;

    #[test]
    fn scans_jsonl_incrementally() {
        let dir = std::env::temp_dir().join(format!("token_leaderboard_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create dir");
        let file = dir.join("events.jsonl");
        fs::write(
            &file,
            r#"{"timestamp":"2026-05-13T10:00:00Z","model":"gpt-5.5","input_tokens":10,"output_tokens":20,"cached_tokens":0,"session_id":"s1"}"#,
        )
        .expect("write");

        let result = scan("u_demo", Some(dir.clone()), &BTreeMap::new()).expect("scan");
        assert_eq!(result.events.len(), 1);

        fs::remove_dir_all(dir).ok();
    }
}
