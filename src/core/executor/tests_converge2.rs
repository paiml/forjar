//! FJ-132: Executor edge case tests (part 2).

use super::*;

// --- FJ-132: Executor edge case tests ---

#[test]
fn test_fj132_apply_idempotent_second_run() {
    // Second apply with same config should have 0 converged (all unchanged)
    let output_dir = tempfile::tempdir().unwrap();
    let file_path = output_dir.path().join("idempotent.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: idempotent-test
machines: {{}}
resources:
  test-file:
    type: file
    machine: localhost
    path: "{}"
    content: "stable"
policy:
  lock_file: true
  tripwire: false
"#,
        file_path.display()
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let state_dir = tempfile::tempdir().unwrap();

    // First apply
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
    };
    let r1 = apply(&cfg).unwrap();
    assert_eq!(r1[0].resources_converged, 1);

    // Second apply — should be unchanged
    let r2 = apply(&cfg).unwrap();
    assert_eq!(r2[0].resources_unchanged, 1);
    assert_eq!(r2[0].resources_converged, 0);
}

#[test]
fn test_fj132_machine_filter_skips_non_matching() {
    // Machine filter should skip machines that don't match
    let output_dir = tempfile::tempdir().unwrap();
    let file_path = output_dir.path().join("machine-filter.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: machine-filter-test
machines: {{}}
resources:
  test-file:
    type: file
    machine: localhost
    path: "{}"
    content: "test"
policy:
  lock_file: true
  tripwire: false
"#,
        file_path.display()
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: state_dir.path(),
        force: false,
        dry_run: false,
        machine_filter: Some("nonexistent-machine"),
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
    };
    let results = apply(&cfg).unwrap();
    // No results for non-matching machine
    assert!(
        results.is_empty(),
        "no results expected for non-matching machine filter"
    );
    assert!(!file_path.exists(), "file should not be created");
}

#[test]
fn test_fj132_apply_multiple_files_all_converge() {
    // Multiple file resources should all converge
    let output_dir = tempfile::tempdir().unwrap();
    let p1 = output_dir.path().join("multi-1.txt");
    let p2 = output_dir.path().join("multi-2.txt");
    let p3 = output_dir.path().join("multi-3.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: multi-file
machines: {{}}
resources:
  file-1:
    type: file
    machine: localhost
    path: "{}"
    content: "one"
  file-2:
    type: file
    machine: localhost
    path: "{}"
    content: "two"
  file-3:
    type: file
    machine: localhost
    path: "{}"
    content: "three"
policy:
  lock_file: true
  tripwire: false
"#,
        p1.display(),
        p2.display(),
        p3.display()
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
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
    };
    let results = apply(&cfg).unwrap();
    assert_eq!(results[0].resources_converged, 3);
    assert_eq!(std::fs::read_to_string(&p1).unwrap().trim(), "one");
    assert_eq!(std::fs::read_to_string(&p2).unwrap().trim(), "two");
    assert_eq!(std::fs::read_to_string(&p3).unwrap().trim(), "three");
}

#[test]
fn test_fj132_apply_result_has_duration() {
    // ApplyResult should have a non-zero total_duration
    let output_dir = tempfile::tempdir().unwrap();
    let file_path = output_dir.path().join("duration-test.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: duration-test
machines: {{}}
resources:
  test-file:
    type: file
    machine: localhost
    path: "{}"
    content: "test"
policy:
  lock_file: true
  tripwire: false
"#,
        file_path.display()
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
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
    };
    let results = apply(&cfg).unwrap();
    assert!(
        results[0].total_duration.as_nanos() > 0,
        "apply should take some non-zero time"
    );
}

#[test]
fn test_fj132_force_apply_reconverges_unchanged() {
    // Force apply should re-apply even when hash matches (second run)
    let output_dir = tempfile::tempdir().unwrap();
    let file_path = output_dir.path().join("force-reconverge.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: force-test
machines: {{}}
resources:
  test-file:
    type: file
    machine: localhost
    path: "{}"
    content: "force-me"
policy:
  lock_file: true
  tripwire: false
"#,
        file_path.display()
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let state_dir = tempfile::tempdir().unwrap();

    // First apply
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
    };
    apply(&cfg).unwrap();

    // Second apply with force=true
    let cfg_force = ApplyConfig {
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
    };
    let results = apply(&cfg_force).unwrap();
    assert_eq!(
        results[0].resources_converged, 1,
        "force should reconverge even when unchanged"
    );
}

#[test]
fn test_fj132_collect_machines_from_config() {
    // collect_machines returns machines referenced by resources, not all declared machines
    let yaml = r#"
version: "1.0"
name: collect-test
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
  db:
    hostname: db-01
    addr: 10.0.0.2
resources:
  web-pkg:
    type: package
    machine: web
    provider: apt
    packages: [curl]
  db-pkg:
    type: package
    machine: db
    provider: apt
    packages: [postgresql]
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let machines = collect_machines(&config);
    assert!(machines.contains(&"web".to_string()));
    assert!(machines.contains(&"db".to_string()));
    assert_eq!(machines.len(), 2);
}
