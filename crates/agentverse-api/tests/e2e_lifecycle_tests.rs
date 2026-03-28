//! End-to-end lifecycle tests covering the full agentverse product surface.
//!
//! Tests are organised into logical sections:
//!
//! 1.  Artifact CRUD (create, get, list, update, deprecate)
//! 2.  Version management (publish, list, get-by-semver)
//! 3.  Social — likes (add, list, remove)
//! 4.  Social — comments (add, list, update, delete, threaded replies)
//! 5.  Social — ratings (add, list, aggregate stats)
//! 6.  Social — tags (add, remove)
//! 7.  Social — fork
//! 8.  Social — agent interactions (learn, benchmark)
//! 9.  Aggregate stats
//! 10. Full skill lifecycle (create → package → import)
//! 11. Trending endpoint
//! 12. Package CRUD (get by id, delete)
//! 13. Version-specific packages
//! 14. Skill install records
//! 15. Agent skill inventory

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::{build_test_app, build_test_app_with_state, TEST_JWT_SECRET};
use tower::ServiceExt;

// ── Test helpers ──────────────────────────────────────────────────────────────

fn json_req(method: &str, uri: &str, body: serde_json::Value, token: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(t) = token {
        b = b.header("authorization", format!("Bearer {t}"));
    }
    b.body(Body::from(serde_json::to_string(&body).unwrap())).unwrap()
}

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    use http_body_util::BodyExt;
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

/// Register a test user and return (user_id, jwt_token).
async fn register_and_login(app: &axum::Router) -> (uuid::Uuid, String) {
    use agentverse_auth::JwtManager;
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let username = format!("user_{n}");

    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/auth/register",
            serde_json::json!({
                "username": username,
                "email": format!("{username}@test.example"),
                "password": "Password1!",
                "kind": "human",
                "capabilities": { "compute": "cpu", "storage": "local", "network": "http" }
            }),
            None,
        ))
        .await
        .unwrap();

    let json = body_json(resp).await;
    let uid: uuid::Uuid = json["user"]["id"].as_str().unwrap().parse().unwrap();
    let jwt = JwtManager::new(TEST_JWT_SECRET, 3600);
    let token = jwt.generate(uid, &username, "human").unwrap();
    (uid, token)
}

/// Create a skill artifact and return its JSON.
async fn create_skill(app: &axum::Router, token: &str, name: &str) -> serde_json::Value {
    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills",
            serde_json::json!({
                "namespace": "testorg",
                "name": name,
                "manifest": {
                    "description": format!("Test skill {name}"),
                    "capabilities": {
                        "input_modalities": ["text"],
                        "output_modalities": ["text"],
                        "protocols": ["mcp"],
                        "permissions": [],
                        "max_tokens": null
                    },
                    "dependencies": {},
                    "tags": ["test"],
                    "license": "MIT",
                    "extra": {}
                },
                "content": {}
            }),
            Some(token),
        ))
        .await
        .unwrap();
    body_json(resp).await
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. Artifact CRUD
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn create_skill_returns_201_with_artifact() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;

    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills",
            serde_json::json!({
                "namespace": "testorg",
                "name": "my-skill",
                "manifest": {
                    "description": "My cool skill",
                    "capabilities": {
                        "input_modalities": ["text"],
                        "output_modalities": ["text"],
                        "protocols": ["mcp"],
                        "permissions": [],
                        "max_tokens": null
                    },
                    "dependencies": {},
                    "tags": [],
                    "extra": {}
                },
                "content": {}
            }),
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["artifact"]["namespace"], "testorg");
    assert_eq!(json["artifact"]["name"], "my-skill");
    assert!(json["version"]["version"].as_str().is_some());
}

