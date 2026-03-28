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
use bytes::Bytes;
use common::{build_test_app, build_test_app_with_github_base, build_test_app_with_state};
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

/// Build a minimal valid ZIP archive in memory containing one `SKILL.md` entry.
fn make_zip_bytes() -> Bytes {
    use std::io::{Cursor, Write as _};
    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("SKILL.md", opts).unwrap();
        zip.write_all(b"---\nname: test-upload\nversion: 0.1.0\n---\n# Test\n")
            .unwrap();
        zip.finish().unwrap();
    }
    Bytes::from(buf)
}

/// Build an HTTP multipart/form-data request carrying one `file` field.
fn multipart_upload_req(uri: &str, zip_data: Bytes, token: Option<&str>) -> Request<Body> {
    const BOUNDARY: &str = "----TestBoundary7890";
    let mut body: Vec<u8> = Vec::new();
    // -- file field --
    body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"skill.zip\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: application/zip\r\n\r\n");
    body.extend_from_slice(&zip_data);
    body.extend_from_slice(b"\r\n");
    // -- end --
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());

    let mut b = Request::builder().method("POST").uri(uri).header(
        "content-type",
        format!("multipart/form-data; boundary={BOUNDARY}"),
    );
    if let Some(tok) = token {
        b = b.header("authorization", tok);
    }
    b.body(Body::from(body)).unwrap()
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

/// Helper: create a skill and register one Clawhub package; returns the package UUID string.
async fn register_one_package(app: &axum::Router, token: &str, skill_name: &str) -> String {
    create_skill(app, token, skill_name).await;
    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            &format!("/api/v1/skills/testorg/{skill_name}/packages"),
            serde_json::json!({
                "source_type": "clawhub",
                "download_url": format!("https://hub.openclaw.io/skills/testorg/{skill_name}/0.1.0.zip"),
                "checksum": "aabbccdd"
            }),
            Some(token),
        ))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "register_one_package failed"
    );
    let json = body_json(resp).await;
    json["package"]["id"].as_str().unwrap().to_string()
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

// ── parse_skill_md full frontmatter tests ────────────────────────────────────

#[cfg(test)]
mod parse_skill_md_tests {
    use agentverse_skills::parse_skill_md;

    /// The standard simple format: scalar fields + inline tag list.
    const SIMPLE_SKILL: &str = "---\n\
        name: ripgrep-search\n\
        description: Blazing-fast code search using ripgrep.\n\
        version: \"0.1.0\"\n\
        tags: [search, code, cli, productivity]\n\
        license: MIT\n\
        ---\n\
        # Ripgrep Search";

    /// Advanced format: metadata.openclaw block, no top-level tags.
    ///
    /// NOTE: We use `concat!` here instead of `\n\` line-continuation because
    /// line-continuation strips ALL leading whitespace from the next source
    /// line, destroying the YAML indentation that encodes parent-child
    /// relationships (e.g. `metadata.openclaw` would be flattened to two
    /// sibling top-level keys).
    const OPENCLAW_SKILL: &str = concat!(
        "---\n",
        "name: agentverse-cli\n",
        "description: \"Manage AI skills from the command line.\"\n",
        "version: 0.1.4\n",
        "metadata:\n",
        "  openclaw:\n",
        "    homepage: https://github.com/loonghao/agentverse\n",
        "    requires:\n",
        "      bins:\n",
        "        - agentverse\n",
        "      env:\n",
        "        - AGENTVERSE_TOKEN\n",
        "    install:\n",
        "      - kind: shell\n",
        "        linux: \"curl -fsSL https://example.com/install.sh | bash\"\n",
        "        windows: \"irm https://example.com/install.ps1 | iex\"\n",
        "---\n",
        "# AgentVerse CLI",
    );

    /// Skill with multiline tag list (block sequence).
    const MULTILINE_TAGS_SKILL: &str = "---\n\
        name: jq-processor\n\
        description: Transform JSON with jq.\n\
        tags:\n\
          - json\n\
          - data\n\
          - cli\n\
        ---";

    /// No frontmatter at all — should fall back to the provided name.
    const NO_FRONTMATTER: &str = "# Just a skill\nNo frontmatter here.";

    /// Frontmatter missing the name field — should use fallback.
    const MISSING_NAME: &str = "---\ndescription: A skill without a name.\ntags: [orphan]\n---";

