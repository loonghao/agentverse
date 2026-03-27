use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::{client::HubClient, config::CliConfig};

#[derive(Args)]
pub struct RegisterArgs {
    /// Username (3–32 chars, alphanumeric / _ / -)
    pub username: String,
    /// Email address (optional)
    #[arg(long)]
    pub email: Option<String>,
    /// Password (prompted if omitted)
    #[arg(long)]
    pub password: Option<String>,
    /// Register as an AI agent instead of a human user
    #[arg(long)]
    pub agent: bool,
    /// Ed25519 public key hex (for agents using key-based auth)
    #[arg(long)]
    pub public_key: Option<String>,
}

pub async fn run(args: RegisterArgs, client: &HubClient) -> Result<()> {
    let password = match args.password {
        Some(p) => p,
        None if args.agent => String::new(), // agents don't need passwords
        None => {
            let p1 = rpassword::prompt_password("Password: ")?;
            let p2 = rpassword::prompt_password("Confirm password: ")?;
            if p1 != p2 {
                anyhow::bail!("passwords do not match");
            }
            p1
        }
    };

    let body = serde_json::json!({
        "username": args.username,
        "email": args.email,
        "password": password,
        "kind": if args.agent { "agent" } else { "human" },
        "public_key": args.public_key,
    });

    let resp: serde_json::Value = client.post_json("/api/v1/auth/register", &body).await?;

    // Auto-save token if returned
    if let Some(token) = resp["access_token"].as_str() {
        let mut cfg = CliConfig::load().unwrap_or_default();
        cfg.token = Some(token.to_string());
        cfg.username = Some(args.username.clone());
        cfg.save()?;
    }

    let kind = if args.agent { "agent" } else { "human" };
    println!(
        "\n{} Registered {} {} and logged in\n",
        "✓".green().bold(),
        kind,
        args.username.bold(),
    );
    Ok(())
}

