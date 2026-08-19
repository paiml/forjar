//! Regression tests for the three LSP defects found by dogfooding forjar
//! 1.12.3 (GH #208 — families #210 unverified success, #212 machine output
//! malformed, #215 tail).
//!
//! Each test was verified RED against the unfixed code:
//!   * pull diagnostics  -> `{"error":{"code":-32601,"message":"Method not found"}}`
//!   * completion item   -> serialised key `insert_text`, so clients drop it
//!   * diagnostic range  -> every finding at line 0, zero-width

use super::lsp::*;

/// The document used by the range tests. `ghost-resource` on the last line
/// does not exist, so `validate_config` produces one semantic error.
const DEP_DOC: &str = "version: \"1.0\"\nname: range-test\n\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\n\nresources:\n  r1:\n    type: file\n    machine: local\n    path: /tmp/r1.txt\n    content: \"one\"\n\n  r2:\n    type: file\n    machine: local\n    path: /tmp/r2.txt\n    content: \"two\"\n    depends_on: [ghost-resource]\n";

fn open_doc(server: &mut LspServer, uri: &str, text: &str) {
    server.handle_message(&serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {"textDocument": {"uri": uri, "languageId": "yaml", "version": 1, "text": text}}
    }));
}

// ── GH-210: an advertised capability must be dispatched ──────────────

#[test]
fn advertised_diagnostic_provider_is_actually_dispatched() {
    let mut server = LspServer::new();
    let uri = "file:///tmp/fj-lsp-pull.yaml";
    open_doc(&mut server, uri, DEP_DOC);

    let resp = server
        .handle_message(&serde_json::json!({
            "jsonrpc": "2.0", "id": 2,
            "method": "textDocument/diagnostic",
            "params": {"textDocument": {"uri": uri}}
        }))
        .expect("pull diagnostics must produce a response");

    // RED before the fix: {"error":{"code":-32601,"message":"Method not found"}}
    assert!(
        resp.get("error").is_none(),
        "advertised diagnosticProvider still answers with an error: {resp}"
    );
    let result = &resp["result"];
    assert_eq!(result["kind"], "full", "expected a full report: {resp}");
    let items = result["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1, "expected the ghost-resource error: {resp}");
    assert!(items[0]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("ghost-resource"));
}

#[test]
fn every_advertised_capability_has_a_dispatcher_arm() {
    // The structural ratchet: advertising a capability is a PROMISE that the
    // corresponding request is handled. Adding a capability without an arm
    // now fails here instead of shipping a -32601 to real editors.
    let caps = initialize_result();
    let caps = &caps["capabilities"];
    let advertised: &[(&str, &str)] = &[
        ("completionProvider", "textDocument/completion"),
        ("hoverProvider", "textDocument/hover"),
        ("diagnosticProvider", "textDocument/diagnostic"),
    ];

    let mut server = LspServer::new();
    let uri = "file:///tmp/fj-lsp-caps.yaml";
    open_doc(&mut server, uri, DEP_DOC);

    for (cap, method) in advertised {
        assert!(
            !caps[cap].is_null(),
            "capability '{cap}' is no longer advertised — drop it from this table too"
        );
        let resp = server
            .handle_message(&serde_json::json!({
                "jsonrpc": "2.0", "id": 9, "method": method,
                "params": {"textDocument": {"uri": uri}, "position": {"line": 10, "character": 4}}
            }))
            .unwrap_or_else(|| panic!("'{method}' produced no response at all"));
        assert!(
            resp.get("error").is_none(),
            "capability '{cap}' is advertised but '{method}' answers: {resp}"
        );
    }
}

#[test]
fn unadvertised_methods_still_get_method_not_found() {
    // Non-regression: the default arm must keep working; "fixed" must not
    // mean "answers everything".
    let mut server = LspServer::new();
    let resp = server
        .handle_message(&serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "method": "textDocument/definition", "params": {}
        }))
        .expect("a request always gets a response");
    assert_eq!(resp["error"]["code"], -32601);
}

// ── GH-212: the completion item's wire name ──────────────────────────

