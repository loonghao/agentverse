//! `agentverse memory` — manage the agent's skill memory lifecycle.
//!
//! Subcommands:
//!   status   — show all bound skills and their current memory states
//!   archive  — back up cold skills and remove them from the active path
//!   restore  — bring an archived skill back to hot
//!   stats    — show usage leaderboard
//!   gc       — purge forgotten skills (backup deleted)
//!   use      — manually record a usage event for a skill

use anyhow::Result;
use clap::{Args, Subcommand};
use colored::Colorize;

use crate::{config::CliConfig, memory::MemoryStore};
use agentverse_core::memory::MemoryState;

#[derive(Args)]
pub struct MemoryArgs {
    #[command(subcommand)]
    pub command: MemoryCommand,
}

#[derive(Subcommand)]
pub enum MemoryCommand {
    /// Show all installed skills and their memory states.
    Status,
    /// Archive cold (≥30 days unused) skills to free active space.
    Archive,
    /// Restore an archived skill back to hot memory.
    Restore(RestoreArgs),
    /// Show a usage leaderboard of installed skills.
    Stats,
    /// Purge forgotten skills (removes backup files, frees disk).
    Gc,
    /// Manually record a usage event for a skill.
    Use(UseArgs),
}

#[derive(Args)]
pub struct RestoreArgs {
    /// Skill reference: `skill/<namespace>/<name>`
    pub skill_ref: String,
    /// Agent kind (defaults to config value).
    #[arg(long, short = 'a')]
    pub agent_kind: Option<String>,
}

#[derive(Args)]
pub struct UseArgs {
    /// Skill reference: `skill/<namespace>/<name>`
    pub skill_ref: String,
    /// Agent kind (defaults to config value).
    #[arg(long, short = 'a')]
    pub agent_kind: Option<String>,
}

pub async fn run(args: MemoryArgs) -> Result<()> {
    match args.command {
        MemoryCommand::Status => run_status(),
        MemoryCommand::Archive => run_archive(),
        MemoryCommand::Restore(a) => run_restore(a),
        MemoryCommand::Stats => run_stats(),
        MemoryCommand::Gc => run_gc(),
        MemoryCommand::Use(a) => run_use(a),
    }
}

fn resolve_agent_kind(override_kind: Option<String>) -> String {
    override_kind
        .or_else(|| CliConfig::load().ok().and_then(|c| c.agent_kind))
        .unwrap_or_else(|| "custom".to_string())
}

// ── status ────────────────────────────────────────────────────────────────────

fn run_status() -> Result<()> {
    let mut store = MemoryStore::load()?;
    let changed = store.refresh_all_states();
    if changed > 0 {
        store.save()?;
    }

    if store.bindings.is_empty() {
        println!(
            "\n{} No skills installed yet. Run `agentverse install skill/<ns>/<name>`.\n",
            "🧠".bold()
        );
        return Ok(());
    }

    println!("\n{}\n", "🧠 Agent Skill Memory".bold().underline());
    println!(
        "{:<40} {:<12} {:<10} {:<14} {}",
        "Skill".bold(),
        "Agent".bold(),
        "Version".bold(),
        "Memory".bold(),
        "Uses".bold()
    );
    println!("{}", "─".repeat(85).dimmed());

    let mut bindings: Vec<_> = store.bindings.values().collect();
    bindings.sort_by(|a, b| a.skill_ref.cmp(&b.skill_ref));

    for b in bindings {
        let state_str = match &b.memory_state {
            MemoryState::Hot => "hot 🔥".green().bold().to_string(),
            MemoryState::Warm => "warm 🌤".yellow().to_string(),
            MemoryState::Cold => "cold 🧊".cyan().to_string(),
            MemoryState::Archived => "archived 📦".dimmed().to_string(),
            MemoryState::Forgotten => "forgotten 🌫".red().dimmed().to_string(),
        };
        let last_used = b
            .last_used_at
            .map(|t| t.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "never".to_string());
        println!(
            "{:<40} {:<12} {:<10} {:<23} {} (last: {})",
            b.skill_ref.cyan(),
            b.agent_kind.yellow(),
            b.version,
            state_str,
            b.use_count,
            last_used.dimmed(),
        );
    }
    println!();
    Ok(())
}

// ── archive ───────────────────────────────────────────────────────────────────

