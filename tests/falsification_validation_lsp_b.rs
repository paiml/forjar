//! FJ-2501/2504/2500: Popperian falsification for format validation,
//! LSP diagnostics, and unknown field detection.
//!
//! Each test states conditions under which the validation or LSP
//! subsystem would be rejected as invalid.

use forjar::cli::lsp::{CompletionItem, Diagnostic, DiagnosticSeverity, LspServer};
use forjar::core::parser::{check_unknown_fields, parse_config, validate_config};

// ── FJ-2501: Format Validation — Mode ──────────────────────────────

#[test]
fn f_2504_1_valid_yaml_no_error_diagnostics() {
    use forjar::cli::lsp::validate_yaml;

    // A well-formed config with machine + resource should have no Error-level diags
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#;
    let diags = validate_yaml(yaml);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "valid YAML must produce no errors: {errors:?}"
    );
}

#[test]
fn f_2504_2_invalid_yaml_produces_error() {
    use forjar::cli::lsp::validate_yaml;

    let diags = validate_yaml("{ invalid: [yaml");
    assert!(
        !diags.is_empty(),
        "malformed YAML must produce at least one diagnostic"
    );
    assert_eq!(
        diags[0].severity,
        DiagnosticSeverity::Error,
        "YAML parse error must be severity Error"
    );
    assert!(diags[0].message.contains("parse error"));
}

#[test]
fn f_2504_3_tab_in_yaml_produces_warning() {
    use forjar::cli::lsp::validate_yaml;

    // Tab on its own line within valid YAML (tab in key-value is still valid YAML for serde)
    let yaml = "version: \"1.0\"\nname: test\nresources:\n  svc:\n    type: service\n    ensure:\tpresent\n";
    let diags = validate_yaml(yaml);
    let tab_warnings: Vec<_> = diags
        .iter()
        .filter(|d| d.message.contains("Tabs") || d.message.contains("tab"))
        .collect();
    assert!(
        !tab_warnings.is_empty(),
        "tabs in YAML must produce a warning: got {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn f_2504_4_unknown_ensure_value_flagged() {
    use forjar::cli::lsp::validate_yaml;

    let yaml =
        "version: \"1.0\"\nname: test\nresources:\n  svc:\n    type: service\n    ensure: bogus\n";
    let diags = validate_yaml(yaml);
    let ensure_diags: Vec<_> = diags
        .iter()
        .filter(|d| d.message.contains("ensure"))
        .collect();
    assert!(
        !ensure_diags.is_empty(),
        "unknown ensure value 'bogus' must be flagged"
    );
}

// ── FJ-2504: LSP Server Protocol ──────────────────────────────────

#[test]
fn f_2504_5_lsp_server_initialize() {
    let mut server = LspServer::new();
    assert!(!server.initialized);

    let init_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "rootUri": "file:///tmp/test"
        }
    });
    let response = server.handle_message(&init_msg);
    assert!(response.is_some(), "initialize must return a response");
    assert!(server.initialized, "server must be initialized after init");
    assert_eq!(server.root_uri.as_deref(), Some("file:///tmp/test"));

    let resp = response.unwrap();
    assert!(resp["result"]["capabilities"]["completionProvider"].is_object());
}

#[test]
fn f_2504_6_lsp_server_did_open_stores_document() {
    let mut server = LspServer::new();
    server.initialized = true;

    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///tmp/test.yaml",
                "text": "version: \"1.0\"\nname: test\n"
            }
        }
    });
    server.handle_message(&msg);
    assert!(server.documents.contains_key("file:///tmp/test.yaml"));
}

#[test]
fn f_2504_7_lsp_validate_document_returns_diagnostics() {
    let mut server = LspServer::new();
    server.initialized = true;
    server
        .documents
        .insert("file:///tmp/bad.yaml".into(), "{ bad: [yaml".into());

    let diags = server.validate_document("file:///tmp/bad.yaml");
    assert!(!diags.is_empty(), "invalid YAML must produce diagnostics");
}

