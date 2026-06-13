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
        refresh: false,
        force_tag: None,
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
        refresh: false,
        force_tag: None,
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
        refresh: false,
        force_tag: None,
    };
    let results = apply(&cfg).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].resources_failed, 0);

    let _ = std::fs::remove_file("/tmp/forjar-pct.txt");
}

// ========================================================================
// FJ-154 / #21: max_fail_percentage gate uses exact integer math (no `as u8`
// truncation that floored fractional rates past the gate).
// ========================================================================

#[test]
fn test_fj154_rolling_gate_boundary_table() {
    use super::strategies::{fail_percentage, rolling_fail_gate_exceeded};

    // (failed, total, max_pct, expect_abort, expect_display_pct)
    // Gate semantics preserved: abort only when the TRUE ratio strictly
    // exceeds max_pct (a rate of exactly max_pct% does NOT abort).
    let cases: &[(usize, usize, u8, bool, u8)] = &[
        // Exactly at the threshold → continue (50% == 50, not > 50).
        (1, 2, 50, false, 50),
        // 1/3 = 33.33% with max 33 → the old `as u8` floored to 33 and let it
        // pass; integer math sees 33.33% > 33% → MUST abort.
        (1, 3, 33, true, 33),
        // 2/3 = 66.66% with max 66 → old code floored to 66 (pass); fixed
        // code aborts (66.66% > 66%).
        (2, 3, 66, true, 67),
        // 49.x boundary: 49/100 = 49% with a `> 50` gate (max 50) → continue.
        (49, 100, 50, false, 49),
        // 50/100 = exactly 50% with max 50 → continue (not strictly greater).
        (50, 100, 50, false, 50),
        // 51/100 = 51% with max 50 → abort.
        (51, 100, 50, true, 51),
        // Near-total failure just under an integer threshold: 996/1000 =
        // 99.6% with max 99 → old code floored to 99 (pass, gate defeated);
        // fixed code aborts.
        (996, 1000, 99, true, 100),
        // Zero failures never abort.
        (0, 10, 0, false, 0),
        // Empty machine set is a safe no-abort.
        (0, 0, 0, false, 0),
    ];

    for &(failed, total, max_pct, expect_abort, expect_pct) in cases {
        assert_eq!(
            rolling_fail_gate_exceeded(failed, total, max_pct),
            expect_abort,
            "gate({failed}/{total} vs {max_pct}%) expected abort={expect_abort}"
        );
        assert_eq!(
            fail_percentage(failed, total),
            expect_pct,
            "display pct for {failed}/{total}"
        );
    }
}

#[test]
fn test_fj154_rolling_gate_no_lossy_truncation() {
    use super::strategies::rolling_fail_gate_exceeded;
    // Regression: the old `(failed/total*100.0) as u8` truncated 33.9% → 33,
    // making a `> 33` gate pass. Confirm the true sub-1% margin now triggers.
    assert!(
        rolling_fail_gate_exceeded(339, 1000, 33),
        "33.9% must exceed a 33% gate"
    );
    assert!(
        !rolling_fail_gate_exceeded(330, 1000, 33),
        "exactly 33.0% must NOT exceed a 33% gate"
    );
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
        refresh: false,
        force_tag: None,
    };
    let results = apply(&cfg).unwrap();
    assert_eq!(results.len(), 2);

    let _ = std::fs::remove_file("/tmp/forjar-s1-m1.txt");
    let _ = std::fs::remove_file("/tmp/forjar-s1-m2.txt");
}
