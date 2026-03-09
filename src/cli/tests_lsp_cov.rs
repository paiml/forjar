//! Coverage tests for lsp.rs — publish_diagnostics, validate_yaml structural,
//! completion edge cases, read/write message edge cases.

use super::lsp::*;

// ── publish_diagnostics (previously untested) ──────────────────────

#[test]
fn validate_document_with_content() {
    let mut server = LspServer::new();
    server
        .documents
        .insert("file:///t.yaml".into(), "ensure: bogus\n".into());
    let diags = server.validate_document("file:///t.yaml");
    assert!(diags.iter().any(|d| d.message.contains("Unknown ensure")));
}

#[test]
fn validate_document_with_tabs_and_bad_ensure() {
    let mut server = LspServer::new();
    server.documents.insert(
        "file:///t.yaml".into(),
        "key:\tval\nensure: wrong\n".into(),
    );
    let diags = server.validate_document("file:///t.yaml");
    assert!(diags.len() >= 2);
}

// ── validate_yaml — structural checks via check_unknown_fields ─────

#[test]
fn validate_yaml_unknown_top_level_field() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources: {}
bogus_field: true
"#;
    let diags = validate_yaml(yaml);
    assert!(diags
        .iter()
        .any(|d| d.severity == DiagnosticSeverity::Warning));
}

#[test]
fn validate_yaml_valid_config() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /tmp/x
    content: hello
"#;
    let diags = validate_yaml(yaml);
    // Should have no errors (may have warnings for unknown fields depending on parser strictness)
    assert!(diags.iter().all(|d| d.severity != DiagnosticSeverity::Error));
}

#[test]
fn validate_yaml_parseable_but_incomplete() {
    // Valid YAML that doesn't match forjar schema — no parse error, structural warnings possible
    let yaml = "resources:\n  pkg:\n    type: package\n";
    let diags = validate_yaml(yaml);
    // Should not produce parse errors — it's valid YAML
    assert!(diags
        .iter()
        .all(|d| d.severity != DiagnosticSeverity::Error || !d.message.contains("parse")));
}

// ── initialize without root URI ────────────────────────────────────

#[test]
fn initialize_without_root_uri() {
    let mut s = LspServer::new();
    let msg = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {}
    });
    let resp = s.handle_message(&msg);
    assert!(resp.is_some());
    assert!(s.initialized);
    assert!(s.root_uri.is_none());
}

// ── completion "- type:" prefix ────────────────────────────────────

#[test]
fn completion_dash_type_prefix() {
    let items = completion_items(4, "- type:");
    assert!(items.iter().any(|i| i.label == "package"));
    assert!(items.iter().any(|i| i.label == "service"));
    assert!(items.iter().any(|i| i.label == "mount"));
    assert!(items.iter().any(|i| i.label == "gpu_driver"));
}

#[test]
fn completion_top_level_with_partial_key() {
    let items = completion_items(0, "mach");
    // Still returns top-level keys since line doesn't contain ':'
    assert!(items.iter().any(|i| i.label == "machines"));
}

#[test]
fn completion_top_level_with_colon() {
    // When the line has a colon at indent 0, no top-level items
    let items = completion_items(0, "machines:");
    // At indent 0 with colon, doesn't match top-level or type conditions
    // indent is 0 and line contains ':' → no top-level items
    // not starting with "type:" or "- type:" → no type items
    // indent < 2 → no resource fields
    assert!(items.is_empty());
}

// ── hover for resource fields ──────────────────────────────────────

#[test]
fn hover_resource_field() {
    let mut server = LspServer::new();
    server.documents.insert(
        "file:///t.yaml".into(),
        "resources:\n  pkg:\n    depends_on: [other]\n".into(),
    );
    let msg = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "textDocument/hover",
        "params": {
            "textDocument": {"uri": "file:///t.yaml"},
            "position": {"line": 2, "character": 4}
        }
    });
    let resp = server.handle_message(&msg).unwrap();
    let content = &resp["result"]["contents"]["value"];
    assert!(content.as_str().unwrap().contains("depends_on"));
}

#[test]
fn hover_resource_type_line() {
    let mut server = LspServer::new();
    server.documents.insert(
        "file:///t.yaml".into(),
        "resources:\n  pkg:\n    type: package\n".into(),
    );
    let msg = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "textDocument/hover",
        "params": {
            "textDocument": {"uri": "file:///t.yaml"},
            "position": {"line": 2, "character": 4}
        }
    });
    let resp = server.handle_message(&msg).unwrap();
    let content = &resp["result"]["contents"]["value"];
    assert!(content.as_str().unwrap().contains("type"));
}

