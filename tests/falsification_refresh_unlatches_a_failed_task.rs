//! `--refresh` must re-check a resource the lock records as FAILED.
//!
//! forjar#487, P0, reported by the operator against 1.25.2 and reproduced on two
//! fleet machines. A `type: task` may legitimately carry a command that always
//! fails — the pattern forjar itself makes necessary. Registering a GitHub
//! Actions runner needs an ephemeral API token that cannot live in a config, so
//! the resource's job is to refuse loudly and name the make target rather than
//! report converged on a box with no runner:
//!
//! ```yaml
//! runner-registered:
//!   type: task
//!   command: |
//!     echo "gx10 runner is not registered." >&2
//!     echo "Run: make -C machines/gx10 runner-register" >&2
//!     exit 1
//!   completion_check: |
//!     f=$RUNNER_DIR/.runner
//!     [ -f "$f" ] || exit 1
//!     grep -q '"poolName": *"gpu-nodes"' "$f" || exit 1
//! ```
//!
//! That works only while forjar SKIPS the resource, because the generated script
//! is command-then-check:
//!
//! ```sh
//! set -euo pipefail
//! <command>                   # exits 1, unconditionally
//! if ! { <completion_check> } # never reached
//! then ... exit 1 fi
//! ```
//!
//! So the moment the lock records the resource as failed, every later apply
//! re-runs a command that cannot succeed, never reaches the check, and
//! re-records the failure. A latch with no documented way out.
//!
//! MEASURED on yoga, on a correctly registered runner (`.runner` present with
//! the right `gitHubUrl` and `poolName`):
//!
//! - the completion_check PASSES by hand, including inside forjar's own
//!   `if ! { ... }` wrapper: `CHECK: PASS`;
//! - `forjar drift` SKIPS the resource — "not converged in the lock 3";
//! - `forjar apply --refresh -r runner-registered` still reported
//!   "0 converged, 0 unchanged, 2 failed".
//!
//! Reproduced on gx10, where the same resource had been converged for months,
//! failed once, and then stayed failed across repeated applies. With
//! `policy.failure: stop_on_first` a latched resource takes its dependents with
//! it; on gx10 it hid three others.
//!
//! `--refresh` is documented as "Re-run check scripts, only re-apply what
//! fails", and it did not re-check an entry already recorded as FAILED before
//! running its command. A resource the lock believes is broken is the one whose
//! live state is most worth measuring.
//!
//! These tests drive the real binary end to end against a state dir inside the
//! test's own tempdir — never the repository's `state/`, never the host's — so
//! they fail if the behaviour regresses, not if a string changes.

use std::fs;
use std::path::Path;

/// The operator's resource, reduced to its mechanism: a command that always
/// fails and writes a marker proving it ran, and a completion_check satisfied
/// exactly when the out-of-band registration has happened.
fn config_yaml(ran: &Path, registered: &Path) -> String {
    format!(
        r#"version: "1.0"
name: refresh-unlatch
machines:
  localhost:
    hostname: localhost
    addr: localhost
resources:
  runner-registered:
    type: task
    machine: localhost
    command: "touch {ran}; echo 'runner is not registered. Run: make runner-register' >&2; exit 1"
    completion_check: "test -f {reg}"
"#,
        ran = ran.display(),
        reg = registered.display(),
    )
}

fn forjar_bin() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

/// Run an apply against a state dir the test owns, and return (output, success).
fn apply(cfg: &Path, state: &Path, extra: &[&str]) -> (String, bool) {
    let mut c = std::process::Command::new(forjar_bin());
    c.arg("apply")
        .arg("-f")
        .arg(cfg)
        .arg("--state-dir")
        .arg(state)
        .arg("--yes");
    for e in extra {
        c.arg(e);
    }
    let out = c.output().expect("forjar must run");
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.success())
}

/// The fixture: a config, a state dir under the test's own tempdir, and a lock
/// entry recording the resource as FAILED — seeded the way the fleet seeded it,
/// by applying while the registration had not happened yet.
struct Latched {
    _dir: tempfile::TempDir,
    cfg: std::path::PathBuf,
    state: std::path::PathBuf,
    ran: std::path::PathBuf,
    registered: std::path::PathBuf,
}

