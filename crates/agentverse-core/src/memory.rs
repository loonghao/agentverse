//! Agent memory model — tracks which skills an agent has installed, how often
//! they are used, and their current "memory state" (hot → warm → cold → archived).
//!
//! # Human-Memory Analogy
//!
//! Just as humans naturally remember frequently-used knowledge and forget
//! rarely-touched facts, agent skills transition through memory states based on
//! recency and frequency of use:
//!
//! ```text
//! install ──► hot ──► warm ──► cold ──► archived ──► forgotten
//!               (7d)     (30d)    (60d)     (90d)
//! ```
//!
//! - **hot**: used within the last 7 days
//! - **warm**: used within the last 30 days
//! - **cold**: not used for 30–60 days — candidate for archiving
//! - **archived**: not used for 60+ days — files backed up, removed from active path
//! - **forgotten**: archived for 90+ days — backup deleted, fully removed

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Lifecycle state of a skill in an agent's memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryState {
    /// Used within the last 7 days — always available.
    #[default]
    Hot,
    /// Used within the last 30 days — available but less prioritised.
    Warm,
    /// Not used for 30–60 days — candidate for archiving.
    Cold,
    /// Not used for 60+ days — files backed up and removed from active path.
    Archived,
    /// Archived for 90+ days — backup deleted, fully forgotten.
    Forgotten,
}

impl MemoryState {
    /// Compute the memory state from recency (days since last use) and
    /// total use count.  Use count acts as a long-term weight so high-value
    /// skills degrade more slowly.
    pub fn from_usage(days_since_last_use: u64, use_count: u64) -> Self {
        // High-use skills get a bonus of up to 14 extra "warm" days.
        let bonus_days = (use_count / 5).min(14);
        let effective_days = days_since_last_use.saturating_sub(bonus_days);

        match effective_days {
            0..=6 => MemoryState::Hot,
            7..=29 => MemoryState::Warm,
            30..=59 => MemoryState::Cold,
            60..=149 => MemoryState::Archived,
            _ => MemoryState::Forgotten,
        }
    }

    /// Whether the skill's files should be backed up and removed from the
    /// active install path.
    pub fn should_archive(&self) -> bool {
        matches!(self, MemoryState::Archived | MemoryState::Forgotten)
    }

    /// Whether the backup itself should be purged (skill fully forgotten).
    pub fn should_purge(&self) -> bool {
        matches!(self, MemoryState::Forgotten)
    }
}

impl std::fmt::Display for MemoryState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            MemoryState::Hot => "hot 🔥",
            MemoryState::Warm => "warm 🌤",
            MemoryState::Cold => "cold 🧊",
            MemoryState::Archived => "archived 📦",
            MemoryState::Forgotten => "forgotten 🌫",
        };
        write!(f, "{s}")
    }
}

/// A record binding a specific skill version to an agent, including full
/// lifecycle metadata.  This is the central entity of the memory system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillBinding {
    /// Logical key: `<kind>/<namespace>/<name>`, e.g. `skill/openai/code-review`.
    pub skill_ref: String,
    /// Installed semver string, e.g. `"1.2.0"`.
    pub version: String,
    /// Which agent runtime owns this binding.
    pub agent_kind: String,
    /// Absolute path where the skill is (or was) installed.
    pub install_path: String,
    /// Where the archived backup lives (if `memory_state == Archived`).
    pub backup_path: Option<String>,
    pub installed_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    /// Total invocation / touch count recorded by the CLI.
    pub use_count: u64,
    pub memory_state: MemoryState,
}

impl AgentSkillBinding {
    /// Create a new binding with `Hot` state and zero usage.
    pub fn new(
        skill_ref: impl Into<String>,
        version: impl Into<String>,
        agent_kind: impl Into<String>,
        install_path: impl Into<String>,
    ) -> Self {
        Self {
            skill_ref: skill_ref.into(),
            version: version.into(),
            agent_kind: agent_kind.into(),
            install_path: install_path.into(),
            backup_path: None,
            installed_at: Utc::now(),
            last_used_at: None,
            use_count: 0,
            memory_state: MemoryState::Hot,
        }
    }

