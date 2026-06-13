//! #165: pre_apply hook retry-safety tests (companion to tests_hooks.rs).

use super::*;

// #165: a failing pre_apply hook must NOT be retried under --retry +
// ContinueIndependent. #157 made a failing pre_apply return Failed{should_stop}
// instead of Skipped; the FJ-283 retry loop only retries
// Failed{should_stop:false}, so with continue_independent the pre_apply gate
// hook started re-running up to N times, re-executing its (non-idempotent)
// side effects. After the fix the gate failure is non-retryable: the hook runs
// exactly ONCE, the resource still fails, and dependents still cascade-skip.
#[test]
fn test_gh165_pre_apply_hook_not_retried() {
    let tmp = std::env::temp_dir().join(format!("gh165-pre-retry-{}", std::process::id()));
    let state_dir = tmp.join("state");
    let _ = std::fs::create_dir_all(&state_dir);
    // Sentinel: each pre_apply invocation appends one line. Count == invocations.
    let sentinel = tmp.join("pre_invocations.log");
    let base = tmp.join("base.txt");
    let child = tmp.join("child.txt");
    let yaml = format!(
        r#"
version: "1.0"
name: test
policy:
  parallel_resources: false
  failure: continue_independent
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
    pre_apply: "echo x >> {sentinel}; exit 1"
  child:
    type: file
    machine: local
    path: {child}
    content: "child"
    depends_on: [base]
"#,
        base = base.display(),
        child = child.display(),
        sentinel = sentinel.display(),
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
        // retry > 0 is the trigger: the pre-fix bug looped the gate hook.
        retry: 2,
        // Force sequential so should_stop=false flows into the retry loop.
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

    // Hook ran EXACTLY ONCE — not retried (would be 1 + retry = 3 before fix).
    let invocations = std::fs::read_to_string(&sentinel)
        .map(|s| s.lines().count())
        .unwrap_or(0);
    assert_eq!(
        invocations, 1,
        "pre_apply gate hook must run exactly once, not be retried (got {invocations})"
    );
    // Resource still fails (#157 correctness preserved) and dependent cascades.
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
