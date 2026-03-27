use agentverse_core::versioning::{VersionBump, VersionEngine};

#[test]
fn patch_bump() {
    assert_eq!(
        VersionEngine::bump("1.2.3", VersionBump::Patch).unwrap(),
        "1.2.4"
    );
}

#[test]
fn minor_bump_resets_patch() {
    assert_eq!(
        VersionEngine::bump("1.2.3", VersionBump::Minor).unwrap(),
        "1.3.0"
    );
}

#[test]
fn major_bump_resets_minor_and_patch() {
    assert_eq!(
        VersionEngine::bump("1.2.3", VersionBump::Major).unwrap(),
        "2.0.0"
    );
}

#[test]
fn bump_clears_pre_release() {
    assert_eq!(
        VersionEngine::bump("1.0.0-alpha.1", VersionBump::Patch).unwrap(),
        "1.0.1"
    );
}

#[test]
fn infer_breaking_change_protocol_removed() {
    let old = serde_json::json!({ "capabilities": { "protocols": ["mcp", "rest"] } });
    let new = serde_json::json!({ "capabilities": { "protocols": ["mcp"] } });
    assert_eq!(VersionEngine::infer_bump(&old, &new), VersionBump::Major);
}

#[test]
fn infer_breaking_change_permission_removed() {
    let old = serde_json::json!({ "capabilities": { "permissions": ["read", "write"] } });
    let new = serde_json::json!({ "capabilities": { "permissions": ["read"] } });
    assert_eq!(VersionEngine::infer_bump(&old, &new), VersionBump::Major);
}

#[test]
fn infer_additive_protocol() {
    let old = serde_json::json!({ "capabilities": { "protocols": ["mcp"] } });
    let new = serde_json::json!({ "capabilities": { "protocols": ["mcp", "rest"] } });
    assert_eq!(VersionEngine::infer_bump(&old, &new), VersionBump::Minor);
}

#[test]
fn infer_patch_for_description_only_change() {
    let old = serde_json::json!({ "capabilities": { "protocols": ["mcp"] }, "description": "old" });
    let new = serde_json::json!({ "capabilities": { "protocols": ["mcp"] }, "description": "new" });
    assert_eq!(VersionEngine::infer_bump(&old, &new), VersionBump::Patch);
}

#[test]
fn infer_additive_modality() {
    let old = serde_json::json!({ "capabilities": { "input_modalities": ["text"] } });
    let new = serde_json::json!({ "capabilities": { "input_modalities": ["text", "code"] } });
    assert_eq!(VersionEngine::infer_bump(&old, &new), VersionBump::Minor);
}

#[test]
fn invalid_semver_returns_error() {
    assert!(VersionEngine::bump("not-semver", VersionBump::Patch).is_err());
}