fn run_archive() -> Result<()> {
    let mut store = MemoryStore::load()?;
    store.refresh_all_states();

    let cold: Vec<_> = store
        .cold_bindings()
        .iter()
        .map(|b| {
            (
                b.skill_ref.clone(),
                b.agent_kind.clone(),
                b.install_path.clone(),
            )
        })
        .collect();

    if cold.is_empty() {
        println!("\n{} No cold skills to archive.\n", "✅".bold());
        return Ok(());
    }

    let backup_base = dirs_next::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("agentverse")
        .join("archives");
    std::fs::create_dir_all(&backup_base)?;

    let mut archived = 0usize;
    for (skill_ref, agent_kind, _install_path) in &cold {
        let safe_name = skill_ref.replace('/', "_");
        let backup_path = backup_base.join(format!("{safe_name}@{agent_kind}"));
        let backup_str = backup_path.to_string_lossy().to_string();
        store.mark_archived(skill_ref, agent_kind, &backup_str);
        println!(
            "  {} {} → {}",
            "📦".bold(),
            skill_ref.cyan(),
            backup_str.dimmed()
        );
        archived += 1;
    }

    store.save()?;
    println!("\n{} Archived {} skill(s).\n", "✅".bold(), archived);
    Ok(())
}

// ── restore ───────────────────────────────────────────────────────────────────

fn run_restore(args: RestoreArgs) -> Result<()> {
    let agent_kind = resolve_agent_kind(args.agent_kind);
    let mut store = MemoryStore::load()?;
    if store.mark_restored(&args.skill_ref, &agent_kind) {
        store.save()?;
        println!(
            "\n{} Restored {} (agent: {}) → hot 🔥\n",
            "✅".bold(),
            args.skill_ref.cyan().bold(),
            agent_kind.yellow()
        );
    } else {
        println!(
            "\n{} Skill not found: {} (agent: {})\n",
            "❌".bold(),
            args.skill_ref,
            agent_kind
        );
    }
    Ok(())
}

// ── stats ─────────────────────────────────────────────────────────────────────

fn run_stats() -> Result<()> {
    let store = MemoryStore::load()?;
    let sorted = store.sorted_by_usage();

    if sorted.is_empty() {
        println!("\n{} No usage data yet.\n", "📊".bold());
        return Ok(());
    }

    println!("\n{}\n", "📊 Skill Usage Leaderboard".bold().underline());
    println!(
        "{:<3} {:<40} {:<12} {}",
        "#".bold(),
        "Skill".bold(),
        "Agent".bold(),
        "Uses".bold()
    );
    println!("{}", "─".repeat(65).dimmed());
    for (i, b) in sorted.iter().enumerate().take(20) {
        println!(
            "{:<3} {:<40} {:<12} {}",
            i + 1,
            b.skill_ref.cyan(),
            b.agent_kind.yellow(),
            b.use_count
        );
    }
    println!();
    Ok(())
}

// ── gc ────────────────────────────────────────────────────────────────────────

fn run_gc() -> Result<()> {
    let mut store = MemoryStore::load()?;
    store.refresh_all_states();

    let forgotten: Vec<_> = store
        .bindings
        .values()
        .filter(|b| matches!(b.memory_state, MemoryState::Forgotten))
        .map(|b| (b.skill_ref.clone(), b.agent_kind.clone()))
        .collect();

    if forgotten.is_empty() {
        println!("\n{} Nothing to garbage-collect.\n", "✅".bold());
        return Ok(());
    }

    let mut purged = 0usize;
    for (skill_ref, agent_kind) in &forgotten {
        store.purge(skill_ref, agent_kind);
        println!(
            "  {} {} ({})",
            "🗑".bold(),
            skill_ref.dimmed(),
            agent_kind.dimmed()
        );
        purged += 1;
    }
    store.save()?;
    println!("\n{} Purged {} forgotten skill(s).\n", "✅".bold(), purged);
    Ok(())
}

// ── use ───────────────────────────────────────────────────────────────────────

fn run_use(args: UseArgs) -> Result<()> {
    let agent_kind = resolve_agent_kind(args.agent_kind);
    let mut store = MemoryStore::load()?;
    if store.record_use(&args.skill_ref, &agent_kind) {
        let count = store
            .bindings
            .values()
            .find(|b| b.skill_ref == args.skill_ref && b.agent_kind == agent_kind)
            .map(|b| b.use_count)
            .unwrap_or(0);
        store.save()?;
        println!(
            "\n{} Recorded use of {} (total: {})\n",
            "🧠".bold(),
            args.skill_ref.cyan().bold(),
            count
        );
    } else {
        println!(
            "\n{} Skill not installed: {}\n",
            "❌".bold(),
            args.skill_ref
        );
    }
    Ok(())
}
