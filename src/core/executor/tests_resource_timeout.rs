//! `--resource-timeout` must bound a resource's apply script on the wave
//! (parallel) path, not only the verify/state-query RecordCtx.

use super::*;

#[test]
fn test_resource_timeout_bounds_wave_apply_script() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let yaml = r#"
version: "1.0"
name: resource-timeout
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  slow:
    type: task
    machine: local
    command: "sleep 30"
policy:
  parallel_resources: true
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
        resource_timeout: Some(2),
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: None,
        refresh: false,
        force_tag: None,
    };
    let start = std::time::Instant::now();
    let results = apply(&cfg).unwrap();
    let elapsed = start.elapsed().as_secs();
    assert!(
        elapsed < 6,
        "apply ran {elapsed}s; --resource-timeout 2 did not bound it"
    );
    assert_eq!(results[0].resources_failed, 1);
    assert_eq!(results[0].resources_converged, 0);
}
