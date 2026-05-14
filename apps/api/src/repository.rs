use std::collections::{BTreeMap, HashSet};

use chrono::{Duration, Utc};
use common::{
    AdminMutationResponse, BatchIngestResponse, RewardItem, TeamMembershipImportRequest,
    TeamProfile, ToolKind, UsageEvent, UserProfile,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Repository {
    pub users: BTreeMap<String, UserProfile>,
    pub teams: BTreeMap<String, TeamProfile>,
    pub events: Vec<UsageEvent>,
    pub rewards: BTreeMap<String, Vec<RewardItem>>,
    pub devices: BTreeMap<String, String>,
    known_idempotency_keys: HashSet<String>,
}

impl Repository {
    pub fn new(seed_demo_data: bool) -> Self {
        if seed_demo_data {
            Self::seeded()
        } else {
            Self {
                users: BTreeMap::new(),
                teams: BTreeMap::new(),
                events: Vec::new(),
                rewards: BTreeMap::new(),
                devices: BTreeMap::new(),
                known_idempotency_keys: HashSet::new(),
            }
        }
    }

    pub fn register_device(&mut self, device_id: String) {
        self.devices.entry(device_id).or_default();
    }

    pub fn bind_device_user(&mut self, device_id: String, user_id: String) {
        self.devices.insert(device_id, user_id);
    }

    pub fn upsert_user(&mut self, user: UserProfile) {
        self.users.insert(user.user_id.clone(), user);
    }

    pub fn import_memberships(
        &mut self,
        request: TeamMembershipImportRequest,
    ) -> AdminMutationResponse {
        let updated_users = request.rows.len();
        for row in request.rows {
            self.teams
                .entry(row.team_id.clone())
                .or_insert(TeamProfile {
                    team_id: row.team_id.clone(),
                    name: row.team_name.clone(),
                    parent_team_id: None,
                });
            self.users.insert(
                row.user_id.clone(),
                UserProfile {
                    user_id: row.user_id,
                    display_name: row.display_name,
                    email: None,
                    team_id: row.team_id,
                },
            );
        }
        AdminMutationResponse { updated_users }
    }

    pub fn assign_user_team(&mut self, user_id: &str, team_id: &str, team_name: &str) -> bool {
        self.teams.entry(team_id.to_owned()).or_insert(TeamProfile {
            team_id: team_id.to_owned(),
            name: team_name.to_owned(),
            parent_team_id: None,
        });

        if let Some(user) = self.users.get_mut(user_id) {
            user.team_id = team_id.to_owned();
            return true;
        }

        false
    }

    pub fn ingest_events(&mut self, events: Vec<UsageEvent>) -> BatchIngestResponse {
        let mut accepted = 0usize;
        let mut deduped = 0usize;
        let mut rejected = Vec::new();

        for event in events {
            if event.user_id.trim().is_empty() {
                rejected.push(common::RejectedEvent {
                    event_id: event.event_id.clone(),
                    reason: "missing user_id".into(),
                });
                continue;
            }
            if event.model.trim().is_empty() {
                rejected.push(common::RejectedEvent {
                    event_id: event.event_id.clone(),
                    reason: "missing model".into(),
                });
                continue;
            }

            let key = event.idempotency_key();
            if !self.known_idempotency_keys.insert(key) {
                deduped += 1;
                continue;
            }

            self.events.push(event);
            accepted += 1;
        }

        BatchIngestResponse {
            accepted,
            deduped,
            rejected,
            next_sync_after_seconds: 60,
        }
    }

    fn seeded() -> Self {
        let mut users = BTreeMap::new();
        let mut teams = BTreeMap::new();
        let mut rewards = BTreeMap::new();

        teams.insert(
            "t_platform".into(),
            TeamProfile {
                team_id: "t_platform".into(),
                name: "Platform".into(),
                parent_team_id: None,
            },
        );
        teams.insert(
            "t_growth".into(),
            TeamProfile {
                team_id: "t_growth".into(),
                name: "Growth".into(),
                parent_team_id: None,
            },
        );

        for user in [
            UserProfile {
                user_id: "u_alice".into(),
                display_name: "Alice".into(),
                email: Some("alice@example.com".into()),
                team_id: "t_platform".into(),
            },
            UserProfile {
                user_id: "u_bob".into(),
                display_name: "Bob".into(),
                email: Some("bob@example.com".into()),
                team_id: "t_growth".into(),
            },
            UserProfile {
                user_id: "u_cindy".into(),
                display_name: "Cindy".into(),
                email: Some("cindy@example.com".into()),
                team_id: "t_platform".into(),
            },
        ] {
            users.insert(user.user_id.clone(), user);
        }

        rewards.insert(
            "u_alice".into(),
            vec![RewardItem {
                label: "月度 Token TOP 1".into(),
                description: "四月个人总 token 第一".into(),
                granted_at: Utc::now().date_naive(),
            }],
        );
        rewards.insert(
            "u_bob".into(),
            vec![RewardItem {
                label: "增长最快".into(),
                description: "本周相较上周 token 增长最高".into(),
                granted_at: Utc::now().date_naive(),
            }],
        );

        let mut repo = Self {
            users,
            teams,
            events: Vec::new(),
            rewards,
            devices: BTreeMap::new(),
            known_idempotency_keys: HashSet::new(),
        };

        let now = Utc::now();
        let samples = vec![
            sample_event(
                "u_alice",
                ToolKind::Codex,
                "gpt-5.5",
                2600,
                900,
                80,
                now - Duration::days(1),
                "codex-1.jsonl",
                1,
                "sess_a",
            ),
            sample_event(
                "u_alice",
                ToolKind::Cursor,
                "claude-opus-4.1",
                1900,
                1300,
                0,
                now - Duration::days(2),
                "cursor-1.jsonl",
                1,
                "sess_b",
            ),
            sample_event(
                "u_bob",
                ToolKind::Codex,
                "gpt-5.5",
                1100,
                700,
                0,
                now - Duration::days(1),
                "codex-2.jsonl",
                5,
                "sess_c",
            ),
            sample_event(
                "u_bob",
                ToolKind::OpenCode,
                "deepseek-reasoner",
                4800,
                2500,
                0,
                now - Duration::days(6),
                "opencode-1.jsonl",
                2,
                "sess_d",
            ),
            sample_event(
                "u_cindy",
                ToolKind::ClaudeCode,
                "claude-opus-4.1",
                3300,
                1700,
                120,
                now - Duration::days(4),
                "claude-1.jsonl",
                9,
                "sess_e",
            ),
            sample_event(
                "u_cindy",
                ToolKind::Codex,
                "gpt-5.5",
                2400,
                1800,
                50,
                now - Duration::days(10),
                "codex-3.jsonl",
                8,
                "sess_f",
            ),
            sample_event(
                "u_alice",
                ToolKind::DeepseekTui,
                "deepseek-reasoner",
                900,
                400,
                0,
                now - Duration::days(12),
                "deepseek-1.jsonl",
                4,
                "sess_g",
            ),
        ];

        let _ = repo.ingest_events(samples);
        repo
    }
}

fn sample_event(
    user_id: &str,
    tool: ToolKind,
    model: &str,
    input_tokens: u64,
    output_tokens: u64,
    cached_tokens: u64,
    occurred_at: chrono::DateTime<Utc>,
    source_file: &str,
    source_offset: u64,
    session_id: &str,
) -> UsageEvent {
    let estimated_cost_usd = (input_tokens as f64 * 0.00001) + (output_tokens as f64 * 0.00003);

    UsageEvent {
        id: Uuid::new_v4(),
        event_id: format!("evt_{}", Uuid::new_v4()),
        user_id: user_id.into(),
        tool,
        model: model.into(),
        occurred_at,
        input_tokens,
        output_tokens,
        cached_tokens,
        estimated_cost_usd,
        session_id: Some(session_id.into()),
        source_file: source_file.into(),
        source_offset,
        raw_hash: format!("{source_file}:{source_offset}"),
    }
}
