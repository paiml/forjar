//! PMAT-5880 / paiml/infra#2307: a runtime secret must reach a 0600 file on the
//! machine and NOTHING else forjar writes or prints.
//!
//! #406 closed the run transcripts under `state/<m>/runs/`. It did not close the
//! failure EXCERPT: when a command echoes its secret (`set -x`, an error naming
//! its argument) and fails, `failure_text::exec_failure` copies the tail of
//! stdout/stderr, unredacted, into the lock, `events.jsonl`, `last-apply.yaml`
//! and the process's own stderr. This test greps every file under the state dir
//! AND the captured process output, at both wave widths, for the canary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CANARY: &str = "2307-CANARY-Hs8Qw3Zx7Lp1Vb6Nm-NEVER-LOGGED";
const SECRET_ENV: &str = "FORJAR_SECRET_GATE_TOKEN";

fn walk(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return found;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            found.extend(walk(&p));
        } else {
            found.push(p);
        }
    }
    found
}

/// Apply a config whose task writes the secret to a 0600 file, then echoes it
/// on both streams. `fail` makes the task exit 3 after echoing.
fn run(parallel: bool, fail: bool) -> (tempfile::TempDir, i32, String) {
    let dir = tempfile::tempdir().unwrap();
    let out_file = dir.path().join("delivered-token");
    let tail = if fail { "exit 3" } else { "true" };
    let cfg = dir.path().join("forjar.yaml");
    fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: t2307\npolicy: {{ parallel_resources: {parallel} }}\n\
             machines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\n\
             secrets:\n  provider: env\n  ephemeral: true\n\
             resources:\n  deliver:\n    type: task\n    machine: local\n    command: |\n\
             \x20     umask 077\n\
             \x20     printf %s '{{{{secrets.gate-token}}}}' > {out}\n\
             \x20     echo \"out={{{{secrets.gate-token}}}}\"\n\
             \x20     echo \"err={{{{secrets.gate-token}}}}\" >&2\n\
             \x20     {tail}\n\x20   completion_check: test -f {out}\n",
            out = out_file.display(),
        ),
    )
    .unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .env(SECRET_ENV, CANARY)
        .args([
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            dir.path().join("state").to_str().unwrap(),
            "--yes",
        ])
        .output()
        .expect("forjar failed to start");
    let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&o.stderr));
    (dir, o.status.code().unwrap_or(-1), s)
}

fn assert_no_leak(dir: &tempfile::TempDir, printed: &str, what: &str) {
    let mut leaked: Vec<String> = Vec::new();
    for p in walk(&dir.path().join("state")) {
        if let Ok(b) = fs::read(&p) {
            if String::from_utf8_lossy(&b).contains(CANARY) {
                leaked.push(p.strip_prefix(dir.path()).unwrap().display().to_string());
            }
        }
    }
    if printed.contains(CANARY) {
        leaked.push("<forjar stdout/stderr>".into());
    }
    assert!(leaked.is_empty(), "{what}: plaintext secret in {leaked:?}");
}

fn assert_delivered_0600(dir: &tempfile::TempDir) {
    use std::os::unix::fs::PermissionsExt;
    let f = dir.path().join("delivered-token");
    assert_eq!(
        fs::read_to_string(&f).unwrap(),
        CANARY,
        "delivery must work"
    );
    assert_eq!(
        fs::metadata(&f).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn failing_task_excerpt_never_carries_the_secret_width_1() {
    let (dir, rc, printed) = run(false, true);
    assert_ne!(rc, 0, "the task must fail for this to be the excerpt path");
    assert_delivered_0600(&dir);
    assert_no_leak(&dir, &printed, "failure, width 1");
}

#[test]
fn failing_task_excerpt_never_carries_the_secret_parallel() {
    let (dir, rc, printed) = run(true, true);
    assert_ne!(rc, 0);
    assert_delivered_0600(&dir);
    assert_no_leak(&dir, &printed, "failure, parallel");
}

#[test]
fn succeeding_task_output_never_carries_the_secret() {
    let (dir, rc, printed) = run(false, false);
    assert_eq!(rc, 0, "{printed}");
    assert_delivered_0600(&dir);
    assert_no_leak(&dir, &printed, "success");
}
