//! Skill-specific API routes.
//!
//! These routes layer on top of the generic artifact CRUD endpoints and
//! add skill-package management: registering downloadable packages,
//! listing available packages, and triggering download + deployment.
//!
//! ## Endpoints
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | POST | `/api/v1/skills/import`             | One-click import from a GitHub repo URL (auto-registers) |
//! | POST | `/api/v1/skills/:ns/:name/packages` | Register a skill package (fires publish hooks) |
//! | GET  | `/api/v1/skills/:ns/:name/packages` | List all packages for the latest skill version |
//! | POST | `/api/v1/skills/:ns/:name/install`  | Download and deploy to agent runtimes |

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use agentverse_core::{
    artifact::{Artifact, ArtifactKind, ArtifactStatus, ArtifactVersion, Manifest},
    skill::{AgentKind, SkillPackage, SourceType},
};
use agentverse_skills::{
    all_known_agents, parse_github_tree_url, parse_skill_md, GitHubRepoBackend, HookRegistry,
    LoggingHook, MetadataHook,
};

use crate::{
    error::{ApiError, ApiResult},
    extractors::AuthUser,
    state::AppState,
};

// ── Request / Response DTOs ───────────────────────────────────────────────────

/// Register a downloadable package for an existing skill version.
///
/// For `source_type = "github_repo"`, `download_url` may be supplied as either
/// - the GitHub tree URL: `https://github.com/org/repo/tree/main/skills/my-skill`
/// - the raw archive URL: `https://github.com/org/repo/archive/main.zip`
///
/// In the first case the API auto-converts and fills `metadata.github_repo`.
#[derive(Debug, Deserialize)]
pub struct RegisterPackageRequest {
    /// "clawhub" | "github" | "github_repo" | "url"
    pub source_type: String,
    /// Download URL or GitHub tree URL (auto-detected for github_repo).
    pub download_url: String,
    /// Optional SHA-256 hex checksum.
    pub checksum: Option<String>,
    /// Optional compressed file size in bytes.
    pub file_size: Option<i64>,
    /// Extra metadata (platform hints, agent compatibility, etc.)
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// One-click import of a skill directly from a GitHub repository URL.
///
/// The handler fetches `SKILL.md` to discover the skill name and description,
/// then creates the artifact + version + package in one step.
#[derive(Debug, Deserialize)]
pub struct ImportSkillRequest {
    /// GitHub tree URL, e.g.:
    /// `https://github.com/anthropics/skills/tree/main/skills/algorithmic-art`
    pub url: String,
    /// Override the skill namespace (defaults to the repo owner).
    pub namespace: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportSkillResponse {
    pub skill: serde_json::Value,
    pub package: SkillPackage,
    pub created: bool,
}

#[derive(Debug, Deserialize)]
pub struct InstallRequest {
    /// Specific version to install. Defaults to the latest version.
    pub version: Option<String>,
    /// Which agent runtimes to deploy to. Defaults to all known agents.
    pub agents: Option<Vec<String>>,
    /// Source backend to use. Defaults to the first available package.
    pub source_type: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/v1/skills/:namespace/:name/packages
///
/// Registers a new downloadable package for the latest version of a skill.
/// After persisting, the publishing hook pipeline is fired (MetadataHook +
/// LoggingHook) to record the download URL in the database.
pub async fn register_package(
    Path((namespace, name)): Path<(String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<RegisterPackageRequest>,
) -> ApiResult<impl IntoResponse> {
    // Resolve skill artifact
    let artifact = state
        .artifacts
        .find_by_namespace_name(&ArtifactKind::Skill, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("skill/{namespace}/{name}")))?;

    // Only the author may attach packages
    if artifact.author_id != claims.sub {
        return Err(ApiError::Forbidden(
            "only the skill author can register packages".into(),
        ));
    }

    // Resolve latest version
    let version = state
        .versions
        .find_latest(artifact.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("no version found for skill".into()))?;

    let source_type = SourceType::from_str(&req.source_type).map_err(ApiError::BadRequest)?;

    // For github_repo: if the caller provided a tree URL, auto-convert to
    // the archive download URL and fill in metadata.github_repo.
    let (download_url, metadata) = if source_type == SourceType::GitHubRepo {
        let info = parse_github_tree_url(&req.download_url).ok_or_else(|| {
            ApiError::BadRequest(format!(
                "github_repo requires a GitHub tree URL \
                 (https://github.com/{{owner}}/{{repo}}/tree/{{ref}}/{{path}}), \
                 got: {}",
                req.download_url
            ))
        })?;
        let archive_url = info.archive_url();
        let repo_meta = info.to_metadata_json();
        // Merge caller-supplied metadata with the auto-generated github_repo key.
        let mut merged = req.metadata.clone();
        if let (Some(obj), Some(extra)) = (merged.as_object_mut(), repo_meta.as_object()) {
            for (k, v) in extra {
                obj.insert(k.clone(), v.clone());
            }
        } else {
            merged = repo_meta;
        }
        (archive_url, merged)
    } else {
        (req.download_url, req.metadata)
    };

    let pkg = SkillPackage {
        id: Uuid::new_v4(),
        artifact_version_id: version.id,
        source_type,
        download_url,
        checksum: req.checksum,
        file_size: req.file_size,
        metadata,
        created_at: Utc::now(),
    };

    // Fire publish hooks: MetadataHook persists to DB, LoggingHook traces.
    let mut registry = HookRegistry::new();
    registry.register(std::sync::Arc::new(MetadataHook::new(
        state.skill_packages.clone(),
    )));
    registry.register(std::sync::Arc::new(LoggingHook));
    registry.run_all(&pkg).await;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "package": pkg })),
    ))
}