fn latched() -> Latched {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let ran = dir.path().join("command-ran");
    let registered = dir.path().join("dot-runner");
    let cfg = dir.path().join("forjar.yaml");
    fs::write(&cfg, config_yaml(&ran, &registered)).unwrap();
    fs::create_dir_all(&state).unwrap();

    // Not registered yet, so the apply fails and the lock records FAILED. This
    // is the latch closing, and it is the state every later apply inherits.
    let (out, ok) = apply(&cfg, &state, &[]);
    assert!(
        !ok,
        "the unregistered guard must fail its first apply:\n{out}"
    );
    assert!(ran.exists(), "the command must have run:\n{out}");
    fs::remove_file(&ran).unwrap();

    Latched {
        _dir: dir,
        cfg,
        state,
        ran,
        registered,
    }
}

/// THE REGRESSION. The runner is registered — the completion_check passes — and
/// the only thing still claiming otherwise is a lock entry from before. Under
/// `--refresh` the check must be consulted BEFORE the command is run.
#[test]
fn refresh_rechecks_a_resource_the_lock_records_as_failed() {
    let f = latched();

    // The operator runs the make target. The check now passes; nothing else
    // changed.
    fs::write(&f.registered, "{\"poolName\": \"gpu-nodes\"}").unwrap();

    let (out, ok) = apply(&f.cfg, &f.state, &["--refresh"]);
    assert!(
        ok,
        "--refresh promises 'only re-apply what fails'. The completion_check \
         passes, so nothing here fails and the apply must be green:\n{out}"
    );
    assert!(
        !f.ran.exists(),
        "--refresh ran the command of a resource whose completion_check already \
         passes. The command exits 1 by design and the check is never reached, \
         so this is the latch: once failed, always failed.\noutput:\n{out}"
    );
    assert!(
        !out.contains("1 failed"),
        "the resource is satisfied on the host and must not be reported \
         failed:\n{out}"
    );
}

/// The same, scoped the way the operator ran it: `--refresh -r runner-registered`
/// was the measured command on yoga, and `refresh_in_scope` honours `-r`, so the
/// scoped path is a separate way for this to stay broken.
#[test]
fn refresh_rechecks_a_failed_resource_when_scoped_by_name() {
    let f = latched();
    fs::write(&f.registered, "{\"poolName\": \"gpu-nodes\"}").unwrap();

    let (out, ok) = apply(&f.cfg, &f.state, &["--refresh", "-r", "runner-registered"]);
    assert!(ok, "yoga's exact command must be green:\n{out}");
    assert!(
        !f.ran.exists(),
        "`--refresh -r runner-registered` re-ran the command instead of the \
         check — the measured 1.25.2 failure, verbatim.\noutput:\n{out}"
    );
}

/// THE NEGATIVE, and the reason the fix cannot simply forgive failed entries:
/// `--refresh` must not become a way to launder a genuinely broken resource into
/// converged. With the registration still missing, the check fails, the command
/// runs, and the apply is red.
#[test]
fn refresh_still_applies_and_still_fails_when_the_check_fails() {
    let f = latched();
    assert!(
        !f.registered.exists(),
        "precondition: the runner is NOT registered"
    );

    let (out, ok) = apply(&f.cfg, &f.state, &["--refresh"]);
    assert!(
        !ok,
        "the check fails, so --refresh must run the command and report the \
         failure. A green run here means a broken resource was laundered into \
         converged:\n{out}"
    );
    assert!(
        f.ran.exists(),
        "the check fails, so the command must have run:\n{out}"
    );
    assert!(
        out.contains("runner is not registered"),
        "the operator must still get the report that names the make target:\n{out}"
    );
}

/// The scope boundary, asserted so the fix cannot quietly widen. Without
/// `--refresh` nothing changes: a plain apply is lock-based, sees a FAILED
/// entry, plans Update, and runs the command — even though the completion_check
/// would now pass. That is what a plain apply has always done, and PMAT-214
/// does not touch it.
#[test]
fn a_plain_apply_still_reruns_the_command_of_a_failed_resource() {
    let f = latched();
    fs::write(&f.registered, "{\"poolName\": \"gpu-nodes\"}").unwrap();

    let (out, ok) = apply(&f.cfg, &f.state, &[]);
    assert!(
        !ok,
        "a plain apply must still run the failing command:\n{out}"
    );
    assert!(
        f.ran.exists(),
        "without --refresh the lock is the authority and the command runs:\n{out}"
    );
}
