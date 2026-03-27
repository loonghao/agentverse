use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use serde::Deserialize;

use crate::client::HubClient;

#[derive(Args)]
pub struct PublishArgs {
    /// Path to the manifest file (TOML or JSON)
    #[arg(default_value = "agentverse.toml")]
    pub manifest: std::path::PathBuf,
    /// Content file to publish (JSON). Defaults to manifest dir/content.json
    #[arg(long)]
    pub content: Option<std::path::PathBuf>,
    /// Bump override: patch | minor | major (auto-inferred if omitted)
    #[arg(long)]
    pub bump: Option<String>,
    /// Changelog message for this version
    #[arg(long)]
    pub changelog: Option<String>,
}

/// Manifest file format (TOML).
#[derive(Debug, Deserialize)]
struct ManifestFile {
    package: PackageSection,
    #[serde(default)]
    capabilities: serde_json::Value,
    #[serde(default)]
    dependencies: serde_json::Value,
    #[serde(default)]
    metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct PackageSection {
    kind: String,
    namespace: String,
    name: String,
    description: Option<String>,
}

pub async fn run(args: PublishArgs, client: &HubClient) -> Result<()> {
    // Load manifest
    let raw = std::fs::read_to_string(&args.manifest)
        .with_context(|| format!("reading {}", args.manifest.display()))?;

    let mf: ManifestFile = if args
        .manifest
        .extension()
        .map(|e| e == "toml")
        .unwrap_or(false)
    {
        toml::from_str(&raw).context("parsing TOML manifest")?
    } else {
        serde_json::from_str(&raw).context("parsing JSON manifest")?
    };

    // Load content file
    let content_path = args.content.unwrap_or_else(|| {
        args.manifest
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("content.json")
    });
    let content: serde_json::Value = if content_path.exists() {
        let raw = std::fs::read_to_string(&content_path)
            .with_context(|| format!("reading {}", content_path.display()))?;
        serde_json::from_str(&raw).context("parsing content JSON")?
    } else {
        serde_json::json!({})
    };

    let manifest_json = serde_json::json!({
        "description": mf.package.description.unwrap_or_default(),
        "capabilities": mf.capabilities,
        "dependencies": mf.dependencies,
        "tags": mf.metadata["tags"],
        "extra": {},
    });

    let body = serde_json::json!({
        "namespace": mf.package.namespace,
        "name": mf.package.name,
        "manifest": manifest_json,
        "content": content,
        "bump": args.bump,
        "changelog": args.changelog,
    });

    let path = format!("/api/v1/{}", mf.package.kind);

    // Try create first; if conflict, publish new version instead
    let resp: serde_json::Value = match client.post_json(&path, &body).await {
        Ok(r) => r,
        Err(e) if e.to_string().contains("409") || e.to_string().contains("already exists") => {
            let pub_path = format!(
                "/api/v1/{}/{}/{}/publish",
                mf.package.kind, mf.package.namespace, mf.package.name
            );
            let pub_body = serde_json::json!({
                "content": content,
                "changelog": args.changelog,
                "bump": args.bump,
            });
            client.post_json(&pub_path, &pub_body).await?
        }
        Err(e) => return Err(e),
    };

    let ver = resp["version"]["version"]
        .as_str()
        .or_else(|| resp["bump"].as_str())
        .unwrap_or("?");

    println!(
        "\n{} Published {}/{}/{}  {}\n",
        "✓".green().bold(),
        mf.package.kind,
        mf.package.namespace,
        mf.package.name,
        format!("v{ver}").green().bold(),
    );
    Ok(())
}
