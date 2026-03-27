use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client::HubClient;

#[derive(Args)]
pub struct DeprecateArgs {
    /// Artifact in the form <kind>/<namespace>/<name>, e.g. skill/my-org/my-skill
    pub artifact: String,
    /// Skip confirmation prompt
    #[arg(long, short = 'y')]
    pub yes: bool,
}

pub async fn run(args: DeprecateArgs, client: &HubClient) -> Result<()> {
    let parts: Vec<&str> = args.artifact.splitn(3, '/').collect();
    if parts.len() != 3 {
        anyhow::bail!("artifact must be <kind>/<namespace>/<name>");
    }
    let (kind_str, ns, name) = (parts[0], parts[1], parts[2]);

    if !args.yes {
        eprint!(
            "Deprecate {}/{}  {}  (y/N)? ",
            ns.bold(),
            name.bold(),
            "[irreversible via CLI]".dimmed(),
        );
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !matches!(line.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let path = format!("/api/v1/{kind_str}/{ns}/{name}/deprecate");
    let resp: serde_json::Value = client.post_json(&path, &serde_json::json!({})).await?;

    println!(
        "\n{} {} deprecated\n",
        "⚠".yellow().bold(),
        resp["artifact"]["namespace"].as_str().unwrap_or(ns),
    );
    Ok(())
}
