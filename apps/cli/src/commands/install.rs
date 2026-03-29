//! `agentverse install` — fetch a skill and record it in the agent's memory.
//!
//! Unlike the raw `get` command (which only displays metadata), `install`:
//!   1. Resolves the skill package from the server.
//!   2. Records the binding in `~/.config/agentverse/memory.json`.
//!   3. Bumps the server-side install counter via the `/install` endpoint.
//!   4. Prints a human-readable confirmation including the skill's memory state.

use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::{client::HubClient, config::CliConfig, memory::MemoryStore};

/// Install a skill and bind it to the current agent.
#[derive(Args)]
pub struct InstallArgs {
    /// Skill reference: `skill/<namespace>/<name>[@version]`
    /// e.g. `skill/openai/code-review` or `skill/openai/code-review@1.2.0`
    pub skill: String,

    /// Override the agent kind this skill is installed for.
    /// Defaults to the value in `~/.config/agentverse/config.toml`.
    #[arg(long, short = 'a')]
    pub agent_kind: Option<String>,

    /// Custom install path (defaults to `~/.config/agentverse/skills/<name>`).
    #[arg(long)]
    pub path: Option<String>,

    /// Do not persist to memory store (dry-run).
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: InstallArgs, client: &HubClient) -> Result<()> {
    // ── Resolve agent kind ────────────────────────────────────────────────────
    let cfg = CliConfig::load()?;
    let agent_kind = args
        .agent_kind
        .or(cfg.agent_kind)
        .unwrap_or_else(|| "custom".to_string());

    // ── Parse skill reference ─────────────────────────────────────────────────
    let (skill_ref_clean, version_hint) = split_version(&args.skill);
    let parts: Vec<&str> = skill_ref_clean.splitn(3, '/').collect();
    if parts.len() != 3 || parts[0] != "skill" {
        anyhow::bail!(
            "skill must be skill/<namespace>/<name>[@version], got: {}",
            args.skill
        );
    }
    let (ns, name) = (parts[1], parts[2]);

    // ── Fetch metadata from server ────────────────────────────────────────────
    let api_path = match version_hint.as_deref() {
        Some(v) => format!("/api/v1/skill/{ns}/{name}/{v}"),
        None => format!("/api/v1/skill/{ns}/{name}"),
    };

    let resp: serde_json::Value = client.get_json(&api_path).await?;
    let artifact = &resp["artifact"];
    let version_obj = &resp["version"];
    let version = version_obj["version"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    // ── Determine install path ────────────────────────────────────────────────
    let install_path = args.path.clone().unwrap_or_else(|| {
        let base = dirs_next::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        base.join("agentverse")
            .join("skills")
            .join(&agent_kind)
            .join(name)
            .to_string_lossy()
            .to_string()
    });

    // ── Record in local memory store ──────────────────────────────────────────
    if !args.dry_run {
        let mut store = MemoryStore::load()?;
        store.record_install(&skill_ref_clean, &version, &agent_kind, &install_path);
        store.save()?;

        // Notify server about the install event (best-effort, non-fatal).
        let install_body = serde_json::json!({
            "agent_kind": agent_kind,
            "version": version,
        });
        let notify_path = format!("/api/v1/skill/{ns}/{name}/install");
        if let Err(e) = client
            .post_json::<_, serde_json::Value>(&notify_path, &install_body)
            .await
        {
            tracing::debug!("server install notification failed (non-fatal): {e}");
        }
    }

    // ── Print result ──────────────────────────────────────────────────────────
    let dry = if args.dry_run { " (dry-run)" } else { "" };
    println!(
        "\n{} Installed {}/{} {} for agent {}{}\n",
        "✅".bold(),
        ns.cyan(),
        name.cyan().bold(),
        format!("v{version}").green(),
        agent_kind.yellow().bold(),
        dry.dimmed(),
    );

    if let Some(desc) = artifact["manifest"]["description"].as_str() {
        println!("  {}\n", desc.dimmed());
    }

    println!(
        "  {} {}\n  {} {}\n",
        "install path:".dimmed(),
        install_path.bold(),
        "memory state:".dimmed(),
        "hot 🔥".bold(),
    );

    Ok(())
}

fn split_version(s: &str) -> (String, Option<String>) {
    if let Some(pos) = s.rfind('@') {
        (s[..pos].to_string(), Some(s[pos + 1..].to_string()))
    } else {
        (s.to_string(), None)
    }
}