// ── read_message with extra headers ────────────────────────────────

#[test]
fn read_message_with_content_type_header() {
    let json_body = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
    let header = format!(
        "Content-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
        json_body.len(),
        json_body
    );
    let mut cursor = std::io::Cursor::new(header.into_bytes());
    let parsed = read_message(&mut cursor).unwrap();
    assert_eq!(parsed["method"], "test");
}

// ── completion for indent 1 (between top-level and resource) ──────

#[test]
fn completion_at_indent_one() {
    // indent=1, not matching type prefix → falls through
    let items = completion_items(1, "hostname:");
    // indent 1 < 2, has colon → no matches
    assert!(items.is_empty());
}

// ── validate_yaml with valid ensure values ─────────────────────────

#[test]
fn validate_yaml_ensure_present() {
    let yaml = "ensure: present\n";
    let diags = validate_yaml(yaml);
    assert!(diags
        .iter()
        .all(|d| !d.message.contains("Unknown ensure")));
}

#[test]
fn validate_yaml_ensure_absent() {
    let yaml = "ensure: absent\n";
    let diags = validate_yaml(yaml);
    assert!(diags
        .iter()
        .all(|d| !d.message.contains("Unknown ensure")));
}

#[test]
fn validate_yaml_ensure_latest() {
    let yaml = "ensure: latest\n";
    let diags = validate_yaml(yaml);
    assert!(diags
        .iter()
        .all(|d| !d.message.contains("Unknown ensure")));
}

#[test]
fn validate_yaml_ensure_running() {
    let yaml = "ensure: running\n";
    let diags = validate_yaml(yaml);
    assert!(diags
        .iter()
        .all(|d| !d.message.contains("Unknown ensure")));
}

#[test]
fn validate_yaml_ensure_stopped() {
    let yaml = "ensure: stopped\n";
    let diags = validate_yaml(yaml);
    assert!(diags
        .iter()
        .all(|d| !d.message.contains("Unknown ensure")));
}

// ── server default trait ───────────────────────────────────────────

#[test]
fn server_default_creates_new() {
    let s = LspServer::default();
    assert!(!s.initialized);
    assert!(s.documents.is_empty());
    assert!(!s.shutdown_requested);
}

// ── handle_message with missing method ─────────────────────────────

#[test]
fn handle_message_no_method() {
    let mut server = LspServer::new();
    let msg = serde_json::json!({"jsonrpc": "2.0", "id": 1});
    let resp = server.handle_message(&msg);
    assert!(resp.is_none());
}

// ── did_open/did_change edge cases ─────────────────────────────────

#[test]
fn did_open_missing_params() {
    let mut server = LspServer::new();
    let msg = serde_json::json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {}
    });
    server.handle_message(&msg);
    assert!(server.documents.is_empty());
}

#[test]
fn did_change_missing_params() {
    let mut server = LspServer::new();
    let msg = serde_json::json!({
        "jsonrpc": "2.0", "method": "textDocument/didChange",
        "params": {}
    });
    server.handle_message(&msg);
    assert!(server.documents.is_empty());
}

#[test]
fn did_change_empty_content_changes() {
    let mut server = LspServer::new();
    server
        .documents
        .insert("file:///t.yaml".into(), "old content".into());
    let msg = serde_json::json!({
        "jsonrpc": "2.0", "method": "textDocument/didChange",
        "params": {
            "textDocument": {"uri": "file:///t.yaml"},
            "contentChanges": []
        }
    });
    server.handle_message(&msg);
    assert_eq!(server.documents["file:///t.yaml"], "old content");
}

// ── completion from server handle ──────────────────────────────────

#[test]
fn completion_resource_fields_from_server() {
    let mut server = LspServer::new();
    server.documents.insert(
        "file:///t.yaml".into(),
        "resources:\n  pkg:\n    \n".into(),
    );
    let msg = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "textDocument/completion",
        "params": {
            "textDocument": {"uri": "file:///t.yaml"},
            "position": {"line": 2, "character": 4}
        }
    });
    let resp = server.handle_message(&msg).unwrap();
    let items = resp["result"].as_array().unwrap();
    assert!(!items.is_empty());
}
