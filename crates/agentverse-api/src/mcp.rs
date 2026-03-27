//! MCP (Model Context Protocol) server endpoint.
//! Exposes agentverse as an MCP server so AI agents can discover and
//! interact with the registry using standard tool calls.
//!
//! Protocol: JSON-RPC 2.0 over HTTP POST /mcp

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use agentverse_core::repository::ArtifactFilter;

// ── JSON-RPC 2.0 types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

impl JsonRpcResponse {
    fn ok(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn err(id: Option<serde_json::Value>, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

// ── MCP Tool Definitions (returned by tools/list) ────────────────────────────

fn mcp_tools() -> serde_json::Value {
    serde_json::json!({
        "tools": [
            {
                "name": "search_skills",
                "description": "Search available skills, agents, workflows, prompts, or souls by capability or keyword.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Natural language or keyword search query" },
                        "kind": { "type": "string", "enum": ["skill", "soul", "agent", "workflow", "prompt"] },
                        "tag": { "type": "string" },
                        "limit": { "type": "integer", "default": 10 }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "get_artifact",
                "description": "Get the latest version of a specific artifact by its full identifier.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind":      { "type": "string" },
                        "namespace": { "type": "string" },
                        "name":      { "type": "string" },
                        "version":   { "type": "string", "description": "Specific semver; omit for latest" }
                    },
                    "required": ["kind", "namespace", "name"]
                }
            },
            {
                "name": "publish_artifact",
                "description": "Publish a new artifact or a new version of an existing one.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind":      { "type": "string" },
                        "namespace": { "type": "string" },
                        "name":      { "type": "string" },
                        "content":   { "type": "object" },
                        "manifest":  { "type": "object" },
                        "changelog": { "type": "string" },
                        "bump":      { "type": "string", "enum": ["patch", "minor", "major"] }
                    },
                    "required": ["kind", "namespace", "name", "content"]
                }
            },
            {
                "name": "fork_artifact",
                "description": "Fork an existing artifact to create a derivative version.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind":            { "type": "string" },
                        "namespace":       { "type": "string" },
                        "name":            { "type": "string" },
                        "new_name":        { "type": "string" },
                        "new_namespace":   { "type": "string" },
                        "source_version":  { "type": "string" }
                    },
                    "required": ["kind", "namespace", "name", "new_name"]
                }
            },
            {
                "name": "list_artifacts",
                "description": "List artifacts in the registry, optionally filtered by kind, namespace, or tag.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind":      { "type": "string", "enum": ["skill", "soul", "agent", "workflow", "prompt"] },
                        "namespace": { "type": "string" },
                        "tag":       { "type": "string" },
                        "limit":     { "type": "integer", "default": 20 }
                    }
                }
            },
            {
                "name": "submit_learning",
                "description": "Submit a learning insight about an artifact (for AI agents).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind":             { "type": "string" },
                        "namespace":        { "type": "string" },
                        "name":             { "type": "string" },
                        "content":          { "type": "string" },
                        "confidence_score": { "type": "number", "minimum": 0.0, "maximum": 1.0 }
                    },
                    "required": ["kind", "namespace", "name", "content"]
                }
            },
            {
                "name": "add_like",
                "description": "Like an artifact to signal approval or usefulness to other agents.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind":      { "type": "string", "enum": ["skill", "soul", "agent", "workflow", "prompt"] },
                        "namespace": { "type": "string" },
                        "name":      { "type": "string" }
                    },
                    "required": ["kind", "namespace", "name"]
                }
            },
            {
                "name": "add_rating",
                "description": "Rate an artifact on a 1-5 scale with an optional review.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind":        { "type": "string", "enum": ["skill", "soul", "agent", "workflow", "prompt"] },
                        "namespace":   { "type": "string" },
                        "name":        { "type": "string" },
                        "score":       { "type": "integer", "minimum": 1, "maximum": 5 },
                        "review_text": { "type": "string" }
                    },
                    "required": ["kind", "namespace", "name", "score"]
                }
            },
            {
                "name": "add_comment",
                "description": "Post a comment on an artifact. Use kind=learning for AI insights, kind=benchmark for perf data.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind":      { "type": "string", "enum": ["skill", "soul", "agent", "workflow", "prompt"] },
                        "namespace": { "type": "string" },
                        "name":      { "type": "string" },
                        "content":   { "type": "string" },
                        "comment_kind": {
                            "type": "string",
                            "enum": ["review", "learning", "benchmark", "question", "note"],
                            "default": "review"
                        }
                    },
                    "required": ["kind", "namespace", "name", "content"]
                }
            },
            {
                "name": "get_artifact_stats",
                "description": "Get aggregate social statistics (likes, comments, ratings, interactions) for an artifact. No auth required.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind":      { "type": "string", "enum": ["skill", "soul", "agent", "workflow", "prompt"] },
                        "namespace": { "type": "string" },
                        "name":      { "type": "string" }
                    },
                    "required": ["kind", "namespace", "name"]
                }
            },
            {
                "name": "get_comments",
                "description": "List all comments on an artifact. No auth required.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind":      { "type": "string", "enum": ["skill", "soul", "agent", "workflow", "prompt"] },
                        "namespace": { "type": "string" },
                        "name":      { "type": "string" }
                    },
                    "required": ["kind", "namespace", "name"]
                }
            },
            {
                "name": "get_ratings",
                "description": "List all ratings for an artifact and compute average score. No auth required.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind":      { "type": "string", "enum": ["skill", "soul", "agent", "workflow", "prompt"] },
                        "namespace": { "type": "string" },
                        "name":      { "type": "string" }
                    },
                    "required": ["kind", "namespace", "name"]
                }
            },
            {
                "name": "get_versions",
                "description": "List all published versions of an artifact, newest first. No auth required.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind":      { "type": "string", "enum": ["skill", "soul", "agent", "workflow", "prompt"] },
                        "namespace": { "type": "string" },
                        "name":      { "type": "string" }
                    },
                    "required": ["kind", "namespace", "name"]
                }
            }
        ]
    })
}

