//! #780 review blockers (forjar#828 isolation merged onto the #660 loader).
//!
//! 1. An `outputs:` entry that references a FAILED data source must be skipped with the
//!    resources that reference it, not abort `apply` before its post-actions run.
//! 2. `codegen` must emit what `apply` runs: a data value is a literal, never re-scanned.
//! 3. `explain` prints a data value as written: no U+E001/U+E002 stand-in reaches stdout.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(forjar())
        .args(args)
        .output()
        .expect("forjar failed to start");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn write_cfg(dir: &tempfile::TempDir, body: &str) -> PathBuf {
    let cfg = dir.path().join("forjar.yaml");
    fs::write(&cfg, body).unwrap();
    cfg
}

/// Blocker 1: the post-apply hook is the witness that apply reached its post-actions.
#[test]
fn an_output_of_a_failed_data_source_does_not_abort_the_post_actions() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("post-apply-ran");
    let unrelated = dir.path().join("unrelated.txt");
    let body = format!(
        "version: \"1.0\"\nname: o780\n\
machines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\n\
data:\n\x20 broken: {{ type: command, value: \"exit 127\" }}\n\
outputs:\n\x20 from_broken: {{ value: \"{{{{data.broken}}}}\" }}\n\x20 plain: {{ value: \"fixed\" }}\n\
policy:\n\x20 post_apply: \"touch {}\"\n\
resources:\n\x20 unrelated: {{ type: file, machine: local, path: {}, content: \"fine\\n\" }}\n",
        marker.display(),
        unrelated.display()
    );
    let cfg = write_cfg(&dir, &body);
    let state = dir.path().join("state");
    let (ok, so, se) = run(&[
        "apply",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        state.to_str().unwrap(),
        "--yes",
    ]);
    let all = format!("{so}{se}");
    assert!(!ok, "a failed data source must still fail the run:\n{all}");
    assert!(unrelated.exists(), "unrelated resource applied:\n{all}");
    assert!(
        marker.exists(),
        "post_apply hook must run despite the failed source's output:\n{all}"
    );
    assert!(
        all.contains("from_broken"),
        "the skipped output must be named:\n{all}"
    );
}

// base64 of "v={{params.port}}\n" (the literal) and of "v=8080\n" (a re-scanned value).
const LITERAL_B64: &str = "dj17e3BhcmFtcy5wb3J0fX0K";
const EXPANDED_B64: &str = "dj04MDgwCg==";

const LITERAL_CFG: &str = "version: \"1.0\"\nname: l780\n\
machines: { local: { hostname: localhost, addr: 127.0.0.1 } }\n\
params: { port: \"8080\" }\n\
data:\n  lit: { type: command, value: \"printf '%s' '{{params.port}}'\" }\n\
resources:\n  r: { type: file, machine: local, path: /tmp/forjar-780-never, content: \"v={{data.lit}}\\n\" }\n";

/// Blocker 2: the data value is the literal text `{{params.port}}`; codegen must not expand it.
#[test]
fn codegen_emits_a_data_value_as_a_literal() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_cfg(&dir, LITERAL_CFG);
    let (ok, so, se) = run(&["codegen", "-f", cfg.to_str().unwrap(), "-r", "r"]);
    assert!(ok, "codegen failed:\n{so}{se}");
    assert!(
        so.contains(LITERAL_B64),
        "the data value must stay literal:\n{so}"
    );
    assert!(
        !so.contains(EXPANDED_B64),
        "data value was re-scanned:\n{so}"
    );
}

/// Blocker 3: no stand-in character may be printed by `explain`, text or JSON.
#[test]
fn explain_prints_no_stand_in_characters() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_cfg(&dir, LITERAL_CFG);
    for json in [false, true] {
        let mut args = vec!["explain", "-f", cfg.to_str().unwrap(), "r"];
        if json {
            args.push("--json");
        }
        let (ok, so, se) = run(&args);
        assert!(ok, "explain (json={json}) failed:\n{so}{se}");
        assert!(
            !so.contains('\u{e001}') && !so.contains('\u{e002}'),
            "stand-in leaked (json={json}):\n{so}"
        );
        // The text form prints the content; the JSON form carries it inside the base64 script.
        let want = if json { LITERAL_B64 } else { "{{params.port}}" };
        assert!(so.contains(want), "value as written (json={json}):\n{so}");
    }
}

