use std::{
    collections::{BTreeMap, hash_map::DefaultHasher},
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use chrono::{DateTime, Utc};
use common::{ScanResult, ToolKind, UsageEvent};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

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

#[derive(Debug, Clone, Default)]
struct SessionState {
    session_id: Option<String>,
    current_model: Option<String>,
    previous_usage: Option<TokenUsageTotals>,
}

#[derive(Debug, Clone, Copy, Default)]
struct TokenUsageTotals {
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
    reasoning_output_tokens: u64,
}

#[derive(Debug, Clone, Copy)]
struct TokenUsageDelta {
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
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
        let mut state = SessionState::default();

        for (index, line) in content.lines().enumerate() {
            let line_number = (index + 1) as u64;
            if line.trim().is_empty() {
                continue;
            }

            let event =
                parse_line(user_id, file, line_number, line, &mut state).with_context(|| {
                    format!("failed to parse line {} in {}", line_number, file.display())
                })?;

            if line_number > current_cursor {
                if let Some(event) = event {
                    events.push(event);
                }
                next_cursors.insert(file_key.clone(), line_number);
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

pub fn resolve_log_dir(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    if let Ok(path) = std::env::var("CLI_CODEX_LOG_DIR") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
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

fn parse_line(
    user_id: &str,
    file: &Path,
    line_number: u64,
    line: &str,
    state: &mut SessionState,
) -> anyhow::Result<Option<UsageEvent>> {
    if let Ok(record) = serde_json::from_str::<RawCodexRecord>(line) {
        return Ok(Some(to_legacy_usage_event(
            user_id,
            file,
            line_number,
            record,
        )));
    }

    let value: Value = serde_json::from_str(line)?;
    let line_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();

    match line_type {
        "session_meta" => {
            state.session_id = value
                .get("payload")
                .and_then(|payload| payload.get("id"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            Ok(None)
        }
        "turn_context" => {
            if let Some(model) = value
                .get("payload")
                .and_then(|payload| payload.get("model"))
                .and_then(Value::as_str)
            {
                state.current_model = Some(model.to_owned());
            }
            Ok(None)
        }
        "event_msg" => parse_event_msg(user_id, file, line_number, &value, state),
        _ => Ok(None),
    }
}

fn parse_event_msg(
    user_id: &str,
    file: &Path,
    line_number: u64,
    value: &Value,
    state: &mut SessionState,
) -> anyhow::Result<Option<UsageEvent>> {
    let payload = match value.get("payload") {
        Some(payload) => payload,
        None => return Ok(None),
    };

    if payload.get("type").and_then(Value::as_str) != Some("token_count") {
        return Ok(None);
    }

    let info = match payload.get("info") {
        Some(Value::Object(_)) => payload.get("info").expect("info exists"),
        _ => return Ok(None),
    };

    let totals = parse_totals(
        info.get("total_token_usage")
            .context("missing total_token_usage in token_count event")?,
    )?;
    let delta = totals.delta_since(state.previous_usage)?;
    state.previous_usage = Some(totals);

    let Some(delta) = delta else {
        return Ok(None);
    };
    if delta.input_tokens == 0 && delta.cached_input_tokens == 0 && delta.output_tokens == 0 {
        return Ok(None);
    }

    let timestamp = parse_timestamp(value)?;
    let model = state
        .current_model
        .clone()
        .unwrap_or_else(|| "unknown".into());
    let raw_hash = make_hash(&(file.display().to_string(), line_number, &model, timestamp));

    Ok(Some(UsageEvent {
        id: Uuid::new_v4(),
        event_id: format!("evt_{}", Uuid::new_v4()),
        user_id: user_id.into(),
        tool: ToolKind::Codex,
        model: model.clone(),
        occurred_at: timestamp,
        input_tokens: delta.input_tokens,
        output_tokens: delta.output_tokens,
        cached_tokens: delta.cached_input_tokens,
        estimated_cost_usd: estimate_cost(&model, delta.input_tokens, delta.output_tokens),
        session_id: state.session_id.clone(),
        source_file: file.display().to_string(),
        source_offset: line_number,
        raw_hash,
    }))
}

fn parse_timestamp(value: &Value) -> anyhow::Result<DateTime<Utc>> {
    let raw = value
        .get("timestamp")
        .and_then(Value::as_str)
        .context("missing timestamp")?;
    Ok(DateTime::parse_from_rfc3339(raw)?.with_timezone(&Utc))
}

fn parse_totals(value: &Value) -> anyhow::Result<TokenUsageTotals> {
    Ok(TokenUsageTotals {
        input_tokens: get_u64_field(value, "input_tokens")?,
        cached_input_tokens: get_u64_field(value, "cached_input_tokens")?,
        output_tokens: get_u64_field(value, "output_tokens")?,
        reasoning_output_tokens: get_u64_field(value, "reasoning_output_tokens")?,
    })
}

fn get_u64_field(value: &Value, field: &str) -> anyhow::Result<u64> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .with_context(|| format!("missing `{field}`"))
}

impl TokenUsageTotals {
    fn delta_since(self, previous: Option<Self>) -> anyhow::Result<Option<TokenUsageDelta>> {
        let previous = previous.unwrap_or_default();

        if self.input_tokens < previous.input_tokens
            || self.cached_input_tokens < previous.cached_input_tokens
            || self.output_tokens < previous.output_tokens
            || self.reasoning_output_tokens < previous.reasoning_output_tokens
        {
            bail!("token usage counters decreased unexpectedly");
        }

        let input_delta = self.input_tokens - previous.input_tokens;
        let cached_delta = self.cached_input_tokens - previous.cached_input_tokens;
        let output_delta = self.output_tokens - previous.output_tokens;
        let reasoning_delta = self.reasoning_output_tokens - previous.reasoning_output_tokens;

        if input_delta == 0 && cached_delta == 0 && output_delta == 0 && reasoning_delta == 0 {
            return Ok(None);
        }

        Ok(Some(TokenUsageDelta {
            input_tokens: input_delta.saturating_sub(cached_delta),
            cached_input_tokens: cached_delta,
            output_tokens: output_delta,
        }))
    }
}

fn to_legacy_usage_event(
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
        "gpt-5.4" => input_tokens as f64 * 0.00001 + output_tokens as f64 * 0.00003,
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

    use super::{resolve_log_dir, scan};

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

    #[test]
    fn ignores_empty_env_log_dir() {
        unsafe {
            std::env::set_var("CLI_CODEX_LOG_DIR", "");
        }
        let path = resolve_log_dir(None).expect("resolve");
        assert!(path.ends_with("sample-data/codex"));
        unsafe {
            std::env::remove_var("CLI_CODEX_LOG_DIR");
        }
    }

    #[test]
    fn scans_real_codex_session_and_skips_duplicate_totals() {
        let dir = std::env::temp_dir().join(format!("token_leaderboard_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create dir");
        let file = dir.join("rollout.jsonl");
        fs::write(
            &file,
            concat!(
                "{\"timestamp\":\"2026-05-14T01:31:49.431Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"sess_1\"}}\n",
                "{\"timestamp\":\"2026-05-14T01:31:49.448Z\",\"type\":\"turn_context\",\"payload\":{\"model\":\"gpt-5.4\"}}\n",
                "{\"timestamp\":\"2026-05-14T01:31:51.558Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":null}}\n",
                "{\"timestamp\":\"2026-05-14T01:31:58.332Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"input_tokens\":100,\"cached_input_tokens\":20,\"output_tokens\":5,\"reasoning_output_tokens\":1,\"total_tokens\":105}}}}\n",
                "{\"timestamp\":\"2026-05-14T01:31:59.000Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"input_tokens\":100,\"cached_input_tokens\":20,\"output_tokens\":5,\"reasoning_output_tokens\":1,\"total_tokens\":105}}}}\n",
                "{\"timestamp\":\"2026-05-14T01:32:05.985Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"input_tokens\":140,\"cached_input_tokens\":30,\"output_tokens\":9,\"reasoning_output_tokens\":1,\"total_tokens\":149}}}}\n"
            ),
        )
        .expect("write");

        let result = scan("u_demo", Some(dir.clone()), &BTreeMap::new()).expect("scan");
        assert_eq!(result.events.len(), 2);
        assert_eq!(result.events[0].model, "gpt-5.4");
        assert_eq!(result.events[0].session_id.as_deref(), Some("sess_1"));
        assert_eq!(result.events[0].input_tokens, 80);
        assert_eq!(result.events[0].cached_tokens, 20);
        assert_eq!(result.events[0].output_tokens, 5);
        assert_eq!(result.events[1].input_tokens, 30);
        assert_eq!(result.events[1].cached_tokens, 10);
        assert_eq!(result.events[1].output_tokens, 4);

        let mut cursors = BTreeMap::new();
        cursors.insert(file.display().to_string(), 4);
        let incremental = scan("u_demo", Some(dir.clone()), &cursors).expect("incremental");
        assert_eq!(incremental.events.len(), 1);
        assert_eq!(incremental.events[0].input_tokens, 30);
        assert_eq!(incremental.events[0].cached_tokens, 10);
        assert_eq!(incremental.events[0].output_tokens, 4);

        fs::remove_dir_all(dir).ok();
    }
}