// ── Main MCP handler ─────────────────────────────────────────────────────────

/// POST /mcp — MCP JSON-RPC 2.0 endpoint
pub async fn mcp_handler(
    State(state): State<AppState>,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let id = req.id.clone();

    let response = match req.method.as_str() {
        "initialize" => JsonRpcResponse::ok(
            id,
            serde_json::json!({
                "protocolVersion": "2025-06-18",
                "capabilities": { "tools": {}, "prompts": {} },
                "serverInfo": {
                    "name": "agentverse",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        ),
        "tools/list" => JsonRpcResponse::ok(id, mcp_tools()),
        "tools/call" => handle_tool_call(id, req.params, &state).await,
        "prompts/list" => JsonRpcResponse::ok(
            id,
            serde_json::json!({
                "prompts": [
                    {
                        "name": "discover_agents",
                        "description": "Find the best agents for a specific task",
                        "arguments": [
                            { "name": "task", "description": "Describe the task", "required": true }
                        ]
                    }
                ]
            }),
        ),
        method => JsonRpcResponse::err(id, -32601, format!("method not found: {method}")),
    };

    (StatusCode::OK, Json(response))
}

async fn handle_tool_call(
    id: Option<serde_json::Value>,
    params: Option<serde_json::Value>,
    state: &AppState,
) -> JsonRpcResponse {
    let tool_name = params
        .as_ref()
        .and_then(|p| p["name"].as_str())
        .unwrap_or("");
    // Convert args Map to Value for safe indexing (Map["key"] panics on missing keys,
    // but Value["key"] returns Null instead).
    let args: serde_json::Value = params
        .as_ref()
        .and_then(|p| p["arguments"].as_object())
        .cloned()
        .map(serde_json::Value::Object)
        .unwrap_or(serde_json::Value::Null);

    match tool_name {
        "search_skills" => {
            let query = args["query"].as_str().unwrap_or("");
            let kind = args["kind"].as_str();
            let limit = args["limit"].as_u64().unwrap_or(10);

            match state.fulltext.search(query, kind, None, limit).await {
                Ok(results) => JsonRpcResponse::ok(
                    id,
                    serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&results).unwrap_or_default()
                        }],
                        "results": results,
                    }),
                ),
                Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
            }
        }
        "get_artifact" => {
            let kind_str = args["kind"].as_str().unwrap_or("skill");
            let ns = args["namespace"].as_str().unwrap_or("");
            let name = args["name"].as_str().unwrap_or("");
            let ver = args["version"].as_str();

            let kind = parse_kind_mcp(kind_str);

            match state
                .artifacts
                .find_by_namespace_name(&kind, ns, name)
                .await
            {
                Ok(Some(artifact)) => {
                    let version_result = if let Some(v) = ver {
                        state.versions.find_by_semver(artifact.id, v).await
                    } else {
                        state.versions.find_latest(artifact.id).await
                    };
                    let version = version_result.ok().flatten();
                    JsonRpcResponse::ok(
                        id,
                        serde_json::json!({
                            "content": [{
                                "type": "text",
                                "text": format!("{}/{}/{}", kind_str, ns, name)
                            }],
                            "artifact": artifact,
                            "version": version,
                        }),
                    )
                }
                Ok(None) => {
                    JsonRpcResponse::err(id, -32001, format!("{kind_str}/{ns}/{name} not found"))
                }
                Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
            }
        }
        "list_artifacts" => {
            let kind_str = args["kind"].as_str();
            let ns = args["namespace"].as_str();
            let tag = args["tag"].as_str();
            let limit = args["limit"].as_u64().unwrap_or(20);

            let kind = kind_str.map(parse_kind_mcp);

            let filter = ArtifactFilter {
                kind,
                namespace: ns.map(String::from),
                tag: tag.map(String::from),
                limit: Some(limit),
                ..Default::default()
            };

            match state.artifacts.list(filter).await {
                Ok(items) => JsonRpcResponse::ok(
                    id,
                    serde_json::json!({
                        "content": [{ "type": "text", "text": format!("Found {} artifacts", items.len()) }],
                        "items": items,
                        "total": items.len(),
                    }),
                ),
                Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
            }
        }
        "publish_artifact" => {
            let kind = args["kind"].as_str().unwrap_or("skill");
            let ns = args["namespace"].as_str().unwrap_or("");
            let name = args["name"].as_str().unwrap_or("");
            // Publishing requires auth — return actionable guidance
            JsonRpcResponse::ok(
                id,
                serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": format!(
                            "To publish, send a POST /api/v1/{kind} request with a valid Bearer token.\n\
                             Body: {{\"namespace\":\"{ns}\",\"name\":\"{name}\",\"content\":{{...}},\"manifest\":{{...}}}}"
                        )
                    }],
                    "_meta": { "method": "POST", "path": format!("/api/v1/{kind}"), "requires_auth": true }
                }),
            )
        }
        "fork_artifact" => {
            let kind = args["kind"].as_str().unwrap_or("skill");
            let ns = args["namespace"].as_str().unwrap_or("");
            let name = args["name"].as_str().unwrap_or("");
            let path = format!("/api/v1/{kind}/{ns}/{name}/fork");
            JsonRpcResponse::ok(
                id,
                serde_json::json!({
                    "content": [{ "type": "text", "text": format!("POST {path} with Bearer token to fork") }],
                    "_meta": { "method": "POST", "path": path, "requires_auth": true }
                }),
            )
        }
        "submit_learning" => {
            let kind = args["kind"].as_str().unwrap_or("skill");
            let ns = args["namespace"].as_str().unwrap_or("");
            let name = args["name"].as_str().unwrap_or("");
            let path = format!("/api/v1/{kind}/{ns}/{name}/learn");
            JsonRpcResponse::ok(
                id,
                serde_json::json!({
                    "content": [{ "type": "text", "text": format!("POST {path} with Bearer token to submit learning") }],
                    "_meta": { "method": "POST", "path": path, "requires_auth": true }
                }),
            )
        }
        "add_like" => {
            let kind = args["kind"].as_str().unwrap_or("skill");
            let ns = args["namespace"].as_str().unwrap_or("");
            let name = args["name"].as_str().unwrap_or("");
            let path = format!("/api/v1/{kind}/{ns}/{name}/likes");
            JsonRpcResponse::ok(
                id,
                serde_json::json!({
                    "content": [{ "type": "text", "text": format!(
                        "Send POST {path} with your Bearer token to like this artifact."
                    )}],
                    "_meta": { "method": "POST", "path": path, "requires_auth": true }
                }),
            )
        }
        "add_rating" => {
            let kind = args["kind"].as_str().unwrap_or("skill");
            let ns = args["namespace"].as_str().unwrap_or("");
            let name = args["name"].as_str().unwrap_or("");
            let score = args["score"].as_u64().unwrap_or(5);
            let review = args["review_text"].as_str().unwrap_or("");
            let path = format!("/api/v1/{kind}/{ns}/{name}/ratings");
            JsonRpcResponse::ok(
                id,
                serde_json::json!({
                    "content": [{ "type": "text", "text": format!(
                        "Send POST {path} with Bearer token. Body: {{\"score\":{score},\"review_text\":\"{review}\"}}"
                    )}],
                    "_meta": {
                        "method": "POST", "path": path, "requires_auth": true,
                        "body": { "score": score, "review_text": review }
                    }
                }),
            )
        }
        "add_comment" => {
            let kind = args["kind"].as_str().unwrap_or("skill");
            let ns = args["namespace"].as_str().unwrap_or("");
            let name = args["name"].as_str().unwrap_or("");
            let content = args["content"].as_str().unwrap_or("");
            let comment_kind = args["comment_kind"].as_str().unwrap_or("review");
            let path = format!("/api/v1/{kind}/{ns}/{name}/comments");
            JsonRpcResponse::ok(
                id,
                serde_json::json!({
                    "content": [{ "type": "text", "text": format!(
                        "Send POST {path} with Bearer token. Body: {{\"content\":\"{content}\",\"kind\":\"{comment_kind}\"}}"
                    )}],
                    "_meta": {
                        "method": "POST", "path": path, "requires_auth": true,
                        "body": { "content": content, "kind": comment_kind }
                    }
                }),
            )
        }
        "get_artifact_stats" => {
            let kind_str = args["kind"].as_str().unwrap_or("skill");
            let ns = args["namespace"].as_str().unwrap_or("");
            let name = args["name"].as_str().unwrap_or("");

            let kind = parse_kind_mcp(kind_str);
            match state
                .artifacts
                .find_by_namespace_name(&kind, ns, name)
                .await
            {
                Ok(Some(artifact)) => match state.social.get_stats(artifact.id).await {
                    Ok(stats) => {
                        let summary = format!(
                                "{}/{}/{}: {} likes, {} comments, {} ratings (avg {:.1}/5), {} downloads",
                                kind_str, ns, name,
                                stats.likes_count,
                                stats.comments_count,
                                stats.ratings_count,
                                stats.avg_rating.unwrap_or(0.0),
                                artifact.downloads,
                            );
                        JsonRpcResponse::ok(
                            id,
                            serde_json::json!({
                                "content": [{ "type": "text", "text": summary }],
                                "stats": stats,
                                "downloads": artifact.downloads,
                            }),
                        )
                    }
                    Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                },
                Ok(None) => {
                    JsonRpcResponse::err(id, -32001, format!("{kind_str}/{ns}/{name} not found"))
                }
                Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
            }
        }
        "get_comments" => {
            let kind_str = args["kind"].as_str().unwrap_or("skill");
            let ns = args["namespace"].as_str().unwrap_or("");
            let name = args["name"].as_str().unwrap_or("");

            let kind = parse_kind_mcp(kind_str);
            match state
                .artifacts
                .find_by_namespace_name(&kind, ns, name)
                .await
            {
                Ok(Some(artifact)) => match state.social.list_comments(artifact.id).await {
                    Ok(comments) => JsonRpcResponse::ok(
                        id,
                        serde_json::json!({
                            "content": [{ "type": "text", "text": format!("{} comments on {}/{}/{}", comments.len(), kind_str, ns, name) }],
                            "comments": comments,
                            "total": comments.len(),
                        }),
                    ),
                    Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                },
                Ok(None) => {
                    JsonRpcResponse::err(id, -32001, format!("{kind_str}/{ns}/{name} not found"))
                }
                Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
            }
        }
        "get_ratings" => {
            let kind_str = args["kind"].as_str().unwrap_or("skill");
            let ns = args["namespace"].as_str().unwrap_or("");
            let name = args["name"].as_str().unwrap_or("");

            let kind = parse_kind_mcp(kind_str);
            match state
                .artifacts
                .find_by_namespace_name(&kind, ns, name)
                .await
            {
                Ok(Some(artifact)) => match state.social.list_ratings(artifact.id).await {
                    Ok(ratings) => {
                        let avg = if ratings.is_empty() {
                            None
                        } else {
                            let sum: f64 = ratings.iter().map(|r| r.score as f64).sum();
                            Some(sum / ratings.len() as f64)
                        };
                        let summary = match avg {
                            Some(a) => format!("{} ratings, average {:.1}/5", ratings.len(), a),
                            None => "no ratings yet".to_string(),
                        };
                        JsonRpcResponse::ok(
                            id,
                            serde_json::json!({
                                "content": [{ "type": "text", "text": summary }],
                                "ratings": ratings,
                                "total": ratings.len(),
                                "avg_score": avg,
                            }),
                        )
                    }
                    Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                },
                Ok(None) => {
                    JsonRpcResponse::err(id, -32001, format!("{kind_str}/{ns}/{name} not found"))
                }
                Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
            }
        }
        "get_versions" => {
            let kind_str = args["kind"].as_str().unwrap_or("skill");
            let ns = args["namespace"].as_str().unwrap_or("");
            let name = args["name"].as_str().unwrap_or("");

            let kind = parse_kind_mcp(kind_str);
            match state
                .artifacts
                .find_by_namespace_name(&kind, ns, name)
                .await
            {
                Ok(Some(artifact)) => match state.versions.list_for_artifact(artifact.id).await {
                    Ok(versions) => {
                        let latest = versions
                            .first()
                            .map(|v| v.version.as_str())
                            .unwrap_or("none");
                        JsonRpcResponse::ok(
                            id,
                            serde_json::json!({
                                "content": [{ "type": "text", "text": format!("{} versions, latest: {}", versions.len(), latest) }],
                                "versions": versions,
                                "total": versions.len(),
                            }),
                        )
                    }
                    Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
                },
                Ok(None) => {
                    JsonRpcResponse::err(id, -32001, format!("{kind_str}/{ns}/{name} not found"))
                }
                Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
            }
        }
        other => JsonRpcResponse::err(id, -32602, format!("unknown tool: {other}")),
    }
}

/// Map a string kind to the enum (defaults to Skill for unknown values).
fn parse_kind_mcp(k: &str) -> agentverse_core::artifact::ArtifactKind {
    match k {
        "soul" => agentverse_core::artifact::ArtifactKind::Soul,
        "agent" => agentverse_core::artifact::ArtifactKind::Agent,
        "workflow" => agentverse_core::artifact::ArtifactKind::Workflow,
        "prompt" => agentverse_core::artifact::ArtifactKind::Prompt,
        _ => agentverse_core::artifact::ArtifactKind::Skill,
    }
}
