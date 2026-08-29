//! `forjar lint --fix` deleted every comment in the config (paiml/forjar#359).
//!
//! THE DEFECT, as reproduced against the shipped binary:
//!
//! ```text
//! $ grep -c '#' forjar.yaml          # 3
//! $ forjar lint -f forjar.yaml --fix
//! Wrote normalized config to forjar.yaml
//! $ grep -c '#' forjar.yaml          # 0
//! ```
//!
//! `lint_auto_fix` round-tripped the whole document through
//! `serde_yaml_ng::Value`, which does not carry comments, so every comment was
//! deleted as a side effect. In an IaC config the comments hold the
//! operational reasoning — why a host is pinned, why an ordering matters,
//! which runbook depends on it — which is precisely the part a reviewer needs
//! and the machine does not.
//!
//! Two more defects rode along in the same eleven lines. The sort was pushed
//! onto `fixes_applied` UNCONDITIONALLY, whenever a `resources:` mapping
//! existed, so `--fix` claimed "sorted resource keys alphabetically" on an
//! already-sorted file — and, because the claim was non-empty, wrote the file
//! to prove it. A single-resource config, where sorting is a no-op by
//! construction, still lost its comments. The round-trip also churned quote
//! style (`"1.0"` -> `'1.0'`), so the diff an operator reads is dominated by
//! noise unrelated to any lint finding.
//!
//! WHY THESE TESTS DRIVE THE BINARY. The defect is only visible in the FILE
//! ON DISK after the command exits; a unit test on the transformation would
//! have been just as green with the old implementation, which is presumably
//! why the old implementation shipped. These run `forjar lint --fix` and read
//! the bytes back.

use std::path::Path;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

/// One resource, three comments, one inline comment — the issue's own fixture.
/// The keys are already sorted, so a correct `--fix` has nothing to do.
const ALREADY_SORTED: &str = r#"version: "1.0"
name: comment-preservation
# This machine is the production web tier. Do not point it at staging.
machines:
  sandbox:
    hostname: sandbox
    addr: 127.0.0.1
resources:
  # a-file must stay first: the deploy runbook references it by position.
  a-file:
    type: file
    machine: sandbox
    path: /tmp/forjar-lintfix.txt
    content: hello   # inline comment
"#;

/// Two resources in the wrong order, each carrying a comment, plus a comment
/// that belongs to the NEXT top-level key rather than to the entry above it.
const UNSORTED: &str = r#"version: "1.0"
name: comment-preservation
# This machine is the production web tier. Do not point it at staging.
machines:
  sandbox:
    hostname: sandbox
    addr: 127.0.0.1
resources:
  # z-file is written last on purpose.
  z-file:
    type: file
    machine: sandbox
    path: /tmp/forjar-z.txt
    content: zed   # inline comment
  # a-file must stay first: the deploy runbook references it by position.
  a-file:
    type: file
    machine: sandbox
    path: /tmp/forjar-a.txt
    content: hello
# this comment is about the policy block
policies: []
"#;

fn write_config(dir: &Path, body: &str) -> std::path::PathBuf {
    let path = dir.join("forjar.yaml");
    std::fs::write(&path, body).expect("fixture written");
    path
}

/// Both streams: `OutputWriter::success` writes the "fixed: ..." line to
/// stderr and `result` writes the summary to stdout, so a test that read only
/// one of them would be blind to half of what the command claimed.
fn run_lint_fix(path: &Path) -> String {
    let out = Command::new(FORJAR)
        .args(["lint", "-f"])
        .arg(path)
        .arg("--fix")
        .output()
        .expect("forjar lint ran");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn comment_lines(text: &str) -> Vec<&str> {
    text.lines().filter(|l| l.contains('#')).collect()
}

/// The headline defect: comments before == comments after.
#[test]
fn every_comment_survives_lint_fix() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_config(dir.path(), UNSORTED);
    run_lint_fix(&path);
    let after = std::fs::read_to_string(&path).expect("read back");

    let before = comment_lines(UNSORTED);
    let mut kept = comment_lines(&after);
    kept.sort_unstable();
    let mut expected = before.clone();
    expected.sort_unstable();
    assert_eq!(
        kept, expected,
        "lint --fix deleted a comment.\nbefore:\n{UNSORTED}\nafter:\n{after}"
    );
}

/// An already-sorted file must come back byte-for-byte. Nothing needed doing,
/// so nothing may be written — not the comments, not the quote style, not a
/// re-indent.
#[test]
fn an_already_sorted_file_is_left_byte_identical() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_config(dir.path(), ALREADY_SORTED);
    run_lint_fix(&path);
    let after = std::fs::read_to_string(&path).expect("read back");
    assert_eq!(
        after, ALREADY_SORTED,
        "lint --fix rewrote a file it had no fix for"
    );
}

/// And it must not SAY it sorted anything. Claiming work that did not happen
/// is the defect family this repo tracks; here the claimed work was
/// unnecessary and the unclaimed work was destructive.
#[test]
fn an_already_sorted_file_is_not_reported_as_fixed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_config(dir.path(), ALREADY_SORTED);
    let stdout = run_lint_fix(&path);
    assert!(
        !stdout.contains("sorted resource keys"),
        "lint --fix claimed a sort on an already-sorted file:\n{stdout}"
    );
}

/// The sort still has to happen, and a comment above an entry has to travel
/// with it — a comment that stays put while its entry moves is a lie in the
/// file, which is worse than the unsorted keys.
#[test]
fn the_sort_still_happens_and_carries_each_comment_with_its_entry() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_config(dir.path(), UNSORTED);
    run_lint_fix(&path);
    let after = std::fs::read_to_string(&path).expect("read back");

    let a_key = after.find("  a-file:").expect("a-file present");
    let z_key = after.find("  z-file:").expect("z-file present");
    assert!(a_key < z_key, "resources were not sorted:\n{after}");

    let a_comment = after.find("# a-file must stay first").expect("a comment");
    let z_comment = after.find("# z-file is written last").expect("z comment");
    assert!(a_comment < a_key, "a-file's comment did not move with it");
    assert!(z_comment < z_key, "z-file's comment did not move with it");
    assert!(
        a_comment < z_comment,
        "the comments are in the pre-sort order"
    );

    let policy_comment = after.find("# this comment is about").expect("kept");
    assert!(
        policy_comment > z_key,
        "a comment between the mapping and the next top-level key was dragged \
         along with the last entry:\n{after}"
    );
}

/// Quote style is content, not formatting: rewriting `"1.0"` to `'1.0'` buries
/// the real change in a diff the operator did not ask for.
#[test]
fn quote_style_is_not_churned() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_config(dir.path(), UNSORTED);
    run_lint_fix(&path);
    let after = std::fs::read_to_string(&path).expect("read back");
    assert!(
        after.contains("version: \"1.0\""),
        "lint --fix rewrote the quote style:\n{after}"
    );
}

/// Running it twice must be a fixpoint. A transformation that keeps finding
/// work on its own output is reporting work that did not need doing.
#[test]
fn lint_fix_is_a_fixpoint() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_config(dir.path(), UNSORTED);
    run_lint_fix(&path);
    let first = std::fs::read_to_string(&path).expect("read back");
    let stdout = run_lint_fix(&path);
    let second = std::fs::read_to_string(&path).expect("read back");
    assert_eq!(first, second, "the second --fix changed the file again");
    assert!(
        !stdout.contains("sorted resource keys"),
        "the second --fix claimed a sort it did not need:\n{stdout}"
    );
}