/// GET /api/v1/skills/:namespace/:name/packages
///
/// Lists all registered packages for the latest version of a skill.
pub async fn list_packages(
    Path((namespace, name)): Path<(String, String)>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let artifact = state
        .artifacts
        .find_by_namespace_name(&ArtifactKind::Skill, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("skill/{namespace}/{name}")))?;

    let version = state
        .versions
        .find_latest(artifact.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("no version found for skill".into()))?;

    let packages = state.skill_packages.list_for_version(version.id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "artifact_id": artifact.id,
            "version": version.version,
            "packages": packages,
            "total": packages.len(),
        })),
    ))
}

/// POST /api/v1/skills/:namespace/:name/install
///
/// Downloads the skill package from the registered backend and extracts it
/// into the installation directories for the requested agent runtimes.
pub async fn install_skill(
    Path((namespace, name)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(req): Json<Option<InstallRequest>>,
) -> ApiResult<impl IntoResponse> {
    let req = req.unwrap_or(InstallRequest {
        version: None,
        agents: None,
        source_type: None,
    });

    // Resolve artifact
    let artifact = state
        .artifacts
        .find_by_namespace_name(&ArtifactKind::Skill, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("skill/{namespace}/{name}")))?;

    // Resolve version
    let version = match req.version {
        Some(ref v) => state
            .versions
            .find_by_semver(artifact.id, v)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("version {v}")))?,
        None => state
            .versions
            .find_latest(artifact.id)
            .await?
            .ok_or_else(|| ApiError::NotFound("no version found for skill".into()))?,
    };

    // Resolve package
    let packages = state.skill_packages.list_for_version(version.id).await?;
    if packages.is_empty() {
        return Err(ApiError::NotFound(format!(
            "no packages registered for skill/{namespace}/{name}@{}",
            version.version
        )));
    }

    // Pick the package matching the requested source_type (or any)
    let pkg = if let Some(ref st) = req.source_type {
        let target = SourceType::from_str(st).map_err(ApiError::BadRequest)?;
        packages
            .into_iter()
            .find(|p| p.source_type == target)
            .ok_or_else(|| ApiError::NotFound(format!("no {st} package found")))?
    } else {
        packages.into_iter().next().unwrap()
    };

    // Resolve agent list
    let agents: Vec<AgentKind> = match req.agents {
        Some(ref list) => list
            .iter()
            .map(|s| AgentKind::from_str(s).unwrap_or_else(|_| AgentKind::Custom(s.clone())))
            .collect(),
        None => all_known_agents(),
    };

    // Pick backend based on source_type
    let backend: std::sync::Arc<dyn agentverse_skills::PackageBackend> = match pkg.source_type {
        SourceType::Clawhub => std::sync::Arc::new(agentverse_skills::ClawhubBackend::new()),
        SourceType::GitHub => std::sync::Arc::new(agentverse_skills::GitHubBackend::default()),
        SourceType::GitHubRepo => std::sync::Arc::new(GitHubRepoBackend::default()),
        SourceType::Url => std::sync::Arc::new(agentverse_skills::UrlBackend::new()),
    };

    let installs = agentverse_skills::deploy_skill(&pkg, &namespace, &name, &agents, backend)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;

    // Persist install records
    for install in &installs {
        state.skill_installs.record(install.clone()).await.ok(); // non-fatal — still return success
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "skill": format!("{namespace}/{name}@{}", version.version),
            "package_id": pkg.id,
            "installs": installs,
            "total": installs.len(),
        })),
    ))
}

