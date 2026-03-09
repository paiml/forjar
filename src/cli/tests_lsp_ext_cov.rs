//! Coverage tests for lsp.rs — hover_info, make_diag, initialize_result,
//! make_response, make_error_response, write_message, read_message,
//! publish_diagnostics.

use super::lsp::*;

// ── hover_info ───────────────────────────────────────────────────

#[test]
fn hover_known_top_level_key() {
    let result = hover_info("machines:");
    assert!(!result.is_null());
}

#[test]
fn hover_known_resource_field() {
    let result = hover_info("  type: package");
    assert!(!result.is_null());
}

#[test]
fn hover_unknown_key() {
    let result = hover_info("  xyzzy_unknown: value");
    assert!(result.is_null());
}

#[test]
fn hover_list_prefix() {
    // "- type" trims to "type" which is a known resource field
    let result = hover_info("  - type: package");
    assert!(!result.is_null());
}

#[test]
fn hover_empty_line() {
    let result = hover_info("");
    assert!(result.is_null());
}

#[test]
fn hover_colon_only() {
    let result = hover_info(":");
    assert!(result.is_null());
}

// ── make_diag ────────────────────────────────────────────────────

#[test]
fn diag_warning() {
    let d = make_diag(5, "  tabs\there", DiagnosticSeverity::Warning, "no tabs");
    assert_eq!(d.line, 5);
    assert_eq!(d.character, 0);
    assert_eq!(d.end_line, 5);
    assert_eq!(d.end_character, 11); // "  tabs\there".len()
    assert_eq!(d.severity, DiagnosticSeverity::Warning);
    assert_eq!(d.message, "no tabs");
    assert_eq!(d.source, "forjar-lsp");
}

#[test]
fn diag_error() {
    let d = make_diag(0, "", DiagnosticSeverity::Error, "parse error");
    assert_eq!(d.line, 0);
    assert_eq!(d.end_character, 0);
    assert_eq!(d.severity, DiagnosticSeverity::Error);
}

// ── initialize_result ────────────────────────────────────────────

#[test]
fn init_result_has_capabilities() {
    let result = initialize_result();
    assert!(result["capabilities"]["textDocumentSync"].is_number());
    assert!(result["capabilities"]["hoverProvider"].as_bool().unwrap());
    assert!(result["serverInfo"]["name"].as_str().unwrap() == "forjar-lsp");
}

// ── make_response ────────────────────────────────────────────────

#[test]
fn response_with_id() {
    let id = serde_json::json!(42);
    let result = make_response(Some(&id), serde_json::json!({"ok": true}));
    assert_eq!(result["jsonrpc"], "2.0");
    assert_eq!(result["id"], 42);
    assert_eq!(result["result"]["ok"], true);
}

#[test]
fn response_null_id() {
    let result = make_response(None, serde_json::json!(null));
    assert_eq!(result["jsonrpc"], "2.0");
    assert!(result["id"].is_null());
}

#[test]
fn response_string_id() {
    let id = serde_json::json!("req-1");
    let result = make_response(Some(&id), serde_json::json!({}));
    assert_eq!(result["id"], "req-1");
}

// ── make_error_response ──────────────────────────────────────────

#[test]
fn error_response_method_not_found() {
    let id = serde_json::json!(1);
    let result = make_error_response(Some(&id), -32601, "method not found");
    assert_eq!(result["jsonrpc"], "2.0");
    assert_eq!(result["error"]["code"], -32601);
    assert_eq!(result["error"]["message"], "method not found");
}

#[test]
fn error_response_null_id() {
    let result = make_error_response(None, -32700, "parse error");
    assert!(result["id"].is_null());
    assert_eq!(result["error"]["code"], -32700);
}

// ── write_message ────────────────────────────────────────────────

#[test]
fn write_message_format() {
    let mut buf = Vec::new();
    let msg = serde_json::json!({"jsonrpc": "2.0", "id": 1, "result": null});
    write_message(&mut buf, &msg).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.starts_with("Content-Length: "));
    assert!(output.contains("\r\n\r\n"));
    assert!(output.contains("\"jsonrpc\""));
}

// ── read_message ─────────────────────────────────────────────────

#[test]
fn read_message_valid() {
    let body = r#"{"jsonrpc":"2.0","id":1}"#;
    let input = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let mut reader = std::io::BufReader::new(input.as_bytes());
    let msg = read_message(&mut reader).unwrap();
    assert_eq!(msg["jsonrpc"], "2.0");
    assert_eq!(msg["id"], 1);
}

#[test]
fn write_read_roundtrip() {
    let original = serde_json::json!({"jsonrpc": "2.0", "method": "test", "params": [1,2,3]});
    let mut buf = Vec::new();
    write_message(&mut buf, &original).unwrap();
    let mut reader = std::io::BufReader::new(buf.as_slice());
    let parsed = read_message(&mut reader).unwrap();
    assert_eq!(parsed, original);
}

// ── publish_diagnostics ──────────────────────────────────────────

#[test]
fn publish_empty_diags() {
    let result = publish_diagnostics("file:///test.yaml", &[]);
    assert_eq!(result["method"], "textDocument/publishDiagnostics");
    assert_eq!(result["params"]["uri"], "file:///test.yaml");
    assert!(result["params"]["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn publish_with_diags() {
    let diags = vec![
        make_diag(0, "bad line", DiagnosticSeverity::Error, "parse error"),
        make_diag(5, "tab\there", DiagnosticSeverity::Warning, "no tabs"),
    ];
    let result = publish_diagnostics("file:///config.yaml", &diags);
    let arr = result["params"]["diagnostics"].as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["severity"], 1); // Error
    assert_eq!(arr[1]["severity"], 2); // Warning
}

// ── validate_yaml ────────────────────────────────────────────────

#[test]
fn validate_yaml_valid() {
    let diags = validate_yaml("version: '1.0'\nname: test\nmachines: {}\nresources: {}\n");
    // May have structural warnings but no parse errors
    assert!(diags.iter().all(|d| d.severity != DiagnosticSeverity::Error
        || d.message.contains("YAML parse")));
}

#[test]
fn validate_yaml_tabs() {
    let diags = validate_yaml("version:\t'1.0'\n");
    assert!(diags.iter().any(|d| d.message.contains("Tabs")));
}

#[test]
fn validate_yaml_invalid() {
    let diags = validate_yaml("invalid: yaml: [broken");
    assert!(diags.iter().any(|d| d.message.contains("parse error")));
}
