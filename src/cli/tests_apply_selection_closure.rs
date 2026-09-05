//! PMAT-160 (#466 #467 #468): one selection path, resolved by graph closure.
//!
//! Each apply mode used to derive its own resource set, so `--dry-run` rendered
//! the unscoped plan, `--check` ignored every scope flag, and `--subset a`
//! refused `a`'s own declared `depends_on` as "unknown" because the prune ran
//! before validation. The RED condition for every test below is that behaviour:
//! a selector either dropped a prerequisite silently or rejected a config the
//! file declares correctly.

use super::apply_selection::{resolve_selection, Selection, Selectors};
use crate::core::types;

fn machine() -> types::Machine {
    serde_yaml_ng::from_str("hostname: h\naddr: localhost\n").expect("machine fixture parses")
}

fn res(deps: &[&str], group: Option<&str>, machines: &[&str]) -> types::Resource {
    types::Resource {
        resource_type: types::ResourceType::File,
        machine: types::MachineTarget::Multiple(
            machines.iter().map(|m| (*m).to_string()).collect(),
        ),
        depends_on: deps.iter().map(|d| (*d).to_string()).collect(),
        resource_group: group.map(str::to_string),
        ..Default::default()
    }
}

/// `a -> b`, `d -> b`, `c` unrelated. `a` and `d` are in group `web`; `c` lives
/// on the second machine so the machine selectors have something to narrow.
fn cfg() -> types::ForjarConfig {
    let mut config = types::ForjarConfig {
        version: "1.0".to_string(),
        name: "selection".to_string(),
        ..Default::default()
    };
    config.machines.insert("local".to_string(), machine());
    config.machines.insert("other".to_string(), machine());
    config
        .resources
        .insert("a".to_string(), res(&["b"], Some("web"), &["local"]));
    config
        .resources
        .insert("b".to_string(), res(&[], None, &["local"]));
    config
        .resources
        .insert("c".to_string(), res(&[], None, &["other"]));
    config
        .resources
        .insert("d".to_string(), res(&["b"], Some("web"), &["local"]));
    config
}

/// `a -> b -> c`: the chain that proves contraction is transitive.
fn chain() -> types::ForjarConfig {
    let mut config = types::ForjarConfig {
        version: "1.0".to_string(),
        name: "chain".to_string(),
        ..Default::default()
    };
    config.machines.insert("local".to_string(), machine());
    config
        .resources
        .insert("a".to_string(), res(&["b"], None, &["local"]));
    config
        .resources
        .insert("b".to_string(), res(&["c"], None, &["local"]));
    config
        .resources
        .insert("c".to_string(), res(&[], None, &["local"]));
    config
}

fn ids(config: &types::ForjarConfig) -> Vec<&str> {
    config.resources.keys().map(String::as_str).collect()
}

fn deps<'a>(config: &'a types::ForjarConfig, id: &str) -> &'a [String] {
    &config.resources[id].depends_on
}

fn resolve(config: &mut types::ForjarConfig, sel: Selectors<'_>) -> Result<Selection, String> {
    resolve_selection(config, &sel, false)
}

#[test]
fn subset_pulls_the_dependency_closure_in() {
    let mut c = cfg();
    // RED: `--subset a` pruned b, then build_execution_order refused the
    // survivor with "resource 'a' depends on unknown 'b'" (#468).
    let out = resolve(
        &mut c,
        Selectors {
            subset: Some("a"),
            ..Default::default()
        },
    )
    .expect("a declared dependency is never 'unknown'");
    assert_eq!(ids(&c), vec!["a", "b"], "config order is preserved");
    assert_eq!(out.selected, 1);
    assert_eq!(out.dependencies_added, 1);
    assert_eq!(out.total, 4);
    assert!(out.removed.is_empty());
    assert!(out.cut_edges.is_empty());
}

#[test]
fn resource_selector_pulls_the_dependency_closure_in() {
    let mut c = cfg();
    // RED: `-r a` was exact-match with no closure — `make -o`, not `make`.
    let out = resolve(
        &mut c,
        Selectors {
            resource: Some("a"),
            ..Default::default()
        },
    )
    .expect("-r resolves");
    assert_eq!(ids(&c), vec!["a", "b"]);
    assert_eq!(out.selected, 1);
    assert_eq!(out.dependencies_added, 1);
}

