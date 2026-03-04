//! Additional LSP tests for cli/lsp.rs — boost line coverage.

use super::lsp::*;

#[test]
fn test_write_read_message_roundtrip() {
    let msg = serde_json::json!({"jsonrpc": "2.0", "id": 1, "result": null});
    let mut buf = Vec::new();
    write_message(&mut buf, &msg).unwrap();
    let s = String::from_utf8(buf.clone()).unwrap();
    assert!(s.starts_with("Content-Length: "));
    assert!(s.contains("\r\n\r\n"));
    // Read it back
    let mut cursor = std::io::Cursor::new(buf);
    let parsed = read_message(&mut cursor).unwrap();
    assert_eq!(parsed["id"], 1);
}

#[test]
fn test_read_message_no_content_length() {
    let data = b"\r\n";
    let mut cursor = std::io::Cursor::new(data.to_vec());
    let result = read_message(&mut cursor);
    assert!(result.is_err());
}

#[test]
fn test_read_message_invalid_json() {
    let header = "Content-Length: 5\r\n\r\nhello";
    let mut cursor = std::io::Cursor::new(header.as_bytes().to_vec());
    let result = read_message(&mut cursor);
    assert!(result.is_err());
}

#[test]
fn test_handle_shutdown() {
    let mut server = LspServer::new();
    let msg = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "shutdown"});
    let resp = server.handle_message(&msg);
    assert!(resp.is_some());
    assert!(server.shutdown_requested);
}

#[test]
fn test_handle_unknown_method_with_id() {
    let mut server = LspServer::new();
    let msg = serde_json::json!({"jsonrpc": "2.0", "id": 99, "method": "unknown/method"});
    let resp = server.handle_message(&msg).unwrap();
    assert!(resp.get("error").is_some());
    assert_eq!(resp["error"]["code"], -32601);
}

#[test]
fn test_handle_unknown_notification() {
    let mut server = LspServer::new();
    // No id = notification
    let msg = serde_json::json!({"jsonrpc": "2.0", "method": "$/cancelRequest"});
    let resp = server.handle_message(&msg);
    assert!(resp.is_none());
}

#[test]
fn test_handle_initialized_notification() {
    let mut server = LspServer::new();
    let msg = serde_json::json!({"jsonrpc": "2.0", "method": "initialized"});
    let resp = server.handle_message(&msg);
    assert!(resp.is_none());
}

#[test]
fn test_handle_did_change() {
    let mut server = LspServer::new();
    // First open a doc
    let open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {"textDocument": {"uri": "file:///test.yaml", "text": "machines:\n  m:\n"}}
    });
    server.handle_message(&open);
    // Then change it
    let change = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {"uri": "file:///test.yaml"},
            "contentChanges": [{"text": "resources:\n  pkg:\n    type: package\n"}]
        }
    });
    server.handle_message(&change);
    assert!(server.documents["file:///test.yaml"].contains("resources"));
}

#[test]
fn test_hover_known_key() {
    let mut server = LspServer::new();
    let open = serde_json::json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {"uri": "file:///t.yaml", "text": "machines:\n  m:\n"}}
    });
    server.handle_message(&open);
    let hover = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "textDocument/hover",
        "params": {
            "textDocument": {"uri": "file:///t.yaml"},
            "position": {"line": 0, "character": 0}
        }
    });
    let resp = server.handle_message(&hover).unwrap();
    let content = &resp["result"]["contents"]["value"];
    assert!(content.as_str().unwrap().contains("machines"));
}

#[test]
fn test_hover_unknown_key() {
    let mut server = LspServer::new();
    let open = serde_json::json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {"uri": "file:///t.yaml", "text": "bogus_key: val\n"}}
    });
    server.handle_message(&open);
    let hover = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "textDocument/hover",
        "params": {
            "textDocument": {"uri": "file:///t.yaml"},
            "position": {"line": 0, "character": 0}
        }
    });
    let resp = server.handle_message(&hover).unwrap();
    assert!(resp["result"].is_null());
}

#[test]
fn test_hover_no_document() {
    let mut server = LspServer::new();
    let hover = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "textDocument/hover",
        "params": {
            "textDocument": {"uri": "file:///nonexistent.yaml"},
            "position": {"line": 0, "character": 0}
        }
    });
    let resp = server.handle_message(&hover).unwrap();
    assert!(resp["result"].is_null());
}

#[test]
fn test_completion_no_document() {
    let server = LspServer::new();
    let msg = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "textDocument/completion",
        "params": {
            "textDocument": {"uri": "file:///nonexistent.yaml"},
            "position": {"line": 0, "character": 0}
        }
    });
    let mut s = server;
    let resp = s.handle_message(&msg).unwrap();
    let items = resp["result"].as_array().unwrap();
    assert!(items.is_empty());
}

#[test]
fn test_completion_type_prefix() {
    let items = completion_items(4, "type: ");
    // When line starts with "type:", should get resource types
    assert!(items.iter().any(|i| i.label == "package"));
    assert!(items.iter().any(|i| i.label == "file"));
}

#[test]
fn test_completion_deep_indent() {
    let items = completion_items(4, "");
    // At indent >= 2, should get resource fields
    assert!(items.iter().any(|i| i.label == "depends_on"));
    assert!(items.iter().any(|i| i.label == "owner"));
}

#[test]
fn test_validate_yaml_with_tabs() {
    // The tab detection checks if trimmed line contains \t
    // But after trimming, tabs at the start are removed.
    // The check is: `if trimmed.contains("\t")`
    // This catches tabs within values, not just leading tabs
    let yaml = "key:\tvalue\n";
    let diags = validate_yaml(yaml);
    assert!(diags.iter().any(|d| d.message.contains("Tabs") || d.message.contains("tab")));
}

#[test]
fn test_validate_yaml_bad_ensure() {
    let yaml = "resources:\n  pkg:\n    ensure: bogus_value\n";
    let diags = validate_yaml(yaml);
    assert!(diags.iter().any(|d| d.message.contains("Unknown ensure value")));
}

#[test]
fn test_validate_yaml_invalid() {
    let yaml = ":\n  - :\n  [[[invalid";
    let diags = validate_yaml(yaml);
    assert!(diags.iter().any(|d| d.severity == DiagnosticSeverity::Error));
}

#[test]
fn test_validate_document_missing() {
    let server = LspServer::new();
    let diags = server.validate_document("file:///missing.yaml");
    assert!(diags.is_empty());
}

#[test]
fn test_diagnostic_serde() {
    let d = Diagnostic {
        line: 1,
        character: 0,
        end_line: 1,
        end_character: 10,
        severity: DiagnosticSeverity::Warning,
        message: "test".to_string(),
        source: "test".to_string(),
    };
    let json = serde_json::to_string(&d).unwrap();
    assert!(json.contains("Warning"));
    let back: Diagnostic = serde_json::from_str(&json).unwrap();
    assert_eq!(back.line, 1);
}

#[test]
fn test_completion_item_serde() {
    let item = CompletionItem {
        label: "test".to_string(),
        detail: "detail".to_string(),
        insert_text: "test".to_string(),
        kind: 14,
    };
    let json = serde_json::to_string(&item).unwrap();
    let back: CompletionItem = serde_json::from_str(&json).unwrap();
    assert_eq!(back.label, "test");
    assert_eq!(back.kind, 14);
}
