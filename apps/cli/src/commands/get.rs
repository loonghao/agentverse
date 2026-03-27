use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client::HubClient;

#[derive(Args)]
pub struct GetArgs {
    /// kind/namespace/name[@version]  e.g. skill/openai/code-review@1.2.0
    pub artifact: String,
}

pub async fn run(args: GetArgs, client: &HubClient) -> Result<()> {
    let path = build_path(&args.artifact)?;
    let resp: serde_json::Value = client.get_json(&path).await?;

    let artifact = &resp["artifact"];
    let version = &resp["version"];

    println!(
        "\n{} {}/{} {}",
        format!("[{}]", artifact["kind"].as_str().unwrap_or("?")).cyan().bold(),
        artifact["namespace"].as_str().unwrap_or("?"),
        artifact["name"].as_str().unwrap_or("?"),
        format!("v{}", version["version"].as_str().unwrap_or("?")).green().bold(),
    );

    if let Some(desc) = artifact["manifest"]["description"].as_str() {
        println!("\n  {}\n", desc);
    }

    if let Some(tags) = artifact["manifest"]["tags"].as_array() {
        let tags: Vec<_> = tags.iter().filter_map(|t| t.as_str()).collect();
        if !tags.is_empty() {
            println!("  {} {}", "tags:".dimmed(), tags.join(", ").cyan());
        }
    }

    println!(
        "  {} {} | {} {} | {} {}",
        "checksum:".dimmed(), version["checksum"].as_str().unwrap_or("?"),
        "downloads:".dimmed(), artifact["downloads"].as_i64().unwrap_or(0),
        "status:".dimmed(), artifact["status"].as_str().unwrap_or("?"),
    );

    if let Some(changelog) = version["changelog"].as_str() {
        println!("\n  {}\n  {}", "changelog:".dimmed(), changelog);
    }

    println!();
    Ok(())
}

fn build_path(artifact: &str) -> Result<String> {
    // Parse "kind/namespace/name" or "kind/namespace/name@version"
    let (base, ver) = if let Some(pos) = artifact.find('@') {
        (&artifact[..pos], Some(&artifact[pos + 1..]))
    } else {
        (artifact.as_ref(), None)
    };

    let parts: Vec<&str> = base.splitn(3, '/').collect();
    if parts.len() != 3 {
        anyhow::bail!("artifact must be kind/namespace/name[@version], got: {artifact}");
    }

    let path = format!("/api/v1/{}/{}/{}", parts[0], parts[1], parts[2]);
    Ok(if let Some(v) = ver {
        format!("{path}/{v}")
    } else {
        path
    })
}

