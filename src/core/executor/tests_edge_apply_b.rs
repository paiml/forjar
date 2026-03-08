//! Edge-case tests: apply variants (force, lock, tripwire, empty config).

use super::test_fixtures::*;
use super::*;

#[test]
fn test_fj012_apply_force_noop_reapplies() {
    // With force=true, even NoOp resources should be re-applied
    let config = local_config();
    let dir = tempfile::tempdir().unwrap();

    // First apply
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
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
    };
    let r1 = apply(&cfg).unwrap();
    assert_eq!(r1[0].resources_converged, 1);

    // Second apply with force → should converge again (not unchanged)
    let cfg2 = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: true,
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
    };
    let r2 = apply(&cfg2).unwrap();
    assert_eq!(
        r2[0].resources_converged, 1,
        "force should re-converge even NoOp resources"
    );
    assert_eq!(r2[0].resources_unchanged, 0);
    let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
}

#[test]
fn test_fj012_apply_lock_file_disabled() {
    // With policy.lock_file=false, no lock should be saved
    let yaml = r#"
version: "1.0"
name: no-lock-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/forjar-test-no-lock.txt
    content: "no lock"
policy:
  lock_file: false
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
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
    };
    let results = apply(&cfg).unwrap();
    assert_eq!(results[0].resources_converged, 1);

    // Lock should NOT exist
    let lock = state::load_lock(dir.path(), "local").unwrap();
    assert!(lock.is_none(), "lock_file=false should not save lock");
    let _ = std::fs::remove_file("/tmp/forjar-test-no-lock.txt");
}

#[test]
fn test_fj012_apply_tripwire_disabled_no_events() {
    // With policy.tripwire=false, no events.jsonl should be written
    let yaml = r#"
version: "1.0"
name: no-tripwire-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/forjar-test-no-tripwire.txt
    content: "no tripwire"
policy:
  tripwire: false
  lock_file: true
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
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
    };
    let results = apply(&cfg).unwrap();
    assert_eq!(results[0].resources_converged, 1);

    let events_path = dir.path().join("local").join("events.jsonl");
    assert!(
        !events_path.exists(),
        "tripwire=false should not create events.jsonl"
    );
    let _ = std::fs::remove_file("/tmp/forjar-test-no-tripwire.txt");
}

#[test]
fn test_fj012_collect_machines_empty_config() {
    // Config with no resources → no machines
    let yaml = r#"
version: "1.0"
name: empty
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources: {}
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let machines = collect_machines(&config);
    assert!(machines.is_empty(), "no resources means no machines");
}