#[test]
fn group_selector_pulls_the_dependency_closure_in() {
    let mut c = cfg();
    let out = resolve(
        &mut c,
        Selectors {
            group: Some("web"),
            ..Default::default()
        },
    )
    .expect("-g resolves");
    assert_eq!(ids(&c), vec!["a", "b", "d"]);
    assert_eq!(out.selected, 2, "a and d are in the group");
    assert_eq!(out.dependencies_added, 1, "b is their shared prerequisite");
}

#[test]
fn resource_filter_is_a_positive_selector_like_subset() {
    let mut c = cfg();
    let out = resolve(
        &mut c,
        Selectors {
            resource_filter: Some("a"),
            ..Default::default()
        },
    )
    .expect("--resource-filter resolves");
    assert_eq!(ids(&c), vec!["a", "b"]);
    assert_eq!(out.dependencies_added, 1);
}

#[test]
fn no_selector_is_the_identity() {
    let mut c = cfg();
    let before = c.clone();
    let out = resolve(&mut c, Selectors::default()).expect("an unscoped apply resolves");
    assert_eq!(ids(&c), ids(&before), "same keys, same order");
    for id in ids(&before) {
        assert_eq!(deps(&c, id), deps(&before, id), "{id} depends_on is intact");
    }
    assert_eq!(
        out,
        Selection {
            total: 4,
            selected: 4,
            dependencies_added: 0,
            removed: Vec::new(),
            cut_edges: Vec::new(),
        }
    );
}

#[test]
fn exclude_contracts_the_edge_it_cuts() {
    let mut c = cfg();
    // The operator asked for a and asked b out. a still runs, and it must not
    // be left naming a resource that is no longer in the config.
    let out = resolve(
        &mut c,
        Selectors {
            subset: Some("a"),
            exclude: Some("b"),
            ..Default::default()
        },
    )
    .expect("an explicit exclusion is a decision, not an error");
    assert_eq!(ids(&c), vec!["a"]);
    assert!(deps(&c, "a").is_empty(), "the cut edge was contracted away");
    assert_eq!(out.removed, vec!["b".to_string()]);
    assert_eq!(out.cut_edges, vec![("a".to_string(), "b".to_string())]);
}

#[test]
fn skip_contracts_transitively_so_ordering_survives() {
    let mut c = chain();
    // a -> b -> c with b skipped must leave a -> c, or the apply loses the
    // ordering constraint that c comes first.
    let out = resolve(
        &mut c,
        Selectors {
            skip: Some("b"),
            ..Default::default()
        },
    )
    .expect("--skip resolves");
    assert_eq!(ids(&c), vec!["a", "c"]);
    assert_eq!(deps(&c, "a"), ["c".to_string()]);
    assert_eq!(out.cut_edges, vec![("a".to_string(), "b".to_string())]);
    assert_eq!(out.removed, vec!["b".to_string()]);
}

#[test]
fn only_machine_narrows_and_contracts() {
    let mut c = cfg();
    let out = resolve(
        &mut c,
        Selectors {
            only_machine: Some("other"),
            ..Default::default()
        },
    )
    .expect("--only-machine resolves");
    assert_eq!(ids(&c), vec!["c"]);
    assert_eq!(
        c.resources["c"].machine,
        types::MachineTarget::Single("other".to_string()),
        "the survivor targets only the requested machine"
    );
    assert_eq!(
        out.removed,
        vec!["a".to_string(), "b".to_string(), "d".to_string()]
    );
}

#[test]
fn an_undeclared_dependency_is_reported_before_any_pruning() {
    let mut c = cfg();
    c.resources["a"].depends_on = vec!["ghost".to_string()];
    let err = resolve(
        &mut c,
        Selectors {
            subset: Some("a"),
            ..Default::default()
        },
    )
    .expect_err("a dependency the file does not declare is still an error");
    assert!(err.contains("depends on unknown 'ghost'"), "{err}");
    assert_eq!(ids(&c), vec!["a", "b", "c", "d"], "validation runs first");
}

