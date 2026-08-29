//! Unit coverage for mapping-entry reordering.

use super::blocks::*;
use super::verify::changed_paths_of_text;
use super::AnchorError;

const DOC: &str = "\
version: \"1.0\"
resources:
  # b-file is referenced by the deploy runbook
  b-file:
    type: file
    path: /tmp/b

  a-file:
    type: file
    path: /tmp/a
# this comment is about policies, not about a-file
policies: []
";

#[test]
fn blocks_are_contiguous_and_cover_the_region() {
    let blocks = key_blocks(DOC, &["resources"]).expect("partitioned");
    assert_eq!(
        blocks.iter().map(|b| b.key.as_str()).collect::<Vec<_>>(),
        vec!["b-file", "a-file"]
    );
    for w in blocks.windows(2) {
        assert_eq!(w[0].end, w[1].start, "blocks must be contiguous");
    }
}

#[test]
fn a_trailing_comment_belongs_to_the_next_key_not_the_last_entry() {
    let blocks = key_blocks(DOC, &["resources"]).expect("partitioned");
    let last = blocks.last().expect("one block");
    assert!(
        !DOC[last.start..last.end].contains("about policies"),
        "a comment between the mapping and the next top-level key must stay put"
    );
}

#[test]
fn sorting_moves_the_comment_with_its_entry_and_keeps_every_byte() {
    let blocks = key_blocks(DOC, &["resources"]).expect("partitioned");
    assert!(!is_sorted(&blocks));
    let out = reorder(DOC, &blocks, &sorted_order(&blocks)).expect("reordered");
    assert_eq!(out.len(), DOC.len(), "a reorder must not add or drop bytes");
    let idx_comment = out
        .find("# b-file is referenced")
        .expect("comment survived");
    let idx_key = out.find("  b-file:").expect("key survived");
    assert!(idx_comment < idx_key, "the comment moved with its entry");
    assert!(out.find("  a-file:").expect("a-file") < idx_key);
    assert!(
        out.find("# this comment is about policies").expect("kept")
            > out.find("  b-file:").expect("b"),
        "the trailing comment stayed above policies"
    );
    assert!(changed_paths_of_text(DOC, &out)
        .expect("both parse")
        .is_empty());
}

#[test]
fn an_already_sorted_mapping_is_recognised() {
    let doc = "resources:\n  a:\n    type: file\n  b:\n    type: file\n";
    let blocks = key_blocks(doc, &["resources"]).expect("partitioned");
    assert!(is_sorted(&blocks));
    let out = reorder(doc, &blocks, &sorted_order(&blocks)).expect("reordered");
    assert_eq!(out, doc, "sorting a sorted mapping is the identity");
}

#[test]
fn a_region_without_a_terminating_newline_is_refused() {
    let doc = "resources:\n  b:\n    type: file\n  a:\n    type: file";
    assert_eq!(
        key_blocks(doc, &["resources"]),
        Err(AnchorError::Unterminated)
    );
}

#[test]
fn a_flow_style_mapping_is_refused() {
    let doc = "resources: {b: 1, a: 2}\n";
    assert_eq!(key_blocks(doc, &["resources"]), Err(AnchorError::FlowStyle));
}

#[test]
fn a_duplicate_entry_is_refused() {
    let doc = "resources:\n  a:\n    type: file\n  a:\n    type: dir\n";
    assert_eq!(key_blocks(doc, &["resources"]), Err(AnchorError::Duplicate));
}

#[test]
fn a_non_permutation_is_refused() {
    let blocks = key_blocks(DOC, &["resources"]).expect("partitioned");
    assert!(reorder(DOC, &blocks, &[0, 0]).is_err());
    assert!(reorder(DOC, &blocks, &[0]).is_err());
    assert!(reorder(DOC, &blocks, &[0, 9]).is_err());
}
