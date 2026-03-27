use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client::HubClient;

/// Submit a learning insight about an artifact (agent-to-agent knowledge sharing).
#[derive(Args)]
pub struct LearnArgs {
    /// Artifact: <kind>/<namespace>/<name>
    pub artifact: String,
    /// Learning insight text
    pub content: String,
    /// Confidence score 0.0–1.0 (optional)
    #[arg(long)]
    pub confidence: Option<f64>,
    /// Pin to a specific version (semver string)
    #[arg(long)]
    pub version: Option<String>,
    /// Extra payload as JSON string (optional)
    #[arg(long)]
    pub payload: Option<String>,
}

pub async fn run(args: LearnArgs, client: &HubClient) -> Result<()> {
    let parts: Vec<&str> = args.artifact.splitn(3, '/').collect();
    if parts.len() != 3 {
        anyhow::bail!("artifact must be <kind>/<namespace>/<name>");
    }
    let (kind_str, ns, name) = (parts[0], parts[1], parts[2]);

    let payload: Option<serde_json::Value> = args.payload.as_deref()
        .map(|s| serde_json::from_str(s))
        .transpose()
        .map_err(|e| anyhow::anyhow!("invalid --payload JSON: {e}"))?;

    let body = serde_json::json!({
        "content": args.content,
        "confidence_score": args.confidence,
        "payload": payload,
        "version": args.version,
    });

    let path = format!("/api/v1/{kind_str}/{ns}/{name}/learn");
    let resp: serde_json::Value = client.post_json(&path, &body).await?;
    let id = resp["comment"]["id"].as_str().unwrap_or("?");

    println!(
        "\n{} Learning insight submitted for {} (id: {})\n",
        "🧠".bold(),
        args.artifact.bold(),
        id.dimmed(),
    );
    Ok(())
}

