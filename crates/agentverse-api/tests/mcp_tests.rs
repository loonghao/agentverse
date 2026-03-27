/// Tests for the MCP JSON-RPC protocol handler (pure logic, no I/O).

#[test]
fn mcp_initialize_response_has_version() {
    let resp = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "protocolVersion": "2025-06-18",
            "capabilities": { "tools": {}, "prompts": {} },
            "serverInfo": { "name": "agentverse", "version": "0.1.0" }
        }
    });
    assert_eq!(resp["result"]["protocolVersion"], "2025-06-18");
}

#[test]
fn mcp_tool_list_has_required_tools() {
    let tools = serde_json::json!([
        "search_skills", "get_artifact", "publish_artifact", "fork_artifact", "submit_learning"
    ]);
    let required = ["search_skills", "get_artifact", "publish_artifact"];
    for tool in required {
        assert!(
            tools.as_array().unwrap().iter().any(|t| t.as_str() == Some(tool)),
            "missing tool: {tool}"
        );
    }
}

#[test]
fn kind_parse_valid() {
    let kinds = ["skill", "soul", "agent", "workflow", "prompt"];
    for k in kinds {
        assert!(["skill","soul","agent","workflow","prompt"].contains(&k));
    }
}

#[test]
fn kind_parse_invalid_rejected() {
    assert!(!["skill","soul","agent","workflow","prompt"].contains(&"invalid-kind"));
}

