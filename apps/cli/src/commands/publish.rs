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
    /// Path to a zip archive to upload as an internal skill package.
    ///
    /// When provided, the zip is uploaded to the server's configured object
    /// store (S3, COS, local-disk, etc.) after the metadata is published.
    /// The returned download URL is printed so you can verify it.
    #[arg(long, value_name = "FILE")]
    pub zip: Option<std::path::PathBuf>,
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

    // Replace JSON `null` with appropriate empty defaults so the server can
    // deserialize fields like `dependencies` (HashMap) without a 422 error.
    let capabilities = if mf.capabilities.is_null() {
        serde_json::json!({})
    } else {
        mf.capabilities
    };
    let dependencies = if mf.dependencies.is_null() {
        serde_json::json!({})
    } else {
        mf.dependencies
    };
    let tags = match mf.metadata.get("tags") {
        Some(t) if !t.is_null() => t.clone(),
        _ => serde_json::json!([]),
    };

    let manifest_json = serde_json::json!({
        "description": mf.package.description.unwrap_or_default(),
        "capabilities": capabilities,
        "dependencies": dependencies,
        "tags": tags,
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

    // ── Optional zip upload ───────────────────────────────────────────────────
    if let Some(zip_path) = args.zip {
        upload_zip(
            client,
            &mf.package.namespace,
            &mf.package.name,
            &zip_path,
            args.changelog.as_deref(),
        )
        .await?;
    }

    Ok(())
}

/// Upload a skill zip to the server's internal object store.
///
/// Calls `POST /api/v1/skills/{namespace}/{name}/upload` with the zip as a
/// multipart `file` field.  On success, prints the returned download URL.
async fn upload_zip(
    client: &HubClient,
    namespace: &str,
    name: &str,
    zip_path: &std::path::Path,
    changelog: Option<&str>,
) -> Result<()> {
    use reqwest::multipart;

    println!("  {} Uploading {} …", "↑".cyan().bold(), zip_path.display());

    let zip_bytes = std::fs::read(zip_path)
        .with_context(|| format!("reading zip file {}", zip_path.display()))?;

    let file_name = zip_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("skill.zip")
        .to_string();

    let mut form = multipart::Form::new().part(
        "file",
        multipart::Part::bytes(zip_bytes)
            .file_name(file_name)
            .mime_str("application/zip")?,
    );

    if let Some(cl) = changelog {
        form = form.text("changelog", cl.to_string());
    }

    let upload_path = format!("/api/v1/skills/{namespace}/{name}/upload");
    let resp = client
        .post(&upload_path)
        .multipart(form)
        .send()
        .await
        .context("sending upload request")?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.context("reading upload response")?;

    if !status.is_success() {
        anyhow::bail!(
            "upload failed ({}): {}",
            status,
            body["error"]["message"].as_str().unwrap_or("unknown")
        );
    }

    let download_url = body["download_url"].as_str().unwrap_or("?");
    println!(
        "  {} Package uploaded  {}\n",
        "✓".green().bold(),
        download_url.cyan()
    );

    // Warn if the server returned a separate download token (bearer auth).
    if let Some(token) = body["download_token"].as_str() {
        println!(
            "  {} Download token (store in CLI config or pass as --token):\n  {}\n",
            "!".yellow().bold(),
            token
        );
    }

    Ok(())
}
