//! FJ-132: Integration tests (part 1).

use super::test_fixtures::*;
use super::*;

// ── FJ-132: Integration tests ──────────────────────────────

#[test]
fn test_fj132_force_apply_reconverges() {
    // Force apply should re-apply even when hash matches
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
        refresh: false,
        force_tag: None,
    };
    let r1 = apply(&cfg).unwrap();
    assert_eq!(r1[0].resources_converged, 1);

    // Normal re-apply should skip (unchanged)
    let r2 = apply(&cfg).unwrap();
    assert_eq!(r2[0].resources_unchanged, 1);
    assert_eq!(r2[0].resources_converged, 0);

    // Force apply should re-converge
    let force_cfg = ApplyConfig {
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
    let r3 = apply(&force_cfg).unwrap();
    assert_eq!(r3[0].resources_converged, 1);
    assert_eq!(r3[0].resources_unchanged, 0);

    let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
}

#[test]
fn test_fj132_resource_filter_applies_only_matching() {
    // Resource filter should only apply the specified resource
    let output_dir = tempfile::tempdir().unwrap();
    let path_a = output_dir.path().join("filter-a.txt");
    let path_b = output_dir.path().join("filter-b.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: filter-test
machines: {{}}
resources:
  file-a:
    type: file
    machine: localhost
    path: "{}"
    content: "alpha"
  file-b:
    type: file
    machine: localhost
    path: "{}"
    content: "beta"
policy:
  lock_file: true
  tripwire: false
"#,
        path_a.display(),
        path_b.display()
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: Some("file-a"),
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
    // Only file-a should be applied
    assert_eq!(results[0].resources_converged, 1);

    // Verify file-a exists but file-b doesn't
    assert!(path_a.exists(), "file-a should be created");
    assert!(
        !path_b.exists(),
        "file-b should not be created when filtered to file-a"
    );
}

#[test]
fn test_fj132_tag_filter_applies_only_tagged() {
    let output_dir = tempfile::tempdir().unwrap();
    let path_tagged = output_dir.path().join("tagged.txt");
    let path_untagged = output_dir.path().join("untagged.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: tag-test
machines: {{}}
resources:
  tagged-file:
    type: file
    machine: localhost
    path: "{}"
    content: "tagged"
    tags: [web]
  untagged-file:
    type: file
    machine: localhost
    path: "{}"
    content: "untagged"
policy:
  lock_file: true
  tripwire: false
"#,
        path_tagged.display(),
        path_untagged.display()
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: None,
        tag_filter: Some("web"),
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
    assert_eq!(results[0].resources_converged, 1);

    assert!(path_tagged.exists(), "tagged file should be created");
    assert!(
        !path_untagged.exists(),
        "untagged file should not be created when filtered by tag"
    );
}

#[test]
fn test_fj132_apply_with_dependencies_order() {
    // Verify that dependency order is respected in actual apply
    let output_dir = tempfile::tempdir().unwrap();
    let path_first = output_dir.path().join("first.txt");
    let path_second = output_dir.path().join("second.txt");
    let path_third = output_dir.path().join("third.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: dep-order
machines: {{}}
resources:
  first:
    type: file
    machine: localhost
    path: "{}"
    content: "first"
  second:
    type: file
    machine: localhost
    path: "{}"
    content: "second"
    depends_on: [first]
  third:
    type: file
    machine: localhost
    path: "{}"
    content: "third"
    depends_on: [second]
policy:
  lock_file: true
  tripwire: false
"#,
        path_first.display(),
        path_second.display(),
        path_third.display()
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
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
    assert_eq!(results[0].resources_converged, 3);

    // All three files should exist
    assert_eq!(
        std::fs::read_to_string(&path_first).unwrap().trim(),
        "first"
    );
    assert_eq!(
        std::fs::read_to_string(&path_second).unwrap().trim(),
        "second"
    );
    assert_eq!(
        std::fs::read_to_string(&path_third).unwrap().trim(),
        "third"
    );
}

#[test]
fn test_fj132_global_lock_written_after_apply() {
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

    // Per-machine lock should exist after apply
    let machine_lock = state::load_lock(dir.path(), "local").unwrap();
    assert!(machine_lock.is_some(), "per-machine lock should exist");
    let ml = machine_lock.unwrap();
    assert!(ml.resources.contains_key("test-file"));
    assert_eq!(ml.resources["test-file"].status, ResourceStatus::Converged);

    let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
}

#[test]
fn test_fj132_dry_run_creates_no_files() {
    let output_dir = tempfile::tempdir().unwrap();
    let path = output_dir.path().join("dry-run-no-exist.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: dry-run-test
machines: {{}}
resources:
  test-file:
    type: file
    machine: localhost
    path: "{}"
    content: "should not be created"
"#,
        path.display()
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let dir = tempfile::tempdir().unwrap();
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
    assert_eq!(results[0].machine, "dry-run");

    assert!(!path.exists(), "dry-run should not create files");
}
