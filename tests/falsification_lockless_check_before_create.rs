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

#[test]
fn a_lockless_apply_records_the_guard_it_did_not_run() {
    // Skipping the command must not skip the record: the converge it replaces
    // wrote `status: converged`, and drift inspects only what a lock holds.
    let (dir, cfg, ran) = setup("true");
    let state = dir.path().join("state");
    let (out, ok) = forjar(&["apply", "--yes"], &cfg, &state);
    assert!(ok, "apply failed:\n{out}");
    assert!(!ran.exists(), "the command ran:\n{out}");
    let lock = fs::read_to_string(state.join("localhost").join("state.lock.yaml"))
        .unwrap_or_else(|e| panic!("no lock written ({e}):\n{out}"));
    assert!(
        lock.contains("guarded:") && lock.contains("status: converged"),
        "the lock does not record the satisfied guard:\n{lock}"
    );
    assert!(
        !lock.contains("applied_at: 20"),
        "nothing was applied, so nothing may be dated:\n{lock}"
    );
}

fn setup_yaml(yaml: impl Fn(&Path) -> String) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("forjar.yaml");
    fs::write(&cfg, yaml(dir.path())).unwrap();
    (dir, cfg)
}

#[test]
fn a_templated_lockless_apply_does_not_run_a_command_whose_check_passes() {
    // Quorum lane 1 on #617: the seeded entry hashed the RAW resource and the
    // planner hashes the RESOLVED one, so any `{{params.*}}` planned
    // `update (state changed)` and ran the command anyway.
    let (dir, cfg) = setup_yaml(|d| {
        format!(
            r#"version: "1.0"
name: lockless-templated
params:
  who: world
machines:
  localhost:
    hostname: localhost
    addr: localhost
resources:
  guarded:
    type: task
    machine: localhost
    command: "echo hello-{{{{params.who}}}} >> {r}"
    completion_check: "true"
"#,
            r = d.join("ran").display()
        )
    });
    let state = dir.path().join("state");
    let (out, ok) = forjar(&["apply", "--yes"], &cfg, &state);
    assert!(ok, "apply failed:\n{out}");
    assert!(
        !dir.path().join("ran").exists(),
        "a templated command ran although its completion_check passed:\n{out}"
    );
    assert!(out.contains("1 unchanged"), "expected unchanged:\n{out}");
}

#[test]
fn a_resource_the_planner_filters_out_is_neither_asked_nor_recorded() {
    // Quorum lane 1 on #617: an arch mismatch or a false `when:` is skipped by
    // the planner, yet the seed ran its check and recorded it converged.
    let (dir, cfg) = setup_yaml(|d| {
        let asked = d.join("asked").display().to_string();
        format!(
            r#"version: "1.0"
name: lockless-filtered
params:
  flag: "no"
machines:
  localhost:
    hostname: localhost
    addr: localhost
resources:
  arm-only:
    type: task
    machine: localhost
    arch: [riscv64]
    command: "true"
    completion_check: "echo arm >> {asked}"
  gated-off:
    type: task
    machine: localhost
    when: '{{{{params.flag}}}} == "yes"'
    command: "true"
    completion_check: "echo gated >> {asked}"
"#
        )
    });
    let state = dir.path().join("state");
    let (out, ok) = forjar(&["apply", "--yes"], &cfg, &state);
    assert!(ok, "apply failed:\n{out}");
    let asked = fs::read_to_string(dir.path().join("asked")).unwrap_or_default();
    assert!(
        asked.is_empty(),
        "filtered-out checks ran: {asked:?}\n{out}"
    );
    let lock =
        fs::read_to_string(state.join("localhost").join("state.lock.yaml")).unwrap_or_default();
    assert!(
        !lock.contains("arm-only") && !lock.contains("gated-off"),
        "the lock records resources the plan excluded:\n{lock}"
    );
}

#[test]
fn a_dry_run_asks_each_lockless_check_once() {
    // Quorum lane 1 on #617: the executor seeded and then the preview seeded
    // again, so every check ran twice.
    let (dir, cfg) = setup_yaml(|d| {
        format!(
            r#"version: "1.0"
name: lockless-dry
machines:
  localhost:
    hostname: localhost
    addr: localhost
resources:
  guarded:
    type: task
    machine: localhost
    command: "true"
    completion_check: "echo asked >> {a}"
"#,
            a = d.join("asked").display()
        )
    });
    let state = dir.path().join("state");
    let (out, ok) = forjar(&["apply", "--dry-run"], &cfg, &state);
    assert!(ok, "dry-run failed:\n{out}");
    let asked = fs::read_to_string(dir.path().join("asked")).unwrap_or_default();
    assert_eq!(asked.lines().count(), 1, "check ran {asked:?}:\n{out}");
}

#[test]
fn a_forced_dry_run_previews_the_command_the_forced_apply_runs() {
    // --force and --force-tag build their own locks and never ask the check,
    // so the apply re-runs the command. A preview that asked it anyway would
    // promise the operator a no-op.
    for flags in [&["--force"][..], &["--force-tag", "anything"][..]] {
        let (dir, cfg, ran) = setup("true");
        let state = dir.path().join("state");
        let mut dry = vec!["apply", "--dry-run"];
        dry.extend_from_slice(flags);
        let (out, ok) = forjar(&dry, &cfg, &state);
        assert!(ok, "{flags:?} dry-run failed:\n{out}");
        assert!(
            out.contains("1 to add"),
            "{flags:?}: the preview hides the create the apply performs:\n{out}"
        );
        let mut real = vec!["apply", "--yes"];
        real.extend_from_slice(flags);
        let (out, ok) = forjar(&real, &cfg, &state);
        assert!(ok, "{flags:?} apply failed:\n{out}");
        assert!(ran.exists(), "{flags:?}: the apply did not run the command, so the preview was right to hide it:\n{out}");
    }
    // Control: --refresh DOES seed a passing check, so its preview keeps saying so.
    let (dir, cfg, ran) = setup("true");
    let state = dir.path().join("state");
    let (out, ok) = forjar(&["apply", "--dry-run", "--refresh"], &cfg, &state);
    assert!(ok, "--refresh dry-run failed:\n{out}");
    assert!(
        !out.contains("1 to add"),
        "--refresh preview plans a create:\n{out}"
    );
    let (out, ok) = forjar(&["apply", "--yes", "--refresh"], &cfg, &state);
    assert!(ok, "--refresh apply failed:\n{out}");
    assert!(
        !ran.exists(),
        "--refresh ran a command whose check passes:\n{out}"
    );
}
