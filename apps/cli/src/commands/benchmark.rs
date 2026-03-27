use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client::HubClient;

/// Submit benchmark results for an artifact (agent performance evaluation).
#[derive(Args)]
pub struct BenchmarkArgs {
    /// Artifact: <kind>/<namespace>/<name>
    pub artifact: String,
    /// Benchmark metrics as a JSON string, e.g. '{"latency_ms":42,"accuracy":0.95}'
    pub metrics: String,
    /// Confidence score 0.0–1.0 (optional)
    #[arg(long)]
    pub confidence: Option<f64>,
    /// Pin to a specific version (semver string)
    #[arg(long)]
    pub version: Option<String>,
}

pub async fn run(args: BenchmarkArgs, client: &HubClient) -> Result<()> {
    let parts: Vec<&str> = args.artifact.splitn(3, '/').collect();
    if parts.len() != 3 {
        anyhow::bail!("artifact must be <kind>/<namespace>/<name>");
    }
    let (kind_str, ns, name) = (parts[0], parts[1], parts[2]);

    let metrics: serde_json::Value = serde_json::from_str(&args.metrics)
        .map_err(|e| anyhow::anyhow!("invalid metrics JSON: {e}"))?;

    let body = serde_json::json!({
        "metrics": metrics,
        "confidence_score": args.confidence,
        "version": args.version,
    });

    let path = format!("/api/v1/{kind_str}/{ns}/{name}/benchmark");
    let resp: serde_json::Value = client.post_json(&path, &body).await?;
    let id = resp["comment"]["id"].as_str().unwrap_or("?");

    println!(
        "\n{} Benchmark submitted for {} (id: {})\n",
        "📈".bold(),
        args.artifact.bold(),
        id.dimmed(),
    );

    // Pretty-print the metrics
    if let Ok(pretty) = serde_json::to_string_pretty(&metrics) {
        println!("  Metrics:");
        for line in pretty.lines() {
            println!("    {line}");
        }
        println!();
    }
    Ok(())
}

