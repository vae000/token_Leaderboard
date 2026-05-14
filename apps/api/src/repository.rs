use std::collections::{BTreeMap, HashSet};

use common::{
    AdminMutationResponse, BatchIngestResponse, RewardItem, TeamMembershipImportRequest,
    TeamProfile, UsageEvent, UserProfile,
};
use serde::Serialize;

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
    pub fn new() -> Self {
        Self {
            users: BTreeMap::new(),
            teams: BTreeMap::new(),
            events: Vec::new(),
            rewards: BTreeMap::new(),
            devices: BTreeMap::new(),
            known_idempotency_keys: HashSet::new(),
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
}
