//! Skill deployment: download, extract, and copy to agent-specific paths.
//!
//! Two extraction strategies are supported:
//!
//! - **Full archive** (`extract_zip`): used for `clawhub`, `github`, and `url` packages
//!   where the zip contains exactly the skill files at the archive root.
//!
//! - **Subpath extraction** (`extract_zip_subpath`): used for `github_repo` packages
//!   where the zip is an entire repo archive and the skill lives under a subdirectory.
//!   The archive root (e.g. `skills-main/`) is auto-detected from the first zip entry.

pub mod paths;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use agentverse_core::skill::{AgentKind, SkillInstall, SkillPackage, SourceType};
use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::backends::PackageBackend;
use crate::error::SkillError;

pub use paths::{agent_skills_root, all_known_agents, skill_install_path};

/// Deploy a skill package to one or more agent runtimes.
///
/// Steps:
/// 1. Download the archive from the backend to a temp file.
/// 2. Verify SHA-256 checksum (if provided).
/// 3. Extract into `{agent_skills_root}/{namespace}/{name}/`.
/// 4. Return `SkillInstall` records for each successful deployment.
pub async fn deploy_skill(
    pkg: &SkillPackage,
    namespace: &str,
    name: &str,
    agents: &[AgentKind],
    backend: Arc<dyn PackageBackend>,
) -> Result<Vec<SkillInstall>, SkillError> {
    // Download to a temp file
    let tmp_path = {
        let mut p = std::env::temp_dir();
        p.push(format!("agentverse-skill-{}.zip", Uuid::new_v4()));
        p
    };

    tracing::info!(url = %pkg.download_url, ?tmp_path, "downloading skill package");
    backend.download(&pkg.download_url, &tmp_path).await?;

    // Verify checksum if present
    if let Some(ref expected) = pkg.checksum {
        let actual = sha256_file(&tmp_path)?;
        if actual != *expected {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(SkillError::ChecksumMismatch {
                expected: expected.clone(),
                actual,
            });
        }
    }

    let mut installs = Vec::new();

    // For GitHub repo archives, read the skill subdirectory from metadata.
    // `metadata` is `serde_json::Value`; indexing a missing key returns `Value::Null`
    // and `.as_str()` on Null returns `None`, so this is always safe.
    let github_repo_skill_path: Option<String> = if pkg.source_type == SourceType::GitHubRepo {
        pkg.metadata["github_repo"]["skill_path"]
            .as_str()
            .map(str::to_owned)
    } else {
        None
    };

    for agent in agents {
        let dest = skill_install_path(agent, namespace, name);
        tracing::info!(agent = %agent, ?dest, "deploying skill");
        std::fs::create_dir_all(&dest)
            .map_err(|e| SkillError::Deploy(format!("mkdir {}: {e}", dest.display())))?;

        if let Some(ref skill_path) = github_repo_skill_path {
            // Extract only the skill subdirectory from the whole-repo archive.
            extract_zip_subpath(&tmp_path, skill_path, &dest)?;
        } else {
            extract_zip(&tmp_path, &dest)?;
        }

        installs.push(SkillInstall {
            id: Uuid::new_v4(),
            skill_package_id: pkg.id,
            agent_kind: agent.clone(),
            install_path: dest.to_string_lossy().into_owned(),
            installed_at: Utc::now(),
        });
    }

    let _ = std::fs::remove_file(&tmp_path);
    Ok(installs)
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn sha256_file(path: &Path) -> Result<String, SkillError> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn extract_zip(archive: &Path, dest: &Path) -> Result<(), SkillError> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let out_path = sanitize_zip_path(dest, entry.name())?;

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }
    Ok(())
}

/// Extract only a specific subdirectory from a whole-repo GitHub archive.
///
/// GitHub archive zips have the structure:
/// ```text
/// {repo}-{ref}/
/// {repo}-{ref}/{skill_path}/SKILL.md
/// {repo}-{ref}/{skill_path}/...
/// ```
///
/// This function:
/// 1. Auto-detects the archive root prefix from the first entry.
/// 2. Builds the full prefix: `{archive_root}/{skill_path}/`.
/// 3. Extracts only matching entries, stripping the prefix so files land flat in `dest`.
pub fn extract_zip_subpath(
    archive: &Path,
    skill_path: &str,
    dest: &Path,
) -> Result<(), SkillError> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;

    // Auto-detect archive root: first entry that ends with '/' is the repo root dir.
    let archive_root: String = (0..zip.len())
        .find_map(|i| {
            zip.by_index(i).ok().and_then(|e| {
                let n = e.name().to_owned();
                if e.is_dir() {
                    Some(n)
                } else {
                    None
                }
            })
        })
        .unwrap_or_default();

    // Full prefix inside the zip that all skill files share.
    let prefix = format!("{archive_root}{skill_path}/");
    tracing::debug!(%prefix, "extracting subpath from GitHub repo archive");

    // Re-open; ZipArchive must be iterated from scratch after detection.
    let file2 = std::fs::File::open(archive)?;
    let mut zip2 = zip::ZipArchive::new(file2)?;

    let mut extracted = 0usize;
    for i in 0..zip2.len() {
        let mut entry = zip2.by_index(i)?;
        let entry_name = entry.name().to_owned();

        let Some(rel) = entry_name.strip_prefix(&prefix) else {
            continue;
        };
        if rel.is_empty() {
            continue; // skip the directory entry itself
        }

        let out_path = sanitize_zip_path(dest, rel)?;

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
            extracted += 1;
        }
    }

    if extracted == 0 {
        return Err(SkillError::Deploy(format!(
            "no files found under '{prefix}' in archive — check skill_path"
        )));
    }

    tracing::info!(%extracted, ?dest, "github_repo subpath extracted successfully");
    Ok(())
}

/// Prevent path traversal: strip leading `/` and reject `..` components.
fn sanitize_zip_path(base: &Path, entry_name: &str) -> Result<PathBuf, SkillError> {
    let rel = entry_name.trim_start_matches('/');
    let joined = base.join(rel);
    if !joined.starts_with(base) {
        return Err(SkillError::Deploy(format!(
            "path traversal attempt in zip entry: {entry_name}"
        )));
    }
    Ok(joined)
}
