//! Local agent memory store.
//!
//! Persists a JSON file at `~/.config/agentverse/memory.json` that tracks
//! every skill binding for the current agent, including installation records,
//! usage counts, and memory states.
//!
//! The store is intentionally lightweight (no SQLite dependency) so it works
//! offline and requires no migration logic beyond schema versioning in JSON.

use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use agentverse_core::memory::{AgentSkillBinding, MemoryState};

/// Top-level structure persisted to disk.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MemoryStore {
    /// Schema version for future migrations.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,

    /// Keyed by `<skill_ref>@<agent_kind>`, e.g. `skill/openai/code-review@augment`.
    pub bindings: HashMap<String, AgentSkillBinding>,
}

fn default_schema_version() -> u32 {
    1
}

impl MemoryStore {
    pub fn path() -> PathBuf {
        let base = dirs_next::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("agentverse").join("memory.json")
    }

    /// Load from disk, returning an empty store if the file does not exist.
    pub fn load() -> Result<Self> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    /// Persist to disk (creates parent directories as needed).
    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, raw)?;
        Ok(())
    }

    fn key(skill_ref: &str, agent_kind: &str) -> String {
        format!("{skill_ref}@{agent_kind}")
    }

    /// Record a new installation. If an existing binding exists it is updated
    /// in-place (re-install / upgrade), preserving historical usage data.
    pub fn record_install(
        &mut self,
        skill_ref: &str,
        version: &str,
        agent_kind: &str,
        install_path: &str,
    ) {
        let key = Self::key(skill_ref, agent_kind);
        let binding = self.bindings.entry(key).or_insert_with(|| {
            AgentSkillBinding::new(skill_ref, version, agent_kind, install_path)
        });
        // On reinstall / upgrade, update mutable fields.
        binding.version = version.to_string();
        binding.install_path = install_path.to_string();
        binding.installed_at = Utc::now();
        binding.memory_state = MemoryState::Hot;
    }

    /// Record that a skill was actively used by the agent.
    pub fn record_use(&mut self, skill_ref: &str, agent_kind: &str) -> bool {
        let key = Self::key(skill_ref, agent_kind);
        if let Some(b) = self.bindings.get_mut(&key) {
            b.record_use();
            true
        } else {
            false
        }
    }

    /// Recompute memory states for all bindings and return how many changed.
    pub fn refresh_all_states(&mut self) -> usize {
        let mut changed = 0;
        for b in self.bindings.values_mut() {
            let before = b.memory_state.clone();
            b.refresh_state();
            if b.memory_state != before {
                changed += 1;
            }
        }
        changed
    }

    /// Return bindings whose state is Cold or worse (candidates for archiving).
    pub fn cold_bindings(&self) -> Vec<&AgentSkillBinding> {
        self.bindings
            .values()
            .filter(|b| {
                matches!(
                    b.memory_state,
                    MemoryState::Cold | MemoryState::Archived | MemoryState::Forgotten
                )
            })
            .collect()
    }

    /// Mark a binding as Archived and record its backup path.
    pub fn mark_archived(&mut self, skill_ref: &str, agent_kind: &str, backup_path: &str) -> bool {
        let key = Self::key(skill_ref, agent_kind);
        if let Some(b) = self.bindings.get_mut(&key) {
            b.memory_state = MemoryState::Archived;
            b.backup_path = Some(backup_path.to_string());
            true
        } else {
            false
        }
    }

    /// Restore a binding to Hot (e.g. after `memory restore`).
    pub fn mark_restored(&mut self, skill_ref: &str, agent_kind: &str) -> bool {
        let key = Self::key(skill_ref, agent_kind);
        if let Some(b) = self.bindings.get_mut(&key) {
            b.memory_state = MemoryState::Hot;
            b.backup_path = None;
            true
        } else {
            false
        }
    }

    /// Remove a forgotten binding entirely (after purging its backup).
    pub fn purge(&mut self, skill_ref: &str, agent_kind: &str) -> bool {
        let key = Self::key(skill_ref, agent_kind);
        self.bindings.remove(&key).is_some()
    }

    /// All bindings sorted by use_count descending (most-used first).
    pub fn sorted_by_usage(&self) -> Vec<&AgentSkillBinding> {
        let mut v: Vec<&AgentSkillBinding> = self.bindings.values().collect();
        v.sort_by_key(|b| std::cmp::Reverse(b.use_count));
        v
    }
}
