use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client::HubClient;

#[derive(Args)]
pub struct SearchArgs {
    /// Search query
    pub query: String,
    /// Filter by kind: skill | soul | agent | workflow | prompt
    #[arg(short, long)]
    pub kind: Option<String>,
    /// Filter by tag
    #[arg(short, long)]
    pub tag: Option<String>,
    /// Max results
    #[arg(short, long, default_value = "10")]
    pub limit: u64,
}

pub async fn run(args: SearchArgs, client: &HubClient) -> Result<()> {
    let mut path = format!(
        "/api/v1/search?q={}&limit={}",
        urlenccode(&args.query),
        args.limit
    );
    if let Some(k) = &args.kind {
        path.push_str(&format!("&kind={}", k));
    }
    if let Some(t) = &args.tag {
        path.push_str(&format!("&tag={}", t));
    }

    let resp: serde_json::Value = client.get_json(&path).await?;
    let items = resp["items"].as_array().cloned().unwrap_or_default();

    if items.is_empty() {
        println!("{}", "No results found.".yellow());
        return Ok(());
    }

    println!(
        "\n{}\n",
        format!("Found {} result(s) for '{}'", items.len(), args.query).bold()
    );

    for item in &items {
        let kind = item["kind"].as_str().unwrap_or("?");
        let ns = item["namespace"].as_str().unwrap_or("?");
        let name = item["name"].as_str().unwrap_or("?");
        let desc = item["description"].as_str().unwrap_or("");
        let score = item["score"].as_f64().unwrap_or(0.0);
        let dl = item["downloads"].as_i64().unwrap_or(0);

        println!(
            "  {} {} {}",
            format!("[{kind}]").cyan().bold(),
            format!("{ns}/{name}").green().bold(),
            format!("(score: {score:.3}, ↓{dl})").dimmed(),
        );
        if !desc.is_empty() {
            println!("      {}", desc.dimmed());
        }
        println!();
    }
    Ok(())
}

fn urlenccode(s: &str) -> String {
    s.replace(' ', "+")
}