#[tokio::test]
async fn create_skill_without_auth_returns_401() {
    let app = build_test_app();
    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills",
            serde_json::json!({
                "namespace": "testorg",
                "name": "unauth-skill",
                "manifest": {
                    "description": "x",
                    "capabilities": {
                        "input_modalities": [],
                        "output_modalities": [],
                        "protocols": ["mcp"],
                        "permissions": [],
                        "max_tokens": null
                    },
                    "dependencies": {},
                    "tags": [],
                    "extra": {}
                },
                "content": {}
            }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_skill_by_namespace_name_returns_200() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "get-skill").await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/testorg/get-skill")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["artifact"]["name"], "get-skill");
}

#[tokio::test]
async fn get_nonexistent_skill_returns_404() {
    let app = build_test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/ghost/ghost-skill")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_skills_includes_created_artifact() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "list-me").await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    // list_artifacts returns { "items": [...], "total": N }
    let items = json["items"].as_array().unwrap();
    assert!(
        items.iter().any(|a| a["name"] == "list-me"),
        "created skill should appear in list"
    );
}

#[tokio::test]
async fn deprecate_skill_returns_200() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "deprecate-me").await;

    let resp = app
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/deprecate-me/deprecate",
            serde_json::json!({}),
            Some(&token),
        ))
        .await
        .unwrap();

    // 200 or 204 depending on impl
    assert!(
        resp.status().is_success(),
        "deprecate should succeed, got {}",
        resp.status()
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. Version management
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn create_skill_publishes_initial_version() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    let json = create_skill(&app, &token, "version-skill").await;
    let version = json["version"]["version"].as_str().unwrap_or("");
    assert!(!version.is_empty(), "initial version should be set");
}

#[tokio::test]
async fn list_versions_returns_published_version() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "ver-list-skill").await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/testorg/ver-list-skill/versions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let versions = json["versions"].as_array().unwrap();
    assert!(!versions.is_empty(), "should have at least one version");
}

#[tokio::test]
async fn publish_new_version_increments_version_list() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "bump-skill").await;

    // Publish a minor bump via the /publish endpoint
    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/bump-skill/publish",
            serde_json::json!({
                "bump": "minor",
                "content": {},
                "changelog": "Added new capability"
            }),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // List → expect 2 versions
    let list_resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/testorg/bump-skill/versions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(list_resp).await;
    let versions = json["versions"].as_array().unwrap();
    assert!(versions.len() >= 2, "expected at least 2 versions after bump");
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. Social — Likes
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn like_then_list_shows_one_like() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "like-skill").await;

    // Add like
    let resp = app
        .clone()
        .oneshot(json_req("POST", "/api/v1/skills/testorg/like-skill/likes", serde_json::json!({}), Some(&token)))
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // List likes
    let resp = app
        .oneshot(Request::builder().uri("/api/v1/skills/testorg/like-skill/likes").body(Body::empty()).unwrap())
        .await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["total"], 1, "should have 1 like");
}

#[tokio::test]
async fn unlike_removes_like() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "unlike-skill").await;

    // Like then unlike
    app.clone().oneshot(json_req("POST", "/api/v1/skills/testorg/unlike-skill/likes", serde_json::json!({}), Some(&token))).await.unwrap();
    let resp = app
        .clone()
        .oneshot(Request::builder().method("DELETE").uri("/api/v1/skills/testorg/unlike-skill/likes")
            .header("authorization", format!("Bearer {token}")).body(Body::empty()).unwrap())
        .await.unwrap();
    assert!(resp.status().is_success());

    // Verify 0 likes
    let resp = app.oneshot(Request::builder().uri("/api/v1/skills/testorg/unlike-skill/likes").body(Body::empty()).unwrap()).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["total"], 0, "like should be removed");
}

// ═════════════════════════════════════════════════════════════════════════════
// 4. Social — Comments
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn add_and_list_comment() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "comment-skill").await;

    let resp = app
        .clone()
        .oneshot(json_req("POST", "/api/v1/skills/testorg/comment-skill/comments",
            serde_json::json!({ "content": "Great skill!", "kind": "review" }), Some(&token)))
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created = body_json(resp).await;
    assert_eq!(created["comment"]["content"], "Great skill!");

    // List
    let resp = app.oneshot(Request::builder().uri("/api/v1/skills/testorg/comment-skill/comments")
        .body(Body::empty()).unwrap()).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["total"], 1);
}

