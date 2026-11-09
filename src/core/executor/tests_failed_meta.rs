//! #688: a failed resource is recorded as `action: failed` in the run's
//! `meta.yaml`, never as `action: converged`.

use super::*;

#[test]
fn fj688_failed_resource_row_says_action_failed() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let yaml = r#"
version: "1.0"
name: meta-fixture
machines:
  box:
    hostname: box
    addr: 127.0.0.1
resources:
  guard:
    type: task
    machine: box
    command: "echo guard-check; exit 1"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
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
        run_id: Some("r-688".to_string()),
        refresh: false,
        force_tag: None,
    };
    let results = apply(&cfg).unwrap();
    assert_eq!(results[0].resources_failed, 1);
    let meta = std::fs::read_to_string(state_dir.join("box/runs/r-688/meta.yaml")).unwrap();
    assert!(meta.contains("action: failed"), "{meta}");
    assert!(!meta.contains("action: converged"), "{meta}");
}

#[test]
fn fj688_legacy_converged_failed_true_still_counts_as_failed() {
    let old = "run_id: r-1\nmachine: m\ncommand: apply\nresources:\n  a:\n    action: converged\n    exit_code: 1\n    failed: true\n";
    let meta: crate::core::types::RunMeta = serde_yaml_ng::from_str(old).unwrap();
    assert!(matches!(
        meta.resources["a"],
        crate::core::types::ResourceRunStatus::Converged { failed: true, .. }
    ));
}
