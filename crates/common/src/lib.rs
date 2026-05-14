use std::{collections::BTreeMap, fmt, path::PathBuf, str::FromStr};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Codex,
    Cursor,
    ClaudeCode,
    OpenCode,
    DeepseekTui,
}

impl ToolKind {
    pub const ALL: [Self; 5] = [
        Self::Codex,
        Self::Cursor,
        Self::ClaudeCode,
        Self::OpenCode,
        Self::DeepseekTui,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Cursor => "cursor",
            Self::ClaudeCode => "claude_code",
            Self::OpenCode => "opencode",
            Self::DeepseekTui => "deepseek_tui",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Cursor => "Cursor",
            Self::ClaudeCode => "Claude Code",
            Self::OpenCode => "OpenCode",
            Self::DeepseekTui => "DeepSeek-TUI",
        }
    }
}

impl fmt::Display for ToolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ToolKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "codex" => Ok(Self::Codex),
            "cursor" => Ok(Self::Cursor),
            "claude_code" | "claude-code" => Ok(Self::ClaudeCode),
            "opencode" | "open-code" => Ok(Self::OpenCode),
            "deepseek_tui" | "deepseek-tui" => Ok(Self::DeepseekTui),
            _ => Err(format!("unsupported tool: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub id: Uuid,
    pub event_id: String,
    pub user_id: String,
    pub tool: ToolKind,
    pub model: String,
    pub occurred_at: DateTime<Utc>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub estimated_cost_usd: f64,
    pub session_id: Option<String>,
    pub source_file: String,
    pub source_offset: u64,
    pub raw_hash: String,
}

impl UsageEvent {
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cached_tokens
    }

    pub fn idempotency_key(&self) -> String {
        if !self.raw_hash.is_empty() {
            return self.raw_hash.clone();
        }

        format!(
            "{}:{}:{}:{}",
            self.tool,
            self.source_file,
            self.source_offset,
            self.occurred_at.timestamp()
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStartRequest {
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStartResponse {
    pub device_id: String,
    pub login_url: String,
    pub poll_after_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bound_user: Option<AuthenticatedUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCallbackResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub display_name: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSessionResponse {
    pub user: AuthenticatedUser,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordLoginResponse {
    pub session_token: String,
    pub user: AuthenticatedUser,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCredential {
    pub user_id: String,
    pub display_name: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    pub device_id: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchIngestRequest {
    pub device_id: String,
    pub user_id: String,
    pub events: Vec<UsageEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedEvent {
    pub event_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchIngestResponse {
    pub accepted: usize,
    pub deduped: usize,
    pub rejected: Vec<RejectedEvent>,
    pub next_sync_after_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricBreakdown {
    pub label: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendPoint {
    pub date: NaiveDate,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub generated_at: DateTime<Utc>,
    pub total_tokens: u64,
    pub weekly_active_users: usize,
    pub monthly_cost_usd: f64,
    pub ai_penetration_rate: f64,
    pub top_tools: Vec<MetricBreakdown>,
    pub top_models: Vec<MetricBreakdown>,
    pub trend: Vec<TrendPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLeaderboardRow {
    pub rank: usize,
    pub user_id: String,
    pub display_name: String,
    pub team_name: String,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub requests: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamLeaderboardRow {
    pub rank: usize,
    pub team_id: String,
    pub team_name: String,
    pub member_count: usize,
    pub total_tokens: u64,
    pub avg_tokens_per_member: f64,
    pub total_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolLeaderboardRow {
    pub rank: usize,
    pub tool: ToolKind,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub active_users: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLeaderboardRow {
    pub rank: usize,
    pub model: String,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub active_users: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthLeaderboardRow {
    pub rank: usize,
    pub subject: String,
    pub current_total_tokens: u64,
    pub previous_total_tokens: u64,
    pub growth_tokens: i64,
    pub growth_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardResponse<T> {
    pub generated_at: DateTime<Utc>,
    pub period: String,
    pub filters: BTreeMap<String, String>,
    pub rows: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeOverviewResponse {
    pub generated_at: DateTime<Utc>,
    pub user_id: String,
    pub display_name: String,
    pub team_name: Option<String>,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub request_count: usize,
    pub favorite_tool: Option<String>,
    pub favorite_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionItem {
    pub label: String,
    pub total_tokens: u64,
    pub share: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeDistributionResponse {
    pub generated_at: DateTime<Utc>,
    pub user_id: String,
    pub tools: Vec<DistributionItem>,
    pub models: Vec<DistributionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeTrendResponse {
    pub generated_at: DateTime<Utc>,
    pub user_id: String,
    pub points: Vec<TrendPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardItem {
    pub label: String,
    pub description: String,
    pub granted_at: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardsResponse {
    pub generated_at: DateTime<Utc>,
    pub user_id: String,
    pub rewards: Vec<RewardItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub display_name: String,
    pub email: Option<String>,
    pub team_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamProfile {
    pub team_id: String,
    pub name: String,
    pub parent_team_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMembershipImportRow {
    pub user_id: String,
    pub display_name: String,
    pub team_id: String,
    pub team_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMembershipImportRequest {
    pub rows: Vec<TeamMembershipImportRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminMutationResponse {
    pub updated_users: usize,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub events: Vec<UsageEvent>,
    pub next_cursors: BTreeMap<String, u64>,
    pub discovered_files: usize,
    pub log_dir: PathBuf,
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::{ToolKind, UsageEvent};

    #[test]
    fn usage_event_prefers_raw_hash_for_idempotency() {
        let event = UsageEvent {
            id: uuid::Uuid::new_v4(),
            event_id: "evt_1".into(),
            user_id: "u_1".into(),
            tool: ToolKind::Codex,
            model: "gpt-5.5".into(),
            occurred_at: Utc::now(),
            input_tokens: 10,
            output_tokens: 20,
            cached_tokens: 0,
            estimated_cost_usd: 0.1,
            session_id: None,
            source_file: "sample.jsonl".into(),
            source_offset: 42,
            raw_hash: "hash".into(),
        };

        assert_eq!(event.idempotency_key(), "hash");
    }
}
