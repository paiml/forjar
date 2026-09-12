//! Controls: one fixture per shape a review lane claimed could hide a hosted
//! runner from this suite's parser, plus one that a fleet job is not mistaken
//! for a hosted one.
//!
//! Split out of `main.rs` when that file reached this repository's 500-line
//! gate.

use crate::scan::{fixture, hosted_in};

// ---------------------------------------------------------------------------
// Controls. Every one of these is a shape a review lane claimed could hide a
// hosted runner from this file's parser. Each is a fixture rather than an
// argument, because the four cases above are only worth what the parser under
// them is worth, and "I considered that shape" is not a measurement.
// ---------------------------------------------------------------------------

/// `runs-on` also takes an OBJECT: `{ group: …, labels: [ubuntu-latest] }`.
///
/// This is the shape that hides best. A reader scanning for
/// `runs-on: ubuntu-latest` does not see it, and neither did the first draft of
/// `push_labels`, which matched only `String` and `Sequence`.
#[test]
fn the_object_form_of_runs_on_cannot_hide_a_hosted_runner() {
    let doc = fixture(
        r#"
jobs:
  build:
    runs-on:
      group: ubuntu-runners
      labels: [ubuntu-latest]
"#,
    );
    assert_eq!(
        hosted_in(&doc),
        vec![("build".to_string(), "ubuntu-latest".to_string())],
        "the object form of `runs-on` hid a hosted runner from the parser"
    );
}

/// GitHub accepts `macOS-latest` and `Ubuntu-Latest`. A case-sensitive prefix
/// check lets both through while looking correct.
#[test]
fn a_hosted_label_in_another_case_is_still_a_hosted_label() {
    let doc = fixture(
        r#"
jobs:
  a:
    runs-on: macOS-Latest
  b:
    runs-on: Ubuntu-Latest
"#,
    );
    let found: Vec<String> = hosted_in(&doc).into_iter().map(|(_, l)| l).collect();
    assert_eq!(
        found,
        vec!["macOS-Latest".to_string(), "Ubuntu-Latest".to_string()],
        "a hosted label spelled in another case was not recognised"
    );
}

/// The matrix key need not be called `runner` or `os`.
#[test]
fn a_matrix_key_by_any_name_is_still_read() {
    let doc = fixture(
        r#"
jobs:
  build:
    strategy:
      matrix:
        machine: [ubuntu-latest, self-hosted]
    runs-on: ${{ matrix.machine }}
"#,
    );
    let found: Vec<String> = hosted_in(&doc).into_iter().map(|(_, l)| l).collect();
    assert_eq!(
        found,
        vec!["ubuntu-latest".to_string()],
        "a matrix key not named `runner` or `os` hid a hosted runner"
    );
}

/// An expression nested inside a `runs-on` LIST still reaches the matrix.
#[test]
fn an_expression_inside_a_runs_on_list_still_reaches_the_matrix() {
    let doc = fixture(
        r#"
jobs:
  build:
    strategy:
      matrix:
        pool: [macos-latest]
    runs-on: [self-hosted, "${{ matrix.pool }}"]
"#,
    );
    let found: Vec<String> = hosted_in(&doc).into_iter().map(|(_, l)| l).collect();
    assert_eq!(
        found,
        vec!["macos-latest".to_string()],
        "an expression inside a runs-on list did not reach the matrix"
    );
}

/// And the controls do not fire on a job that is genuinely on the fleet — a
/// parser that called everything hosted would pass every case above.
#[test]
fn a_fleet_job_is_not_mistaken_for_a_hosted_one() {
    let doc = fixture(
        r#"
jobs:
  a:
    runs-on: [self-hosted, clean-room]
  b:
    runs-on:
      group: fleet
      labels: [self-hosted, build]
"#,
    );
    assert!(
        hosted_in(&doc).is_empty(),
        "a fleet job was reported as hosted: {:?}",
        hosted_in(&doc)
    );
}
