//! forjar#615: a resource with NO lock entry must ask its `completion_check`
//! before it runs `create`.
//!
//! Measured on paiml/infra's lambda-labs (forjar 1.32.0, no lambda-labs lock in
//! the state dir): `apply -r ollama-model-qwen35-4b` pulled in `ollama-binary`
//! and prompted "Apply 2 change(s) (2 create)". `ollama-binary`'s check exited 0
//! on the box. Its command runs `rm -rf /usr/local/lib/ollama`, re-extracts and
//! restarts the shared daemon. So a lockless apply would have torn down a
//! working install the resource's own check said was already there.
//!
//! These tests run a real apply against `localhost` with a fresh state dir.
//! The command writes a sentinel; the sentinel existing is the verdict.

use std::fs;
use std::path::Path;

fn config_yaml(ran: &Path, check: &str) -> String {
    format!(
        r#"version: "1.0"
name: lockless-check
machines:
  localhost:
    hostname: localhost
    addr: localhost
resources:
  guarded:
    type: task
    machine: localhost
    command: "echo ran >> {r}"
    completion_check: "{check}"
"#,
        r = ran.display()
    )
}

fn forjar(args: &[&str], cfg: &Path, state: &Path) -> (String, bool) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(args)
        .arg("-f")
        .arg(cfg)
        .arg("--state-dir")
        .arg(state)
        .output()
        .expect("forjar must run");
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.success())
}

fn setup(check: &str) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("forjar.yaml");
    let ran = dir.path().join("ran");
    fs::write(&cfg, config_yaml(&ran, check)).unwrap();
    (dir, cfg, ran)
}

#[test]
fn a_lockless_apply_does_not_run_a_command_whose_check_already_passes() {
    // THE DEFECT. Before the fix: no lock entry ⇒ Create ⇒ the command ran.
    let (dir, cfg, ran) = setup("true");
    let state = dir.path().join("state");
    let (out, ok) = forjar(&["apply", "--yes"], &cfg, &state);
    assert!(ok, "apply failed:\n{out}");
    assert!(
        !ran.exists(),
        "the command ran although its completion_check passed:\n{out}"
    );
    assert!(out.contains("1 unchanged"), "expected unchanged:\n{out}");
}

#[test]
fn a_lockless_apply_still_runs_a_command_whose_check_fails() {
    // The control: the check is asked, not skipped. A failing check still
    // plans create and the command runs exactly once.
    let (dir, cfg, ran) = setup("false");
    let state = dir.path().join("state");
    let (out, _) = forjar(&["apply", "--yes"], &cfg, &state);
    let body = fs::read_to_string(&ran).unwrap_or_default();
    assert_eq!(body.lines().count(), 1, "command must run once:\n{out}");
}

#[test]
fn dry_run_previews_what_apply_will_do_on_an_empty_lock() {
    // The preview must not promise a create the apply will not perform.
    let (dir, cfg, ran) = setup("true");
    let state = dir.path().join("state");
    let (out, ok) = forjar(&["apply", "--dry-run"], &cfg, &state);
    assert!(ok, "dry-run failed:\n{out}");
    assert!(!ran.exists(), "dry-run executed the command");
    assert!(
        !out.contains("1 to add"),
        "dry-run plans a create the apply will skip:\n{out}"
    );
}
