//! #471 (PMAT-884): `--only-machine X -m Y` / `--exclude-machine X -m X` are
//! contradictions. Driven through the real binary so deleting the wiring in
//! `cmd_apply_scoped` or `selectors_of` turns these red (rc=0, nothing done).

use std::process::{Command, Output};

const CONFIG: &str = r#"version: '1.0'
name: contra
machines:
  local:
    hostname: localhost
    addr: localhost
  other:
    hostname: other
    addr: other
resources:
  a-file:
    type: file
    machine: local
    path: SANDBOX/a.txt
    content: A
"#;

fn run(extra: &[&str]) -> Output {
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("forjar.yaml");
    let body = CONFIG.replace("SANDBOX", d.path().to_str().unwrap());
    std::fs::write(&cfg, body).unwrap();
    let state = d.path().join("state");
    Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(d.path())
        .args(["apply", "-f"])
        .arg(&cfg)
        .arg("--state-dir")
        .arg(&state)
        .args(extra)
        .output()
        .expect("run forjar")
}

fn refused(extra: &[&str], needle: &str) {
    let o = run(extra);
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(!o.status.success(), "{extra:?} must exit non-zero: {all}");
    assert!(all.contains(needle), "{extra:?}: {all}");
}

#[test]
fn apply_refuses_only_machine_with_a_different_m() {
    refused(
        &["--only-machine", "local", "-m", "other"],
        "select disjoint machines",
    );
}

#[test]
fn apply_check_refuses_only_machine_with_a_different_m() {
    refused(
        &["--check", "--only-machine", "local", "-m", "other"],
        "select disjoint machines",
    );
}

#[test]
fn apply_refuses_exclude_machine_with_the_same_m() {
    refused(
        &["--exclude-machine", "local", "-m", "local"],
        "name the same machine",
    );
}
