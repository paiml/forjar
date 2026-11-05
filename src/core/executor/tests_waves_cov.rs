//! PMAT-088: Coverage for record_wave_outcomes — success, unchanged, update,
//! exit-code failure, and transport-error branches via parallel waves.

use super::*;
use std::path::Path;

/// ApplyConfig forcing the parallel wave execution path.
fn parallel_apply_cfg<'a>(config: &'a ForjarConfig, state_dir: &'a Path) -> ApplyConfig<'a> {
    ApplyConfig {
        config,
        state_dir,
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
        parallel: Some(true),
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: None,
        refresh: false,
        force_tag: None,
    }
}

/// Localhost config with three independent file resources inside `dir`.
fn three_file_yaml(dir: &Path, content: &str) -> String {
    format!(
        r#"
version: "1.0"
name: waves-cov
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  r1:
    type: file
    machine: local
    path: {d}/r1.txt
    content: "{content}-1"
  r2:
    type: file
    machine: local
    path: {d}/r2.txt
    content: "{content}-2"
  r3:
    type: file
    machine: local
    path: {d}/r3.txt
    content: "{content}-3"
"#,
        d = dir.display()
    )
}

#[test]
fn wave_outcomes_parallel_converge_then_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config: ForjarConfig = serde_yaml_ng::from_str(&three_file_yaml(dir.path(), "v1")).unwrap();
    let cfg = parallel_apply_cfg(&config, &state_dir);

    // Wave 0 of three resources → record_success branch ("create" span).
    let r1 = apply(&cfg).unwrap();
    assert_eq!(r1[0].resources_converged, 3);
    assert_eq!(r1[0].resources_failed, 0);

    // Re-apply: all unchanged → skipped_or_unchanged recording branch.
    let r2 = apply(&cfg).unwrap();
    assert_eq!(r2[0].resources_converged, 0);
    assert_eq!(r2[0].resources_unchanged, 3);
}

#[test]
fn wave_outcomes_parallel_update_action() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config1: ForjarConfig =
        serde_yaml_ng::from_str(&three_file_yaml(dir.path(), "v1")).unwrap();
    let cfg1 = parallel_apply_cfg(&config1, &state_dir);
    assert_eq!(apply(&cfg1).unwrap()[0].resources_converged, 3);

    // Changed content → Update action label branch in record_wave_outcomes.
    let config2: ForjarConfig =
        serde_yaml_ng::from_str(&three_file_yaml(dir.path(), "v2")).unwrap();
    let cfg2 = parallel_apply_cfg(&config2, &state_dir);
    let r = apply(&cfg2).unwrap();
    assert_eq!(r[0].resources_converged, 3, "all updated in parallel");
}

#[test]
fn wave_outcomes_parallel_mixed_failure() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let yaml = format!(
        r#"
version: "1.0"
name: waves-fail
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  good:
    type: file
    machine: local
    path: {d}/good.txt
    content: "fine"
  bad:
    type: file
    machine: local
    path: /dev/null/forjar-waves/bad.txt
    content: "doomed"
"#,
        d = dir.path().display()
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let cfg = parallel_apply_cfg(&config, &state_dir);

    let r = apply(&cfg).unwrap();
    // Exit-code failure branch: Ok(out) with non-zero exit recorded as failed.
    assert_eq!(r[0].resources_converged, 1, "good file still converges");
    assert_eq!(r[0].resources_failed, 1, "unwritable path fails");
    assert!(dir.path().join("good.txt").exists());
}

#[test]
fn wave_outcomes_parallel_pre_hook_error_branch() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let yaml = format!(
        r#"
version: "1.0"
name: waves-hook
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  h1:
    type: file
    machine: local
    path: {d}/h1.txt
    content: "x"
    pre_apply: "false"
  h2:
    type: file
    machine: local
    path: {d}/h2.txt
    content: "y"
    pre_apply: "false"
"#,
        d = dir.path().display()
    );
    let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let cfg = parallel_apply_cfg(&config, &state_dir);

    let r = apply(&cfg).unwrap();
    // Failing pre_apply hooks surface as Err(_) exec results →
    // "transport error" recording branch for every wave member.
    assert_eq!(r[0].resources_failed, 2, "both hooks fail the resources");
    assert_eq!(r[0].resources_converged, 0);
    assert!(!dir.path().join("h1.txt").exists(), "apply never ran");
}

/// forjar#777: `--resource-timeout` must bound the apply script and the
/// `pre_apply` gate on the wave path, not only the verify RecordCtx.
#[test]
fn test_resource_timeout_bounds_wave_apply_script() {
    for body in [
        "command: \"sleep 30\"",
        "command: \"true\"\n    pre_apply: \"sleep 30\"",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let yaml = format!(
            "version: \"1.0\"\nname: rt\nmachines:\n  local:\n    hostname: local\n    addr: 127.0.0.1\nresources:\n  slow:\n    type: task\n    machine: local\n    {body}\npolicy:\n  parallel_resources: true\n"
        );
        let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        let state_dir = dir.path().join("state");
        let mut cfg = parallel_apply_cfg(&config, &state_dir);
        cfg.resource_timeout = Some(2);
        let start = std::time::Instant::now();
        let results = apply(&cfg).unwrap();
        let elapsed = start.elapsed().as_secs();
        assert!(
            elapsed < 6,
            "ran {elapsed}s; timeout 2 did not bound: {body}"
        );
        assert_eq!(results[0].resources_failed, 1);
        assert_eq!(results[0].resources_converged, 0);
    }
}
