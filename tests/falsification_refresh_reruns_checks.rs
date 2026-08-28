//! `--refresh` must re-run check scripts and act on their verdict.
//!
//! The flag's `--help` says: "FJ-3010: Re-run check scripts, only re-apply what
//! fails (softer than --force)". It did neither. `ApplyConfig::refresh` was
//! declared, described in a comment, set to `false` in dozens of call sites,
//! and **never read anywhere in production code** — `grep -rn refresh src/`
//! returns the declaration, the comment, and test fixtures. Nothing else.
//!
//! So `--refresh` silently behaved exactly like a plain apply.
//!
//! Measured on paiml/infra's intel, 2026-08-19, with `pzsh` replaced by a
//! symlink to a non-existent target (the exact wreckage rust-cache's post step
//! leaves in a shared ~/.cargo/bin):
//!
//!     $ forjar apply -r stack-tool-pzsh --refresh --yes
//!     intel: 0 converged, 1 unchanged, 0 failed (0.1s)
//!     Apply complete: 0 converged, 1 unchanged.
//!
//!     $ pzsh --version
//!     command not found: pzsh
//!
//! 0.1 seconds — it never contacted the host. The resource's own check script,
//! run by hand against the same host at the same moment, printed `missing:pzsh`
//! and exited 1. The check was correct; nothing ever asked it.
//!
//! That is the mechanism by which the 2026-08-19 toolchain outage stayed
//! invisible: `forjar apply -t stack-tools` reported 5 of 5 resources converged
//! on a host with no rustup, no cargo and no rustc, because it compared the
//! config against its own lock file and never looked at the machine.
//!
//! These tests execute a real apply against `localhost` with a real state dir,
//! so they fail if the flag stops working end-to-end — not if a string changes.

use std::fs;
use std::path::Path;

/// A config with one `task` resource whose completion_check tests for a marker
/// file. The command creates it. So: converged iff the marker exists.
fn config_yaml(marker: &Path) -> String {
    format!(
        r#"version: "1.0"
name: refresh-test
machines:
  localhost:
    hostname: localhost
    addr: localhost
resources:
  marker:
    type: task
    machine: localhost
    command: "touch {m}"
    completion_check: "test -f {m}"
"#,
        m = marker.display()
    )
}

fn forjar_bin() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

/// Run an apply and return (stdout+stderr, success).
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

#[test]
fn refresh_reapplies_a_resource_whose_live_check_now_fails() {
    // THE REGRESSION. Converge, then destroy the result out of band — exactly
    // what a CI job deleting the shared toolchain does — and ask --refresh to
    // notice. Before the fix it returned "unchanged" without contacting
    // anything, and the marker stayed deleted.
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let marker = dir.path().join("marker");
    let cfg = dir.path().join("forjar.yaml");
    fs::write(&cfg, config_yaml(&marker)).unwrap();
    fs::create_dir_all(&state).unwrap();

    let (out, ok) = apply(&cfg, &state, &[]);
    assert!(ok, "initial apply must converge:\n{out}");
    assert!(marker.exists(), "the command must have created the marker");

    // Out-of-band destruction. The lock still says converged.
    fs::remove_file(&marker).unwrap();
    assert!(!marker.exists());

    let (out, ok) = apply(&cfg, &state, &["--refresh"]);
    assert!(ok, "--refresh apply must succeed:\n{out}");
    assert!(
        marker.exists(),
        "--refresh claims to re-run check scripts and re-apply what fails. The \
         check (`test -f marker`) fails and the marker is still gone, so it did \
         neither.\noutput:\n{out}"
    );
}

#[test]
fn refresh_does_not_reapply_a_resource_that_is_genuinely_converged() {
    // --refresh must stay SOFTER than --force, or it is just a slow --force and
    // people will reach for neither. A resource whose check passes must be left
    // alone.
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let marker = dir.path().join("marker");
    let cfg = dir.path().join("forjar.yaml");
    fs::write(&cfg, config_yaml(&marker)).unwrap();
    fs::create_dir_all(&state).unwrap();

    let (out, ok) = apply(&cfg, &state, &[]);
    assert!(ok, "initial apply must converge:\n{out}");

    // Nothing destroyed this time. Record when the marker was made so we can
    // tell "left alone" from "re-created", which a converged-count alone cannot.
    let before = fs::metadata(&marker).unwrap().modified().unwrap();

    let (out, ok) = apply(&cfg, &state, &["--refresh"]);
    assert!(ok, "--refresh on a healthy host must succeed:\n{out}");
    assert!(
        out.contains("unchanged"),
        "a converged resource must report unchanged under --refresh:\n{out}"
    );

    let after = fs::metadata(&marker).unwrap().modified().unwrap();
    assert_eq!(
        before, after,
        "--refresh must not re-apply a resource whose check passes"
    );
}

