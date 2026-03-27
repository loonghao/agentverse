use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client::HubClient;

// ── comment ───────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct CommentArgs {
    /// e.g. skill/my-org/my-skill
    pub artifact: String,
    /// Comment text
    pub content: String,
    /// Comment kind: review | learning | suggestion | bug | benchmark
    #[arg(long, default_value = "review")]
    pub kind: String,
    /// Reply to an existing comment (UUID)
    #[arg(long)]
    pub parent_id: Option<String>,
}

pub async fn run_comment(args: CommentArgs, client: &HubClient) -> Result<()> {
    let (kind_str, ns, name) = parse_artifact(&args.artifact)?;
    let path = format!("/api/v1/{kind_str}/{ns}/{name}/comments");
    let body = serde_json::json!({
        "content": args.content,
        "kind": args.kind,
        "parent_id": args.parent_id,
    });
    let resp: serde_json::Value = client.post_json(&path, &body).await?;
    let id = resp["comment"]["id"].as_str().unwrap_or("?");
    println!(
        "\n{} Comment posted (id: {})\n",
        "✓".green().bold(),
        id.dimmed()
    );
    Ok(())
}

// ── like / unlike ────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct LikeArgs {
    /// e.g. skill/my-org/my-skill
    pub artifact: String,
}

pub async fn run_like(args: LikeArgs, client: &HubClient) -> Result<()> {
    let (kind_str, ns, name) = parse_artifact(&args.artifact)?;
    let path = format!("/api/v1/{kind_str}/{ns}/{name}/likes");
    let resp: serde_json::Value = client.post_json(&path, &serde_json::json!({})).await?;
    println!(
        "\n{} Liked {} ({:?})\n",
        "♥".red().bold(),
        args.artifact.bold(),
        resp["like"]["id"]
    );
    Ok(())
}

pub async fn run_unlike(args: LikeArgs, client: &HubClient) -> Result<()> {
    let (kind_str, ns, name) = parse_artifact(&args.artifact)?;
    let path = format!("/api/v1/{kind_str}/{ns}/{name}/likes");
    let resp: serde_json::Value = client.delete_json(&path).await?;
    println!(
        "\n{} {}\n",
        "✓".green().bold(),
        resp["message"].as_str().unwrap_or("unliked")
    );
    Ok(())
}

// ── rate ──────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct RateArgs {
    /// e.g. skill/my-org/my-skill
    pub artifact: String,
    /// Score from 1 to 5
    pub score: i16,
    /// Optional review text
    #[arg(long)]
    pub review: Option<String>,
}

pub async fn run_rate(args: RateArgs, client: &HubClient) -> Result<()> {
    let (kind_str, ns, name) = parse_artifact(&args.artifact)?;
    let path = format!("/api/v1/{kind_str}/{ns}/{name}/ratings");
    let body = serde_json::json!({
        "score": args.score,
        "review_text": args.review,
    });
    client
        .post_json::<_, serde_json::Value>(&path, &body)
        .await?;
    println!(
        "\n{} Rated {} → {}/5\n",
        "★".yellow().bold(),
        args.artifact.bold(),
        args.score
    );
    Ok(())
}

// ── stats ─────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct StatsArgs {
    /// e.g. skill/my-org/my-skill
    pub artifact: String,
}

pub async fn run_stats(args: StatsArgs, client: &HubClient) -> Result<()> {
    let (kind_str, ns, name) = parse_artifact(&args.artifact)?;
    let path = format!("/api/v1/{kind_str}/{ns}/{name}/stats");
    let resp: serde_json::Value = client.get_json(&path).await?;
    println!("\n{} Stats for {}\n", "📊".bold(), args.artifact.bold());
    println!(
        "  Likes:        {}",
        resp["likes_count"].as_i64().unwrap_or(0)
    );
    println!(
        "  Comments:     {}",
        resp["comments_count"].as_i64().unwrap_or(0)
    );
    println!(
        "  Ratings:      {}",
        resp["ratings_count"].as_i64().unwrap_or(0)
    );
    if let Some(avg) = resp["avg_rating"].as_f64() {
        println!("  Avg rating:   {:.1}/5", avg);
    }
    println!(
        "  Interactions: {}",
        resp["interactions_count"].as_i64().unwrap_or(0)
    );
    println!();
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Parse "kind/namespace/name" into (kind, namespace, name).
fn parse_artifact(s: &str) -> Result<(String, String, String)> {
    let parts: Vec<&str> = s.splitn(3, '/').collect();
    if parts.len() != 3 {
        anyhow::bail!(
            "artifact must be in the form <kind>/<namespace>/<name>, e.g. skill/my-org/my-skill"
        );
    }
    Ok((
        parts[0].to_string(),
        parts[1].to_string(),
        parts[2].to_string(),
    ))
}