// ── Import endpoint ───────────────────────────────────────────────────────────

/// POST /api/v1/skills/import
///
/// One-click import of a skill from a GitHub repository directory URL.
///
/// Fetches the skill's `SKILL.md`, extracts its name and description, then
/// creates (or re-uses) an artifact + version before registering the package.
/// Idempotent: calling it twice for the same URL returns the existing skill.
///
/// ## Example request
/// ```json
/// { "url": "https://github.com/anthropics/skills/tree/main/skills/algorithmic-art" }
/// ```
pub async fn import_skill(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<ImportSkillRequest>,
) -> ApiResult<impl IntoResponse> {
    // Parse the GitHub tree URL
    let info = parse_github_tree_url(&req.url).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "expected a GitHub tree URL \
             (https://github.com/{{owner}}/{{repo}}/tree/{{ref}}/{{path}}), \
             got: {}",
            req.url
        ))
    })?;

    // Fetch SKILL.md to get name / description
    let backend = GitHubRepoBackend::default();
    let skill_md = backend.fetch_skill_md(&info).await.map_err(|e| {
        ApiError::BadRequest(format!(
            "could not fetch SKILL.md from {}: {e}",
            info.raw_url("SKILL.md")
        ))
    })?;

    // Parse the full SKILL.md frontmatter — tags, metadata, version, etc.
    let fallback_name = info
        .skill_path
        .split('/')
        .next_back()
        .unwrap_or("unknown-skill")
        .to_owned();
    let parsed = parse_skill_md(&skill_md, &fallback_name);
    let namespace = req.namespace.unwrap_or_else(|| info.owner.clone());

    // Merge github_repo source metadata with the skill's own metadata block.
    let mut extra = info.to_metadata_json();
    if let (Some(obj), serde_json::Value::Object(skill_meta)) =
        (extra.as_object_mut(), &parsed.metadata)
    {
        for (k, v) in skill_meta {
            obj.entry(k).or_insert_with(|| v.clone());
        }
    }

    // Resolve or create the skill artifact
    let (artifact, created) = match state
        .artifacts
        .find_by_namespace_name(&ArtifactKind::Skill, &namespace, &parsed.name)
        .await?
    {
        Some(existing) => (existing, false),
        None => {
            let manifest = Manifest {
                description: parsed.description.clone().unwrap_or_default(),
                tags: parsed.tags.clone(),
                homepage: parsed.homepage.clone(),
                license: parsed.license.clone(),
                extra,
                ..Default::default()
            };

            let artifact = Artifact {
                id: Uuid::new_v4(),
                kind: ArtifactKind::Skill,
                namespace: namespace.clone(),
                name: parsed.name.clone(),
                display_name: None,
                manifest,
                status: ArtifactStatus::Active,
                author_id: claims.sub,
                downloads: 0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            let saved = state.artifacts.create(artifact).await?;
            (saved, true)
        }
    };

    // Determine initial version: prefer frontmatter hint, default to "0.1.0"
    let init_version = parsed.version.as_deref().unwrap_or("0.1.0").to_owned();
    let semver =
        semver::Version::parse(&init_version).unwrap_or_else(|_| semver::Version::new(0, 1, 0));

    // Resolve or create version
    let version = match state.versions.find_latest(artifact.id).await? {
        Some(v) => v,
        None => {
            let v = ArtifactVersion {
                id: Uuid::new_v4(),
                artifact_id: artifact.id,
                version: semver.to_string(),
                major: semver.major,
                minor: semver.minor,
                patch: semver.patch,
                pre_release: None,
                content: serde_json::json!({ "source_url": req.url }),
                checksum: "".into(),
                signature: None,
                changelog: Some(format!("Imported from {}", req.url)),
                bump_reason: "minor".into(),
                published_by: claims.sub,
                published_at: Utc::now(),
            };
            state.versions.publish(v).await?
        }
    };

    // Build and register the package
    let archive_url = info.archive_url();
    let repo_meta = info.to_metadata_json();

    let pkg = SkillPackage {
        id: Uuid::new_v4(),
        artifact_version_id: version.id,
        source_type: SourceType::GitHubRepo,
        download_url: archive_url,
        checksum: None,
        file_size: None,
        metadata: repo_meta,
        created_at: Utc::now(),
    };

    let mut registry = HookRegistry::new();
    registry.register(std::sync::Arc::new(MetadataHook::new(
        state.skill_packages.clone(),
    )));
    registry.register(std::sync::Arc::new(LoggingHook));
    registry.run_all(&pkg).await;

    let skill_json = serde_json::json!({
        "id": artifact.id,
        "namespace": artifact.namespace,
        "name": artifact.name,
        "description": artifact.manifest.description,
        "tags": artifact.manifest.tags,
        "homepage": artifact.manifest.homepage,
        "license": artifact.manifest.license,
        "version": version.version,
        "source_url": req.url,
    });

    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        Json(ImportSkillResponse {
            skill: skill_json,
            package: pkg,
            created,
        }),
    ))
}