#[test]
fn a_plain_apply_converges_a_task_the_way_it_converges_a_file() {
    // CHANGED DELIBERATELY, which is what the previous version of this test
    // asked for by name. It was `a_plain_apply_still_trusts_the_lock`, and it
    // asserted the marker STAYED deleted:
    //
    //     "a plain apply is documented as lock-based; if it now repairs
    //      out-of-band damage, --refresh has no distinct meaning and every
    //      apply just got slower. Change this test deliberately, not by
    //      accident."
    //
    // That premise stopped being true in 1.18.0, for files. `apply` converges a
    // file changed on the target — that release's headline, taken after a 3/3
    // NO-GO gate. Verified on main: `apply_restores_a_tampered_source_file`
    // passes.
    //
    // So the fleet was left asserting BOTH: a plain apply repairs a drifted
    // file and does not repair a drifted task. That inconsistency is harder to
    // reason about than either rule alone — an operator cannot answer "does
    // apply fix this?" without first knowing the resource type — and it is
    // exactly forjar#279: a task's drift observable was `echo` of its own
    // declaration, so a task could never drift and the question never arose.
    //
    // The cost objection stands and is answered rather than dismissed: the
    // observable is the resource's OWN `completion_check`, which apply already
    // runs. No new round-trip is introduced for a task that declares one, and a
    // task that declares none stays unobservable and costs nothing.
    //
    // --refresh keeps its distinct meaning: it re-checks resources the lock
    // reports Converged regardless of what drift detection can see, which is
    // what `refresh_reapplies_a_resource_whose_live_check_now_fails` above
    // pins.
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let marker = dir.path().join("marker");
    let cfg = dir.path().join("forjar.yaml");
    fs::write(&cfg, config_yaml(&marker)).unwrap();
    fs::create_dir_all(&state).unwrap();

    let (out, ok) = apply(&cfg, &state, &[]);
    assert!(ok, "initial apply must converge:\n{out}");
    fs::remove_file(&marker).unwrap();

    let (_out, ok) = apply(&cfg, &state, &[]);
    assert!(ok);
    assert!(
        marker.exists(),
        "a plain apply repaired a drifted FILE but not a drifted TASK. Whether \
         apply fixes out-of-band damage must not depend on the resource type — \
         an operator cannot answer 'does apply fix this?' without first \
         knowing what kind of thing it is."
    );
}

/// THE GAP THE TWO TESTS ABOVE LEAVE: both converge FIRST, so a lock always
/// exists by the time `--refresh` runs.
///
/// `refresh_locks` could only ever REMOVE lock entries whose check failed. With
/// an EMPTY lock there is nothing to remove, so `--refresh` contacted the host,
/// learned the check passed, and planned `create` anyway.
///
/// An empty lock is not an edge case — it is every CI checkout, every reimaged
/// box, every `--state-dir` that has not been written yet. It is also the only
/// state in which forjar can be used to express an ASSERTION: a guard whose
/// `completion_check` is the claim and whose `command` reports the violation.
/// Without this, every such guard runs its failure path on a healthy host.
#[test]
fn refresh_does_not_reapply_when_the_host_is_converged_and_there_is_no_lock() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let marker = dir.path().join("marker");
    let cfg = dir.path().join("forjar.yaml");
    fs::write(&cfg, config_yaml(&marker)).unwrap();
    fs::create_dir_all(&state).unwrap();

    // The host is ALREADY in the declared state, and nothing has ever been
    // applied here — no lock, by construction.
    fs::write(&marker, "").unwrap();
    let before = fs::metadata(&marker).unwrap().modified().unwrap();

    let (out, ok) = apply(&cfg, &state, &["--refresh"]);
    assert!(ok, "--refresh apply must succeed:\n{out}");
    assert!(
        out.contains("1 unchanged"),
        "the host satisfies the check and there is no lock, so --refresh must \
         report it UNCHANGED rather than re-applying. It said:\n{out}"
    );
    assert_eq!(
        fs::metadata(&marker).unwrap().modified().unwrap(),
        before,
        "the command ran even though the check already passed — `touch` moved \
         the marker's mtime. --refresh promises 'only re-apply what fails'."
    );
}

/// The other half of the same contract, and the reason the fix cannot simply
/// seed every resource: a FAILING check with no lock must still apply.
///
/// If seeding were unconditional, a fresh checkout would report everything
/// converged and provision nothing — strictly worse than the bug it replaces.
#[test]
fn refresh_still_applies_when_the_host_is_diverged_and_there_is_no_lock() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let marker = dir.path().join("marker");
    let cfg = dir.path().join("forjar.yaml");
    fs::write(&cfg, config_yaml(&marker)).unwrap();
    fs::create_dir_all(&state).unwrap();

    assert!(!marker.exists(), "precondition: the host is NOT converged");

    let (out, ok) = apply(&cfg, &state, &["--refresh"]);
    assert!(ok, "--refresh apply must succeed:\n{out}");
    assert!(
        marker.exists(),
        "the check fails and there is no lock, so the command must run:\n{out}"
    );
}

/// A guard expressed as a forjar resource: `completion_check` is the assertion,
/// `command` is the violation report.
///
/// This is the shape paiml/infra needs to move its CI guards off Python and onto
/// forjar. It only works if a satisfied check on an unlocked host is UNCHANGED
/// (green) and a violated one runs the command (red). Both directions asserted
/// here so neither can regress alone.
#[test]
fn a_guard_resource_is_green_when_satisfied_and_red_when_violated() {
    let dir = tempfile::tempdir().unwrap();
    let offender = dir.path().join("offender");
    let cfg = dir.path().join("forjar.yaml");
    fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: guard
machines:
  localhost:
    hostname: localhost
    addr: localhost
resources:
  no-offender:
    type: task
    machine: localhost
    command: "echo 'GUARD FAILED' >&2; exit 1"
    completion_check: "test ! -f {o}"
"#,
            o = offender.display()
        ),
    )
    .unwrap();

    let green_state = dir.path().join("green");
    fs::create_dir_all(&green_state).unwrap();
    let (out, ok) = apply(&cfg, &green_state, &["--refresh"]);
    assert!(ok, "a satisfied guard must exit 0:\n{out}");
    assert!(out.contains("1 unchanged"), "expected unchanged:\n{out}");

    fs::write(&offender, "").unwrap();
    let red_state = dir.path().join("red");
    fs::create_dir_all(&red_state).unwrap();
    let (out, ok) = apply(&cfg, &red_state, &["--refresh"]);
    assert!(!ok, "a violated guard must exit non-zero:\n{out}");
    assert!(out.contains("GUARD FAILED"), "expected the report:\n{out}");
}