/// Review round 2 (a): `policy` inspects the config, it does not render it. A data value
/// holding braces is compared as written, so an `assert` over it is not a false violation.
#[test]
fn policy_compares_a_data_value_as_written() {
    let dir = tempfile::tempdir().unwrap();
    let body = "version: \"1.0\"\nname: p780\n\
machines: { local: { hostname: localhost, addr: 127.0.0.1 } }\n\
params: { port: \"8080\" }\n\
data:\n  lit: { type: command, value: \"printf '%s' '{{params.port}}'\" }\n\
resources:\n  r: { type: file, machine: local, path: /tmp/forjar-780-never, content: x, owner: \"{{data.lit}}\" }\n\
policies:\n  - type: assert\n    id: P1\n    message: owner is the literal\n    resource_type: file\n    condition_field: owner\n    condition_value: \"{{params.port}}\"\n";
    let cfg = write_cfg(&dir, body);
    let (ok, so, se) = run(&["policy", "-f", cfg.to_str().unwrap()]);
    assert!(ok, "false policy violation on a data value:\n{so}{se}");
}

/// Review round 2 (b): `--abort-on-drift` must name a failed data source, not call it drift.
#[test]
fn abort_on_drift_names_a_failed_data_source() {
    let dir = tempfile::tempdir().unwrap();
    let body = "version: \"1.0\"\nname: d780\n\
machines: { local: { hostname: localhost, addr: 127.0.0.1 } }\n\
data:\n  broken: { type: command, value: \"exit 127\" }\n\
resources:\n  r: { type: file, machine: local, path: /tmp/forjar-780-never, content: \"v={{data.broken}}\\n\" }\n";
    let cfg = write_cfg(&dir, body);
    let state = dir.path().join("state");
    let (ok, so, se) = run(&[
        "apply",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        state.to_str().unwrap(),
        "--yes",
        "--abort-on-drift",
    ]);
    let all = format!("{so}{se}");
    assert!(!ok, "must fail:\n{all}");
    assert!(
        all.contains("data source") && all.contains("broken"),
        "must name the failed source:\n{all}"
    );
}

/// Review round 3: `apply`'s policy gate reads the same form as `forjar policy`.
#[test]
fn apply_dry_run_policy_gate_compares_a_data_value_as_written() {
    let dir = tempfile::tempdir().unwrap();
    let body = "version: \"1.0\"\nname: g780\n\
machines: { local: { hostname: localhost, addr: 127.0.0.1 } }\n\
params: { port: \"8080\" }\n\
data:\n  lit: { type: command, value: \"printf '%s' '{{params.port}}'\" }\n\
resources:\n  r: { type: file, machine: local, path: /tmp/forjar-780-never, content: x, owner: \"{{data.lit}}\" }\n\
policies:\n  - type: assert\n    id: P1\n    message: owner is the literal\n    resource_type: file\n    condition_field: owner\n    condition_value: \"{{params.port}}\"\n";
    let cfg = write_cfg(&dir, body);
    let state = dir.path().join("state");
    let (ok, so, se) = run(&[
        "apply",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        state.to_str().unwrap(),
        "--dry-run",
    ]);
    assert!(
        ok,
        "apply's policy gate denied a literal data value:\n{so}{se}"
    );
}

/// Review round 3: an output that cannot resolve (`{{secrets.nope}}`) must not skip the
/// post-actions of an apply that already converged (main falls back to the raw value).
#[test]
fn an_unresolvable_output_does_not_skip_the_post_actions() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("post-apply-ran");
    let target = dir.path().join("t.txt");
    let body = format!(
        "version: \"1.0\"\nname: u780\n\
machines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\n\
outputs:\n\x20 bad: {{ value: \"{{{{secrets.nope}}}}\" }}\n\
policy:\n\x20 post_apply: \"touch {}\"\n\
resources:\n\x20 r: {{ type: file, machine: local, path: {}, content: \"x\\n\" }}\n",
        marker.display(),
        target.display()
    );
    let cfg = write_cfg(&dir, &body);
    let state = dir.path().join("state");
    let (_ok, so, se) = run(&[
        "apply",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        state.to_str().unwrap(),
        "--yes",
    ]);
    assert!(target.exists(), "resource converged:\n{so}{se}");
    assert!(marker.exists(), "post_apply must still run:\n{so}{se}");
}

/// Review round 4: one unresolvable output must not stop the good ones being saved.
#[test]
fn a_good_output_beside_a_bad_one_is_saved() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("t.txt");
    let body = format!(
        "version: \"1.0\"\nname: s780\n\
machines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\n\
outputs:\n\x20 good: {{ value: \"kept-780\" }}\n\x20 bad: {{ value: \"{{{{secrets.nope}}}}\" }}\n\
resources:\n\x20 r: {{ type: file, machine: local, path: {}, content: \"x\\n\" }}\n",
        target.display()
    );
    let cfg = write_cfg(&dir, &body);
    let state = dir.path().join("state");
    let (_ok, so, se) = run(&[
        "apply",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        state.to_str().unwrap(),
        "--yes",
    ]);
    let lock = fs::read_to_string(state.join("forjar.lock.yaml")).unwrap_or_default();
    assert!(
        lock.contains("kept-780"),
        "good output not saved:\n{lock}\n{so}{se}"
    );
    assert!(
        se.contains("bad"),
        "the bad output must be named:\n{so}{se}"
    );
}
