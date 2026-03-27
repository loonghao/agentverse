use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client::HubClient;

#[derive(Args)]
pub struct VersionsArgs {
    /// kind/namespace/name  e.g. skill/openai/code-review
    pub artifact: String,
}

pub async fn run(args: VersionsArgs, client: &HubClient) -> Result<()> {
    let parts: Vec<&str> = args.artifact.splitn(3, '/').collect();
    if parts.len() != 3 {
        anyhow::bail!("artifact must be kind/namespace/name, got: {}", args.artifact);
    }
    let path = format!("/api/v1/{}/{}/{}/versions", parts[0], parts[1], parts[2]);
    let resp: serde_json::Value = client.get_json(&path).await?;
    let versions = resp["versions"].as_array().cloned().unwrap_or_default();

    println!(
        "\n{}\n",
        format!("Versions for {} ({} total)", args.artifact, versions.len()).bold()
    );

    for v in &versions {
        let ver = v["version"].as_str().unwrap_or("?");
        let bump = v["bump_reason"].as_str().unwrap_or("?");
        let cl = v["changelog"].as_str().unwrap_or("");
        let pub_at = v["published_at"].as_str().unwrap_or("?");
        let bump_colored = match bump {
            "major" => bump.red().bold(),
            "minor" => bump.yellow().bold(),
            _ => bump.dimmed(),
        };
        println!(
            "  {} ({})  {}  {}",
            format!("v{ver}").green().bold(),
            bump_colored,
            pub_at.dimmed(),
            cl,
        );
    }
    println!();
    Ok(())
}

