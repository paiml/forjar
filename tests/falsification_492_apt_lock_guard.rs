//! forjar#492: every `apt-get update|install` in a workflow waits for the apt lock.
//!
//! # The defect
//!
//! Sixteen clean-room runners share one machine, so two legs reach `apt-get` at
//! once. Without `-o DPkg::Lock::Timeout=N` apt gives up at once on the held
//! lock and the leg fails (release `build-binaries` failed this way).
//!
//! # What this test runs
//!
//! `scripts/apt-lock-timeout-guard.sh` on the real `.github/workflows`, and
//! `scripts/apt-lock-timeout-guard-test.sh`, which feeds the guard one planted
//! defect per case (a trailing comment that names the option, `Timeout=0`, a
//! chained second call, a continuation line, echo prose, a path with a space)
//! and fails if the guard's exit code is not the expected one. Running both
//! under `cargo test` is what keeps the guard from rotting unobserved; this
//! file is not listed in any workflow or required-check set.
//!
//! # mutations — each measured to redden this test
//!
//! 1. Delete `-o DPkg::Lock::Timeout=600` from one `apt-get` call in
//!    `release.yml` → `guard_passes_on_the_real_workflows` is RED.
//! 2. Make the guard accept `Timeout=0` → the `timeout-zero` case of the script
//!    fails, so `guard_discriminates_planted_defects` is RED (measured on the
//!    script only: the RED run of the cases; the cargo wrapper was not mutated).

use std::path::PathBuf;
use std::process::{Command, Output};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bash(script: &str, args: &[&str]) -> Output {
    let path = root().join("scripts").join(script);
    assert!(path.is_file(), "{} is missing", path.display());
    Command::new("bash")
        .arg(&path)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("run bash {}: {e}", path.display()))
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

#[test]
fn guard_passes_on_the_real_workflows() {
    let wf = root().join(".github/workflows");
    let o = bash(
        "apt-lock-timeout-guard.sh",
        &[wf.to_str().expect("utf-8 path")],
    );
    assert_eq!(o.status.code(), Some(0), "guard output:\n{}", text(&o));
    let out = text(&o);
    let scanned: usize = out
        .split("apt-lock-timeout-guard: ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    assert!(
        scanned > 0,
        "the guard scanned no apt-get call (a pass on nothing):\n{out}"
    );
}

#[test]
fn guard_discriminates_planted_defects() {
    let o = bash("apt-lock-timeout-guard-test.sh", &[]);
    assert_eq!(o.status.code(), Some(0), "self-test output:\n{}", text(&o));
    assert!(
        text(&o).contains("PASS apt-lock-timeout-guard"),
        "{}",
        text(&o)
    );
}

#[test]
fn guard_self_test_flag_runs_the_same_cases() {
    let o = bash("apt-lock-timeout-guard.sh", &["--self-test"]);
    assert_eq!(
        o.status.code(),
        Some(0),
        "--self-test output:\n{}",
        text(&o)
    );
}
