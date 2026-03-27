use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client::HubClient;

#[derive(Args)]
pub struct ListArgs {
    /// kind: skill | soul | agent | workflow | prompt
    pub kind: String,
    #[arg(short, long)]
    pub namespace: Option<String>,
    #[arg(short, long, default_value = "20")]
    pub limit: u64,
    #[arg(short, long, default_value = "0")]
    pub offset: u64,
}

pub async fn run(args: ListArgs, client: &HubClient) -> Result<()> {
    let mut path = format!(
        "/api/v1/{}?limit={}&offset={}",
        args.kind, args.limit, args.offset
    );
    if let Some(ns) = &args.namespace {
        path.push_str(&format!("&namespace={ns}"));
    }

    let resp: serde_json::Value = client.get_json(&path).await?;
    let items = resp["items"].as_array().cloned().unwrap_or_default();

    println!(
        "\n{}\n",
        format!("{} {} artifact(s)", items.len(), args.kind).bold()
    );

    for item in &items {
        let ns = item["namespace"].as_str().unwrap_or("?");
        let name = item["name"].as_str().unwrap_or("?");
        let status = item["status"].as_str().unwrap_or("active");
        let dl = item["downloads"].as_i64().unwrap_or(0);
        let status_colored = match status {
            "active" => status.green(),
            "deprecated" => status.yellow(),
            _ => status.red(),
        };
        println!("  {}/{} {} ↓{}", ns.cyan(), name.bold(), status_colored, dl,);
    }
    println!();
    Ok(())
}