#[test]
fn a_subset_matching_nothing_is_an_error() {
    let mut c = cfg();
    let err = resolve(
        &mut c,
        Selectors {
            subset: Some("zz-*"),
            ..Default::default()
        },
    )
    .expect_err("a selector matching nothing is a typo, not a request for nothing");
    assert_eq!(err, "no resources match subset pattern 'zz-*'");
}

#[test]
fn a_resource_filter_matching_nothing_keeps_its_flag_prefix() {
    let mut c = cfg();
    let err = resolve(
        &mut c,
        Selectors {
            resource_filter: Some("zz-*"),
            ..Default::default()
        },
    )
    .expect_err("--resource-filter matching nothing is an error");
    assert_eq!(
        err,
        "--resource-filter: no resources match subset pattern 'zz-*'"
    );
}

#[test]
fn an_unknown_resource_selector_is_an_error() {
    let mut c = cfg();
    let err = resolve(
        &mut c,
        Selectors {
            resource: Some("zz"),
            ..Default::default()
        },
    )
    .expect_err("typo'd -r must fail");
    assert!(
        err.starts_with("--resource 'zz' matches no resource"),
        "{err}"
    );
    assert!(
        err.contains("a, b, c, d"),
        "the error names what exists: {err}"
    );
}

#[test]
fn an_unknown_skip_is_an_error() {
    let mut c = cfg();
    let err = resolve(
        &mut c,
        Selectors {
            skip: Some("zz"),
            ..Default::default()
        },
    )
    .expect_err("typo'd --skip must fail");
    assert!(err.starts_with("--skip 'zz' matches no resource"), "{err}");
}

#[test]
fn an_unknown_machine_is_an_error() {
    let mut c = cfg();
    let err = resolve(
        &mut c,
        Selectors {
            only_machine: Some("ghost"),
            ..Default::default()
        },
    )
    .expect_err("typo'd --only-machine must fail");
    assert!(err.contains("--only-machine 'ghost'"), "{err}");
}

#[test]
fn an_unknown_tag_is_an_error_and_a_known_tag_prunes_nothing() {
    let mut c = cfg();
    let err = resolve(
        &mut c,
        Selectors {
            tag: Some("nope"),
            ..Default::default()
        },
    )
    .expect_err("typo'd -t must fail");
    assert!(err.contains("--tag 'nope'"), "{err}");

    let mut c = cfg();
    c.resources["a"].tags = vec!["live".to_string()];
    let out = resolve(
        &mut c,
        Selectors {
            tag: Some("live"),
            ..Default::default()
        },
    )
    .expect("-t exists");
    // -t stays a plan-level filter: it is existence-checked here, never pruned.
    assert_eq!(ids(&c), vec!["a", "b", "c", "d"]);
    assert_eq!(out.selected, 4);
}

#[test]
fn intersecting_positive_selectors_that_match_nothing_name_the_selectors() {
    let mut c = cfg();
    let err = resolve(
        &mut c,
        Selectors {
            resource: Some("a"),
            subset: Some("c"),
            ..Default::default()
        },
    )
    .expect_err("an empty intersection is an error, not an empty success");
    assert!(err.contains("--resource 'a'"), "{err}");
    assert!(err.contains("--subset 'c'"), "{err}");
}

#[test]
fn goals_select_their_own_prerequisite_closure() {
    let mut c = cfg();
    let goals = vec!["a".to_string()];
    let out = resolve(
        &mut c,
        Selectors {
            goals: &goals,
            ..Default::default()
        },
    )
    .expect("a known goal resolves");
    assert_eq!(ids(&c), vec!["a", "b"]);
    assert_eq!(out.dependencies_added, 1);
}

#[test]
fn an_unknown_goal_keeps_makes_message() {
    let mut c = cfg();
    let goals = vec!["nope".to_string()];
    let err = resolve(
        &mut c,
        Selectors {
            goals: &goals,
            ..Default::default()
        },
    )
    .expect_err("an unknown goal is an error");
    assert!(err.starts_with("no rule to make target 'nope'"), "{err}");
}