#[test]
fn completion_items_serialise_insert_text_as_camel_case() {
    let items = completion_items(4, "");
    assert!(!items.is_empty(), "resource-field completions must exist");
    let json = serde_json::to_value(&items).expect("serialise");
    let first = &json[0];

    // RED before the fix: the key was `insert_text`, which no LSP client
    // knows, so the trailing ": " was silently dropped and clients inserted
    // the bare label.
    assert!(
        first.get("insertText").is_some(),
        "CompletionItem must use the LSP wire name insertText: {first}"
    );
    assert!(
        first.get("insert_text").is_none(),
        "snake_case insert_text must not appear on the wire: {first}"
    );
    // Non-regression: the value is still the useful one, not the bare label.
    let insert = first["insertText"].as_str().unwrap_or_default();
    assert!(
        insert.ends_with(": "),
        "field completions must still insert 'key: ', got {insert:?}"
    );
    assert_ne!(insert, first["label"].as_str().unwrap_or_default());
}

#[test]
fn completion_response_over_the_wire_uses_camel_case() {
    let mut server = LspServer::new();
    let uri = "file:///tmp/fj-lsp-complete.yaml";
    open_doc(&mut server, uri, DEP_DOC);
    let resp = server
        .handle_message(&serde_json::json!({
            "jsonrpc": "2.0", "id": 5, "method": "textDocument/completion",
            "params": {"textDocument": {"uri": uri}, "position": {"line": 10, "character": 4}}
        }))
        .expect("completion response");
    let items = resp["result"].as_array().expect("item list");
    assert!(!items.is_empty());
    assert!(items[0].get("insertText").is_some(), "{resp}");
}

// ── GH-215: diagnostics must point at the offending text ─────────────

#[test]
fn semantic_diagnostics_are_not_all_anchored_at_line_zero() {
    let diags = validate_yaml(DEP_DOC);
    let ghost = diags
        .iter()
        .find(|d| d.message.contains("ghost-resource"))
        .expect("the unknown-dependency error must still be produced");

    // RED before the fix: line 0, character 0, end 0:0 — a zero-width range
    // on the first line, which most editors render as no squiggle at all.
    assert!(
        ghost.line > 0,
        "diagnostic is still anchored at line 0: {ghost:?}"
    );
    assert!(
        ghost.end_character > ghost.character,
        "zero-width range is invisible in editors: {ghost:?}"
    );
    // It points at the resource that owns the problem (`r2:`), per the fix
    // sketch in #215.
    let line = DEP_DOC.lines().nth(ghost.line as usize).unwrap_or("");
    assert!(
        line.contains("r2") || line.contains("ghost-resource"),
        "diagnostic points at an unrelated line {}: {line:?}",
        ghost.line
    );
}

#[test]
fn yaml_parse_errors_use_the_parsers_own_location() {
    // Block-mapping break on line 6 (1-based) => diagnostic on line 5 (0-based).
    let broken = "version: \"1.0\"\nname: t\nmachines:\n  local:\n    hostname: localhost\n   addr: 127.0.0.1\n";
    let diags = validate_yaml(broken);
    assert_eq!(diags.len(), 1, "{diags:?}");
    let d = &diags[0];
    assert!(d.message.contains("YAML parse error"));
    // RED before the fix: line 0, end_character 80 — a fixed span on line 1
    // while the message itself said "at line 6 column 4".
    assert!(
        d.line > 0,
        "parse error is still anchored at line 0 while the message names a line: {d:?}"
    );
    assert!(d.end_character > d.character, "{d:?}");
}

#[test]
fn line_level_diagnostics_keep_their_own_line() {
    // Non-regression: the diagnostics that were already located correctly
    // (tabs, bad `ensure:` values) must not be moved by the new locator.
    let doc = "version: \"1.0\"\nname: t\nmachines: {}\nresources:\n  r:\n    type: package\n    ensure: banana\n";
    let diags = validate_yaml(doc);
    let ensure = diags
        .iter()
        .find(|d| d.message.contains("Unknown ensure value"))
        .expect("ensure diagnostic");
    assert_eq!(ensure.line, 6, "{ensure:?}");
}
