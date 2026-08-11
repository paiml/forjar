//! GH-215 (#208): forjar YAML validation for the language server.
//!
//! Split out of `lsp.rs` to keep both files under the repo's 500-line ceiling.
//! Re-exported from `lsp` so `validate_yaml` / `make_diag` keep their paths.

use super::lsp::{Diagnostic, DiagnosticSeverity};

/// Basic YAML validation for forjar configs.
pub fn validate_yaml(content: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    // Check YAML parse
    if let Err(e) = serde_yaml_ng::from_str::<serde_json::Value>(content) {
        // GH-215 (#208): the parser already knows WHERE the file broke — the
        // location was being interpolated into the message text ("… at line 6
        // column 4") and thrown away, and every squiggle landed on line 1.
        // Use the location to build the range.
        let (line, character) = match e.location() {
            Some(loc) => (
                loc.line().saturating_sub(1) as u32,
                loc.column().saturating_sub(1) as u32,
            ),
            None => (0, 0),
        };
        let line_len = content
            .lines()
            .nth(line as usize)
            .map(|l| l.len() as u32)
            .unwrap_or(character + 1);
        diags.push(Diagnostic {
            line,
            character,
            end_line: line,
            // Never zero-width: an empty range renders as no squiggle at all.
            end_character: line_len.max(character + 1),
            severity: DiagnosticSeverity::Error,
            message: format!("YAML parse error: {e}"),
            source: "forjar-lsp".to_string(),
        });
        return diags;
    }

    // Check for common line-level issues
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains('\t') {
            diags.push(make_diag(
                i,
                line,
                DiagnosticSeverity::Warning,
                "Tabs should not be used in YAML; use spaces",
            ));
        }
        if trimmed.starts_with("ensure:") {
            let val = trimmed.trim_start_matches("ensure:").trim();
            if !["present", "absent", "latest", "running", "stopped", ""].contains(&val) {
                diags.push(make_diag(i, line, DiagnosticSeverity::Warning,
                    &format!("Unknown ensure value '{val}'; expected present|absent|latest|running|stopped")));
            }
        }
    }

    // FJ-2504: Unknown field detection + structural validation
    // GH-215: these two used to be anchored at `make_diag(0, "", ..)` — line 0,
    // ZERO width, i.e. an invisible squiggle on the first line of every file.
    for w in &crate::core::parser::check_unknown_fields(content) {
        diags.push(located_diag(
            content,
            DiagnosticSeverity::Warning,
            &w.message,
        ));
    }
    if let Ok(config) = crate::core::parser::parse_config(content) {
        for e in &crate::core::parser::validate_config(&config) {
            diags.push(located_diag(content, DiagnosticSeverity::Error, &e.message));
        }
    }

    diags
}

/// Build a diagnostic anchored at the text the message is about (GH-215).
///
/// Falls back to the whole first line — never a zero-width range — when the
/// message names nothing that can be found in the document.
pub(super) fn located_diag(content: &str, severity: DiagnosticSeverity, msg: &str) -> Diagnostic {
    match super::lsp_locate::locate(content, msg) {
        Some(span) => Diagnostic {
            line: span.line,
            character: span.start,
            end_line: span.line,
            end_character: span.end,
            severity,
            message: msg.to_string(),
            source: "forjar-lsp".to_string(),
        },
        None => make_diag(0, content.lines().next().unwrap_or(""), severity, msg),
    }
}

pub(super) fn make_diag(
    line_idx: usize,
    line: &str,
    severity: DiagnosticSeverity,
    msg: &str,
) -> Diagnostic {
    Diagnostic {
        line: line_idx as u32,
        character: 0,
        end_line: line_idx as u32,
        end_character: line.len() as u32,
        severity,
        message: msg.to_string(),
        source: "forjar-lsp".to_string(),
    }
}
