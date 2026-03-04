use super::lsp::*;
use std::io;

#[test]
fn test_server_new() {
    let s = LspServer::new();
    assert!(!s.initialized);
    assert!(s.documents.is_empty());
}

#[test]
fn test_initialize() {
    let mut s = LspServer::new();
    let msg = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": "file:///tmp/test" }
    });
    let resp = s.handle_message(&msg);
    assert!(resp.is_some());
    assert!(s.initialized);
    assert_eq!(s.root_uri.as_deref(), Some("file:///tmp/test"));
}

#[test]
fn test_did_open() {
    let mut s = LspServer::new();
    let msg = serde_json::json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": { "textDocument": { "uri": "file:///t.yaml", "text": "machines:\n  - name: web" } }
    });
    s.handle_message(&msg);
    assert!(s.documents.contains_key("file:///t.yaml"));
}

#[test]
fn test_did_change() {
    let mut s = LspServer::new();
    s.documents.insert("file:///t.yaml".into(), "old".into());
    let msg = serde_json::json!({
        "jsonrpc": "2.0", "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": "file:///t.yaml" },
            "contentChanges": [{ "text": "resources:\n  - name: pkg" }]
        }
    });
    s.handle_message(&msg);
    assert!(s.documents["file:///t.yaml"].contains("resources"));
}

#[test]
fn test_completion_top_level() {
    let mut s = LspServer::new();
    s.documents.insert("file:///t.yaml".into(), "".into());
    let msg = serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": "file:///t.yaml" },
            "position": { "line": 0, "character": 0 }
        }
    });
    let resp = s.handle_message(&msg).unwrap();
    let items = resp.pointer("/result").unwrap();
    assert!(items.as_array().unwrap().len() >= 5);
}

#[test]
fn test_completion_resource_type() {
    let items = completion_items(4, "type:");
    assert!(items.iter().any(|i| i.label == "package"));
    assert!(items.iter().any(|i| i.label == "file"));
}

#[test]
fn test_hover() {
    let mut s = LspServer::new();
    s.documents
        .insert("file:///t.yaml".into(), "machines:\n  - name: web".into());
    let msg = serde_json::json!({
        "jsonrpc": "2.0", "id": 3, "method": "textDocument/hover",
        "params": {
            "textDocument": { "uri": "file:///t.yaml" },
            "position": { "line": 0, "character": 0 }
        }
    });
    let resp = s.handle_message(&msg).unwrap();
    let contents = resp
        .pointer("/result/contents/value")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(contents.contains("machines"));
}

#[test]
fn test_validate_valid_yaml() {
    let diags = validate_yaml(
        "machines:\n  - name: web\nresources:\n  - name: pkg\n    type: package",
    );
    assert!(diags.is_empty());
}

#[test]
fn test_validate_invalid_yaml() {
    let diags = validate_yaml("{ broken yaml: [");
    assert!(!diags.is_empty());
    assert_eq!(diags[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn test_validate_bad_ensure() {
    let diags = validate_yaml("ensure: bogus");
    assert!(diags.iter().any(|d| d.message.contains("Unknown ensure")));
}

#[test]
fn test_shutdown() {
    let mut s = LspServer::new();
    let msg = serde_json::json!({ "jsonrpc": "2.0", "id": 99, "method": "shutdown" });
    let resp = s.handle_message(&msg);
    assert!(resp.is_some());
    assert!(s.shutdown_requested);
}

#[test]
fn test_write_read_message() {
    let msg = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "test"});
    let mut buf = Vec::new();
    write_message(&mut buf, &msg).unwrap();
    let mut reader = io::BufReader::new(&buf[..]);
    let parsed = read_message(&mut reader).unwrap();
    assert_eq!(parsed["method"], "test");
}

#[test]
fn test_unknown_method_request() {
    let mut s = LspServer::new();
    let msg = serde_json::json!({ "jsonrpc": "2.0", "id": 5, "method": "nonexistent" });
    let resp = s.handle_message(&msg).unwrap();
    assert!(resp.get("error").is_some());
}

#[test]
fn test_diagnostic_severity_values() {
    assert_eq!(DiagnosticSeverity::Error as u32, 1);
    assert_eq!(DiagnosticSeverity::Warning as u32, 2);
    assert_eq!(DiagnosticSeverity::Information as u32, 3);
    assert_eq!(DiagnosticSeverity::Hint as u32, 4);
}
