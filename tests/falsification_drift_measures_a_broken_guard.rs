//! forjar#487 (second half): `drift` SKIPPED the resources the lock believed
//! were broken, which are the ones whose live state is most worth measuring.
//!
//! The operator measured it on two machines:
//!
//! ```text
//! skipped 10: in the lock, not in the config 7, not converged in the lock 3
//! ```
//!
//! Those three were `type: task` guards — the refuse-loudly pattern forjar
//! itself makes necessary, where the `command` exits 1 by design and the
//! `completion_check` is the real assertion. Once an apply recorded them
//! `failed`, drift stopped executing their checks, so the one observable that
//! could have said "the runner IS registered" was never run.
//!
//! THE DISTINCTION THIS RULE PINS. `src/tripwire/drift/mod.rs` excludes
//! `Failed`/`Unknown` from the HASH-comparison paths for a good reason, written
//! down there: a failed apply's recorded hash is not a baseline anything can be
//! compared against. That reasoning is correct, and it does not apply here. A
//! `completion_check` is an ASSERTION, not a baseline — it asks the host a
//! question and reads the answer, and it needs nothing from the lock at all. A
//! failed apply does not make that question unanswerable; it makes it urgent.
//!
//! So this rule requires the task path to evaluate a guard the lock records as
//! failed, and deliberately does NOT require the same of the file, image or
//! state-query paths, which have nothing to compare against.
//!
//! DRIVEN THROUGH THE REAL BINARY, because the census line and the exit code
//! are both things the operator reads.

use std::fs;
use std::path::Path;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

/// The operator's resource, reduced to its mechanism: a command that always
/// fails and names the human step, and a `completion_check` satisfied exactly
/// when the out-of-band registration has happened.
fn config_yaml(ran: &Path, registered: &Path) -> String {
    format!(
        r#"version: "1.0"
name: guard-drift
machines:
  box:
    hostname: box
    addr: 127.0.0.1
resources:
  runner-registered:
    type: task
    machine: box
    command: "touch {ran}; echo 'runner is not registered. Run: make runner-register' >&2; exit 1"
    completion_check: "test -f {reg}"
"#,
        ran = ran.display(),
        reg = registered.display(),
    )
}

struct Latched {
    _dir: tempfile::TempDir,
    cfg: std::path::PathBuf,
    state: std::path::PathBuf,
    registered: std::path::PathBuf,
}

fn run(args: &[&str]) -> (String, bool) {
    let out = std::process::Command::new(FORJAR)
        .args(args)
        .arg("--no-color")
        .output()
        .expect("forjar must run");
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.success())
}

/// A guard whose lock entry says `failed`, seeded the way the fleet seeded it:
/// by applying while the registration had not happened yet.
fn latched(name: &str) -> Latched {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let ran = dir.path().join(format!("ran-{name}"));
    let registered = dir.path().join(format!("dot-runner-{name}"));
    let cfg = dir.path().join("forjar.yaml");
    fs::write(&cfg, config_yaml(&ran, &registered)).unwrap();
    fs::create_dir_all(&state).unwrap();

    let c = cfg.display().to_string();
    let s = state.display().to_string();
    let (out, ok) = run(&["apply", "-f", &c, "--state-dir", &s, "--yes"]);
    assert!(!ok, "the unregistered guard must fail its first apply:\n{out}");

    let lock = fs::read_to_string(state.join("box").join("state.lock.yaml")).unwrap();
    assert!(
        lock.contains("status: failed"),
        "the fixture must arm the latch, or this rule measures nothing:\n{lock}"
    );

    Latched {
        _dir: dir,
        cfg,
        state,
        registered,
    }
}

impl Latched {
    fn drift(&self, extra: &[&str]) -> (String, bool) {
        let c = self.cfg.display().to_string();
        let s = self.state.display().to_string();
        let mut args = vec!["drift", "-f", &c, "--state-dir", &s];
        args.extend(extra.iter().copied());
        run(&args)
    }
}

/// THE REGRESSION. The registration happened, so the assertion is TRUE, and the
/// only thing still claiming otherwise is a lock entry from before.
#[test]
fn a_guard_the_lock_calls_broken_is_measured_not_skipped() {
    let f = latched("passing");
    fs::write(&f.registered, "{\"poolName\": \"gpu-nodes\"}").unwrap();

    let (out, _) = f.drift(&[]);
    assert!(
        !out.contains("not converged in the lock"),
        "forjar#487: drift skipped the resource the lock believes is broken, \
         which is the one whose live state is most worth measuring. Its \
         completion_check is an ASSERTION, not a baseline — it needs nothing \
         from the lock to be answerable.\n{out}"
    );
    assert!(
        out.contains("inspected 1 of 1"),
        "the guard must be counted as INSPECTED, not merely absent from the \
         skip list.\n{out}"
    );
    assert!(
        out.contains("No drift detected"),
        "the assertion is true, so there is no drift to report.\n{out}"
    );
}

/// AND THE OTHER DIRECTION, which is the whole point of a guard. The lock says
/// broken and the assertion is still FALSE: that is drift, and a fleet lane has
/// to see it.
#[test]
fn a_guard_whose_assertion_is_still_false_is_reported_as_drift() {
    let f = latched("failing");
    // registration never happened; the check fails

    let (out, _) = f.drift(&[]);
    assert!(
        out.contains("inspected 1 of 1"),
        "the guard must be measured before it can be reported.\n{out}"
    );
    assert!(
        out.contains("DRIFTED") && out.contains("runner-registered"),
        "forjar#487: a guard whose assertion is false is drift, and reporting \
         `No drift detected.` over it is the false green this rule exists to \
         prevent.\n{out}"
    );

    let (tw, ok) = f.drift(&["--tripwire"]);
    assert!(
        !ok,
        "--tripwire is the CI gate: it must exit non-zero on a guard whose \
         assertion is false, or the fleet lane stays green over a broken \
         box.\n{tw}"
    );
}

/// THE CENSUS MUST STILL DISTINGUISH. Measuring more resources must not turn
/// the census into decoration: something that genuinely cannot be evaluated is
/// still counted as not inspected, never as clean.
#[test]
fn what_cannot_be_evaluated_is_still_not_counted_as_clean() {
    let f = latched("census");
    fs::write(&f.registered, "x").unwrap();

    let (out, _) = f.drift(&["--no-task-checks"]);
    assert!(
        out.contains("--no-task-checks"),
        "declining to run the checks must still be NAMED in the census — an \
         unmeasured check and a passed check must never print the same \
         thing.\n{out}"
    );
    assert!(
        out.contains("inspected 0 of 1") || out.contains("not inspected"),
        "and it must not be counted as inspected.\n{out}"
    );
}