#[tokio::test]
async fn update_comment_changes_content() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "edit-comment-skill").await;

    let resp = app
        .clone()
        .oneshot(json_req("POST", "/api/v1/skills/testorg/edit-comment-skill/comments",
            serde_json::json!({ "content": "original", "kind": "review" }), Some(&token)))
        .await.unwrap();
    let comment_id = body_json(resp).await["comment"]["id"].as_str().unwrap().to_owned();

    let update_resp = app
        .clone()
        .oneshot(json_req("PUT",
            &format!("/api/v1/skills/testorg/edit-comment-skill/comments/{comment_id}"),
            serde_json::json!({ "content": "updated content" }), Some(&token)))
        .await.unwrap();
    assert!(update_resp.status().is_success());
    let json = body_json(update_resp).await;
    assert_eq!(json["comment"]["content"], "updated content");
}

#[tokio::test]
async fn delete_comment_removes_it_from_list() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "delete-comment-skill").await;

    let resp = app
        .clone()
        .oneshot(json_req("POST", "/api/v1/skills/testorg/delete-comment-skill/comments",
            serde_json::json!({ "content": "to be deleted", "kind": "review" }), Some(&token)))
        .await.unwrap();
    let comment_id = body_json(resp).await["comment"]["id"].as_str().unwrap().to_owned();

    app.clone()
        .oneshot(Request::builder().method("DELETE")
            .uri(&format!("/api/v1/skills/testorg/delete-comment-skill/comments/{comment_id}"))
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty()).unwrap())
        .await.unwrap();

    let resp = app.oneshot(Request::builder().uri("/api/v1/skills/testorg/delete-comment-skill/comments")
        .body(Body::empty()).unwrap()).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["total"], 0, "comment should be deleted");
}

#[tokio::test]
async fn threaded_reply_has_parent_id_set() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "thread-skill").await;

    // Post parent comment
    let resp = app.clone().oneshot(json_req("POST", "/api/v1/skills/testorg/thread-skill/comments",
        serde_json::json!({ "content": "parent", "kind": "review" }), Some(&token))).await.unwrap();
    let parent_id = body_json(resp).await["comment"]["id"].as_str().unwrap().to_owned();

    // Reply
    let resp = app.clone().oneshot(json_req("POST", "/api/v1/skills/testorg/thread-skill/comments",
        serde_json::json!({ "content": "reply", "kind": "review", "parent_id": parent_id }),
        Some(&token))).await.unwrap();
    let reply = body_json(resp).await;
    assert_eq!(reply["comment"]["parent_id"].as_str().unwrap(), parent_id);
}

// ═════════════════════════════════════════════════════════════════════════════
// 5. Social — Ratings
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn add_rating_appears_in_list_and_stats() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "rate-skill").await;

    let resp = app.clone().oneshot(json_req("POST", "/api/v1/skills/testorg/rate-skill/ratings",
        serde_json::json!({ "score": 5, "review_text": "Excellent!" }), Some(&token)))
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Stats should reflect the rating
    let stats = app.oneshot(Request::builder().uri("/api/v1/skills/testorg/rate-skill/stats")
        .body(Body::empty()).unwrap()).await.unwrap();
    let json = body_json(stats).await;
    assert_eq!(json["ratings_count"], 1);
    assert_eq!(json["avg_rating"], 5.0);
}

