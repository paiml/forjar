//! FJ-2724 (PMAT-199): `goal_closure` — make's prerequisite semantics.

use super::dag::goal_closure;
use crate::core::types::{ForjarConfig, Resource};

fn config(edges: &[(&str, &[&str])]) -> ForjarConfig {
    let mut c = ForjarConfig::default();
    for (id, deps) in edges {
        let r = Resource {
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        c.resources.insert(id.to_string(), r);
    }
    c
}

fn closure(c: &ForjarConfig, goals: &[&str]) -> Vec<String> {
    let goals: Vec<String> = goals.iter().map(|s| s.to_string()).collect();
    let mut v: Vec<String> = goal_closure(c, &goals).unwrap().into_iter().collect();
    v.sort();
    v
}

#[test]
fn a_goal_pulls_in_its_transitive_prerequisites() {
    // link -> obj -> mkdir. `make link` must build all three, in contrast to
    // `apply -r link`, which builds only link and silently links whatever
    // stale objects happen to be on disk.
    let c = config(&[
        ("mkdir", &[]),
        ("obj", &["mkdir"]),
        ("link", &["obj"]),
        ("unrelated", &[]),
    ]);
    assert_eq!(closure(&c, &["link"]), vec!["link", "mkdir", "obj"]);
}

#[test]
fn resources_outside_the_closure_are_excluded() {
    let c = config(&[("a", &[]), ("b", &[]), ("c", &["a"])]);
    assert_eq!(closure(&c, &["c"]), vec!["a", "c"]);
}

#[test]
fn multiple_goals_union_their_closures() {
    let c = config(&[
        ("shared", &[]),
        ("x", &["shared"]),
        ("y", &["shared"]),
        ("z", &[]),
    ]);
    assert_eq!(closure(&c, &["x", "y"]), vec!["shared", "x", "y"]);
}

#[test]
fn a_diamond_visits_the_shared_dependency_once() {
    let c = config(&[
        ("base", &[]),
        ("l", &["base"]),
        ("r", &["base"]),
        ("top", &["l", "r"]),
    ]);
    assert_eq!(closure(&c, &["top"]), vec!["base", "l", "r", "top"]);
}

#[test]
fn no_goals_means_no_closure_not_everything() {
    // `forjar make` with no goals must fall through to a plain apply, decided
    // by the caller — the closure of nothing is nothing, not the whole graph.
    let c = config(&[("a", &[]), ("b", &["a"])]);
    assert!(closure(&c, &[]).is_empty());
}

#[test]
fn unknown_goal_is_an_error_naming_the_known_targets() {
    let c = config(&[("real", &[]), ("other", &[])]);
    let err = goal_closure(&c, &["typo".to_string()]).expect_err("must not silently apply nothing");
    assert!(err.contains("no rule to make target 'typo'"), "{err}");
    assert!(
        err.contains("other, real"),
        "must list known targets: {err}"
    );
}

#[test]
fn a_cycle_terminates_rather_than_hanging() {
    // Reporting the cycle is build_execution_order's job; the closure must
    // simply not spin.
    let c = config(&[("a", &["b"]), ("b", &["a"])]);
    assert_eq!(closure(&c, &["a"]), vec!["a", "b"]);
}

#[test]
fn the_closure_is_downward_closed() {
    // The property that makes this filter safe where --subset is not: every
    // dependency of every member is also a member, so a pruned config can
    // never execute against an unconverged prerequisite.
    let c = config(&[
        ("d", &[]),
        ("c", &["d"]),
        ("b", &["c"]),
        ("a", &["b", "d"]),
        ("far", &["a"]),
    ]);
    let keep = goal_closure(&c, &["a".to_string()]).unwrap();
    for id in &keep {
        for dep in &c.resources[id].depends_on {
            assert!(
                keep.contains(dep),
                "{id} depends on {dep}, which the closure omitted"
            );
        }
    }
    assert!(!keep.contains("far"), "dependents must not be pulled in");
}
