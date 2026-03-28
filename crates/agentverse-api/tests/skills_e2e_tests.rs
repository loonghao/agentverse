//! End-to-End tests for the full skill lifecycle:
//!
//! 1. Register a user → log in → get auth token
//! 2. Create a skill artifact (kind = skill)
//! 3. Register a downloadable package against each backend (clawhub, github, url)
//! 4. List packages and verify metadata is persisted
//! 5. Install endpoint returns correct package and agent paths
//!
//! These tests use in-memory repository stubs and do NOT require a real database
//! or network connection. The install step is tested at the API level (response
//! shape + package resolution) — actual filesystem extraction is tested separately
//! in unit tests inside `agentverse-skills`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::build_test_app_with_state;
use http_body_util::BodyExt;
use tower::ServiceExt;

// ── helpers ───────────────────────────────────────────────────────────────────

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

fn json_req(
    method: &str,
    uri: &str,
    body: serde_json::Value,
    token: Option<&str>,
) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(tok) = token {
        b = b.header("authorization", tok);
    }
    b.body(Body::from(body.to_string())).unwrap()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Helper: register a user and return (user_id, token).
async fn register_and_login(app: &axum::Router) -> (uuid::Uuid, String) {
    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/auth/register",
            serde_json::json!({
                "username": "skill-author",
                "password": "correct-horse-battery-staple",
                "email": "author@example.com"
            }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "register should succeed"
    );
    let json = body_json(resp).await;
    let user_id: uuid::Uuid = json["user"]["id"].as_str().unwrap().parse().unwrap();
    let token = format!("Bearer {}", json["access_token"].as_str().unwrap());
    (user_id, token)
}

/// Helper: create a skill artifact and return the artifact id.
async fn create_skill(app: &axum::Router, token: &str, name: &str) -> uuid::Uuid {
    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills",
            serde_json::json!({
                "namespace": "testorg",
                "name": name,
                "manifest": {
                    "description": "Test skill",
                    "capabilities": {
                        "input_modalities": [],
                        "output_modalities": [],
                        "protocols": ["mcp"],
                        "permissions": [],
                        "max_tokens": null
                    },
                    "dependencies": {},
                    "tags": ["test"],
                    "homepage": null,
                    "license": "MIT",
                    "extra": {}
                },
                "content": { "skill_md": "# Test skill" }
            }),
            Some(token),
        ))
        .await
        .unwrap();
    let status = resp.status();
    if status != StatusCode::CREATED {
        let json = body_json(resp).await;
        panic!("create skill should succeed, got {status}: {json}");
    }
    let json = body_json(resp).await;
    json["artifact"]["id"].as_str().unwrap().parse().unwrap()
}

// ── Lifecycle: register + list packages ───────────────────────────────────────

#[tokio::test]
async fn register_clawhub_package_returns_201() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "my-skill").await;

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/my-skill/packages",
            serde_json::json!({
                "source_type": "clawhub",
                "download_url": "https://hub.openclaw.io/skills/testorg/my-skill/0.1.0.zip",
                "checksum": "deadbeef",
                "metadata": { "platform": "any" }
            }),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["package"]["source_type"], "clawhub");
    assert_eq!(
        json["package"]["download_url"],
        "https://hub.openclaw.io/skills/testorg/my-skill/0.1.0.zip"
    );
}

#[tokio::test]
async fn register_github_package_returns_201() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "gh-skill").await;

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/gh-skill/packages",
            serde_json::json!({
                "source_type": "github",
                "download_url": "https://github.com/testorg/gh-skill/releases/download/v0.1.0/gh-skill-v0.1.0.zip"
            }),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["package"]["source_type"], "github");
}

#[tokio::test]
async fn register_url_package_returns_201() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "url-skill").await;

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/url-skill/packages",
            serde_json::json!({
                "source_type": "url",
                "download_url": "https://my-cdn.example.com/skills/url-skill-0.1.0.zip",
                "file_size": 1024
            }),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["package"]["source_type"], "url");
}

#[tokio::test]
async fn list_packages_returns_registered_packages() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "list-skill").await;

    // Register a package
    app.clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/list-skill/packages",
            serde_json::json!({
                "source_type": "clawhub",
                "download_url": "https://hub.openclaw.io/skills/testorg/list-skill/0.1.0.zip"
            }),
            Some(&token),
        ))
        .await
        .unwrap();

    // Now list
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/testorg/list-skill/packages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["packages"].is_array());
    assert_eq!(json["total"], 1);
    assert_eq!(json["packages"][0]["source_type"], "clawhub");
}

