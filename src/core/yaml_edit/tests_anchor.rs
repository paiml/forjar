//! Unit coverage for the block-mapping scanner.
//!
//! The scanner is the one genuinely new piece of machinery here and it edits a
//! user's config file by byte offset, so these tests are weighted toward the
//! shapes it must REFUSE.

use super::*;

const DOC: &str = "\
version: \"1.0\"
name: demo
# a comment above resources
resources:
  # a-file must stay first
  a-file:
    type: file
    mode: \"0777\"
    content: |
      mode: not-a-key
  b-file:
    type: file
    mode: 0600   # inline comment
";

fn span_text(text: &str, path: &[&str]) -> String {
    let span = find_scalar(text, path).expect("anchored");
    scalar_text(text, &span).to_string()
}

#[test]
fn finds_a_nested_quoted_scalar() {
    assert_eq!(span_text(DOC, &["resources", "a-file", "mode"]), "\"0777\"");
}

#[test]
fn an_inline_comment_is_not_part_of_the_value() {
    assert_eq!(span_text(DOC, &["resources", "b-file", "mode"]), "0600");
}

#[test]
fn a_block_scalar_body_is_not_scanned_for_keys() {
    // `mode: not-a-key` lives inside a `content: |` block. If the scanner
    // treated it as a key line the next assertion would anchor to it.
    let span = find_scalar(DOC, &["resources", "a-file", "mode"]).expect("anchored");
    assert_eq!(span.line, 8);
}

#[test]
fn splicing_touches_only_the_value_bytes() {
    let span = find_scalar(DOC, &["resources", "a-file", "mode"]).expect("anchored");
    let out = splice(DOC, &span, "'0644'");
    assert_eq!(out.lines().count(), DOC.lines().count());
    assert!(out.contains("# a comment above resources"));
    assert!(out.contains("  # a-file must stay first"));
    assert!(out.contains("      mode: not-a-key"));
    assert!(out.contains("    mode: '0644'"));
    let diffs = DOC.lines().zip(out.lines()).filter(|(a, b)| a != b).count();
    assert_eq!(diffs, 1, "exactly one line may differ");
}

#[test]
fn a_block_scalar_value_is_refused() {
    assert_eq!(
        find_scalar(DOC, &["resources", "a-file", "content"]),
        Err(AnchorError::BlockScalar)
    );
}

#[test]
fn flow_style_is_refused_not_guessed() {
    let doc = "resources:\n  web: {type: file, mode: \"0777\"}\n";
    assert_eq!(
        find_scalar(doc, &["resources", "web", "mode"]),
        Err(AnchorError::FlowStyle)
    );
}

#[test]
fn an_alias_is_refused() {
    let doc = "resources:\n  web:\n    mode: *shared\n";
    assert_eq!(
        find_scalar(doc, &["resources", "web", "mode"]),
        Err(AnchorError::Alias)
    );
}

#[test]
fn a_duplicate_key_is_refused() {
    let doc = "resources:\n  web:\n    mode: \"0777\"\n    mode: \"0600\"\n";
    assert_eq!(
        find_scalar(doc, &["resources", "web", "mode"]),
        Err(AnchorError::Duplicate)
    );
}

#[test]
fn a_missing_key_is_not_found() {
    assert_eq!(
        find_scalar(DOC, &["resources", "c-file", "mode"]),
        Err(AnchorError::NotFound)
    );
    assert_eq!(
        find_scalar(DOC, &["resources", "a-file", "owner"]),
        Err(AnchorError::NotFound)
    );
}

#[test]
fn a_multi_line_plain_scalar_is_refused() {
    let doc = "resources:\n  web:\n    mode: this is\n      a continuation\n";
    assert_eq!(
        find_scalar(doc, &["resources", "web", "mode"]),
        Err(AnchorError::Multiline)
    );
}

#[test]
fn an_unterminated_quote_is_refused() {
    let doc = "resources:\n  web:\n    mode: \"0777\n";
    assert_eq!(
        find_scalar(doc, &["resources", "web", "mode"]),
        Err(AnchorError::Multiline)
    );
}

#[test]
fn crlf_line_endings_survive_a_splice() {
    let doc = "resources:\r\n  web:\r\n    mode: \"0777\"\r\n";
    let span = find_scalar(doc, &["resources", "web", "mode"]).expect("anchored");
    let out = splice(doc, &span, "'0644'");
    assert_eq!(out, "resources:\r\n  web:\r\n    mode: '0644'\r\n");
}

#[test]
fn a_tab_indented_line_never_matches() {
    let doc = "resources:\n\tweb:\n\t\tmode: \"0777\"\n";
    assert!(find_scalar(doc, &["resources", "web", "mode"]).is_err());
}

#[test]
fn a_sequence_item_is_not_a_key_line() {
    let doc = "includes:\n  - extra.yaml\nresources:\n  web:\n    mode: \"0777\"\n";
    assert_eq!(span_text(doc, &["resources", "web", "mode"]), "\"0777\"");
    assert_eq!(
        find_scalar(doc, &["includes", "extra.yaml"]),
        Err(AnchorError::NotAMapping)
    );
}

#[test]
fn a_zero_prefixed_value_is_quoted_by_the_emitter() {
    // Not a hand-rolled quoting rule: `0644` must not round-trip as a number.
    let emitted = emit_scalar("0644").expect("one line");
    assert!(!emitted.contains('\n'));
    let back: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&emitted).expect("the emitted scalar parses");
    assert_eq!(back.as_str(), Some("0644"));
}

#[test]
fn unquote_strips_one_layer_only() {
    assert_eq!(unquote("\"0777\""), "0777");
    assert_eq!(unquote("'0777'"), "0777");
    assert_eq!(unquote("0777"), "0777");
    assert_eq!(unquote("\"\""), "");
}

#[test]
fn an_empty_path_is_not_found() {
    assert_eq!(find_scalar(DOC, &[]), Err(AnchorError::NotFound));
}

#[test]
fn every_refusal_says_what_it_refused() {
    for e in [
        AnchorError::NotFound,
        AnchorError::FlowStyle,
        AnchorError::BlockScalar,
        AnchorError::Alias,
        AnchorError::Duplicate,
        AnchorError::Multiline,
        AnchorError::NotAMapping,
        AnchorError::Unterminated,
    ] {
        assert!(e.reason().len() > 12, "{e:?} has no usable reason");
    }
}