    /// Recompute `memory_state` based on current time and usage.
    pub fn refresh_state(&mut self) {
        let days = self
            .last_used_at
            .map(|t| (Utc::now() - t).num_days().max(0) as u64)
            .unwrap_or(0);
        self.memory_state = MemoryState::from_usage(days, self.use_count);
    }

    /// Record a usage event (increments counter and resets recency clock).
    pub fn record_use(&mut self) {
        self.use_count += 1;
        self.last_used_at = Some(Utc::now());
        self.refresh_state();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── MemoryState::from_usage ───────────────────────────────────────────────

    #[test]
    fn brand_new_skill_is_hot() {
        assert_eq!(MemoryState::from_usage(0, 0), MemoryState::Hot);
    }

    #[test]
    fn used_6_days_ago_is_hot() {
        assert_eq!(MemoryState::from_usage(6, 0), MemoryState::Hot);
    }

    #[test]
    fn used_7_days_ago_no_bonus_is_warm() {
        assert_eq!(MemoryState::from_usage(7, 0), MemoryState::Warm);
    }

    #[test]
    fn high_use_count_extends_hot_window() {
        // 50 uses → bonus = min(50/5, 14) = 10 extra days
        // effective_days = 14 - 10 = 4 → Hot
        assert_eq!(MemoryState::from_usage(14, 50), MemoryState::Hot);
    }

    #[test]
    fn used_30_days_ago_no_bonus_is_cold() {
        assert_eq!(MemoryState::from_usage(30, 0), MemoryState::Cold);
    }

    #[test]
    fn used_60_days_ago_is_archived() {
        assert_eq!(MemoryState::from_usage(60, 0), MemoryState::Archived);
    }

    #[test]
    fn used_150_days_ago_is_forgotten() {
        assert_eq!(MemoryState::from_usage(150, 0), MemoryState::Forgotten);
    }

    // ── should_archive / should_purge ─────────────────────────────────────────

    #[test]
    fn archived_state_should_archive() {
        assert!(MemoryState::Archived.should_archive());
        assert!(MemoryState::Forgotten.should_archive());
    }

    #[test]
    fn hot_and_warm_should_not_archive() {
        assert!(!MemoryState::Hot.should_archive());
        assert!(!MemoryState::Warm.should_archive());
        assert!(!MemoryState::Cold.should_archive());
    }

    #[test]
    fn only_forgotten_should_purge() {
        assert!(MemoryState::Forgotten.should_purge());
        assert!(!MemoryState::Archived.should_purge());
    }

    // ── AgentSkillBinding ─────────────────────────────────────────────────────

    #[test]
    fn new_binding_is_hot_with_zero_uses() {
        let b = AgentSkillBinding::new("skill/openai/review", "1.0.0", "augment", "/tmp/review");
        assert_eq!(b.memory_state, MemoryState::Hot);
        assert_eq!(b.use_count, 0);
        assert!(b.last_used_at.is_none());
    }

    #[test]
    fn record_use_increments_count_and_sets_last_used() {
        let mut b = AgentSkillBinding::new("skill/openai/review", "1.0.0", "augment", "/tmp");
        b.record_use();
        assert_eq!(b.use_count, 1);
        assert!(b.last_used_at.is_some());
    }

    #[test]
    fn binding_serde_round_trip() {
        let b = AgentSkillBinding::new("skill/ns/name", "2.0.0", "claude", "/opt/skills/name");
        let json = serde_json::to_string(&b).unwrap();
        let back: AgentSkillBinding = serde_json::from_str(&json).unwrap();
        assert_eq!(back.skill_ref, b.skill_ref);
        assert_eq!(back.version, b.version);
        assert_eq!(back.agent_kind, b.agent_kind);
        assert_eq!(back.memory_state, MemoryState::Hot);
    }

    // ── Display ───────────────────────────────────────────────────────────────

    #[test]
    fn memory_state_display_contains_emoji() {
        assert!(MemoryState::Hot.to_string().contains("🔥"));
        assert!(MemoryState::Warm.to_string().contains("🌤"));
        assert!(MemoryState::Cold.to_string().contains("🧊"));
        assert!(MemoryState::Archived.to_string().contains("📦"));
        assert!(MemoryState::Forgotten.to_string().contains("🌫"));
    }
}
