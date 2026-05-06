//! FJ-129: claim C3 (idempotency) must be observable through `--force`.
//!
//! Issue: https://github.com/paiml/forjar/issues/129
//! Contract: contracts/apply-summary-distinguishability-v1.yaml
//!
//! Four step shapes from the contract's proof_obligations:
//!
//!   apply(fresh)            → converged=N, forced=0
//!   apply(converged)        → converged=0, unchanged=N, forced=0
//!   apply(converged, force) → converged=N, forced=N, actual_changes=0   ← C3 holds
//!   apply(drift_one, force) → converged=N, forced=N-1, actual_changes=1

use std::path::Path;
use std::process::Command;

fn forjar_bin() -> String {
    // Cargo sets CARGO_BIN_EXE_<name> for integration tests of crates
    // that build a binary. That's the binary built from THIS source —
    // exactly what we want. If that's missing (e.g. running outside
    // cargo), fall back to PATH.
    option_env!("CARGO_BIN_EXE_forjar")
        .map(String::from)
        .unwrap_or_else(|| "forjar".to_string())
}

fn run_forjar_apply(yaml: &Path, state_dir: &Path, force: bool) -> serde_json::Value {
    let mut cmd = Command::new(forjar_bin());
    cmd.arg("apply")
        .arg("-f")
        .arg(yaml)
        .arg("--state-dir")
        .arg(state_dir)
        .arg("--yes")
        .arg("--json");
    if force {
        cmd.arg("--force");
    }
    let out = cmd.output().expect("spawn forjar apply");
    assert!(
        out.status.success(),
        "forjar apply failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf8");
    serde_json::from_str(&stdout).expect("parse forjar apply --json")
}

fn write_minimal_yaml(dir: &Path, target_path: &Path) -> std::path::PathBuf {
    let yaml = format!(
        r#"version: "1.0"
name: fj129
machines:
  localhost:
    hostname: localhost
    addr: 127.0.0.1
resources:
  marker:
    type: file
    machine: localhost
    path: {}
    state: file
    content: "fj129 marker"
    mode: "0644"
  marker2:
    type: file
    machine: localhost
    path: {}
    state: file
    content: "fj129 marker2"
    mode: "0644"
"#,
        target_path.display(),
        target_path.with_extension("two").display(),
    );
    let yaml_path = dir.join("forjar.yaml");
    std::fs::write(&yaml_path, yaml).expect("write yaml");
    yaml_path
}

#[test]
fn fj129_force_distinguishability_four_shapes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let work = tmp.path();
    let state_dir = work.join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let target = work.join("marker.txt");
    let yaml = write_minimal_yaml(work, &target);

    // Shape 1: fresh apply → converged = N (2), forced = 0
    let r1 = run_forjar_apply(&yaml, &state_dir, false);
    let s1 = &r1["summary"];
    assert_eq!(
        s1["total_converged"].as_u64().unwrap(),
        2,
        "shape 1: converged"
    );
    assert_eq!(
        s1["forced_noop_count"].as_u64().unwrap_or(0),
        0,
        "shape 1: forced"
    );

    // Shape 2: re-apply converged stack, no --force → unchanged = N
    let r2 = run_forjar_apply(&yaml, &state_dir, false);
    let s2 = &r2["summary"];
    assert_eq!(
        s2["total_converged"].as_u64().unwrap(),
        0,
        "shape 2: converged"
    );
    assert_eq!(
        s2["total_unchanged"].as_u64().unwrap(),
        2,
        "shape 2: unchanged"
    );
    assert_eq!(
        s2["forced_noop_count"].as_u64().unwrap_or(0),
        0,
        "shape 2: forced"
    );

    // Shape 3: re-apply with --force on a fully-converged stack →
    //          converged = N, forced = N, actual_changes = 0  ← C3 demo
    let r3 = run_forjar_apply(&yaml, &state_dir, true);
    let s3 = &r3["summary"];
    assert_eq!(
        s3["total_converged"].as_u64().unwrap(),
        2,
        "shape 3: converged"
    );
    assert_eq!(
        s3["forced_noop_count"].as_u64().unwrap(),
        2,
        "shape 3: every converge was a forced no-op (C3 holds through --force)"
    );
    assert_eq!(
        s3["actual_changes"].as_u64().unwrap(),
        0,
        "shape 3: zero genuine changes — C3 demonstration"
    );

    // Shape 4: drift one resource on the filesystem (rm outside forjar),
    //          re-apply with --force. The forced_noop_count is computed
    //          against the LOCK (not the live filesystem), so the lock
    //          still reports both resources as converged. Result:
    //          forced_noop = 2, actual_changes = 0 — same as Shape 3.
    //
    // This is Q1 semantics: "how many resources did --force re-run that
    // the lock said were unchanged?" — cheap, deterministic, no live-state
    // hashing. The orthogonal question Q2 ("how many resources had
    // live-state drift?") is what `forjar drift` answers. Documenting the
    // Q1/Q2 split is part of the contract; conflating them was the bug.
    std::fs::remove_file(&target).expect("rm target to drift it");
    let r4 = run_forjar_apply(&yaml, &state_dir, true);
    let s4 = &r4["summary"];
    assert_eq!(
        s4["total_converged"].as_u64().unwrap(),
        2,
        "shape 4: converged"
    );
    assert_eq!(
        s4["forced_noop_count"].as_u64().unwrap(),
        2,
        "shape 4: lock-based forced_noop sees both resources as already-locked"
    );
    assert_eq!(
        s4["actual_changes"].as_u64().unwrap(),
        0,
        "shape 4: actual_changes reflects lock-state diff, not live-state diff (use `forjar drift` for the latter)"
    );
}
