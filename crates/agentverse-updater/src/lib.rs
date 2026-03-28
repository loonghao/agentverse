//! Shared self-update logic for agentverse binaries.
//!
//! Both `agentverse` (CLI) and `agentverse-server` use this crate to query
//! GitHub releases, download the appropriate archive, and atomically replace
//! the running binary.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};

const GITHUB_API_RELEASES: &str =
    "https://api.github.com/repos/loonghao/agentverse/releases?per_page=10";

// ── Public types ──────────────────────────────────────────────────────────────

/// A resolved GitHub release asset ready for download.
pub struct ReleaseInfo {
    pub version: String,
    pub asset_name: String,
    pub download_url: String,
}

// ── GitHub API models ─────────────────────────────────────────────────────────

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

// ── Core public API ───────────────────────────────────────────────────────────

/// Check whether a newer release exists for the given binary.
///
/// Returns `Ok(Some(info))` when an update is available, `Ok(None)` when
/// the binary is already at the latest version.
pub async fn check_for_update(
    current_version: &str,
    binary_name: &str,
    token: Option<&str>,
) -> Result<Option<ReleaseInfo>> {
    let client = build_client(token)?;
    let release = fetch_latest_release(&client).await?;
    let latest = release.tag_name.trim_start_matches('v');

    if latest == current_version {
        return Ok(None);
    }

    let asset = find_platform_asset(&release.assets, binary_name).with_context(|| {
        format!(
            "No asset matching '{binary_name}' found for platform '{}' in release v{latest}",
            current_platform_suffix()
        )
    })?;

    Ok(Some(ReleaseInfo {
        version: latest.to_string(),
        asset_name: asset.name.clone(),
        download_url: asset.browser_download_url.clone(),
    }))
}

/// Download and atomically replace the current binary.
pub async fn apply_update(info: &ReleaseInfo, token: Option<&str>) -> Result<()> {
    let client = build_client(token)?;

    // Create a unique temp directory
    let tmp_dir = {
        let mut p = env::temp_dir();
        p.push(format!("agentverse-update-{}", std::process::id()));
        std::fs::create_dir_all(&p)?;
        p
    };

    let archive_path = tmp_dir.join(&info.asset_name);
    tracing::debug!(
        "downloading {} → {}",
        info.download_url,
        archive_path.display()
    );
    download_asset(&client, &info.download_url, &archive_path).await?;

    let bin_name = native_binary_name();
    let binary_path = extract_binary(&archive_path, &tmp_dir, bin_name)?;
    replace_self(&binary_path)?;

    // Best-effort cleanup
    let _ = std::fs::remove_dir_all(&tmp_dir);
    Ok(())
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn build_client(token: Option<&str>) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .user_agent(concat!("agentverse-updater/", env!("CARGO_PKG_VERSION")));

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

async fn fetch_latest_release(client: &reqwest::Client) -> Result<GhRelease> {
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

fn find_platform_asset<'a>(assets: &'a [GhAsset], binary_name: &str) -> Option<&'a GhAsset> {
    let platform = current_platform_suffix();
    assets
        .iter()
        .find(|a| a.name.contains(binary_name) && a.name.contains(platform))
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

fn native_binary_name() -> &'static str {
    if cfg!(windows) {
        ".exe"
    } else {
        ""
    }
}

async fn download_asset(client: &reqwest::Client, url: &str, dest: &Path) -> Result<()> {
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

fn extract_binary(archive: &Path, dest_dir: &Path, _suffix: &str) -> Result<PathBuf> {
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
    }

    #[cfg(windows)]
    {
        let dest_str = dest_dir.to_string_lossy();
        let status = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Expand-Archive -Path '{archive_str}' -DestinationPath '{dest_str}' -Force"
                ),
            ])
            .status()
            .context("running Expand-Archive")?;
        if !status.success() {
            bail!("Expand-Archive failed");
        }
    }

    // Walk the extracted directory and find the first executable file
    find_extracted_binary(dest_dir)
}

fn find_extracted_binary(dir: &Path) -> Result<PathBuf> {
    fn walk(dir: &Path) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name()?.to_string_lossy().into_owned();
                // Match the binary: no extension on Unix, .exe on Windows
                let is_binary = if cfg!(windows) {
                    name.ends_with(".exe")
                } else {
                    !name.contains('.') || name.ends_with("-server")
                };
                if is_binary {
                    return Some(path);
                }
            }
            if path.is_dir() {
                if let Some(found) = walk(&path) {
                    return Some(found);
                }
            }
        }
        None
    }
    walk(dir).context("could not find extracted binary in archive")
}

fn replace_self(new_binary: &Path) -> Result<()> {
    let current_exe = env::current_exe().context("resolving current executable path")?;

    #[cfg(windows)]
    {
        let backup = current_exe.with_extension("exe.bak");
        let _ = std::fs::remove_file(&backup);
        std::fs::rename(&current_exe, &backup).context("renaming current executable")?;
        std::fs::copy(new_binary, &current_exe).context("copying new binary")?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(new_binary, perms).context("setting executable permissions")?;
        std::fs::rename(new_binary, &current_exe)
            .or_else(|_| std::fs::copy(new_binary, &current_exe).map(|_| ()))
            .context("replacing current executable")?;
    }

    Ok(())
}