// ── Error cases ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn register_package_for_nonexistent_skill_returns_404() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/ghost-org/ghost-skill/packages",
            serde_json::json!({
                "source_type": "clawhub",
                "download_url": "https://example.com/skill.zip"
            }),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn register_package_without_auth_returns_401() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "auth-skill").await;

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/auth-skill/packages",
            serde_json::json!({
                "source_type": "clawhub",
                "download_url": "https://example.com/skill.zip"
            }),
            None, // no auth
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn register_package_with_invalid_source_type_returns_400() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "bad-source-skill").await;

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/bad-source-skill/packages",
            serde_json::json!({
                "source_type": "npm",  // unsupported
                "download_url": "https://example.com/skill.zip"
            }),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn install_endpoint_returns_404_when_no_packages_registered() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "no-pkg-skill").await;

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/no-pkg-skill/install",
            serde_json::json!({}),
            Some(&token),
        ))
        .await
        .unwrap();
    // No packages registered → 404
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_packages_for_nonexistent_skill_returns_404() {
    let app = common::build_test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/ghost/ghost/packages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Metadata hook verification ────────────────────────────────────────────────

#[tokio::test]
async fn publish_hook_persists_package_metadata_to_repo() {
    let (app, state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "hook-skill").await;

    let download_url = "https://hub.openclaw.io/skills/testorg/hook-skill/0.1.0.zip";

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/hook-skill/packages",
            serde_json::json!({
                "source_type": "clawhub",
                "download_url": download_url,
                "checksum": "abc123"
            }),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Verify the MetadataHook wrote to the in-memory repo
    let json = body_json(resp).await;
    let pkg_id: uuid::Uuid = json["package"]["id"].as_str().unwrap().parse().unwrap();
    let version_id: uuid::Uuid = json["package"]["artifact_version_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    use agentverse_core::skill::SourceType;
    let stored = state
        .skill_packages
        .find_by_version_and_source(version_id, &SourceType::Clawhub)
        .await
        .unwrap();

    assert!(
        stored.is_some(),
        "MetadataHook should have persisted the package"
    );
    let stored = stored.unwrap();
    assert_eq!(stored.id, pkg_id);
    assert_eq!(stored.download_url, download_url);
    assert_eq!(stored.checksum, Some("abc123".into()));
}

// ── Agent path resolution unit tests ─────────────────────────────────────────

#[tokio::test]
async fn agent_paths_cover_all_known_agents() {
    use agentverse_skills::{agent_skills_root, all_known_agents};

    for agent in all_known_agents() {
        let root = agent_skills_root(&agent);
        assert!(
            !root.to_string_lossy().is_empty(),
            "agent path for {agent} should not be empty"
        );
        println!("{agent} → {}", root.display());
    }
}

#[tokio::test]
async fn skill_install_path_contains_namespace_and_name() {
    use agentverse_core::skill::AgentKind;
    use agentverse_skills::skill_install_path;

    let path = skill_install_path(&AgentKind::OpenClaw, "myorg", "my-tool");
    let s = path.to_string_lossy();
    assert!(s.contains("myorg"), "path should contain namespace");
    assert!(s.ends_with("my-tool"), "path should end with skill name");
}

// ── Backend URL generation tests ──────────────────────────────────────────────

#[test]
fn clawhub_backend_builds_correct_url() {
    use agentverse_skills::{backends::PackageBackend, ClawhubBackend};
    let backend = ClawhubBackend::new();
    let url = backend.build_download_url("myorg", "my-skill", "1.2.3");
    assert_eq!(
        url,
        Some("https://hub.openclaw.io/skills/myorg/my-skill/1.2.3.zip".to_string())
    );
}

#[test]
fn github_backend_builds_correct_url() {
    use agentverse_skills::{backends::PackageBackend, GitHubBackend};
    let backend = GitHubBackend::default();
    let url = backend.build_download_url("myorg", "my-skill", "1.2.3");
    assert_eq!(
        url,
        Some(
            "https://github.com/myorg/my-skill/releases/download/v1.2.3/my-skill-v1.2.3.zip"
                .to_string()
        )
    );
}

#[test]
fn url_backend_returns_none_for_build_url() {
    use agentverse_skills::{backends::PackageBackend, UrlBackend};
    let backend = UrlBackend::new();
    let url = backend.build_download_url("myorg", "my-skill", "1.2.3");
    assert_eq!(
        url, None,
        "UrlBackend cannot derive URL from namespace/name/version"
    );
}

// ── GitHubRepo URL parsing ────────────────────────────────────────────────────

#[test]
fn parse_github_tree_url_anthropics_example() {
    use agentverse_skills::parse_github_tree_url;

    let info = parse_github_tree_url(
        "https://github.com/anthropics/skills/tree/main/skills/algorithmic-art",
    )
    .expect("should parse");

    assert_eq!(info.owner, "anthropics");
    assert_eq!(info.repo, "skills");
    assert_eq!(info.git_ref, "main");
    assert_eq!(info.skill_path, "skills/algorithmic-art");
}

#[test]
fn parse_github_tree_url_vercel_example() {
    use agentverse_skills::parse_github_tree_url;

    let info = parse_github_tree_url(
        "https://github.com/vercel-labs/agent-skills/tree/main/skills/deploy-to-vercel",
    )
    .expect("should parse");

    assert_eq!(info.owner, "vercel-labs");
    assert_eq!(info.repo, "agent-skills");
    assert_eq!(info.git_ref, "main");
    assert_eq!(info.skill_path, "skills/deploy-to-vercel");
}

#[test]
fn parse_github_tree_url_with_trailing_slash() {
    use agentverse_skills::parse_github_tree_url;

    let info = parse_github_tree_url("https://github.com/org/repo/tree/develop/tools/my-tool/")
        .expect("should parse trailing slash");

    assert_eq!(info.skill_path, "tools/my-tool");
}

#[test]
fn parse_github_tree_url_rejects_non_tree_url() {
    use agentverse_skills::parse_github_tree_url;
    // Release URL — not a tree URL
    assert!(parse_github_tree_url(
        "https://github.com/org/repo/releases/download/v1.0.0/skill.zip"
    )
    .is_none());
    // Blob URL
    assert!(parse_github_tree_url("https://github.com/org/repo/blob/main/README.md").is_none());
    // Non-GitHub
    assert!(parse_github_tree_url("https://gitlab.com/org/repo/tree/main/skills/x").is_none());
}

#[test]
fn github_repo_info_builds_correct_archive_url() {
    use agentverse_skills::parse_github_tree_url;

    let info = parse_github_tree_url(
        "https://github.com/anthropics/skills/tree/main/skills/algorithmic-art",
    )
    .unwrap();

    assert_eq!(
        info.archive_url(),
        "https://github.com/anthropics/skills/archive/main.zip"
    );
}

#[test]
fn github_repo_info_builds_correct_raw_url() {
    use agentverse_skills::parse_github_tree_url;

    let info = parse_github_tree_url(
        "https://github.com/anthropics/skills/tree/main/skills/algorithmic-art",
    )
    .unwrap();

    assert_eq!(
        info.raw_url("SKILL.md"),
        "https://raw.githubusercontent.com/anthropics/skills/main/skills/algorithmic-art/SKILL.md"
    );
}

#[test]
fn github_repo_info_to_metadata_json_shape() {
    use agentverse_skills::parse_github_tree_url;

    let info = parse_github_tree_url(
        "https://github.com/anthropics/skills/tree/main/skills/algorithmic-art",
    )
    .unwrap();

    let meta = info.to_metadata_json();
    assert_eq!(meta["github_repo"]["owner"], "anthropics");
    assert_eq!(meta["github_repo"]["repo"], "skills");
    assert_eq!(meta["github_repo"]["ref"], "main");
    assert_eq!(meta["github_repo"]["skill_path"], "skills/algorithmic-art");
}

#[test]
fn github_repo_backend_returns_none_for_build_download_url() {
    use agentverse_skills::{backends::PackageBackend, GitHubRepoBackend};
    let backend = GitHubRepoBackend::default();
    assert_eq!(
        backend.build_download_url("org", "skill", "1.0.0"),
        None,
        "GitHubRepoBackend cannot derive URL from namespace/name/version alone"
    );
}

// ── register_package with github_repo source_type ────────────────────────────

#[tokio::test]
async fn register_github_repo_package_converts_tree_url_to_archive_url() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "repo-skill").await;

    let tree_url = "https://github.com/anthropics/skills/tree/main/skills/algorithmic-art";

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/repo-skill/packages",
            serde_json::json!({
                "source_type": "github_repo",
                "download_url": tree_url
            }),
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;

    // download_url must be the archive URL, not the tree URL
    let stored_url = json["package"]["download_url"].as_str().unwrap();
    assert_eq!(
        stored_url,
        "https://github.com/anthropics/skills/archive/main.zip"
    );

    // metadata must contain the github_repo key
    let meta = &json["package"]["metadata"]["github_repo"];
    assert_eq!(meta["owner"], "anthropics");
    assert_eq!(meta["skill_path"], "skills/algorithmic-art");
}