#[tokio::test]
async fn rating_out_of_range_returns_400() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "bad-rate-skill").await;

    let resp = app.oneshot(json_req("POST", "/api/v1/skills/testorg/bad-rate-skill/ratings",
        serde_json::json!({ "score": 6 }), Some(&token))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ═════════════════════════════════════════════════════════════════════════════
// 6. Social — Tags
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn add_and_remove_tag() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "tag-skill").await;

    let resp = app.clone().oneshot(json_req("POST", "/api/v1/skills/testorg/tag-skill/tags",
        serde_json::json!({ "tag": "ai" }), Some(&token))).await.unwrap();
    assert!(resp.status().is_success(), "add tag should succeed, got {}", resp.status());

    let resp = app.oneshot(Request::builder().method("DELETE")
        .uri("/api/v1/skills/testorg/tag-skill/tags/ai")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty()).unwrap()).await.unwrap();
    assert!(resp.status().is_success(), "remove tag should succeed");
}

// ═════════════════════════════════════════════════════════════════════════════
// 7. Social — Fork
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn fork_creates_new_artifact_with_new_name() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "original-skill").await;

    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/original-skill/fork",
            // Explicitly supply new_namespace so the forked artifact lands in "testorg",
            // not the caller's username.
            serde_json::json!({ "new_name": "forked-skill", "new_namespace": "testorg" }),
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["artifact"]["name"], "forked-skill");
    assert_eq!(json["artifact"]["namespace"], "testorg");
}

// ═════════════════════════════════════════════════════════════════════════════
// 8. Social — Agent interactions (learn + benchmark)
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn record_learning_creates_comment_and_interaction() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "learn-skill").await;

    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/learn-skill/learn",
            serde_json::json!({
                "content": "This skill taught me about algorithmic art patterns",
                "confidence_score": 0.87
            }),
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);

    // Verify interaction was recorded
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/testorg/learn-skill/interactions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["total"], 1);
    assert_eq!(json["interactions"][0]["kind"], "learn");
}

#[tokio::test]
async fn record_benchmark_creates_interaction_with_metrics() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "bench-skill").await;

    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/bench-skill/benchmark",
            serde_json::json!({
                "metrics": { "latency_ms": 42, "tokens": 1200, "cost_usd": 0.002 },
                "confidence_score": 0.95
            }),
            Some(&token),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/testorg/bench-skill/interactions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["interactions"][0]["kind"], "benchmark");
    assert_eq!(json["interactions"][0]["payload"]["latency_ms"], 42);
}

// ═════════════════════════════════════════════════════════════════════════════
// 9. Aggregate stats
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn stats_reflects_all_social_actions() {
    let (app, _state) = build_test_app_with_state();
    let (_uid1, token1) = register_and_login(&app).await;
    let (_uid2, token2) = register_and_login(&app).await;
    create_skill(&app, &token1, "stats-skill").await;

    // Like (user 1)
    app.clone().oneshot(json_req("POST", "/api/v1/skills/testorg/stats-skill/likes",
        serde_json::json!({}), Some(&token1))).await.unwrap();
    // Like (user 2)
    app.clone().oneshot(json_req("POST", "/api/v1/skills/testorg/stats-skill/likes",
        serde_json::json!({}), Some(&token2))).await.unwrap();
    // Comment
    app.clone().oneshot(json_req("POST", "/api/v1/skills/testorg/stats-skill/comments",
        serde_json::json!({ "content": "nice", "kind": "review" }), Some(&token1))).await.unwrap();
    // Rating 4
    app.clone().oneshot(json_req("POST", "/api/v1/skills/testorg/stats-skill/ratings",
        serde_json::json!({ "score": 4 }), Some(&token1))).await.unwrap();
    // Rating 2
    app.clone().oneshot(json_req("POST", "/api/v1/skills/testorg/stats-skill/ratings",
        serde_json::json!({ "score": 2 }), Some(&token2))).await.unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/testorg/stats-skill/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["likes_count"], 2, "should have 2 likes");
    assert_eq!(json["comments_count"], 1);
    assert_eq!(json["ratings_count"], 2);
    assert_eq!(json["avg_rating"], 3.0, "avg of 4 and 2 = 3");
}

