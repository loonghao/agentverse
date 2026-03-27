use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client::HubClient;

#[derive(Args)]
pub struct ForkArgs {
    /// Source artifact: kind/namespace/name[@version]
    pub source: String,
    /// New name for the fork
    pub new_name: String,
    /// New namespace (defaults to source namespace)
    #[arg(long)]
    pub new_namespace: Option<String>,
}

pub async fn run(args: ForkArgs, client: &HubClient) -> Result<()> {
    // Parse source
    let (base, ver) = if let Some(pos) = args.source.find('@') {
        (&args.source[..pos], Some(&args.source[pos + 1..]))
    } else {
        (args.source.as_str(), None)
    };
    let parts: Vec<&str> = base.splitn(3, '/').collect();
    if parts.len() != 3 {
        anyhow::bail!("source must be kind/namespace/name[@version], got: {}", args.source);
    }

    let path = format!("/api/v1/{}/{}/{}/fork", parts[0], parts[1], parts[2]);
    let body = serde_json::json!({
        "new_name": args.new_name,
        "new_namespace": args.new_namespace,
        "source_version": ver,
    });

    let resp: serde_json::Value = client.post_json(&path, &body).await?;
    println!(
        "\n{} Forked {}/{}/{} → {}\n",
        "✓".green().bold(),
        parts[0], parts[1], parts[2],
        args.new_name.green().bold(),
    );
    println!("  {}", serde_json::to_string_pretty(&resp).unwrap_or_default().dimmed());
    Ok(())
}

