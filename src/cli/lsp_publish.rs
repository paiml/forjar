use super::lsp::Diagnostic;

/// Map an internal diagnostic to its LSP wire shape.
///
/// GH-210 (#208): one mapping, used by BOTH the push notification and the
/// `textDocument/diagnostic` pull report, so the two mechanisms cannot report
/// different ranges for the same finding.
pub(super) fn to_wire(d: &Diagnostic) -> serde_json::Value {
    serde_json::json!({
        "range": {
            "start": { "line": d.line, "character": d.character },
            "end": { "line": d.end_line, "character": d.end_character }
        },
        "severity": d.severity as u32,
        "source": d.source,
        "message": d.message
    })
}

pub(super) fn publish_diagnostics(uri: &str, diags: &[Diagnostic]) -> serde_json::Value {
    let lsp_diags: Vec<serde_json::Value> = diags.iter().map(to_wire).collect();
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": { "uri": uri, "diagnostics": lsp_diags }
    })
}

/// Build a `RelatedFullDocumentDiagnosticReport` for a pull request.
///
/// GH-210: the answer to `textDocument/diagnostic`, which the server advertises
/// via `diagnosticProvider` and previously answered with -32601.
pub(super) fn full_report(diags: &[Diagnostic]) -> serde_json::Value {
    let items: Vec<serde_json::Value> = diags.iter().map(to_wire).collect();
    serde_json::json!({ "kind": "full", "items": items })
}
