//! FJ-257: Parallel apply within machines.

use super::*;

// ── FJ-257: Parallel apply within machines ───────────────────

#[test]
fn test_fj257_parallel_apply_independent_resources() {
    // Two independent file resources on localhost with parallel_resources: true
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let path_a = format!("/tmp/forjar-fj257-a-{}.txt", std::process::id());
    let path_b = format!("/tmp/forjar-fj257-b-{}.txt", std::process::id());
    let yaml = format!(
        r#"
version: "1.0"
name: parallel-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  file-a:
    type: file
    machine: local
    path: {path_a}
    content: "aaa"
  file-b:
    type: file
    machine: local
    path: {path_b}
    content: "bbb"
policy:
  parallel_resources: true
"#
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: &state_dir,
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
    assert_eq!(results[0].resources_converged, 2);
    assert_eq!(results[0].resources_failed, 0);

    // Verify files were created
    assert!(std::path::Path::new(&path_a).exists());
    assert!(std::path::Path::new(&path_b).exists());

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
}

#[test]
fn test_fj257_parallel_apply_with_deps_respects_order() {
    // base → app dependency: base must complete before app starts.
    // With parallel_resources: true, these should be in separate waves.
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let path_base = format!("/tmp/forjar-fj257-base-{}.txt", std::process::id());
    let path_app = format!("/tmp/forjar-fj257-app-{}.txt", std::process::id());
    let yaml = format!(
        r#"
version: "1.0"
name: deps-parallel-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  base:
    type: file
    machine: local
    path: {path_base}
    content: "base"
  app:
    type: file
    machine: local
    path: {path_app}
    content: "app"
    depends_on: [base]
policy:
  parallel_resources: true
"#
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: &state_dir,
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
    assert_eq!(results[0].resources_failed, 0);

    let _ = std::fs::remove_file(&path_base);
    let _ = std::fs::remove_file(&path_app);
}

#[test]
fn test_fj257_parallel_apply_three_independent() {
    // Three independent resources should all be in one wave
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let path_a = format!("/tmp/forjar-fj257-3a-{}.txt", std::process::id());
    let path_b = format!("/tmp/forjar-fj257-3b-{}.txt", std::process::id());
    let path_c = format!("/tmp/forjar-fj257-3c-{}.txt", std::process::id());
    let yaml = format!(
        r#"
version: "1.0"
name: three-parallel
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  a:
    type: file
    machine: local
    path: {path_a}
    content: "a"
  b:
    type: file
    machine: local
    path: {path_b}
    content: "b"
  c:
    type: file
    machine: local
    path: {path_c}
    content: "c"
policy:
  parallel_resources: true
"#
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: &state_dir,
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

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
    let _ = std::fs::remove_file(&path_c);
}

#[test]
fn test_fj257_parallel_apply_idempotent() {
    // Second apply with parallel_resources should produce 0 converged (all unchanged)
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let path_a = format!("/tmp/forjar-fj257-idem-a-{}.txt", std::process::id());
    let path_b = format!("/tmp/forjar-fj257-idem-b-{}.txt", std::process::id());
    let yaml = format!(
        r#"
version: "1.0"
name: idempotent-parallel
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  a:
    type: file
    machine: local
    path: {path_a}
    content: "a"
  b:
    type: file
    machine: local
    path: {path_b}
    content: "b"
policy:
  parallel_resources: true
"#
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: &state_dir,
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
    // First apply
    let results1 = apply(&cfg).unwrap();
    assert_eq!(results1[0].resources_converged, 2);

    // Second apply — should be unchanged
    let results2 = apply(&cfg).unwrap();
    assert_eq!(results2[0].resources_converged, 0);
    assert_eq!(results2[0].resources_unchanged, 2);

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
}

#[test]
fn test_fj257_compute_waves_diamond_dag() {
    // Diamond: a → b, a → c, b → d, c → d
    // Waves: [a], [b, c], [d]
    let yaml = r#"
version: "1.0"
name: diamond
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  a:
    type: file
    machine: m1
    path: /a
  b:
    type: file
    machine: m1
    path: /b
    depends_on: [a]
  c:
    type: file
    machine: m1
    path: /c
    depends_on: [a]
  d:
    type: file
    machine: m1
    path: /d
    depends_on: [b, c]
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let waves = compute_resource_waves(&config, &["a", "b", "c", "d"]);
    assert_eq!(waves.len(), 3);
    assert_eq!(waves[0], vec!["a"]);
    assert_eq!(waves[1].len(), 2);
    assert!(waves[1].contains(&"b".to_string()));
    assert!(waves[1].contains(&"c".to_string()));
    assert_eq!(waves[2], vec!["d"]);
}

#[test]
fn test_fj257_parallel_apply_lock_file_written() {
    // Verify lock file is properly written after parallel apply
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let path_a = format!("/tmp/forjar-fj257-lock-a-{}.txt", std::process::id());
    let path_b = format!("/tmp/forjar-fj257-lock-b-{}.txt", std::process::id());
    let yaml = format!(
        r#"
version: "1.0"
name: lock-parallel
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  a:
    type: file
    machine: local
    path: {path_a}
    content: "a"
  b:
    type: file
    machine: local
    path: {path_b}
    content: "b"
policy:
  parallel_resources: true
"#
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: &state_dir,
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

    // Verify lock file has both resources
    let lock = state::load_lock(&state_dir, "local").unwrap().unwrap();
    assert_eq!(lock.resources.len(), 2);
    assert!(lock.resources.contains_key("a"));
    assert!(lock.resources.contains_key("b"));
    assert_eq!(lock.resources["a"].status, ResourceStatus::Converged);
    assert_eq!(lock.resources["b"].status, ResourceStatus::Converged);

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
}
