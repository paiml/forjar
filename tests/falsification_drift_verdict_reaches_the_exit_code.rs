//! PMAT-562 (forjar#562, paiml/infra#605 second signature): the verdict
//! `forjar drift` prints reaches its exit code without a flag.
//!
//! MEASURED, forjar 1.30.0, yoga and gx10, 2026-09-15 11:51Z:
//!
//! ```text
//!   DRIFTED: gitconfig on yoga (/home/noah/.gitconfig content changed)
//!   DRIFTED: zshrc on yoga (/home/noah/.zshrc content changed)
//! Drift detected: 2 resource(s)
//! rc=0
//! ```
//!
//! `--tripwire` ("Exit non-zero on any drift (for CI/cron)") existed, so this
//! was DOCUMENTED — which is what makes it a design defect rather than a bug:
//! the default was fail-open, and the caller had to know to ask for the exit
//! code that means what the output says. forjar#352 was the same class
//! (`--refresh` could only invalidate, never satisfy): the flag that makes the
//! tool tell the truth must not be opt-in.
//!
//! Exit vocabulary: `0 accept · 1 reject · 2 decline · 3 error` (PVL /
//! ONT-001 §3.4); a drift verdict is a reject. `--tripwire` already exited 1
//! (FALSIFY-549-003 pins it), so the number does not move — the DEFAULT does.
//!
//! Driven through the binary, because an exit code is what a cron line and a
//! CI step consume; nothing below reads a Rust function's return value.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

/// One `file` resource with declared content, on a local machine.
fn config_yaml(target: &Path) -> String {
    format!(
        r#"version: "1.0"
name: exit-code
machines:
  box:
    hostname: box
    addr: 127.0.0.1
resources:
  managed:
    type: file
    machine: box
    path: {t}
    content: "declared\n"
"#,
        t = target.display()
    )
}

struct Stack {
    _dir: tempfile::TempDir,
    cfg: PathBuf,
    state: PathBuf,
    target: PathBuf,
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(FORJAR)
        .args(args)
        .arg("--no-color")
        .output()
        .expect("forjar must run");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A converged stack: applied once, lock written, file at its declared bytes.
fn converged() -> Stack {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target.txt");
    let cfg = dir.path().join("forjar.yaml");
    let state = dir.path().join("state");
    fs::write(&cfg, config_yaml(&target)).unwrap();
    let c = cfg.display().to_string();
    let s = state.display().to_string();
    let (code, out, err) = run(&["apply", "-f", &c, "--state-dir", &s, "--yes"]);
    assert_eq!(
        code,
        Some(0),
        "the fixture must converge first:\n{out}\n{err}"
    );
    assert_eq!(fs::read_to_string(&target).unwrap(), "declared\n");
    Stack {
        _dir: dir,
        cfg,
        state,
        target,
    }
}

impl Stack {
    fn drift(&self, extra: &[&str]) -> (Option<i32>, String, String) {
        let c = self.cfg.display().to_string();
        let s = self.state.display().to_string();
        let mut args = vec!["drift", "-f", &c, "--state-dir", &s];
        args.extend(extra.iter().copied());
        run(&args)
    }

    fn tamper(&self) {
        fs::write(&self.target, "tampered\n").unwrap();
    }
}

/// THE REGRESSION. One drifted file resource, no flag: the output says
/// DRIFTED and the exit code must say so too.
#[test]
fn a_drifted_file_resource_exits_1_without_any_flag() {
    let st = converged();
    st.tamper();
    let (code, out, err) = st.drift(&[]);
    assert!(
        out.contains("DRIFTED"),
        "the fixture must drift:\n{out}\n{err}"
    );
    assert_eq!(
        code,
        Some(1),
        "`Drift detected` reached stdout and not the exit code — fail-open by \
         default, the forjar#352 class.\nstdout:\n{out}\nstderr:\n{err}"
    );
}

/// THE CONTROL. Without it, `exit 1 always` discharges the test above.
#[test]
fn a_converged_stack_still_exits_0() {
    let st = converged();
    let (code, out, err) = st.drift(&[]);
    assert!(!out.contains("DRIFTED"), "{out}");
    assert_eq!(code, Some(0), "stdout:\n{out}\nstderr:\n{err}");
}

/// `--tripwire` is kept so every cron line on the fleet keeps parsing, and it
/// changes nothing: same verdict, same exit code, with or without it.
#[test]
fn tripwire_is_a_no_op() {
    let st = converged();
    st.tamper();
    let (plain, plain_out, _) = st.drift(&[]);
    let (flagged, flagged_out, _) = st.drift(&["--tripwire"]);
    assert_eq!(plain, Some(1));
    assert_eq!(
        flagged, plain,
        "--tripwire must not change the verdict's exit code"
    );
    assert_eq!(
        plain_out, flagged_out,
        "--tripwire must not change what is printed either"
    );
}

/// The exit code is not a property of the text renderer: `--json` reaches the
/// same verdict and the same code, because that is the form a machine reads.
#[test]
fn json_output_exits_1_on_drift_too() {
    let st = converged();
    st.tamper();
    let (code, out, err) = st.drift(&["--json"]);
    let first_line = out
        .lines()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or("");
    let report: serde_json::Value = serde_json::from_str(&out[out.find(first_line).unwrap_or(0)..])
        .unwrap_or_else(|e| {
            panic!("--json must print a JSON document: {e}\nstdout:\n{out}\nstderr:\n{err}")
        });
    assert!(report["drift_count"].as_u64().unwrap_or(0) >= 1, "{report}");
    assert_eq!(code, Some(1), "stdout:\n{out}\nstderr:\n{err}");
}

/// The help text must not sell `--tripwire` as the way to get an exit code —
/// that sentence was the documentation of the defect.
#[test]
fn help_no_longer_describes_tripwire_as_the_way_to_get_an_exit_code() {
    let (code, out, _) = run(&["drift", "--help"]);
    assert_eq!(code, Some(0));
    assert!(
        !out.contains("Exit non-zero on any drift (for CI/cron)"),
        "the old --tripwire help line documents fail-open as the default:\n{out}"
    );
    assert!(
        out.contains("--tripwire"),
        "the flag stays, for compatibility:\n{out}"
    );
}
