//! forjar#590: `--refresh` seeding must hash and check the RESOLVED resource,
//! and must not fold "could not measure" into "the check failed".

use super::refresh_seed::{check_on, converged_entry, CheckVerdict};
use super::*;

fn config(check: &str) -> ForjarConfig {
    let yaml = format!(
        r#"
version: "1.0"
name: fj590-unit
params:
  dir: /
machines:
  localhost:
    hostname: localhost
    addr: localhost
resources:
  guard:
    type: task
    machine: localhost
    command: "touch {{{{params.dir}}}}tmp/fj590-unit-never-run"
    completion_check: "{check}"
"#
    );
    serde_yaml_ng::from_str(&yaml).unwrap()
}

fn apply_cfg<'a>(config: &'a ForjarConfig, dir: &'a std::path::Path) -> ApplyConfig<'a> {
    ApplyConfig {
        config,
        state_dir: dir,
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: None,
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
    }
}

/// The seeded entry's hash is the planner's hash of the RESOLVED resource — and
/// that is a different value from the raw one, so this cannot pass vacuously.
#[test]
fn seeded_entry_is_hashed_over_the_resolved_resource() {
    let config = config("[ -d {{params.dir}} ]");
    let dir = tempfile::tempdir().unwrap();
    let cfg = apply_cfg(&config, dir.path());
    let raw = &config.resources["guard"];

    let CheckVerdict::Passed(resolved) = check_on(&cfg, raw, "localhost") else {
        panic!("`[ -d / ]` must pass on any Linux host");
    };
    assert!(
        resolved
            .command
            .as_deref()
            .is_some_and(|c| !c.contains("{{")),
        "Passed must carry the resolved resource, got {:?}",
        resolved.command
    );

    let raw_hash = planner::hash_desired_state(raw);
    let resolved_hash = planner::hash_desired_state(&resolved);
    assert_ne!(raw_hash, resolved_hash, "fixture must contain a template");

    let entry = converged_entry(&resolved);
    assert_eq!(
        entry.hash, resolved_hash,
        "forjar#590: a seeded entry hashed over the raw config never matches \
         the planner's desired hash, so the guard is re-planned as an Update"
    );
}

#[test]
fn a_failing_check_is_failed() {
    let config = config("[ -d {{params.dir}}nonexistent-fj590 ]");
    let dir = tempfile::tempdir().unwrap();
    let cfg = apply_cfg(&config, dir.path());
    assert!(matches!(
        check_on(&cfg, &config.resources["guard"], "localhost"),
        CheckVerdict::Failed
    ));
}

/// "Could not measure" is its own verdict, with a reason — not `Failed`.
#[test]
fn an_unknown_machine_is_unmeasured_not_failed() {
    let config = config("[ -d / ]");
    let dir = tempfile::tempdir().unwrap();
    let cfg = apply_cfg(&config, dir.path());
    match check_on(&cfg, &config.resources["guard"], "nosuch") {
        CheckVerdict::Unmeasured(why) => assert!(why.contains("nosuch"), "{why}"),
        _ => panic!("an unknown machine was not measured, and must say so"),
    }
}

#[test]
fn a_machine_outside_the_filter_is_out_of_scope() {
    let config = config("[ -d / ]");
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = apply_cfg(&config, dir.path());
    cfg.machine_filter = Some("other");
    assert!(matches!(
        check_on(&cfg, &config.resources["guard"], "localhost"),
        CheckVerdict::OutOfScope
    ));
}
