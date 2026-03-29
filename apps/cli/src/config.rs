use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persistent CLI configuration (saved to ~/.config/agentverse/config.toml).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CliConfig {
    pub server: Option<String>,
    pub token: Option<String>,
    pub username: Option<String>,
    /// Which agent runtime this CLI is operating as (e.g. "augment", "claude").
    /// Used as the default `agent_kind` for install / memory commands.
    pub agent_kind: Option<String>,
}

impl CliConfig {
    pub fn path() -> PathBuf {
        let base = dirs_next::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("agentverse").join("config.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path)?;
        Ok(toml::from_str(&raw)?)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self)?;
        std::fs::write(&path, raw)?;
        Ok(())
    }
}