// ═════════════════════════════════════════════════════════════════════════════
// 10. Full skill lifecycle — create → package (clawhub + github_repo) → import
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn full_skill_lifecycle_clawhub_and_github_repo_packages() {
    let (app, state) = build_test_app_with_state();
    let (_uid, token) = register_and_login(&app).await;

    // 1. Create skill
    let skill_json = create_skill(&app, &token, "lifecycle-skill").await;
    let artifact_id = skill_json["artifact"]["id"].as_str().unwrap();
    assert!(!artifact_id.is_empty());

    // 2. Register Clawhub package
    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/lifecycle-skill/packages",
            serde_json::json!({
                "source_type": "clawhub",
                "download_url": "https://hub.openclaw.io/skills/testorg/lifecycle-skill/0.1.0.zip",
                "checksum": "deadbeef"
            }),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 3. Register GitHub repo package (tree URL auto-converts)
    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/lifecycle-skill/packages",
            serde_json::json!({
                "source_type": "github_repo",
                "download_url":
                    "https://github.com/testorg/skills/tree/main/skills/lifecycle-skill"
            }),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let pkg_json = body_json(resp).await;
    // Archive URL, not tree URL
    assert_eq!(
        pkg_json["package"]["download_url"],
        "https://github.com/testorg/skills/archive/main.zip"
    );
    assert_eq!(pkg_json["package"]["metadata"]["github_repo"]["owner"], "testorg");

    // 4. List packages → 2 packages
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/testorg/lifecycle-skill/packages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["total"], 2, "should have 2 registered packages");

    // 5. Import endpoint rejects non-GitHub URL
    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/import",
            serde_json::json!({ "url": "https://example.com/not-a-skill" }),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 6. Verify state: both packages persisted
    let version_id = skill_json["version"]["id"].as_str().unwrap().parse::<uuid::Uuid>().unwrap();
    let pkgs = state.skill_packages.list_for_version(version_id).await.unwrap();
    assert_eq!(pkgs.len(), 2, "both packages should be in the repo");
    let source_types: Vec<String> = pkgs.iter().map(|p| p.source_type.to_string()).collect();
    assert!(source_types.contains(&"clawhub".to_string()));
    assert!(source_types.contains(&"github_repo".to_string()));
}

#[tokio::test]
async fn multi_user_social_isolation() {
    let app = build_test_app();
    let (_uid1, token1) = register_and_login(&app).await;
    let (_uid2, token2) = register_and_login(&app).await;
    create_skill(&app, &token1, "shared-skill").await;

    // User 1 likes
    app.clone().oneshot(json_req("POST", "/api/v1/skills/testorg/shared-skill/likes",
        serde_json::json!({}), Some(&token1))).await.unwrap();

    // User 2 likes
    app.clone().oneshot(json_req("POST", "/api/v1/skills/testorg/shared-skill/likes",
        serde_json::json!({}), Some(&token2))).await.unwrap();

    // User 1 unlikes — should only remove user1's like
    app.clone()
        .oneshot(Request::builder().method("DELETE").uri("/api/v1/skills/testorg/shared-skill/likes")
            .header("authorization", format!("Bearer {token1}")).body(Body::empty()).unwrap())
        .await.unwrap();

    let resp = app.oneshot(Request::builder()
        .uri("/api/v1/skills/testorg/shared-skill/likes").body(Body::empty()).unwrap())
        .await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["total"], 1, "only user2's like should remain after user1 unlikes");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 11. Trending
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn trending_returns_200_with_items_array() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "trending-skill-a").await;
    create_skill(&app, &token, "trending-skill-b").await;

    let resp = app
        .oneshot(Request::builder().uri("/api/v1/trending").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["items"].is_array(), "trending must return an items array");
    assert!(json["total"].as_u64().unwrap_or(0) >= 2);
}

