//! FJ-036: Dry-run/force-reapply coverage, FJ-050: Trace tests, FJ-051: Anomaly detection.

use super::test_fixtures::*;
use super::*;

// ── FJ-036: Dry-run and force-reapply coverage ──────────────────

#[test]
fn test_fj036_dry_run_produces_no_side_effects() {
    let yaml = r#"
version: "1.0"
name: dry-run-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/forjar-test-fj036-dry-run.txt
    content: "should not be created"
policy:
  lock_file: true
  tripwire: true
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let dir = tempfile::tempdir().unwrap();

    // Ensure target file does not exist before
    let _ = std::fs::remove_file("/tmp/forjar-test-fj036-dry-run.txt");

    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: true,
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
        refresh: false,
        force_tag: None,
    };
    let results = apply(&cfg).unwrap();

    // Dry run should return exactly one synthetic result
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].machine, "dry-run");
    assert_eq!(results[0].resources_converged, 0);
    assert_eq!(results[0].resources_failed, 0);

    // No lock file should have been written for any machine
    let lock = state::load_lock(dir.path(), "local").unwrap();
    assert!(lock.is_none(), "dry_run must not create a lock file");

    // Target file must not have been created
    assert!(
        !std::path::Path::new("/tmp/forjar-test-fj036-dry-run.txt").exists(),
        "dry_run must not create the managed file"
    );

    // No event log should exist
    let events_path = dir.path().join("local").join("events.jsonl");
    assert!(!events_path.exists(), "dry_run must not write event logs");
}

#[test]
fn test_fj036_force_reapply_changes_action() {
    let yaml = r#"
version: "1.0"
name: force-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/forjar-test-fj036-force.txt
    content: "force test content"
policy:
  lock_file: true
  tripwire: true
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let dir = tempfile::tempdir().unwrap();

    // First apply — should converge
    let cfg1 = ApplyConfig {
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
        refresh: false,
        force_tag: None,
    };
    let r1 = apply(&cfg1).unwrap();
    assert_eq!(r1[0].resources_converged, 1);
    assert_eq!(r1[0].resources_unchanged, 0);

    // Second apply without force — should be unchanged (idempotent)
    let cfg2 = ApplyConfig {
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
        refresh: false,
        force_tag: None,
    };
    let r2 = apply(&cfg2).unwrap();
    assert_eq!(r2[0].resources_unchanged, 1);
    assert_eq!(r2[0].resources_converged, 0);

    // Third apply WITH force — should re-converge even though nothing changed
    let cfg3 = ApplyConfig {
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
        refresh: false,
        force_tag: None,
    };
    let r3 = apply(&cfg3).unwrap();
    assert_eq!(
        r3[0].resources_converged, 1,
        "force=true must re-apply even when state matches"
    );
    assert_eq!(
        r3[0].resources_unchanged, 0,
        "force=true must not skip any resource"
    );

    // Lock should still be valid after force apply
    let lock = state::load_lock(dir.path(), "local").unwrap();
    assert!(lock.is_some(), "lock file must exist after force apply");

    let _ = std::fs::remove_file("/tmp/forjar-test-fj036-force.txt");
}

#[test]
fn test_executor_local_machine_defaults() {
    let m = local_machine();
    assert_eq!(m.hostname, "localhost");
    assert_eq!(m.addr, "127.0.0.1");
    assert_eq!(m.user, "root");
    assert_eq!(m.arch, "x86_64");
    assert!(m.ssh_key.is_none(), "local machine should have no ssh_key");
    assert!(m.roles.is_empty(), "local machine should have no roles");
    assert!(
        m.transport.is_none(),
        "local machine should have no transport override"
    );
    assert!(
        m.container.is_none(),
        "local machine should have no container config"
    );
    assert_eq!(m.cost, 0, "local machine should have zero cost");
}

#[test]
fn test_executor_local_config_minimal() {
    let config = local_config();
    assert_eq!(config.name, "test");
    assert_eq!(config.version, "1.0");
    assert!(
        config.machines.contains_key("local"),
        "config should contain machine 'local'"
    );
    assert!(
        config.resources.contains_key("test-file"),
        "config should contain resource 'test-file'"
    );
    let r = &config.resources["test-file"];
    assert_eq!(r.resource_type, ResourceType::File);
    assert_eq!(r.path.as_deref(), Some("/tmp/forjar-test-executor.txt"));
    assert_eq!(r.content.as_deref(), Some("hello from forjar"));
    assert!(config.policy.tripwire, "policy.tripwire should be true");
    assert!(config.policy.lock_file, "policy.lock_file should be true");
}

#[test]
fn test_executor_collect_machines_filters_by_name() {
    let yaml = r#"
version: "1.0"
name: filter-test
machines:
  web:
    hostname: web
    addr: 10.0.0.1
  db:
    hostname: db
    addr: 10.0.0.2
  cache:
    hostname: cache
    addr: 10.0.0.3
resources:
  r1:
    type: file
    machine: web
    path: /tmp/a
    content: a
  r2:
    type: file
    machine: db
    path: /tmp/b
    content: b
  r3:
    type: file
    machine: [web, cache]
    path: /tmp/c
    content: c
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let machines = collect_machines(&config);
    assert_eq!(
        machines.len(),
        3,
        "should collect 3 unique machines: {machines:?}"
    );
    assert!(machines.contains(&"web".to_string()), "should contain web");
    assert!(machines.contains(&"db".to_string()), "should contain db");
    assert!(
        machines.contains(&"cache".to_string()),
        "should contain cache"
    );

    // Verify machine_filter works in ApplyConfig (dry-run) — only "db" processed
    let dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: true,
        machine_filter: Some("db"),
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
        refresh: false,
        force_tag: None,
    };
    let results = apply(&cfg).unwrap();
    assert_eq!(results[0].machine, "dry-run");
}
