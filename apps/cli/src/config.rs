use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CliState {
    pub api_base_url: Option<String>,
    pub device_id: Option<String>,
    pub user_id: Option<String>,
    pub display_name: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub upload_cursors: BTreeMap<String, u64>,
}

impl CliState {
    pub fn load() -> anyhow::Result<Self> {
        let path = state_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let bytes =
            fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = state_path()?;
        ensure_parent(&path)?;
        fs::write(&path, serde_json::to_vec_pretty(self)?)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn clear() -> anyhow::Result<()> {
        let path = state_path()?;
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        }
        Ok(())
    }
}

pub fn state_path() -> anyhow::Result<PathBuf> {
    if let Ok(dir) = std::env::var("LEADERBOARD_CONFIG_DIR") {
        return Ok(PathBuf::from(dir).join("config.json"));
    }

    let project_dirs = ProjectDirs::from("com", "token-leaderboard", "leaderboard")
        .context("failed to derive config directory")?;
    Ok(project_dirs.config_dir().join("config.json"))
}

fn ensure_parent(path: &Path) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .context("config path has no parent directory")?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    Ok(())
}