#[tokio::test]
async fn register_github_repo_package_rejects_non_tree_url() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "bad-tree-skill").await;

    // Providing a release URL when source_type = github_repo should fail
    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/bad-tree-skill/packages",
            serde_json::json!({
                "source_type": "github_repo",
                "download_url": "https://github.com/org/repo/releases/download/v1.0.0/skill.zip"
            }),
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ── Import endpoint (no network, URL-parsing path) ────────────────────────────

#[tokio::test]
async fn import_skill_rejects_non_github_url() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/import",
            serde_json::json!({ "url": "https://example.com/not-a-skill" }),
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn import_skill_requires_authentication() {
    let app = common::build_test_app();

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/import",
            serde_json::json!({
                "url": "https://github.com/anthropics/skills/tree/main/skills/algorithmic-art"
            }),
            None, // no auth
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── extract_zip_subpath unit test ─────────────────────────────────────────────

#[test]
fn extract_zip_subpath_extracts_correct_files() {
    use agentverse_skills::extract_zip_subpath;
    use std::io::Write;

    // Build an in-memory zip that mimics a GitHub repo archive:
    // anthropics-skills-main/
    // anthropics-skills-main/skills/
    // anthropics-skills-main/skills/my-skill/
    // anthropics-skills-main/skills/my-skill/SKILL.md
    // anthropics-skills-main/skills/my-skill/README.md
    // anthropics-skills-main/skills/other-skill/SKILL.md  (should NOT be extracted)

    let tmp_zip = tempfile::NamedTempFile::new().expect("temp file");
    let dest_dir = tempfile::TempDir::new().expect("temp dir");

    {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(tmp_zip.path()).unwrap());
        let opts = zip::write::FileOptions::<()>::default();

        zip.add_directory("anthropics-skills-main/", opts).unwrap();
        zip.add_directory("anthropics-skills-main/skills/", opts)
            .unwrap();
        zip.add_directory("anthropics-skills-main/skills/my-skill/", opts)
            .unwrap();

        zip.start_file("anthropics-skills-main/skills/my-skill/SKILL.md", opts)
            .unwrap();
        zip.write_all(b"---\nname: my-skill\n---\n").unwrap();

        zip.start_file("anthropics-skills-main/skills/my-skill/README.md", opts)
            .unwrap();
        zip.write_all(b"# My Skill\n").unwrap();

        // This file should NOT appear in dest
        zip.add_directory("anthropics-skills-main/skills/other-skill/", opts)
            .unwrap();
        zip.start_file("anthropics-skills-main/skills/other-skill/SKILL.md", opts)
            .unwrap();
        zip.write_all(b"other\n").unwrap();

        zip.finish().unwrap();
    }

    extract_zip_subpath(tmp_zip.path(), "skills/my-skill", dest_dir.path())
        .expect("extraction should succeed");

    // Only SKILL.md and README.md should exist in dest
    assert!(dest_dir.path().join("SKILL.md").exists());
    assert!(dest_dir.path().join("README.md").exists());
    assert!(
        !dest_dir
            .path()
            .join("SKILL.md")
            .parent()
            .unwrap()
            .join("other-skill")
            .exists(),
        "other-skill directory should not be extracted"
    );
}

#[test]
fn extract_zip_subpath_errors_when_path_not_found() {
    use agentverse_skills::extract_zip_subpath;

    let tmp_zip = tempfile::NamedTempFile::new().unwrap();
    let dest_dir = tempfile::TempDir::new().unwrap();

    {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(tmp_zip.path()).unwrap());
        let opts = zip::write::FileOptions::<()>::default();
        zip.add_directory("repo-main/", opts).unwrap();
        zip.start_file("repo-main/README.md", opts).unwrap();
        zip.finish().unwrap();
    }

    let result = extract_zip_subpath(tmp_zip.path(), "skills/nonexistent", dest_dir.path());
    assert!(
        result.is_err(),
        "should error when skill_path not in archive"
    );
}