// ── Package management helpers ────────────────────────────────────────────────

/// GET /api/v1/skills/:namespace/:name/packages/:package_id
///
/// Returns metadata for a single registered skill package by its UUID.
pub async fn get_package(
    Path((namespace, name, package_id)): Path<(String, String, Uuid)>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    // Verify the skill exists first to give a meaningful 404.
    state
        .artifacts
        .find_by_namespace_name(&ArtifactKind::Skill, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("skill/{namespace}/{name}")))?;

    let pkg = state
        .skill_packages
        .find_by_id(package_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("package {package_id}")))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "package": pkg }))))
}

/// DELETE /api/v1/skills/:namespace/:name/packages/:package_id
///
/// Removes a package registration.  Does **not** affect already-installed
/// agent runtimes — it only removes the registry entry.
pub async fn delete_package(
    Path((namespace, name, package_id)): Path<(String, String, Uuid)>,
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
) -> ApiResult<impl IntoResponse> {
    state
        .artifacts
        .find_by_namespace_name(&ArtifactKind::Skill, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("skill/{namespace}/{name}")))?;

    state
        .skill_packages
        .find_by_id(package_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("package {package_id}")))?;

    state.skill_packages.delete(package_id).await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "deleted": true,
            "package_id": package_id,
        })),
    ))
}

/// GET /api/v1/skills/:namespace/:name/versions/:version/packages
///
/// Lists all packages registered for a **specific** semver of a skill.
/// Useful when a newer version has been published and callers need to inspect
/// packages for a pinned older release.
pub async fn list_packages_for_version(
    Path((namespace, name, ver)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let artifact = state
        .artifacts
        .find_by_namespace_name(&ArtifactKind::Skill, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("skill/{namespace}/{name}")))?;

    let version = state
        .versions
        .find_by_semver(artifact.id, &ver)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("version {ver}")))?;

    let packages = state.skill_packages.list_for_version(version.id).await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "artifact_id": artifact.id,
            "version":     version.version,
            "packages":    packages,
            "total":       packages.len(),
        })),
    ))
}

/// GET /api/v1/skills/:namespace/:name/installs
///
/// Lists all known install records for the skill (across all agent runtimes
/// and all versions).  Useful for auditing which agents have a skill deployed.
pub async fn list_installs(
    Path((namespace, name)): Path<(String, String)>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let artifact = state
        .artifacts
        .find_by_namespace_name(&ArtifactKind::Skill, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("skill/{namespace}/{name}")))?;

    // Gather all packages for the artifact, then collect their installs.
    let packages = state.skill_packages.list_for_artifact(artifact.id).await?;

    let mut installs = Vec::new();
    for pkg in &packages {
        let pkg_installs = state.skill_installs.list_for_package(pkg.id).await?;
        installs.extend(pkg_installs);
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "artifact_id": artifact.id,
            "installs":    installs,
            "total":       installs.len(),
        })),
    ))
}

/// GET /api/v1/skills/agents/:agent_kind
///
/// Returns all skills that have been installed for the given agent runtime
/// (e.g. `openclaw`, `codebuddy`, `workerbuddy`).
pub async fn list_skills_for_agent(
    Path(agent_kind_str): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let agent = AgentKind::from_str(&agent_kind_str)
        .map_err(|_| ApiError::BadRequest(format!("unknown agent kind: {agent_kind_str}")))?;

    let installs = state.skill_installs.list_for_agent(&agent).await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "agent_kind": agent_kind_str,
            "installs":   installs,
            "total":      installs.len(),
        })),
    ))
}