    #[test]
    fn simple_skill_all_fields_extracted() {
        let p = parse_skill_md(SIMPLE_SKILL, "fallback");
        assert_eq!(p.name, "ripgrep-search");
        assert_eq!(
            p.description.as_deref(),
            Some("Blazing-fast code search using ripgrep.")
        );
        assert_eq!(p.version.as_deref(), Some("0.1.0"));
        assert_eq!(p.tags, ["search", "code", "cli", "productivity"]);
        assert_eq!(p.license.as_deref(), Some("MIT"));
        assert!(p.metadata.is_null(), "no metadata block expected");
    }

    #[test]
    fn openclaw_metadata_block_parsed_as_json() {
        let p = parse_skill_md(OPENCLAW_SKILL, "fallback");
        assert_eq!(p.name, "agentverse-cli");
        assert_eq!(p.version.as_deref(), Some("0.1.4"));
        // homepage promoted from metadata.openclaw.homepage
        assert_eq!(
            p.homepage.as_deref(),
            Some("https://github.com/loonghao/agentverse")
        );
        // metadata block is a JSON object
        let meta = &p.metadata;
        assert!(meta.is_object(), "metadata must be a JSON object");
        // openclaw.requires.bins contains "agentverse"
        assert_eq!(meta["openclaw"]["requires"]["bins"][0], "agentverse");
        // openclaw.requires.env contains "AGENTVERSE_TOKEN"
        assert_eq!(meta["openclaw"]["requires"]["env"][0], "AGENTVERSE_TOKEN");
        // install instructions present
        assert!(meta["openclaw"]["install"].is_array());
    }

    #[test]
    fn multiline_tags_parsed_correctly() {
        let p = parse_skill_md(MULTILINE_TAGS_SKILL, "fallback");
        assert_eq!(p.name, "jq-processor");
        assert_eq!(p.tags, ["json", "data", "cli"]);
    }

    #[test]
    fn no_frontmatter_uses_fallback_name() {
        let p = parse_skill_md(NO_FRONTMATTER, "my-fallback");
        assert_eq!(p.name, "my-fallback");
        assert!(p.description.is_none());
        assert!(p.tags.is_empty());
        assert!(p.metadata.is_null());
    }

    #[test]
    fn missing_name_uses_fallback() {
        let p = parse_skill_md(MISSING_NAME, "path-fallback");
        assert_eq!(p.name, "path-fallback");
        assert_eq!(p.description.as_deref(), Some("A skill without a name."));
        assert_eq!(p.tags, ["orphan"]);
    }

    #[test]
    fn unquoted_version_parsed_as_string() {
        // version: 0.1.4 (no quotes) should still produce "0.1.4"
        let p = parse_skill_md(OPENCLAW_SKILL, "fallback");
        assert_eq!(p.version.as_deref(), Some("0.1.4"));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Upload endpoint: POST /api/v1/skills/:ns/:name/upload
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn upload_zip_returns_201_with_internal_source_type() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "upload-skill").await;

