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
struct ClaudeSession {
    id: String,
    #[serde(default)]
    model: String,
    created_at: DateTime<Utc>,
    #[serde(default)]
    updated_at: DateTime<Utc>,
    #[serde(default)]
    total_input_tokens: u64,
    #[serde(default)]
    total_output_tokens: u64,
    #[serde(default)]
    total_cached_tokens: u64,
    #[serde(default)]
    messages: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ClaudeJsonlRecord {
    #[serde(rename = "type")]
    record_type: String,
    #[serde(default, rename = "sessionId")]
    session_id: Option<String>,
    #[serde(default)]
    timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    message: Option<ClaudeJsonlMessage>,
    #[serde(default)]
    uuid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeJsonlMessage {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: Vec<ClaudeContentPart>,
    #[serde(default)]
    usage: Option<ClaudeUsage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContentPart {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize, Default)]
struct ClaudeUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cached_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
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

        if let Ok(session) = serde_json::from_str::<ClaudeSession>(&content) {
            let event = make_event(user_id, file, &session)?;
            events.push(event);
            next_cursors.insert(file_key, current_mtime);
            continue;
        }

        for (index, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(event) = parse_jsonl_record(user_id, file, (index + 1) as u64, line)? {
                events.push(event);
            }
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
            PathBuf::from(&home).join(".claude/projects"),
            PathBuf::from(&home).join(".claude/sessions"),
            PathBuf::from(&home).join(".config/claude/projects"),
            PathBuf::from(&home).join(".config/claude/sessions"),
        ];
        for candidate in &candidates {
            if candidate.exists() {
                return Ok(candidate.clone());
            }
        }
    }
    Ok(std::env::current_dir()?.join("sample-data/claude-code"))
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
                continue;
            }
            let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
            if matches!(ext, "json" | "jsonl") {
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

fn make_event(user_id: &str, file: &Path, session: &ClaudeSession) -> anyhow::Result<UsageEvent> {
    let cost_est = session.total_input_tokens as f64 * 0.00000015
        + session.total_output_tokens as f64 * 0.00000060
        + session.total_cached_tokens as f64 * 0.000000075;

    Ok(UsageEvent {
        id: Uuid::new_v4(),
        event_id: format!("cc_{}", Uuid::new_v4()),
        user_id: user_id.to_owned(),
        tool: ToolKind::ClaudeCode,
        model: if session.model.is_empty() {
            "claude".into()
        } else {
            session.model.clone()
        },
        occurred_at: session.created_at,
        input_tokens: session.total_input_tokens,
        output_tokens: session.total_output_tokens,
        cached_tokens: session.total_cached_tokens,
        estimated_cost_usd: cost_est,
        session_id: Some(session.id.clone()),
        source_file: file.display().to_string(),
        source_offset: 0,
        raw_hash: format!("cc:{}:{}", file.display(), session.id),
    })
}

fn parse_jsonl_record(
    user_id: &str,
    file: &Path,
    line_number: u64,
    line: &str,
) -> anyhow::Result<Option<UsageEvent>> {
    let record = match serde_json::from_str::<ClaudeJsonlRecord>(line) {
        Ok(record) => record,
        Err(_) => return Ok(None),
    };

    if record.record_type != "assistant" {
        return Ok(None);
    }

    let Some(message) = record.message else {
        return Ok(None);
    };
    if message.role.as_deref() != Some("assistant") {
        return Ok(None);
    }
    let Some(usage) = message.usage else {
        return Ok(None);
    };
    if !message
        .content
        .iter()
        .any(|part| part.kind.as_str() != "thinking")
    {
        return Ok(None);
    }

    let cached_tokens = usage.cached_tokens.max(usage.cache_read_input_tokens);
    if usage.input_tokens == 0 && usage.output_tokens == 0 && cached_tokens == 0 {
        return Ok(None);
    }

    let model = message.model.unwrap_or_else(|| "claude".into());
    let session_id = record.session_id.or(message.id.clone());
    let record_id = message
        .id
        .or(record.uuid)
        .unwrap_or_else(|| format!("line-{line_number}"));

    Ok(Some(UsageEvent {
        id: Uuid::new_v4(),
        event_id: format!("cc_{}", Uuid::new_v4()),
        user_id: user_id.to_owned(),
        tool: ToolKind::ClaudeCode,
        model,
        occurred_at: record.timestamp.unwrap_or_else(Utc::now),
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cached_tokens,
        estimated_cost_usd: (usage.input_tokens as f64 * 0.00000015)
            + (usage.output_tokens as f64 * 0.00000060)
            + (cached_tokens as f64 * 0.000000075),
        session_id,
        source_file: file.display().to_string(),
        source_offset: line_number,
        raw_hash: format!("cc:{}:{}", file.display(), record_id),
    }))
}

#[cfg(test)]
mod tests {
    use super::{resolve_log_dir, scan};
    use std::{collections::BTreeMap, fs};

    #[test]
    fn parses_real_claude_jsonl_shape() {
        let dir =
            std::env::temp_dir().join(format!("token_leaderboard_claude_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create dir");
        let file = dir.join("session.jsonl");
        fs::write(
            &file,
            concat!(
                "{\"type\":\"assistant\",\"uuid\":\"u1\",\"timestamp\":\"2026-04-05T06:32:47.004Z\",\"sessionId\":\"sess-1\",\"message\":{\"id\":\"msg-1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"thinking\",\"thinking\":\"...\"}],\"model\":\"kimi-k2.5\",\"usage\":{\"input_tokens\":20350,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":0,\"output_tokens\":0,\"prompt_tokens\":20350,\"cached_tokens\":0}}}\n",
                "{\"type\":\"assistant\",\"uuid\":\"u2\",\"timestamp\":\"2026-04-05T06:32:47.083Z\",\"sessionId\":\"sess-1\",\"message\":{\"id\":\"msg-1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"How can I help you today?\"}],\"model\":\"kimi-k2.5\",\"usage\":{\"input_tokens\":3454,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":16896,\"output_tokens\":34}}}\n"
            ),
        )
        .expect("write");

        let result = scan("u_demo", Some(dir.clone()), &BTreeMap::new()).expect("scan");
        assert_eq!(result.events.len(), 1);
        let event = &result.events[0];
        assert_eq!(event.model, "kimi-k2.5");
        assert_eq!(event.input_tokens, 3454);
        assert_eq!(event.cached_tokens, 16896);
        assert_eq!(event.output_tokens, 34);
        assert_eq!(event.session_id.as_deref(), Some("sess-1"));

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn default_log_dir_prefers_projects() {
        let path = resolve_log_dir(None).expect("resolve");
        assert!(
            path.ends_with("projects")
                || path.ends_with("sessions")
                || path.ends_with("claude-code"),
            "unexpected path: {}",
            path.display()
        );
    }

    #[test]
    #[ignore = "requires local real Claude Code logs under ~/.claude/projects"]
    fn smoke_real_claude_logs_if_available() {
        let dir = resolve_log_dir(None).expect("resolve");
        if !dir.exists() {
            return;
        }

        let result = scan("u_demo", Some(dir), &BTreeMap::new()).expect("scan");
        eprintln!(
            "parsed {} Claude Code events from {}",
            result.events.len(),
            result.log_dir.display()
        );
        assert!(
            !result.events.is_empty(),
            "real Claude Code log dir exists but no events were parsed"
        );
    }
}
