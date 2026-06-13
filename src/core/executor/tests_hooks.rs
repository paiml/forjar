//! FJ-265: Resource lifecycle hooks — pre_apply / post_apply.

use super::*;

// ========================================================================
// FJ-265: Resource lifecycle hooks — pre_apply / post_apply
// ========================================================================

#[test]
fn test_fj265_pre_apply_success() {
    let tmp = std::env::temp_dir().join(format!("fj265-pre-ok-{}", std::process::id()));
    let state_dir = tmp.join("state");
    let _ = std::fs::create_dir_all(&state_dir);
    let path = tmp.join("pre_ok.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: {path}
    content: "hello"
    pre_apply: "true"
"#,
        path = path.display()
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
    assert_eq!(results[0].resources_converged, 1);
    assert!(path.exists());
    let _ = std::fs::remove_dir_all(&tmp);
}

// Bug-hunt #3 (Refs #154): a failing pre_apply hook on the SEQUENTIAL apply
// path must be counted as FAILED (not silently Skipped). Previously
// execute_resource returned ResourceOutcome::Skipped here, which is a no-op in
// MachineCounters::record, so resources_failed stayed 0 and apply reported
// success on a real failure. The single-resource case below exercises the
// sequential path (parallel kicks in only for >1 change).
#[test]
fn test_fj265_pre_apply_failure_counts_as_failed() {
    let tmp = std::env::temp_dir().join(format!("fj265-pre-fail-{}", std::process::id()));
    let state_dir = tmp.join("state");
    let _ = std::fs::create_dir_all(&state_dir);
    let path = tmp.join("pre_fail.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: {path}
    content: "hello"
    pre_apply: "exit 1"
"#,
        path = path.display()
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
    // pre_apply failure → resource recorded as FAILED, main script never ran.
    assert_eq!(
        results[0].resources_failed, 1,
        "pre_apply failure must count as a failed resource (not Skipped)"
    );
    assert_eq!(results[0].resources_converged, 0);
    assert!(
        !path.exists(),
        "main script must not run after pre_apply fails"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

// Bug-hunt #3 (Refs #154): on the sequential path, a pre_apply failure must
// cascade-skip dependents (FJ-63), and dependents must themselves be counted
// as failed — they must NOT run as if the prerequisite had succeeded.
#[test]
fn test_fj265_pre_apply_failure_cascades_to_dependents() {
    let tmp = std::env::temp_dir().join(format!("fj265-pre-cascade-{}", std::process::id()));
    let state_dir = tmp.join("state");
    let _ = std::fs::create_dir_all(&state_dir);
    let base = tmp.join("base.txt");
    let child = tmp.join("child.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: test
policy:
  parallel_resources: false
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  base:
    type: file
    machine: local
    path: {base}
    content: "base"
    pre_apply: "exit 1"
  child:
    type: file
    machine: local
    path: {child}
    content: "child"
    depends_on: [base]
"#,
        base = base.display(),
        child = child.display(),
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
        // Force sequential so the bug-prone path is the one under test.
        parallel: Some(false),
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: None,
        refresh: false,
        force_tag: None,
    };
    let results = apply(&cfg).unwrap();
    // base failed pre_apply, child cascade-skipped ⇒ both counted as failed.
    assert_eq!(
        results[0].resources_failed, 2,
        "base + cascade-skipped child both count as failed"
    );
    assert_eq!(results[0].resources_converged, 0);
    assert!(!base.exists(), "base main script must not run");
    assert!(
        !child.exists(),
        "dependent must NOT run after prerequisite's pre_apply failed"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_fj265_post_apply_success() {
    let tmp = std::env::temp_dir().join(format!("fj265-post-ok-{}", std::process::id()));
    let state_dir = tmp.join("state");
    let _ = std::fs::create_dir_all(&state_dir);
    let path = tmp.join("post_ok.txt");
    let marker = tmp.join("post_marker.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: {path}
    content: "hello"
    post_apply: "touch {marker}"
"#,
        path = path.display(),
        marker = marker.display()
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
    assert_eq!(results[0].resources_converged, 1);
    assert!(path.exists());
    assert!(marker.exists());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_fj265_post_apply_failure() {
    let tmp = std::env::temp_dir().join(format!("fj265-post-fail-{}", std::process::id()));
    let state_dir = tmp.join("state");
    let _ = std::fs::create_dir_all(&state_dir);
    let path = tmp.join("post_fail.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: {path}
    content: "hello"
    post_apply: "exit 1"
"#,
        path = path.display()
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
    // post_apply failure → resource marked as failed
    assert_eq!(results[0].resources_failed, 1);
    // File was still created (main script ran), but post_apply failed
    assert!(path.exists());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_fj265_pre_apply_with_backup_command() {
    let tmp = std::env::temp_dir().join(format!("fj265-backup-{}", std::process::id()));
    let state_dir = tmp.join("state");
    let _ = std::fs::create_dir_all(&state_dir);
    let path = tmp.join("config.txt");
    let backup = tmp.join("config.txt.bak");
    // Create the original file
    std::fs::write(&path, "original").unwrap();
    let yaml = format!(
        r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: {path}
    content: "updated"
    pre_apply: "cp {path} {backup}"
"#,
        path = path.display(),
        backup = backup.display()
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: &state_dir,
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
    let results = apply(&cfg).unwrap();
    assert_eq!(results[0].resources_converged, 1);
    // Backup was created by pre_apply hook
    assert!(backup.exists());
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), "original");
    // Main file was updated (heredoc adds trailing newline)
    assert_eq!(std::fs::read_to_string(&path).unwrap().trim(), "updated");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_fj265_no_hooks_default() {
    // Verify hooks default to None when not specified
    let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test:
    type: file
    machine: local
    path: /tmp/fj265-default.txt
    content: "test"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let resource = config.resources.get("test").unwrap();
    assert!(resource.pre_apply.is_none());
    assert!(resource.post_apply.is_none());
}

#[test]
fn test_fj265_hooks_yaml_parse() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test:
    type: file
    machine: local
    path: /tmp/fj265-parse.txt
    content: "test"
    pre_apply: "echo backup"
    post_apply: "systemctl restart nginx"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let resource = config.resources.get("test").unwrap();
    assert_eq!(resource.pre_apply.as_deref(), Some("echo backup"));
    assert_eq!(
        resource.post_apply.as_deref(),
        Some("systemctl restart nginx")
    );
}