#[test]
fn f_2504_8_lsp_completion_top_level() {
    let mut server = LspServer::new();
    server.initialized = true;
    server
        .documents
        .insert("file:///tmp/test.yaml".into(), "\n".into());

    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": "file:///tmp/test.yaml" },
            "position": { "line": 0, "character": 0 }
        }
    });
    let response = server.handle_message(&msg);
    assert!(response.is_some());
    let items = response.unwrap();
    let result = &items["result"];
    assert!(result.is_array(), "completion must return an array");
    let arr = result.as_array().unwrap();
    assert!(
        arr.iter().any(|item| item["label"] == "resources"),
        "top-level completions must include 'resources'"
    );
    assert!(
        arr.iter().any(|item| item["label"] == "machines"),
        "top-level completions must include 'machines'"
    );
}

#[test]
fn f_2504_9_lsp_shutdown_sets_flag() {
    let mut server = LspServer::new();
    server.initialized = true;
    assert!(!server.shutdown_requested);

    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 99,
        "method": "shutdown"
    });
    let response = server.handle_message(&msg);
    assert!(response.is_some());
    assert!(server.shutdown_requested);
}

#[test]
fn f_2504_10_lsp_unknown_method_returns_error() {
    let mut server = LspServer::new();
    server.initialized = true;

    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "textDocument/bogusMethod"
    });
    let response = server.handle_message(&msg);
    assert!(response.is_some());
    let resp = response.unwrap();
    assert!(
        resp.get("error").is_some(),
        "unknown method must return an error response"
    );
    assert_eq!(resp["error"]["code"], -32601);
}

#[test]
fn f_2504_11_diagnostic_severity_values() {
    assert_eq!(DiagnosticSeverity::Error as u32, 1);
    assert_eq!(DiagnosticSeverity::Warning as u32, 2);
    assert_eq!(DiagnosticSeverity::Information as u32, 3);
    assert_eq!(DiagnosticSeverity::Hint as u32, 4);
}

// ── FJ-2501/2300: Deny Path Validation ─────────────────────────────

#[test]
fn f_2501_18_deny_path_glob_blocks_resource() {
    let yaml = r#"
version: "1.0"
name: test
policy:
  deny_paths:
    - "/etc/shadow"
    - "/root/**"
resources:
  shadow:
    type: file
    path: /etc/shadow
    content: "denied"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("denied by policy")),
        "path /etc/shadow must be blocked by deny_paths"
    );
}

#[test]
fn f_2501_19_deny_path_wildcard_blocks_subtree() {
    let yaml = r#"
version: "1.0"
name: test
policy:
  deny_paths:
    - "/root/**"
resources:
  rootrc:
    type: file
    path: /root/.bashrc
    content: "export PATH=/usr/bin"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("denied by policy")),
        "/root/.bashrc must be blocked by /root/** deny pattern"
    );
}

// ── FJ-2501: Machine Address Validation ────────────────────────────

#[test]
fn f_2501_20_valid_machine_addr_accepted() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web-01
    addr: 192.168.1.100
resources: {}
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let addr_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("addr"))
        .collect();
    assert!(
        addr_errors.is_empty(),
        "valid IP address must be accepted: {addr_errors:?}"
    );
}

#[test]
fn f_2501_21_empty_machine_addr_rejected() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web-01
    addr: ""
resources: {}
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("addr")),
        "empty machine address must be rejected: {errors:?}"
    );
}

// ── Cross-cutting: Serde Roundtrips ────────────────────────────────

#[test]
fn f_cross_1_diagnostic_serde_roundtrip() {
    let diag = Diagnostic {
        line: 5,
        character: 0,
        end_line: 5,
        end_character: 40,
        severity: DiagnosticSeverity::Warning,
        message: "test warning".into(),
        source: "forjar-lsp".into(),
    };
    let json = serde_json::to_string(&diag).unwrap();
    let parsed: Diagnostic = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.line, 5);
    assert_eq!(parsed.severity, DiagnosticSeverity::Warning);
    assert_eq!(parsed.message, "test warning");
}

#[test]
fn f_cross_2_completion_item_serde_roundtrip() {
    let item = CompletionItem {
        label: "resources".into(),
        detail: "Resource definitions".into(),
        insert_text: "resources:".into(),
        kind: 14,
    };
    let json = serde_json::to_string(&item).unwrap();
    let parsed: CompletionItem = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.label, "resources");
    assert_eq!(parsed.kind, 14);
}
