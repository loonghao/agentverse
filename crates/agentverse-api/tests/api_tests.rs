//! Integration tests for the Axum API router using in-memory stubs.
//! These tests exercise real HTTP routing, middleware, and response shapes
//! without requiring a live database.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{build_test_app, build_test_app_with_user, TEST_JWT_SECRET};
use http_body_util::BodyExt;
use tower::ServiceExt;

// ── helpers ───────────────────────────────────────────────────────────────────

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

fn json_request(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn bearer_token(secret: &str, user_id: uuid::Uuid, username: &str, kind: &str) -> String {
    use agentverse_auth::JwtManager;
    let mgr = JwtManager::new(secret, 3600);
    format!("Bearer {}", mgr.generate(user_id, username, kind).unwrap())
}

// ── GET /health ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_returns_200() {
    let app = build_test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn health_body_has_status_ok() {
    let app = build_test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["status"], "ok");
}

// ── GET /ready ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn readiness_returns_200_with_in_memory_repo() {
    let app = build_test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "ready");
}

// ── POST /api/v1/auth/register ────────────────────────────────────────────────

#[tokio::test]
async fn register_missing_fields_returns_422() {
    let app = build_test_app();
    let resp = app
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/register",
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    // axum returns 422 Unprocessable Entity for JSON deserialization errors
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn register_short_username_returns_400() {
    let app = build_test_app();
    let resp = app
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/register",
            serde_json::json!({
                "username": "ab",
                "password": "secure-password-123"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn register_new_user_returns_201() {
    let app = build_test_app();
    let resp = app
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/register",
            serde_json::json!({
                "username": "alice",
                "password": "correct-horse-battery-staple",
                "email": "alice@example.com"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    // response shape: { "user": {...}, "access_token": "...", ... }
    assert_eq!(json["user"]["username"], "alice");
    // password_hash must never appear in the response
    assert!(json["user"].get("password_hash").is_none());
}

// ── POST /api/v1/auth/login ───────────────────────────────────────────────────

#[tokio::test]
async fn login_unknown_user_returns_401() {
    let app = build_test_app();
    let resp = app
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/login",
            serde_json::json!({ "username": "nobody", "password": "pass" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── GET /api/v1/auth/me ───────────────────────────────────────────────────────

#[tokio::test]
async fn me_without_auth_returns_401() {
    let app = build_test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn me_with_valid_token_returns_200() {
    use agentverse_auth::PasswordManager;
    use agentverse_core::user::{User, UserKind};
    use chrono::Utc;

    let uid = uuid::Uuid::new_v4();
    let user = User {
        id: uid,
        username: "bob".into(),
        email: None,
        kind: UserKind::Human,
        capabilities: None,
        public_key: None,
        password_hash: Some(PasswordManager::hash("hunter2").unwrap()),
        created_at: Utc::now(),
    };
    let app = build_test_app_with_user(user);
    let token = bearer_token(TEST_JWT_SECRET, uid, "bob", "human");

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/me")
                .header("authorization", token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    // response shape: { "user": {...} }
    assert_eq!(json["user"]["username"], "bob");
}

// ── GET /api/v1/{kind} (list) ─────────────────────────────────────────────────

#[tokio::test]
async fn list_artifacts_returns_200_with_empty_array() {
    let app = build_test_app();
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
    // response shape: { "items": [...], "total": N }
    assert!(json["items"].is_array());
}

// ── Error response shape ──────────────────────────────────────────────────────

#[tokio::test]
async fn error_response_has_code_and_message() {
    let app = build_test_app();
    let resp = app
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/login",
            serde_json::json!({ "username": "ghost", "password": "x" }),
        ))
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert!(json["error"]["code"].is_number());
    assert!(json["error"]["message"].is_string());
}
