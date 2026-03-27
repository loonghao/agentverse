use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::{client::HubClient, config::CliConfig};

#[derive(Args)]
pub struct LoginArgs {
    /// Username
    pub username: String,
    /// Password (prompted if omitted)
    #[arg(long)]
    pub password: Option<String>,
}

pub async fn run(args: LoginArgs, client: &HubClient) -> Result<()> {
    let password = match args.password {
        Some(p) => p,
        None => rpassword::prompt_password("Password: ")?,
    };

    let body = serde_json::json!({
        "username": args.username,
        "password": password,
    });

    let resp: serde_json::Value = client.post_json("/api/v1/auth/login", &body).await?;
    let token = resp["access_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("no token in response"))?;

    let mut cfg = CliConfig::load().unwrap_or_default();
    cfg.token = Some(token.to_string());
    cfg.username = Some(args.username.clone());
    cfg.save()?;

    println!(
        "\n{} Logged in as {}\n",
        "✓".green().bold(),
        args.username.bold(),
    );
    Ok(())
}
