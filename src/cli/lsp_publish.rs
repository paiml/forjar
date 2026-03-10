use super::lsp::Diagnostic;

pub(super) fn publish_diagnostics(uri: &str, diags: &[Diagnostic]) -> serde_json::Value {
    let lsp_diags: Vec<serde_json::Value> = diags
        .iter()
        .map(|d| {
            serde_json::json!({
                "range": {
                    "start": { "line": d.line, "character": d.character },
                    "end": { "line": d.end_line, "character": d.end_character }
                },
                "severity": d.severity as u32,
                "source": d.source,
                "message": d.message
            })
        })
        .collect();
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": { "uri": uri, "diagnostics": lsp_diags }
    })
}
