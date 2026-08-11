//! GH-215 (#208): locate a diagnostic in the document it describes.
//!
//! Every diagnostic the language server published used to be anchored at line
//! 0 — and for validation errors at a ZERO-WIDTH range at 0:0, which most
//! editors render as no squiggle at all. The information was already there:
//! `serde_yaml_ng` hands us a line and column, and a semantic error names the
//! offending resource/field in its own message text. This module turns the
//! second of those back into a range.
//!
//! It is deliberately a text scan, not a YAML position index: the validator
//! returns plain strings, so the identifiers it quotes are all we have. The
//! scan is documented as a best effort and always yields a NON-empty span, so
//! a diagnostic is visible even when the guess lands on the resource block
//! rather than the exact offending line.

/// A 0-based, single-line span in a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Span {
    /// 0-based line index.
    pub line: u32,
    /// 0-based start character.
    pub start: u32,
    /// 0-based end character (exclusive).
    pub end: u32,
}

/// Pull the quoted identifiers out of a diagnostic message, in order.
///
/// `resource 'r2' depends on unknown resource 'ghost-resource'`
/// -> `["r2", "ghost-resource"]`.
pub(super) fn quoted_tokens(message: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut open: Option<(char, usize)> = None;
    for (i, ch) in message.char_indices() {
        if ch != '\'' && ch != '"' {
            continue;
        }
        match open {
            Some((q, start)) if q == ch => {
                let tok = &message[start + q.len_utf8()..i];
                if !tok.is_empty() && !tok.contains(char::is_whitespace) {
                    out.push(tok.to_string());
                }
                open = None;
            }
            Some(_) => {}
            None => open = Some((ch, i)),
        }
    }
    out
}

/// Column at which `tok` appears as a YAML key on `line`, if it does.
fn key_column(line: &str, tok: &str) -> Option<usize> {
    let indent = line.len() - line.trim_start().len();
    let rest = line[indent..].strip_prefix("- ").unwrap_or(&line[indent..]);
    let col = line.len() - rest.len();
    let after = rest.strip_prefix(tok)?;
    if after.starts_with(':') {
        Some(col)
    } else {
        None
    }
}

/// Column at which `tok` appears as a standalone word on `line`, if it does.
fn word_column(line: &str, tok: &str) -> Option<usize> {
    let boundary = |c: char| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '.');
    let mut from = 0usize;
    while let Some(rel) = line[from..].find(tok) {
        let at = from + rel;
        let before_ok = line[..at].chars().next_back().is_none_or(boundary);
        let after_ok = line[at + tok.len()..].chars().next().is_none_or(boundary);
        if before_ok && after_ok {
            return Some(at);
        }
        from = at + tok.len().max(1);
    }
    None
}

/// Best-effort span for a validation message inside `content`.
///
/// Pass 1 prefers a token used as a YAML key (the block that owns the
/// problem); pass 2 accepts any standalone occurrence (e.g. a dependency named
/// inside a `depends_on` list). Returns `None` when nothing matches, so the
/// caller can fall back deliberately rather than silently claiming line 0.
pub(super) fn locate(content: &str, message: &str) -> Option<Span> {
    let tokens = quoted_tokens(message);
    for finder in [key_column, word_column] {
        for tok in &tokens {
            for (i, line) in content.lines().enumerate() {
                if let Some(col) = finder(line, tok) {
                    return Some(Span {
                        line: i as u32,
                        start: col as u32,
                        end: (col + tok.len()) as u32,
                    });
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_extracted_in_order() {
        assert_eq!(
            quoted_tokens("resource 'r2' depends on unknown resource 'ghost'"),
            vec!["r2".to_string(), "ghost".to_string()]
        );
    }

    #[test]
    fn multi_word_quotes_are_not_identifiers() {
        assert!(quoted_tokens("expected 'a b c'").is_empty());
    }

    #[test]
    fn key_match_wins_over_value_match() {
        let doc = "resources:\n  r2:\n    depends_on: [r2x]\n";
        let s = locate(doc, "resource 'r2' is bad").expect("located");
        assert_eq!(s.line, 1);
        assert_eq!(s.start, 2);
        assert_eq!(s.end, 4);
    }

    #[test]
    fn value_occurrence_is_found_when_no_key_matches() {
        let doc = "resources:\n  r2:\n    depends_on: [ghost]\n";
        let s = locate(doc, "unknown resource 'ghost'").expect("located");
        assert_eq!(s.line, 2);
        assert_eq!(s.end - s.start, 5);
    }

    #[test]
    fn unmatched_message_yields_none() {
        assert!(locate("a: 1\n", "nothing quoted here").is_none());
        assert!(locate("a: 1\n", "missing 'zzz'").is_none());
    }

    #[test]
    fn word_match_respects_boundaries() {
        // "r2" must not match inside "r2x".
        assert!(word_column("    depends_on: [r2x]", "r2").is_none());
        assert_eq!(word_column("    depends_on: [r2]", "r2"), Some(17));
    }
}
