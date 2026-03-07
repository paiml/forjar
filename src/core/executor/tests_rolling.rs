//! FJ-222: Rolling deploy tests.

use super::*;

// ========================================================================
// FJ-222: Rolling deploys
// ========================================================================

#[test]
fn test_fj222_serial_batches_machines() {
    let yaml = r#"
version: "1.0"
name: rolling-test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
  m2:
    hostname: m2
    addr: 127.0.0.1
  m3:
    hostname: m3
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: m1
    path: /tmp/forjar-rolling-m1.txt
    content: "m1"
  f2:
    type: file
    machine: m2
    path: /tmp/forjar-rolling-m2.txt
    content: "m2"
  f3:
    type: file
    machine: m3
    path: /tmp/forjar-rolling-m3.txt
    content: "m3"
policy:
  serial: 2
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.policy.serial, Some(2));

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
    // All 3 machines should converge (2 in first batch, 1 in second)
    assert_eq!(results.len(), 3);
    let total: u32 = results.iter().map(|r| r.resources_converged).sum();
    assert_eq!(total, 3);

    let _ = std::fs::remove_file("/tmp/forjar-rolling-m1.txt");
    let _ = std::fs::remove_file("/tmp/forjar-rolling-m2.txt");
    let _ = std::fs::remove_file("/tmp/forjar-rolling-m3.txt");
}

#[test]
fn test_fj222_serial_with_parallel() {
    // serial + parallel_machines: batches run in parallel
    let yaml = r#"
version: "1.0"
name: rolling-parallel
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
  m2:
    hostname: m2
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: m1
    path: /tmp/forjar-rp-m1.txt
    content: "m1"
  f2:
    type: file
    machine: m2
    path: /tmp/forjar-rp-m2.txt
    content: "m2"
policy:
  serial: 2
  parallel_machines: true
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.policy.serial, Some(2));
    assert!(config.policy.parallel_machines);

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
    assert_eq!(results.len(), 2);

    let _ = std::fs::remove_file("/tmp/forjar-rp-m1.txt");
    let _ = std::fs::remove_file("/tmp/forjar-rp-m2.txt");
}

#[test]
fn test_fj222_max_fail_percentage_yaml() {
    let yaml = r#"
version: "1.0"
name: fail-pct
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m1
    path: /tmp/forjar-pct.txt
    content: "ok"
policy:
  serial: 1
  max_fail_percentage: 50
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.policy.max_fail_percentage, Some(50));
    assert_eq!(config.policy.serial, Some(1));

    // With one machine and no failures, this should succeed
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
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].resources_failed, 0);

    let _ = std::fs::remove_file("/tmp/forjar-pct.txt");
}

#[test]
fn test_fj222_serial_default_none() {
    let yaml = r#"
version: "1.0"
name: no-serial
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m1
    path: /tmp/forjar-nosrl.txt
    content: "x"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.policy.serial, None);
    assert_eq!(config.policy.max_fail_percentage, None);

    let _ = std::fs::remove_file("/tmp/forjar-nosrl.txt");
}

#[test]
fn test_fj222_serial_one_is_sequential() {
    // serial: 1 means one machine at a time (fully sequential)
    let yaml = r#"
version: "1.0"
name: serial-one
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
  m2:
    hostname: m2
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: m1
    path: /tmp/forjar-s1-m1.txt
    content: "m1"
  f2:
    type: file
    machine: m2
    path: /tmp/forjar-s1-m2.txt
    content: "m2"
policy:
  serial: 1
  parallel_machines: true
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    // serial:1 with parallel_machines:true — batches of 1, so effectively sequential
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
    assert_eq!(results.len(), 2);

    let _ = std::fs::remove_file("/tmp/forjar-s1-m1.txt");
    let _ = std::fs::remove_file("/tmp/forjar-s1-m2.txt");
}
