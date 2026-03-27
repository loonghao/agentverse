use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client::HubClient;

#[derive(Args)]
pub struct WhoamiArgs {
    /// Also print the saved token
    #[arg(long)]
    pub show_token: bool,
}

pub async fn run(args: WhoamiArgs, client: &HubClient) -> Result<()> {
    let resp: serde_json::Value = client.get_json("/api/v1/auth/me").await?;

    let user = &resp["user"];
    println!("\n{} Authenticated as:\n", "👤".bold());
    println!("  Username : {}", user["username"].as_str().unwrap_or("?").bold());
    println!("  ID       : {}", user["id"].as_str().unwrap_or("?").dimmed());
    println!("  Kind     : {}", user["kind"].as_str().unwrap_or("?"));
    if let Some(email) = user["email"].as_str() {
        println!("  Email    : {email}");
    }
    if let Some(pk) = user["public_key"].as_str() {
        println!("  Pub key  : {}…", &pk[..pk.len().min(16)]);
    }

    if args.show_token {
        use crate::config::CliConfig;
        if let Ok(cfg) = CliConfig::load() {
            if let Some(tok) = cfg.token {
                println!("\n  Token    : {}", tok.dimmed());
            }
        }
    }

    println!();
    Ok(())
}