#[tokio::test]
async fn trending_with_kind_filter_returns_only_that_kind() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "kind-filter-skill").await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/trending?kind=skill")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let items = json["items"].as_array().unwrap();
    // Every item returned must be of kind "skill"
    for item in items {
        assert_eq!(item["artifact"]["kind"], "skill");
    }
}

#[tokio::test]
async fn trending_unknown_kind_returns_400() {
    let app = build_test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/trending?kind=unicorn")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 12. Package CRUD (get by id, delete)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn get_package_by_id_returns_200() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "pkg-get-skill").await;

    // Register a package
    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/pkg-get-skill/packages",
            serde_json::json!({
                "source_type": "clawhub",
                "download_url": "https://hub.openclaw.io/skills/testorg/pkg-get-skill/0.1.0.zip",
                "checksum": "abc123"
            }),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let pkg_json = body_json(resp).await;
    let package_id = pkg_json["package"]["id"].as_str().unwrap();

    // Fetch by ID
    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/v1/skills/testorg/pkg-get-skill/packages/{package_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["package"]["id"], package_id);
}

#[tokio::test]
async fn delete_package_removes_it() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "pkg-del-skill").await;

    // Register then delete
    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/pkg-del-skill/packages",
            serde_json::json!({
                "source_type": "clawhub",
                "download_url": "https://hub.openclaw.io/pkg.zip"
            }),
            Some(&token),
        ))
        .await
        .unwrap();
    let pkg_id = body_json(resp).await["package"]["id"].as_str().unwrap().to_owned();

    let del_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/skills/testorg/pkg-del-skill/packages/{pkg_id}"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::OK);
    let json = body_json(del_resp).await;
    assert_eq!(json["deleted"], true);

    // Now the package should be gone
    let get_resp = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/v1/skills/testorg/pkg-del-skill/packages/{pkg_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 13. Version-specific packages
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn version_specific_packages_returns_correct_packages() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "versioned-pkg-skill").await;

    // Register a package for the initial 0.1.0 version
    app.clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/versioned-pkg-skill/packages",
            serde_json::json!({
                "source_type": "clawhub",
                "download_url": "https://hub.openclaw.io/v0.1.0.zip"
            }),
            Some(&token),
        ))
        .await
        .unwrap();

    // Packages for version 0.1.0
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/testorg/versioned-pkg-skill/versions/0.1.0/packages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["version"], "0.1.0");
    assert_eq!(json["total"], 1);
}

#[tokio::test]
async fn version_specific_packages_nonexistent_version_returns_404() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "no-version-skill").await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/testorg/no-version-skill/versions/9.9.9/packages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 14. Skill install records
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn list_installs_returns_200_with_installs_array() {
    let app = build_test_app();
    let (_uid, token) = register_and_login(&app).await;
    create_skill(&app, &token, "install-list-skill").await;

    // Install the skill (this will also create install records)
    let install_resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/skills/testorg/install-list-skill/install",
            serde_json::json!({ "agents": ["openclaw"] }),
            Some(&token),
        ))
        .await
        .unwrap();
    // Install may fail in a test environment (no real HTTP), that's ok —
    // we just verify the installs list endpoint is reachable.
    let _ = install_resp;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/testorg/install-list-skill/installs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["installs"].is_array());
}

// ═══════════════════════════════════════════════════════════════════════════════
// 15. Agent skill inventory
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn list_skills_for_agent_returns_200() {
    let app = build_test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/agents/openclaw")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["agent_kind"], "openclaw");
    assert!(json["installs"].is_array());
}

#[tokio::test]
async fn list_skills_for_custom_agent_returns_200_with_empty_installs() {
    // AgentKind supports Custom(String) — any agent name is accepted.
    // A custom agent with no installs should return an empty list, not an error.
    let app = build_test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/skills/agents/my-custom-mcp-agent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["agent_kind"], "my-custom-mcp-agent");
    assert_eq!(json["total"], 0);
    assert!(json["installs"].as_array().unwrap().is_empty());
}

