//! forjar#753: `apply -r X --refresh` must evaluate X's whole `depends_on`
//! closure, not only X, so a satisfied dependency with no lock entry is seeded
//! as converged instead of being planned as `create`.

use super::*;
use std::collections::HashMap;

const YAML: &str = r#"
version: "1.0"
name: refresh-closure
machines:
  localhost:
    hostname: localhost
    addr: 127.0.0.1
resources:
  dep-leaf:
    type: task
    machine: localhost
    command: "false"
    completion_check: "true"
  dep-mid:
    type: task
    machine: localhost
    depends_on: [dep-leaf]
    command: "false"
    completion_check: "true"
  dep-nocheck:
    type: task
    machine: localhost
    command: "true"
  unrelated:
    type: task
    machine: localhost
    command: "false"
    completion_check: "true"
  target:
    type: task
    machine: localhost
    depends_on: [dep-mid, dep-nocheck]
    command: "false"
    completion_check: "true"
"#;

fn refreshed_ids(filter: Option<&str>) -> Vec<String> {
    refreshed_ids_of(YAML, filter)
}

fn refreshed_ids_of(yaml: &str, filter: Option<&str>) -> Vec<String> {
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: true,
        machine_filter: None,
        resource_filter: filter,
        tag_filter: None,
        group_filter: None,
        timeout_secs: None,
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: None,
        refresh: true,
        force_tag: None,
    };
    let out = refresh_locks(&cfg, &HashMap::new());
    let mut ids: Vec<String> = out["localhost"].resources.keys().cloned().collect();
    ids.sort();
    ids
}

#[test]
fn refresh_under_r_seeds_the_depends_on_closure() {
    let ids = refreshed_ids(Some("target"));
    assert!(ids.contains(&"target".to_string()), "{ids:?}");
    assert!(ids.contains(&"dep-mid".to_string()), "{ids:?}");
    assert!(ids.contains(&"dep-leaf".to_string()), "transitive: {ids:?}");
}

#[test]
fn refresh_under_r_leaves_non_closure_and_checkless_deps_alone() {
    let ids = refreshed_ids(Some("target"));
    assert!(!ids.contains(&"unrelated".to_string()), "{ids:?}");
    assert!(!ids.contains(&"dep-nocheck".to_string()), "{ids:?}");
}

#[test]
fn refresh_without_r_is_unchanged() {
    let ids = refreshed_ids(None);
    assert!(ids.contains(&"unrelated".to_string()), "{ids:?}");
}

/// forjar#590: a guard whose `completion_check` carries a `{{params.*}}`
/// template (its `command` is plain) must be seeded as converged exactly like the
/// template-free guard beside it. Reported on 1.31.0: one template flipped the
/// resource from `unchanged` to `FAILED`, so every templated guard was
/// permanently RED while satisfied. The two resources differ ONLY by the
/// template, so a regression that makes template resolution count as "check
/// failed" drops `templated` and fails this test.
#[test]
fn refresh_seeds_a_templated_guard_like_a_plain_one() {
    let yaml = r#"
version: "1.0"
name: refresh-templated
params:
  root: /
machines:
  localhost:
    hostname: localhost
    addr: 127.0.0.1
resources:
  plain:
    type: task
    machine: localhost
    command: "false"
    completion_check: "[ -d / ]"
  templated:
    type: task
    machine: localhost
    command: "false"
    completion_check: "[ -d {{params.root}} ]"
"#;
    let ids = refreshed_ids_of(yaml, None);
    assert!(ids.contains(&"plain".to_string()), "{ids:?}");
    assert!(ids.contains(&"templated".to_string()), "#590: {ids:?}");
}

/// The negative control: a templated check that FAILS must not be seeded.
#[test]
fn refresh_does_not_seed_a_templated_guard_whose_check_fails() {
    let yaml = r#"
version: "1.0"
name: refresh-templated-fail
params:
  root: /definitely/not/a/dir
machines:
  localhost:
    hostname: localhost
    addr: 127.0.0.1
resources:
  templated:
    type: task
    machine: localhost
    command: "false"
    completion_check: "[ -d {{params.root}} ]"
"#;
    let ids = refreshed_ids_of(yaml, None);
    assert!(!ids.contains(&"templated".to_string()), "{ids:?}");
}
