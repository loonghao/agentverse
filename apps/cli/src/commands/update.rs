use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;

use crate::client::HubClient;

#[derive(Args)]
pub struct UpdateArgs {
    /// Artifact to update: <kind>/<namespace>/<name>
    pub artifact: String,
    /// Path to updated content JSON (optional)
    #[arg(long)]
    pub content: Option<std::path::PathBuf>,
    /// Path to updated manifest TOML/JSON (optional)
    #[arg(long)]
    pub manifest: Option<std::path::PathBuf>,
    /// Bump override: patch | minor | major (auto-inferred if omitted)
    #[arg(long)]
    pub bump: Option<String>,
    /// Changelog message for this version
    #[arg(long)]
    pub changelog: Option<String>,
    /// New display name (optional)
    #[arg(long)]
    pub display_name: Option<String>,
}

pub async fn run(args: UpdateArgs, client: &HubClient) -> Result<()> {
    let parts: Vec<&str> = args.artifact.splitn(3, '/').collect();
    if parts.len() != 3 {
        anyhow::bail!("artifact must be <kind>/<namespace>/<name>");
    }
    let (kind_str, ns, name) = (parts[0], parts[1], parts[2]);

    // Load content if path provided
    let content: Option<serde_json::Value> = if let Some(ref path) = args.content {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Some(serde_json::from_str(&raw).context("parsing content JSON")?)
    } else {
        None
    };

    // Load manifest if path provided
    let manifest: Option<serde_json::Value> = if let Some(ref path) = args.manifest {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let val: serde_json::Value = if path.extension().map(|e| e == "toml").unwrap_or(false) {
            toml::from_str::<serde_json::Value>(&raw).context("parsing TOML manifest")?
        } else {
            serde_json::from_str(&raw).context("parsing JSON manifest")?
        };
        Some(val)
    } else {
        None
    };

    if content.is_none() && manifest.is_none() && args.display_name.is_none() {
        anyhow::bail!("provide at least one of --content, --manifest, or --display-name");
    }

    let body = serde_json::json!({
        "display_name": args.display_name,
        "manifest": manifest,
        "content": content,
        "bump": args.bump,
        "changelog": args.changelog,
    });

    let path = format!("/api/v1/{kind_str}/{ns}/{name}");
    let resp: serde_json::Value = client.put_json(&path, &body).await?;

    let ver = resp["version"]["version"].as_str().unwrap_or("—");
    println!(
        "\n{} Updated {}/{}/{}  {}\n",
        "✓".green().bold(),
        kind_str,
        ns,
        name,
        format!("v{ver}").green().bold(),
    );
    Ok(())
}
