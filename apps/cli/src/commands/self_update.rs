use anyhow::Result;
use clap::Args;
use colored::Colorize;

/// CLI arguments for the `self-update` subcommand.
#[derive(Args)]
pub struct SelfUpdateArgs {
    /// GitHub personal access token to avoid API rate limits
    #[arg(long, env = "GITHUB_TOKEN")]
    token: Option<String>,

    /// Only check for a newer version without installing it
    #[arg(long)]
    check: bool,
}

pub async fn run(args: SelfUpdateArgs) -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    println!(
        "{} Checking for updates (current: v{})...",
        "agentverse".cyan().bold(),
        current
    );

    match agentverse_updater::check_for_update(current, "agentverse", args.token.as_deref()).await?
    {
        None => {
            println!("{} Already up to date (v{})", "✓".green().bold(), current);
        }
        Some(info) => {
            println!(
                "{} New version available: v{} (current: v{})",
                "→".cyan().bold(),
                info.version,
                current
            );

            if args.check {
                println!("Run `agentverse self-update` to install the update.");
                return Ok(());
            }

            println!("{} Downloading {}...", "↓".cyan(), info.asset_name);
            agentverse_updater::apply_update(&info, args.token.as_deref()).await?;

            println!("{} Updated to v{}", "✓".green().bold(), info.version);
            println!("  Run `agentverse --version` to confirm.");
        }
    }

    Ok(())
}
