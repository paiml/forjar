//! forjar#522: a ratchet measurement must not be able to eat the machine.
//!
//! # What happened
//!
//! `scripts/ratchets/comply-count.sh` measures one comply check by running
//! `pmat comply check --format json`. `pmat comply check` evaluates
//! `.pmat-ratchet.toml` (CB-2102) and RUNS every measurement command it
//! declares. A ratchet entry naming this script is therefore a cycle by
//! construction, and on 2026-09-11 one was declared. Measured on a 48-core box
//! that had been idle:
//!
//! | metric | value |
//! |---|---|
//! | total processes | 9,740 |
//! | `comply-count.sh` copies | 5,781 |
//! | `pmat comply check` copies | 1,462 |
//! | load average, 1 min | 379, peaking at 3,026 |
//!
//! It was killed by hand. The `.pmat-ratchet.toml` that caused it was never
//! committed, so the cycle cannot occur in this repository today — but that is
//! the absence of an input, not a safeguard, and the next person to declare a
//! ratchet metric would rediscover it.
//!
//! # What these pin
//!
//! The sentinel refuses the re-entry and exits NON-ZERO without printing a
//! count, because a `0` would read as "this check reports no findings" — the
//! largest improvement in the project's history, spelled identically to a
//! script eating the machine. And no committed ratchet config may name this
//! script, so the cycle cannot be re-declared without this test going red.

#![cfg(unix)]

use std::path::PathBuf;
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run(env: &[(&str, &str)], args: &[&str]) -> (i32, String, String) {
    let mut c = Command::new("bash");
    c.arg(repo().join("scripts/ratchets/comply-count.sh"))
        .args(args)
        .current_dir(repo());
    for (k, v) in env {
        c.env(k, v);
    }
    let out = c.output().expect("bash must run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Under the sentinel the script refuses, says why, and prints NO count.
#[test]
fn a_recursive_invocation_is_refused_and_prints_no_count() {
    let (code, stdout, stderr) = run(&[("COMPLY_COUNT_ACTIVE", "1")], &["CB-2110"]);
    assert_eq!(
        code, 3,
        "a re-entry must exit 3, not run:\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.trim().is_empty(),
        "a refused measurement printed {stdout:?} on stdout — a count there is read as \
         the check's finding count, and 0 would read as perfection"
    );
    assert!(
        stderr.contains("refusing to run inside itself"),
        "the refusal does not say what it is refusing:\n{stderr}"
    );
}

/// The sentinel is EXPORTED, so the re-entry the guard is about is the one it
/// catches: the child `pmat comply check` must see it.
#[test]
fn the_sentinel_reaches_the_child_process() {
    let body = std::fs::read_to_string(repo().join("scripts/ratchets/comply-count.sh"))
        .expect("the script must exist");
    assert!(
        body.contains("export COMPLY_COUNT_ACTIVE=1"),
        "the sentinel is set but not exported, so the child pmat run would not see it and \
         the cycle would be unguarded"
    );
    let guard = body
        .find("COMPLY_COUNT_ACTIVE:-")
        .expect("the script must test the sentinel");
    let set = body
        .find("export COMPLY_COUNT_ACTIVE=1")
        .expect("the script must set the sentinel");
    assert!(
        guard < set,
        "the sentinel is exported before it is tested, so the script would refuse itself \
         on the first call"
    );
}

/// No committed ratchet config may name this script.
///
/// The cycle needs two halves: a script that runs comply, and a config that
/// makes comply run the script. The sentinel breaks the second entry into the
/// loop; this refuses the declaration that opens it.
#[test]
fn no_committed_ratchet_config_names_this_script() {
    let cfg = repo().join(".pmat-ratchet.toml");
    if let Ok(body) = std::fs::read_to_string(&cfg) {
        assert!(
            !body.contains("comply-count.sh"),
            ".pmat-ratchet.toml declares a metric measured by comply-count.sh, which runs \
             `pmat comply check`, which runs every metric in .pmat-ratchet.toml. That cycle \
             took a 48-core box to 9,740 processes and load 3,026 (forjar#522)."
        );
        assert!(
            !body.contains("pmat comply check") && !body.contains("pmat comply ratchet"),
            ".pmat-ratchet.toml declares a metric whose command runs pmat comply, which \
             evaluates .pmat-ratchet.toml — a cycle by construction (forjar#522)"
        );
    }
}

/// The process cap is measured in the unit the kernel compares.
///
/// `ulimit -u` is RLIMIT_NPROC: per USER, counting THREADS. A fixed cap and a
/// process-derived one were each tried and each killed the script's own fork on
/// a machine already running 226 processes and 2,352 threads. A cap that fails
/// on a busy box is the gate going red for the wrong reason.
#[test]
fn the_process_cap_counts_threads_and_is_relative() {
    let body = std::fs::read_to_string(repo().join("scripts/ratchets/comply-count.sh"))
        .expect("the script must exist");
    assert!(
        body.contains("ps -u \"$(id -un)\" -L --no-headers"),
        "the cap is not derived from the THREAD count (`ps -L`), and RLIMIT_NPROC compares \
         threads — a process-derived cap kills the script on any busy machine"
    );
    assert!(
        body.contains("ulimit -u $((threads + 512))"),
        "the cap is not relative to the measured thread count with headroom for a comply run \
         (measured: one run costs about 134 threads)"
    );
    // And the script still measures, on this very machine, with the cap on.
    let (code, stdout, _) = run(&[], &["CB-2110"]);
    assert_eq!(
        code, 0,
        "the cap must not stop the measurement it guards; it printed {stdout:?}"
    );
    assert!(
        stdout.trim().parse::<i64>().is_ok(),
        "the measurement printed {stdout:?}, which is not a count"
    );
}
