//! FJ-2725 (PMAT-199): goal-only phony semantics.

use super::apply_selection::strip_unrequested_phony;
use crate::core::types::{ForjarConfig, Resource};

fn config() -> ForjarConfig {
    let mut c = ForjarConfig::default();
    let mut clean = Resource {
        phony: true,
        command: Some("rm -rf build".to_string()),
        ..Default::default()
    };
    clean.name = Some("clean".to_string());
    c.resources.insert("clean".to_string(), clean);

    let build = Resource {
        command: Some("cc -o build/app a.c".to_string()),
        output_artifacts: vec!["build/app".to_string()],
        depends_on: vec!["clean".to_string(), "mkdir".to_string()],
        ..Default::default()
    };
    c.resources.insert("build".to_string(), build);
    c.resources
        .insert("mkdir".to_string(), Resource::default());
    c
}

#[test]
fn a_bulk_apply_drops_every_phony_resource() {
    // The naive reading of make — "phony runs on every apply" — would make
    // `plan` show a perpetual change and, through propagation, rebuild the
    // whole transitive closure every time.
    let mut c = config();
    strip_unrequested_phony(&mut c, &[]);
    assert!(!c.resources.contains_key("clean"));
    assert!(c.resources.contains_key("build"));
    assert!(c.resources.contains_key("mkdir"));
}

#[test]
fn dropping_a_phony_resource_scrubs_edges_to_it() {
    // A dangling `depends_on` would make build_execution_order fail with an
    // unknown-dependency error, so the edge must go with the node. Dropping
    // the edge IS the "ordering only, never auto-run" rule.
    let mut c = config();
    strip_unrequested_phony(&mut c, &[]);
    assert_eq!(c.resources["build"].depends_on, vec!["mkdir".to_string()]);
}

#[test]
fn an_explicitly_requested_phony_resource_is_kept() {
    let mut c = config();
    strip_unrequested_phony(&mut c, &["clean".to_string()]);
    assert!(c.resources.contains_key("clean"));
    assert_eq!(
        c.resources["build"].depends_on,
        vec!["clean".to_string(), "mkdir".to_string()],
        "its edges survive when it does"
    );
}

#[test]
fn a_phony_prerequisite_is_not_pulled_in_by_another_goal() {
    // `make build` must not run `clean` just because build lists it. Requesting
    // build is not requesting the action clean.
    let mut c = config();
    strip_unrequested_phony(&mut c, &["build".to_string()]);
    assert!(!c.resources.contains_key("clean"));
}

#[test]
fn non_phony_resources_are_never_touched() {
    let mut c = ForjarConfig::default();
    c.resources.insert("a".to_string(), Resource::default());
    let before = c.resources.len();
    strip_unrequested_phony(&mut c, &[]);
    assert_eq!(c.resources.len(), before);
}

#[test]
fn a_requested_phony_resource_always_plans_as_a_change() {
    // Even with a converged lock entry whose hash matches, because it names an
    // action rather than a state. This is what makes `forjar make test` run the
    // tests a second time.
    use crate::core::planner;
    use crate::core::types::StateLock;

    let mut c: ForjarConfig = serde_yaml_ng::from_str(
        r#"
version: "1.0"
name: phony-plan
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  test:
    type: task
    machine: local
    phony: true
    command: "cargo test"
"#,
    )
    .expect("fixture parses");

    let r = c.resources["test"].clone();
    let hash = planner::hash_desired_state(&r);
    // Build the lock through serde so the test does not depend on the shape of
    // StateLock/ResourceLock, neither of which implements Default.
    let lock: StateLock = serde_yaml_ng::from_str(&format!(
        "schema: \"1.0\"\nmachine: local\nhostname: localhost\ngenerated_at: \"now\"\n\
         generator: test\nblake3_version: \"1\"\nresources:\n  test:\n    type: task\n\
         \x20   status: converged\n    hash: \"{hash}\"\n"
    ))
    .expect("lock fixture parses");
    let mut locks = std::collections::HashMap::new();
    locks.insert("local".to_string(), lock);

    let plan = planner::plan(&c, &["test".to_string()], &locks, None);
    assert_eq!(
        plan.changes.len(),
        1,
        "a converged, hash-matching phony must still be planned: {:?}",
        plan.changes
    );
    assert!(
        !format!("{:?}", plan.changes[0].action).contains("NoOp"),
        "requesting an action by name means running it, got {:?}",
        plan.changes[0].action
    );

    // And the same resource, unrequested, disappears entirely.
    strip_unrequested_phony(&mut c, &[]);
    assert!(c.resources.is_empty(), "bulk apply must not see it");
}

fn validation_errors(resources_yaml: &str) -> Result<(), String> {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("forjar.yaml");
    std::fs::write(
        &f,
        format!(
            "version: \"1.0\"\nname: v\nmachines:\n  local:\n    hostname: localhost\n\
             \x20   addr: localhost\nresources:\n{resources_yaml}"
        ),
    )
    .unwrap();
    super::helpers::parse_and_validate(&f).map(|_| ())
}

#[test]
fn a_phony_resource_may_have_no_command() {
    // make allows both a grouping target (`all: app` — prerequisites, no
    // recipe) and a bare `.PHONY` name with no rule at all, where `make <name>`
    // prints "Nothing to be done". Found by importing forjar's OWN Makefile,
    // which lists a stale `deny` in .PHONY with no rule.
    let grouping = "  app:\n    type: task\n    machine: local\n    command: \"cc\"\n\
                    \x20 all:\n    type: task\n    machine: local\n    phony: true\n\
                    \x20   depends_on: [app]\n";
    assert!(
        validation_errors(grouping).is_ok(),
        "a phony grouping node needs no command: {:?}",
        validation_errors(grouping)
    );

    let bare = "  deny:\n    type: task\n    machine: local\n    phony: true\n";
    assert!(
        validation_errors(bare).is_ok(),
        "a bare .PHONY name with no rule is legal in make: {:?}",
        validation_errors(bare)
    );
}

#[test]
fn a_non_phony_task_still_requires_a_command() {
    // The guard must stay narrow, or every misconfigured task silently passes.
    let err = validation_errors("  t:\n    type: task\n    machine: local\n")
        .expect_err("a non-phony task with no command is invalid");
    assert!(err.contains("has no command"), "{err}");
}