    let resp = app
        .oneshot(multipart_upload_req(
            "/api/v1/skills/testorg/upload-skill/upload",
            make_zip_bytes(),
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["package"]["source_type"], "internal");
    assert!(
        json["download_url"].as_str().is_some(),
        "response must include download_url"
    );
    assert!(
        !json["download_url"].as_str().unwrap().is_empty(),
        "download_url must not be empty"
    );
}

#[tokio::test]
async fn upload_zip_persists_package_to_repo() {
    use agentverse_core::skill::SourceType;

    let (app, state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "persist-skill").await;

    let resp = app
        .oneshot(multipart_upload_req(
            "/api/v1/skills/testorg/persist-skill/upload",
            make_zip_bytes(),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let json = body_json(resp).await;
    let version_id: uuid::Uuid = json["package"]["artifact_version_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    // The MetadataHook should have written the record to the in-memory repo.
    let stored = state
        .skill_packages
        .find_by_version_and_source(version_id, &SourceType::Internal)
        .await
        .unwrap();
    assert!(
        stored.is_some(),
        "package should be persisted via MetadataHook"
    );
    let stored = stored.unwrap();
    assert!(
        stored.checksum.is_some(),
        "checksum (SHA-256) must be stored"
    );
    assert!(stored.file_size.unwrap() > 0, "file_size must be positive");
}

#[tokio::test]
async fn upload_without_auth_returns_401() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "noauth-skill").await;

    // Second request: no Authorization header.
    let resp = app
        .oneshot(multipart_upload_req(
            "/api/v1/skills/testorg/noauth-skill/upload",
            make_zip_bytes(),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn upload_to_nonexistent_skill_returns_404() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;

    let resp = app
        .oneshot(multipart_upload_req(
            "/api/v1/skills/ghost-org/ghost-skill/upload",
            make_zip_bytes(),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn upload_invalid_zip_bytes_returns_400() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "bad-zip-skill").await;

    let resp = app
        .oneshot(multipart_upload_req(
            "/api/v1/skills/testorg/bad-zip-skill/upload",
            Bytes::from_static(b"this is not a zip file"),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    let msg = json["error"]["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("invalid zip"),
        "error message should mention 'invalid zip', got: {msg}"
    );
}

#[tokio::test]
async fn upload_by_non_owner_returns_403() {
    let (app, _state) = build_test_app_with_state();

    // First user creates the skill.
    let (_uid, owner_token) = register_and_login(&app).await;
    create_skill(&app, &owner_token, "owned-skill").await;

    // Second user tries to upload.
    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/auth/register",
            serde_json::json!({
                "username": "intruder",
                "password": "password1234",
                "email": "intruder@example.com"
            }),
            None,
        ))
        .await
        .unwrap();
    let json = body_json(resp).await;
    let intruder_token = format!("Bearer {}", json["access_token"].as_str().unwrap());

    let resp = app
        .oneshot(multipart_upload_req(
            "/api/v1/skills/testorg/owned-skill/upload",
            make_zip_bytes(),
            Some(&intruder_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ─────────────────────────────────────────────────────────────────────────────
// Local file serving: GET /files/{key}
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn serve_local_file_returns_200_after_upload() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "serve-skill").await;

    // Upload a zip so the local backend has a file on disk.
    let upload_resp = app
        .clone()
        .oneshot(multipart_upload_req(
            "/api/v1/skills/testorg/serve-skill/upload",
            make_zip_bytes(),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(upload_resp.status(), StatusCode::CREATED);

    // Extract the relative path from the download_url.
    // Local backend returns "http://localhost:8080/files/<key>".
    let json = body_json(upload_resp).await;
    let download_url = json["download_url"].as_str().unwrap();
    let key = download_url
        .split("/files/")
        .nth(1)
        .expect("download_url should contain /files/");

    // Serve the file back via GET /files/{key}.
    let serve_resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/files/{key}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(serve_resp.status(), StatusCode::OK);
    let content_type = serve_resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(content_type, "application/zip");

    // The body should be a valid ZIP.
    let body_bytes = serve_resp.into_body().collect().await.unwrap().to_bytes();
    assert!(!body_bytes.is_empty());
    zip::ZipArchive::new(std::io::Cursor::new(body_bytes))
        .expect("served file must be a valid zip");
}

#[tokio::test]
async fn serve_missing_file_returns_404() {
    let app = build_test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/files/does/not/exist.zip")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Import endpoint — mock GitHub raw-content server ─────────────────────────
//
// These tests spin up a tiny local Axum server that serves fake SKILL.md
// content at any path.  `AppState.github_raw_base_url` redirects the import
// handler to this server instead of `raw.githubusercontent.com`, so no real
// network calls are made.

/// Spawn a minimal HTTP server that returns `status` and `body` for every GET.
///
/// Returns `(base_url, join_handle)`.  Keep the handle alive for the test
/// duration (drop ⇒ server shuts down).
async fn start_mock_raw_server(status: u16, body: String) -> (String, tokio::task::JoinHandle<()>) {
    use axum::{
        body::Body as AxumBody, http::StatusCode as HStatus, response::Response, routing::any,
        Router,
    };
    use tokio::net::TcpListener;

    let app = Router::new().fallback(any(move || {
        let body = body.clone();
        async move {
            Response::builder()
                .status(HStatus::from_u16(status).unwrap())
                .header("content-type", "text/plain; charset=utf-8")
                .body(AxumBody::from(body))
                .unwrap()
        }
    }));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://127.0.0.1:{port}"), handle)
}

/// Minimal valid SKILL.md with all fields exercised by the import handler.
const MOCK_SKILL_MD: &str = "\
---
name: mock-ripgrep
description: \"Fast regex search powered by ripgrep\"
version: \"1.3.0\"
tags: [search, cli, regex]
license: MIT
metadata:
  openclaw:
    homepage: https://example.com/mock-ripgrep
    emoji: \"🔍\"
---
# Mock Ripgrep

A skill used only in integration tests.
";

#[tokio::test]
async fn import_skill_happy_path_with_mock_github() {
    let (base_url, _server) = start_mock_raw_server(200, MOCK_SKILL_MD.to_string()).await;
    let (app, _state) = build_test_app_with_github_base(base_url);
    let (_uid, token) = register_and_login(&app).await;

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/import",
            serde_json::json!({
                "url": "https://github.com/testorg/testrepo/tree/main/skills/mock-ripgrep",
                "namespace": "testorg"
            }),
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;

    // Skill metadata extracted from SKILL.md frontmatter.
    assert_eq!(json["skill"]["name"].as_str().unwrap(), "mock-ripgrep");
    assert_eq!(json["skill"]["namespace"].as_str().unwrap(), "testorg");
    assert_eq!(json["skill"]["version"].as_str().unwrap(), "1.3.0");
    assert_eq!(json["skill"]["license"].as_str().unwrap(), "MIT");
    assert!(
        json["created"].as_bool().unwrap(),
        "first import should set created=true"
    );

    // Tags must have been persisted.
    let tags: Vec<&str> = json["skill"]["tags"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t.as_str())
        .collect();
    assert!(
        tags.contains(&"search"),
        "tags should contain 'search': {tags:?}"
    );
    assert!(tags.contains(&"cli"), "tags should contain 'cli': {tags:?}");

    // Package points to the repo archive zip, not to the raw SKILL.md URL.
    let pkg = &json["package"];
    assert_eq!(pkg["source_type"].as_str().unwrap(), "github_repo");
    assert!(
        pkg["download_url"]
            .as_str()
            .unwrap()
            .ends_with("archive/main.zip"),
        "download_url should be the repo archive zip, got: {}",
        pkg["download_url"]
    );
}

#[tokio::test]
async fn import_skill_idempotent_returns_existing_on_second_call() {
    let (base_url, _server) = start_mock_raw_server(200, MOCK_SKILL_MD.to_string()).await;
    let (app, _state) = build_test_app_with_github_base(base_url);
    let (_uid, token) = register_and_login(&app).await;

    let req_body = serde_json::json!({
        "url": "https://github.com/testorg/testrepo/tree/main/skills/mock-ripgrep",
        "namespace": "testorg"
    });

    // First import: creates the skill.
    let resp1 = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/import",
            req_body.clone(),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp1.status(), StatusCode::CREATED);
    let json1 = body_json(resp1).await;
    assert!(json1["created"].as_bool().unwrap());

    // Second import: returns 200 with the existing skill.
    let resp2 = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/import",
            req_body,
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let json2 = body_json(resp2).await;
    assert!(!json2["created"].as_bool().unwrap());
    assert_eq!(
        json1["skill"]["name"].as_str().unwrap(),
        json2["skill"]["name"].as_str().unwrap()
    );
}

#[tokio::test]
async fn import_skill_github_404_returns_bad_request() {
    // Mock server returns 404 — simulates a missing branch or deleted SKILL.md.
    let (base_url, _server) = start_mock_raw_server(404, "Not Found".to_string()).await;
    let (app, _state) = build_test_app_with_github_base(base_url);
    let (_uid, token) = register_and_login(&app).await;

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/import",
            serde_json::json!({
                "url": "https://github.com/testorg/testrepo/tree/deleted-branch/skills/gone-skill"
            }),
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    let msg = json["error"]["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("SKILL.md") || msg.contains("404"),
        "error message should mention SKILL.md or 404, got: {msg}"
    );
}

#[tokio::test]
async fn import_skill_namespace_defaults_to_repo_owner() {
    let (base_url, _server) = start_mock_raw_server(200, MOCK_SKILL_MD.to_string()).await;
    let (app, _state) = build_test_app_with_github_base(base_url);
    let (_uid, token) = register_and_login(&app).await;

    // No "namespace" field in the request body.
    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/import",
            serde_json::json!({
                "url": "https://github.com/theowner/somerepo/tree/main/skills/mock-ripgrep"
            }),
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(
        json["skill"]["namespace"].as_str().unwrap(),
        "theowner",
        "namespace should default to the repo owner"
    );
}

#[tokio::test]
async fn import_skill_description_is_populated() {
    let (base_url, _server) = start_mock_raw_server(200, MOCK_SKILL_MD.to_string()).await;
    let (app, _state) = build_test_app_with_github_base(base_url);
    let (_uid, token) = register_and_login(&app).await;

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/import",
            serde_json::json!({
                "url": "https://github.com/testorg/testrepo/tree/main/skills/mock-ripgrep",
                "namespace": "testorg"
            }),
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    let desc = json["skill"]["description"].as_str().unwrap_or("");
    assert!(
        desc.contains("ripgrep"),
        "description should contain 'ripgrep', got: {desc}"
    );
}

// ── Package CRUD + install record routes ─────────────────────────────────────

#[tokio::test]
async fn get_package_by_id_returns_200() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    let pkg_id = register_one_package(&app, &token, "pkg-get-skill").await;

    let resp = app
        .oneshot(json_req(
            "GET",
            &format!("/api/v1/skills/testorg/pkg-get-skill/packages/{pkg_id}"),
            serde_json::Value::Null,
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["package"]["id"].as_str().unwrap(), pkg_id);
    assert_eq!(json["package"]["source_type"].as_str().unwrap(), "clawhub");
}

#[tokio::test]
async fn get_package_unknown_id_returns_404() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "get-404-skill").await;

    let fake_id = uuid::Uuid::new_v4();
    let resp = app
        .oneshot(json_req(
            "GET",
            &format!("/api/v1/skills/testorg/get-404-skill/packages/{fake_id}"),
            serde_json::Value::Null,
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_package_removes_it_and_subsequent_get_returns_404() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    let pkg_id = register_one_package(&app, &token, "del-pkg-skill").await;

    // Delete the package.
    let del_resp = app
        .clone()
        .oneshot(json_req(
            "DELETE",
            &format!("/api/v1/skills/testorg/del-pkg-skill/packages/{pkg_id}"),
            serde_json::Value::Null,
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::OK);

    // Get should now return 404.
    let get_resp = app
        .oneshot(json_req(
            "GET",
            &format!("/api/v1/skills/testorg/del-pkg-skill/packages/{pkg_id}"),
            serde_json::Value::Null,
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_packages_for_version_returns_registered_package() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    let pkg_id = register_one_package(&app, &token, "ver-pkg-skill").await;

    // The default version created by create_skill is "0.1.0".
    let resp = app
        .oneshot(json_req(
            "GET",
            "/api/v1/skills/testorg/ver-pkg-skill/versions/0.1.0/packages",
            serde_json::Value::Null,
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let packages = json["packages"].as_array().unwrap();
    assert!(
        !packages.is_empty(),
        "should return at least one package for version 0.1.0"
    );
    assert!(
        packages
            .iter()
            .any(|p| p["id"].as_str().unwrap_or("") == pkg_id),
        "registered package {pkg_id} should appear in version package list"
    );
}

#[tokio::test]
async fn list_packages_for_unknown_version_returns_404() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "empty-ver-skill").await;

    // Version 99.0.0 was never published → handler returns 404.
    let resp = app
        .oneshot(json_req(
            "GET",
            "/api/v1/skills/testorg/empty-ver-skill/versions/99.0.0/packages",
            serde_json::Value::Null,
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_installs_returns_empty_list_for_new_skill() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "install-list-skill").await;

    let resp = app
        .oneshot(json_req(
            "GET",
            "/api/v1/skills/testorg/install-list-skill/installs",
            serde_json::Value::Null,
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let empty = vec![];
    let installs = json["installs"].as_array().unwrap_or(&empty);
    assert!(
        installs.is_empty(),
        "no installs recorded yet, list should be empty"
    );
}

#[tokio::test]
async fn list_skills_for_agent_returns_empty_when_no_installs() {
    let (app, _state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;

    let resp = app
        .oneshot(json_req(
            "GET",
            "/api/v1/skills/agents/claude",
            serde_json::Value::Null,
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let empty = vec![];
    let skills = json.as_array().unwrap_or(&empty);
    assert!(skills.is_empty(), "no installs yet — list should be empty");
}

#[tokio::test]
async fn list_skills_for_agent_anonymous_read_allowed() {
    let (app, _state) = build_test_app_with_state();

    // The test app has `anonymous_read: true`, so an unauthenticated GET
    // returns 200 with an empty list rather than 401.
    let resp = app
        .oneshot(json_req(
            "GET",
            "/api/v1/skills/agents/augment",
            serde_json::Value::Null,
            None,
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}
