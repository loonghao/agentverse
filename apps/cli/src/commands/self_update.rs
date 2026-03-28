use anyhow::{bail, Context, Result};
use clap::Args;
use colored::Colorize;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};

const GITHUB_API_RELEASES: &str =
    "https://api.github.com/repos/loonghao/agentverse/releases?per_page=10";

#[derive(Args)]
pub struct SelfUpdateArgs {
    /// GitHub personal access token to avoid rate limits
    #[arg(long, env = "GITHUB_TOKEN")]
    token: Option<String>,

    /// Only check for a newer version without installing it
    #[arg(long)]
    check: bool,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    prerelease: bool,
    draft: bool,
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

pub async fn run(args: SelfUpdateArgs) -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    println!(
        "{} Checking for updates (current: v{})...",
        "agentverse".cyan().bold(),
        current
    );

    let client = build_client(args.token.as_deref())?;
    let release = fetch_latest_release(&client).await?;
    let latest = release.tag_name.trim_start_matches('v');

    if latest == current {
        println!("{} Already up to date (v{})", "✓".green().bold(), current);
        return Ok(());
    }

    println!(
        "{} New version available: v{} (current: v{})",
        "→".cyan().bold(),
        latest,
        current
    );

    if args.check {
        println!("Run `agentverse self-update` to install the update.");
        return Ok(());
    }

    let asset = find_platform_asset(&release.assets)
        .context("No compatible binary found for this platform in the latest release")?;

    println!("{} Downloading {}...", "↓".cyan(), asset.name);

    // Create a unique temp directory under the system temp dir
    let tmp_dir = {
        let mut p = env::temp_dir();
        p.push(format!("agentverse-update-{}", std::process::id()));
        std::fs::create_dir_all(&p)?;
        p
    };

    let archive_path = tmp_dir.join(&asset.name);
    download_asset(&client, &asset.browser_download_url, &archive_path).await?;

    println!("{} Extracting...", "⚙".cyan());
    let binary_path = extract_binary(&archive_path, &tmp_dir)?;

    replace_self(&binary_path)?;

    // Cleanup temp dir (best effort)
    let _ = std::fs::remove_dir_all(&tmp_dir);

    println!("{} Updated to v{}", "✓".green().bold(), latest);
    println!(
        "  Run `agentverse --version` to confirm, or `agentverse --help` to see what's new."
    );
    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn build_client(token: Option<&str>) -> Result<Client> {
    let mut builder = Client::builder().user_agent(concat!(
        "agentverse-cli/",
        env!("CARGO_PKG_VERSION")
    ));
    if let Some(tok) = token {
        use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {tok}"))?,
        );
        builder = builder.default_headers(headers);
    }
    Ok(builder.build()?)
}

async fn fetch_latest_release(client: &Client) -> Result<GhRelease> {
    let releases: Vec<GhRelease> = client
        .get(GITHUB_API_RELEASES)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .context("fetching GitHub releases")?
        .json()
        .await
        .context("parsing GitHub releases JSON")?;

    releases
        .into_iter()
        .find(|r| !r.prerelease && !r.draft && !r.assets.is_empty())
        .context("no stable release with assets found")
}

fn find_platform_asset(assets: &[GhAsset]) -> Option<&GhAsset> {
    let platform = current_platform_suffix();
    assets.iter().find(|a| a.name.contains(platform))
}

fn current_platform_suffix() -> &'static str {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc"
    } else {
        "unknown"
    }
}

async fn download_asset(client: &Client, url: &str, dest: &Path) -> Result<()> {
    let bytes = client
        .get(url)
        .send()
        .await
        .context("downloading release asset")?
        .bytes()
        .await
        .context("reading download bytes")?;

    let mut file = std::fs::File::create(dest).context("creating archive file")?;
    file.write_all(&bytes).context("writing archive")?;
    Ok(())
}

fn extract_binary(archive: &Path, dest_dir: &Path) -> Result<PathBuf> {
    let archive_str = archive.to_string_lossy();

    #[cfg(unix)]
    {
        let status = std::process::Command::new("tar")
            .args(["-xzf", &archive_str, "-C", &dest_dir.to_string_lossy()])
            .status()
            .context("running tar")?;
        if !status.success() {
            bail!("tar extraction failed");
        }
        // Find the binary
        find_extracted_binary(dest_dir, "agentverse")
    }

    #[cfg(windows)]
    {
        let dest_str = dest_dir.to_string_lossy();
        let status = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Expand-Archive -Path '{archive_str}' -DestinationPath '{dest_str}' -Force"),
            ])
            .status()
            .context("running Expand-Archive")?;
        if !status.success() {
            bail!("Expand-Archive failed");
        }
        find_extracted_binary(dest_dir, "agentverse.exe")
    }
}

fn find_extracted_binary(dir: &Path, name: &str) -> Result<PathBuf> {
    fn walk(dir: &Path, name: &str) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name().is_some_and(|n| n == name) {
                return Some(path);
            }
            if path.is_dir() {
                if let Some(found) = walk(&path, name) {
                    return Some(found);
                }
            }
        }
        None
    }
    walk(dir, name).with_context(|| format!("could not find '{name}' in extracted archive"))
}

fn replace_self(new_binary: &Path) -> Result<()> {
    let current_exe = env::current_exe().context("resolving current executable path")?;

    // On Windows: rename the running exe first (allowed), then move new binary in
    #[cfg(windows)]
    {
        let backup = current_exe.with_extension("exe.bak");
        let _ = std::fs::remove_file(&backup); // ignore if .bak doesn't exist
        std::fs::rename(&current_exe, &backup).context("renaming current executable")?;
        std::fs::copy(new_binary, &current_exe).context("copying new binary")?;
    }

    // On Unix: direct rename is atomic on the same filesystem
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(new_binary, perms).context("setting executable permissions")?;
        std::fs::rename(new_binary, &current_exe)
            .or_else(|_| {
                // Cross-device rename: fall back to copy
                std::fs::copy(new_binary, &current_exe).map(|_| ())
            })
            .context("replacing current executable")?;
    }

    Ok(())
}

