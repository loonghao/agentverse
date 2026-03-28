//! Agent-specific skill installation path resolution.
//!
//! Each agent runtime has a conventional directory where skills are installed.
//! These conventions are derived from each agent's documentation and from the
//! `skills/` layouts observed in the field.
//!
//! Layout inside every install directory:
//!   `{agent_root}/{namespace}/{name}/SKILL.md`   ← metadata
//!   `{agent_root}/{namespace}/{name}/...`         ← payload files

use std::path::PathBuf;

use agentverse_core::skill::AgentKind;

/// Return the root directory where an agent looks for installed skills.
///
/// On Windows the `APPDATA` env var is used in lieu of `~`.
pub fn agent_skills_root(agent: &AgentKind) -> PathBuf {
    let home = home_dir();
    match agent {
        AgentKind::OpenClaw => home.join(".openclaw").join("skills"),
        AgentKind::CodeBuddy => home.join(".codebuddy").join("skills"),
        AgentKind::WorkerBuddy => home.join(".workerbuddy").join("skills"),
        AgentKind::Claude => home.join(".claude").join("skills"),
        AgentKind::Augment => home.join(".augment").join("skills"),
        AgentKind::Custom(name) => home.join(format!(".{name}")).join("skills"),
    }
}

/// Return the full installation path for a specific skill under a given agent.
///
/// Convention: `{agent_skills_root}/{namespace}/{name}/`
pub fn skill_install_path(agent: &AgentKind, namespace: &str, name: &str) -> PathBuf {
    agent_skills_root(agent).join(namespace).join(name)
}

/// All well-known agent runtimes that will receive a skill on `install --all`.
pub fn all_known_agents() -> Vec<AgentKind> {
    vec![
        AgentKind::OpenClaw,
        AgentKind::CodeBuddy,
        AgentKind::WorkerBuddy,
        AgentKind::Claude,
        AgentKind::Augment,
    ]
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn home_dir() -> PathBuf {
    // Prefer explicit HOME/USERPROFILE, then fall back to the dirs crate approach
    if let Ok(h) = std::env::var("HOME") {
        return PathBuf::from(h);
    }
    if let Ok(h) = std::env::var("USERPROFILE") {
        return PathBuf::from(h);
    }
    // Last-resort: current directory (safe fallback in CI / containers)
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openclaw_skills_root_ends_with_openclaw_skills() {
        let root = agent_skills_root(&AgentKind::OpenClaw);
        let s = root.to_string_lossy();
        assert!(s.contains("openclaw") && s.ends_with("skills"));
    }

    #[test]
    fn skill_install_path_contains_namespace_and_name() {
        let path = skill_install_path(&AgentKind::Augment, "myorg", "my-skill");
        let s = path.to_string_lossy();
        assert!(s.contains("myorg"));
        assert!(s.ends_with("my-skill"));
    }

    #[test]
    fn custom_agent_uses_agent_name_in_path() {
        let path = agent_skills_root(&AgentKind::Custom("mymcp".into()));
        assert!(path.to_string_lossy().contains("mymcp"));
    }

    #[test]
    fn all_known_agents_is_non_empty() {
        assert!(!all_known_agents().is_empty());
    }
}

