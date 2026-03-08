//! FJ-129: Integration tests — apply, drift, re-apply cycle.

use super::test_fixtures::*;
use super::*;

#[test]
fn test_fj129_apply_then_drift_no_change() {
    // Apply a file, then check drift — should find no drift
    let tmp = tempfile::tempdir().unwrap();
    let file_path = tmp.path().join("drift-no-change.txt");
    let config = drift_config(file_path.to_str().unwrap());
    let state_dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: state_dir.path(),
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
    apply(&cfg).unwrap();

    // Load lock and check drift
    let lock = state::load_lock(state_dir.path(), "local")
        .unwrap()
        .unwrap();
    let findings = crate::tripwire::drift::detect_drift(&lock);
    assert!(
        findings.is_empty(),
        "no drift expected immediately after apply"
    );
}

#[test]
fn test_fj129_apply_then_drift_after_modification() {
    // Apply a file, modify it externally, then check drift
    let tmp = tempfile::tempdir().unwrap();
    let file_path = tmp.path().join("drift-tampered.txt");
    let config = drift_config(file_path.to_str().unwrap());
    let state_dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: state_dir.path(),
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
    apply(&cfg).unwrap();

    // Tamper with the file
    std::fs::write(&file_path, "tampered content").unwrap();

    // Drift detection should find the change
    let lock = state::load_lock(state_dir.path(), "local")
        .unwrap()
        .unwrap();
    let findings = crate::tripwire::drift::detect_drift(&lock);
    assert_eq!(
        findings.len(),
        1,
        "should detect drift after file modification"
    );
    assert_eq!(findings[0].resource_id, "test-file");
    assert!(findings[0].detail.contains("content changed"));
}

#[test]
fn test_fj129_apply_drift_reapply_cycle() {
    // Full cycle: apply → drift (no change) → tamper → drift (found) → re-apply → drift (no change)
    let tmp = tempfile::tempdir().unwrap();
    let file_path = tmp.path().join("drift-cycle.txt");
    let config = drift_config(file_path.to_str().unwrap());
    let state_dir = tempfile::tempdir().unwrap();

    // Step 1: Initial apply
    let cfg = ApplyConfig {
        config: &config,
        state_dir: state_dir.path(),
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
    let r1 = apply(&cfg).unwrap();
    assert_eq!(r1[0].resources_converged, 1);

    // Step 2: Verify no drift
    let lock1 = state::load_lock(state_dir.path(), "local")
        .unwrap()
        .unwrap();
    assert!(crate::tripwire::drift::detect_drift(&lock1).is_empty());

    // Step 3: Tamper
    std::fs::write(&file_path, "unauthorized change").unwrap();
    let findings = crate::tripwire::drift::detect_drift(&lock1);
    assert_eq!(findings.len(), 1);

    // Step 4: Re-apply (force to overwrite tampered file)
    let cfg2 = ApplyConfig {
        config: &config,
        state_dir: state_dir.path(),
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
    let r2 = apply(&cfg2).unwrap();
    assert_eq!(r2[0].resources_converged, 1);

    // Step 5: Verify no drift after re-apply
    let lock2 = state::load_lock(state_dir.path(), "local")
        .unwrap()
        .unwrap();
    assert!(
        crate::tripwire::drift::detect_drift(&lock2).is_empty(),
        "no drift expected after re-apply"
    );
}

#[test]
fn test_fj129_multi_resource_dependency_order() {
    // Verify that dependent resources are applied in correct order
    let yaml = r#"
version: "1.0"
name: dep-order
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  dir-first:
    type: file
    machine: local
    state: directory
    path: /tmp/forjar-test-dep-order
    mode: "0755"
  file-second:
    type: file
    machine: local
    path: /tmp/forjar-test-dep-order/config.txt
    content: "depends on dir"
    depends_on: [dir-first]
policy:
  lock_file: true
  tripwire: true
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
        refresh: false,
        force_tag: None,
    };
    let results = apply(&cfg).unwrap();
    assert_eq!(results[0].resources_converged, 2);

    // Verify both artifacts exist
    assert!(std::path::Path::new("/tmp/forjar-test-dep-order").is_dir());
    assert!(std::path::Path::new("/tmp/forjar-test-dep-order/config.txt").exists());
    let content = std::fs::read_to_string("/tmp/forjar-test-dep-order/config.txt").unwrap();
    assert_eq!(content.trim(), "depends on dir");

    // Idempotency check
    let r2 = apply(&cfg).unwrap();
    assert_eq!(r2[0].resources_unchanged, 2);

    // Clean up
    let _ = std::fs::remove_dir_all("/tmp/forjar-test-dep-order");
}

#[test]
fn test_fj129_config_change_triggers_update() {
    // Apply config A, then apply config B (different content), verify UPDATE
    let yaml_a = r#"
version: "1.0"
name: change-detect
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  changeable:
    type: file
    machine: local
    path: /tmp/forjar-test-change-detect.txt
    content: "version A"
policy:
  lock_file: true
"#;
    let yaml_b = r#"
version: "1.0"
name: change-detect
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  changeable:
    type: file
    machine: local
    path: /tmp/forjar-test-change-detect.txt
    content: "version B"
policy:
  lock_file: true
"#;
    let dir = tempfile::tempdir().unwrap();

    // Apply version A
    let config_a: ForjarConfig = serde_yaml_ng::from_str(yaml_a).unwrap();
    let cfg_a = ApplyConfig {
        config: &config_a,
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
    let r1 = apply(&cfg_a).unwrap();
    assert_eq!(r1[0].resources_converged, 1);
    assert_eq!(
        std::fs::read_to_string("/tmp/forjar-test-change-detect.txt")
            .unwrap()
            .trim(),
        "version A"
    );

    // Apply version B (content changed) — should detect update
    let config_b: ForjarConfig = serde_yaml_ng::from_str(yaml_b).unwrap();
    let cfg_b = ApplyConfig {
        config: &config_b,
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
    let r2 = apply(&cfg_b).unwrap();
    assert_eq!(
        r2[0].resources_converged, 1,
        "changed content should trigger re-apply"
    );
    assert_eq!(
        std::fs::read_to_string("/tmp/forjar-test-change-detect.txt")
            .unwrap()
            .trim(),
        "version B"
    );

    let _ = std::fs::remove_file("/tmp/forjar-test-change-detect.txt");
}

#[test]
fn test_fj129_event_log_full_lifecycle() {
    // Verify event log records full lifecycle: started → resource_started → converged → completed
    let config = local_config();
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
    apply(&cfg).unwrap();

    let events_path = dir.path().join("local").join("events.jsonl");
    let content = std::fs::read_to_string(&events_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // Should have at least 3 events: apply_started, resource_started, resource_converged, apply_completed
    assert!(
        lines.len() >= 3,
        "expected at least 3 events, got {}",
        lines.len()
    );

    // Verify event types appear in order
    assert!(
        lines[0].contains("apply_started"),
        "first event should be apply_started"
    );
    assert!(
        content.contains("resource_started"),
        "should contain resource_started"
    );
    assert!(
        content.contains("resource_converged"),
        "should contain resource_converged"
    );
    assert!(
        lines.last().unwrap().contains("apply_completed"),
        "last event should be apply_completed"
    );

    let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
}
