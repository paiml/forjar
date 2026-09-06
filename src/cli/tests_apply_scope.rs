//! GH-211: the four apply scope selectors, proven on the converged set.
//!
//! Each test states its RED condition against the published 1.12.3, where all
//! four flags were inert: the assertion below that names three files is exactly
//! what 1.12.3 produced for every one of these invocations.
//!
//! PMAT-160: the selectors are the same and so are the answers — only the
//! implementation moved. `apply_scope`'s four private prunes were a second copy
//! of `apply_selection::narrow`, applied before validation instead of after, so
//! the copy went and these tests now drive `resolve_selection` directly. Every
//! expectation below is unchanged, which is the point of running them here.

use super::apply_scope::ApplyScope;
use super::apply_selection::{resolve_selection, Selectors};
use crate::core::types;
use std::path::Path;

/// Two machines, four resources: three on `local`, one on `other`, and one of
/// the three also targets both machines.
fn cfg(dir: &Path) -> types::ForjarConfig {
    let p = dir.join("forjar.yaml");
    std::fs::write(
        &p,
        r#"
version: "1.0"
name: scope
machines:
  local:
    hostname: localhost
    addr: localhost
  other:
    hostname: other
    addr: other
resources:
  a-file:
    type: file
    machine: local
    path: /tmp/forjar-scope-a.txt
    content: A
  b-file:
    type: file
    machine: local
    path: /tmp/forjar-scope-b.txt
    content: B
  both-file:
    type: file
    machine: [local, other]
    path: /tmp/forjar-scope-both.txt
    content: BOTH
  o-file:
    type: file
    machine: other
    path: /tmp/forjar-scope-o.txt
    content: O
"#,
    )
    .unwrap();
    super::helpers::parse_and_validate(&p).expect("fixture parses")
}

fn ids(config: &types::ForjarConfig) -> Vec<&str> {
    config.resources.keys().map(String::as_str).collect()
}

fn run(config: &mut types::ForjarConfig, scope: ApplyScope) -> Result<(), String> {
    resolve_selection(config, &Selectors::default().with_scope(&scope), false).map(|_| ())
}

#[test]
fn resource_filter_keeps_only_the_matching_glob() {
    let d = tempfile::tempdir().unwrap();
    let mut c = cfg(d.path());
    run(
        &mut c,
        ApplyScope {
            resource_filter: Some("a-*"),
            ..Default::default()
        },
    )
    .unwrap();
    // RED on 1.12.3: all four survived and all four were applied.
    assert_eq!(ids(&c), vec!["a-file"]);
}

#[test]
fn resource_filter_matching_nothing_is_an_error() {
    let d = tempfile::tempdir().unwrap();
    let mut c = cfg(d.path());
    let err = run(
        &mut c,
        ApplyScope {
            resource_filter: Some("nope-*"),
            ..Default::default()
        },
    )
    .expect_err("a selector matching nothing is a typo, not a request for everything");
    assert!(err.contains("--resource-filter"), "{err}");
}

#[test]
fn skip_removes_exactly_the_named_resource() {
    let d = tempfile::tempdir().unwrap();
    let mut c = cfg(d.path());
    run(
        &mut c,
        ApplyScope {
            skip: Some("a-file"),
            ..Default::default()
        },
    )
    .unwrap();
    // RED on 1.12.3: a.txt was written anyway.
    assert!(!ids(&c).contains(&"a-file"));
    assert_eq!(ids(&c).len(), 3);
}

#[test]
fn skip_naming_an_unknown_resource_is_an_error_that_lists_what_exists() {
    let d = tempfile::tempdir().unwrap();
    let mut c = cfg(d.path());
    let err = run(
        &mut c,
        ApplyScope {
            skip: Some("a-fil"),
            ..Default::default()
        },
    )
    .expect_err("a typo'd --skip must not silently apply the resource");
    assert!(err.contains("a-fil"), "{err}");
    assert!(err.contains("b-file"), "the error must name what IS there: {err}");
}

#[test]
fn only_machine_restricts_to_that_machine_and_narrows_multi_machine_targets() {
    let d = tempfile::tempdir().unwrap();
    let mut c = cfg(d.path());
    run(
        &mut c,
        ApplyScope {
            only_machine: Some("local"),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(ids(&c), vec!["a-file", "b-file", "both-file"]);
    // The frame obligation: nothing left may still reach `other`.
    for r in c.resources.values() {
        assert!(
            r.machine.iter().all(|m| m == "local"),
            "a retained resource still targets another machine"
        );
    }
}

#[test]
fn only_machine_naming_an_unknown_machine_is_an_error() {
    let d = tempfile::tempdir().unwrap();
    let mut c = cfg(d.path());
    let err = run(
        &mut c,
        ApplyScope {
            only_machine: Some("ghost"),
            ..Default::default()
        },
    )
    .expect_err("1.12.3 applied everything to `local` for --only-machine ghost");
    assert!(err.contains("ghost"), "{err}");
    assert!(err.contains("local"), "{err}");
}

#[test]
fn exclude_machine_empties_the_frame_when_it_is_the_only_machine() {
    let d = tempfile::tempdir().unwrap();
    let mut c = cfg(d.path());
    c.resources.retain(|_, r| r.machine.iter().all(|m| m == "local"));
    run(
        &mut c,
        ApplyScope {
            exclude_machine: Some("local"),
            ..Default::default()
        },
    )
    .unwrap();
    // RED on 1.12.3: excluding the only machine still applied everything to it.
    assert!(ids(&c).is_empty(), "excluding the only machine must converge nothing");
}

#[test]
fn exclude_machine_keeps_multi_machine_resources_on_the_remaining_machine() {
    let d = tempfile::tempdir().unwrap();
    let mut c = cfg(d.path());
    run(
        &mut c,
        ApplyScope {
            exclude_machine: Some("local"),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(ids(&c).contains(&"both-file"));
    assert!(!ids(&c).contains(&"a-file"));
    for r in c.resources.values() {
        assert!(
            r.machine.iter().all(|m| m != "local"),
            "an excluded machine survived in a resource target list"
        );
    }
}

#[test]
fn exclude_machine_naming_an_unknown_machine_is_an_error() {
    let d = tempfile::tempdir().unwrap();
    let mut c = cfg(d.path());
    let err = run(
        &mut c,
        ApplyScope {
            exclude_machine: Some("ghost"),
            ..Default::default()
        },
    )
    .expect_err("excluding a machine that does not exist is a typo");
    assert!(err.contains("ghost"), "{err}");
}

#[test]
fn selectors_compose_as_an_intersection() {
    let d = tempfile::tempdir().unwrap();
    let mut c = cfg(d.path());
    run(
        &mut c,
        ApplyScope {
            resource_filter: Some("*-file"),
            skip: Some("b-file"),
            only_machine: Some("local"),
            exclude_machine: None,
        },
    )
    .unwrap();
    assert_eq!(ids(&c), vec!["a-file", "both-file"]);
}

#[test]
fn an_empty_scope_changes_nothing() {
    let d = tempfile::tempdir().unwrap();
    let mut c = cfg(d.path());
    let before = ids(&c).len();
    run(&mut c, ApplyScope::default()).unwrap();
    assert_eq!(ids(&c).len(), before);
}
