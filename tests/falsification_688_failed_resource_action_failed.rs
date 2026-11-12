//! forjar#688: a failed resource is recorded as `action: failed`, not
//! `action: converged` with a `failed: true` flag a reader has to know to look for.
//!
//! The BINARY is driven: `runs/<id>/meta.yaml` is what `jq`-style consumers read.
//! Reverting the fix (`Converged { failed: true }` again) turns assertion 1 red.

use std::path::Path;
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

fn meta_of_only_run(state: &Path) -> String {
    let runs = state.join("box").join("runs");
    let run = std::fs::read_dir(&runs)
        .expect("runs dir")
        .next()
        .expect("one run")
        .expect("entry")
        .path();
    std::fs::read_to_string(run.join("meta.yaml")).expect("meta.yaml")
}

#[test]
fn failed_task_is_recorded_as_action_failed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        "version: \"1.0\"\nname: meta-fixture\nmachines:\n  box: { hostname: box, addr: 127.0.0.1 }\n\
         resources:\n  guard: { type: task, machine: box, command: \"echo guard-check; exit 1\" }\n",
    )
    .expect("write config");
    let state = dir.path().join("state");
    let out = Command::new(forjar())
        .args(["apply", "-f"])
        .arg(&cfg)
        .arg("--state-dir")
        .arg(&state)
        .arg("--yes")
        .output()
        .expect("run forjar");
    assert!(!out.status.success(), "a failing task must fail the apply");
    let meta = meta_of_only_run(&state);
    assert!(
        meta.contains("action: failed"),
        "the failed resource must say `action: failed`:\n{meta}"
    );
    assert!(
        !meta.contains("action: converged"),
        "a failure must never be recorded as `converged`:\n{meta}"
    );
}

#[test]
fn legacy_converged_with_failed_flag_reads_as_failed() {
    use forjar::core::types::{ResourceRunStatus, RunMeta};
    let legacy = "run_id: r-1\nmachine: m\ncommand: apply\nresources:\n  g:\n    action: converged\n    exit_code: 1\n    duration_secs: 0.5\n    failed: true\n";
    let meta: RunMeta = serde_yaml_ng::from_str(legacy).expect("legacy meta parses");
    assert!(
        matches!(meta.resources["g"], ResourceRunStatus::Failed { .. }),
        "got {:?}",
        meta.resources["g"]
    );
}
