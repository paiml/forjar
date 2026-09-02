//! Refs #378 — `forced_noop_count` must describe the run that happened, not a
//! shadow plan nothing reconciled with it.
//!
//! `executor::forced_noop_count` builds its own shadow plan and returns a raw
//! `action == NoOp` count. The planner is never shown `resource_filter` or
//! `group_filter`; the executor applies both, routing the excluded resources to
//! `ResourceOutcome::Skipped`, which `MachineCounters::record` maps to no
//! counter at all. A failure takes the other escape, `Failed`. So any run where
//! a planned NoOp is skipped by `-r`/`-g` or fails reports
//! `forced_noop_count > total_converged`.
//!
//! Measured on a debug binary built from v1.24.0's tree, a converged
//! two-resource stack:
//!
//! ```text
//!   $ forjar apply --yes --json --force -r marker
//!   rc=134 (SIGABRT), stdout 0 bytes
//!   thread 'main' panicked at src/cli/apply_summary.rs:69:5:
//!   C3-FORCE-DISTINGUISHABLE violated: forced_noop (2) > converged (1)
//! ```
//!
//! and on the released 1.24.0 binary, where `debug_assert!` is compiled out:
//!
//! ```text
//!   {"total_converged": 1, "total_failed": 0,
//!    "forced_noop_count": 2, "actual_changes": 0}
//!   note: --force re-ran 2 resource(s) the lock reported as unchanged
//!         (0 differed from the lock, 2 matched it).
//! ```
//!
//! — a `--json` summary that violates its own contract
//! (`apply-summary-distinguishability-v1`) and a note line naming twice the
//! resources the run touched. The SIGABRT matters beyond debug builds: this
//! repo's own integration tests run `CARGO_BIN_EXE_forjar`, which carries
//! `panic = "abort"`, so any test pairing `--force` with a selector dies with
//! zero stdout and no JSON to diagnose.
//!
//! The fix intersects the shadow-plan candidates with what the run actually
//! converged — the same oracle `apply_drift::repaired` already uses for the
//! drift dimension of the very same summary line.

use std::path::Path;
use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// `run_forjar_apply` in the sibling FJ-129 suite asserts `status.success()`,
/// which cannot observe a SIGABRT with an empty stdout. This one returns the
/// raw `Output`.
fn apply_raw(yaml: &Path, state_dir: &Path, extra: &[&str]) -> std::process::Output {
    forjar()
        .arg("apply")
        .arg("-f")
        .arg(yaml)
        .arg("--state-dir")
        .arg(state_dir)
        .arg("--yes")
        .arg("--json")
        .args(extra)
        .output()
        .expect("spawn forjar apply")
}

fn summary(out: &std::process::Output) -> serde_json::Value {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("parse apply --json ({e}); stdout was {stdout:?}"));
    doc["summary"].clone()
}

fn u(v: &serde_json::Value, k: &str) -> u64 {
    v[k].as_u64()
        .unwrap_or_else(|| panic!("summary.{k} missing or not a number: {v}"))
}

/// Two file resources in DIFFERENT resource groups, so `-r` and `-g` each have
/// something to select and something to miss.
fn write_yaml(dir: &Path, marker: &Path, marker2: &Path) -> std::path::PathBuf {
    let yaml = format!(
        "version: \"1.0\"\n\
         name: fj378\n\
         machines:\n\
         \x20 localhost:\n\
         \x20   hostname: localhost\n\
         \x20   addr: 127.0.0.1\n\
         resources:\n\
         \x20 marker:\n\
         \x20   type: file\n\
         \x20   machine: localhost\n\
         \x20   path: {}\n\
         \x20   state: file\n\
         \x20   content: \"fj378 marker\"\n\
         \x20   mode: \"0644\"\n\
         \x20   resource_group: alpha\n\
         \x20 marker2:\n\
         \x20   type: file\n\
         \x20   machine: localhost\n\
         \x20   path: {}\n\
         \x20   state: file\n\
         \x20   content: \"fj378 marker2\"\n\
         \x20   mode: \"0644\"\n\
         \x20   resource_group: beta\n",
        marker.display(),
        marker2.display()
    );
    let path = dir.join("forjar.yaml");
    std::fs::write(&path, yaml).expect("write yaml");
    path
}

struct Fx {
    _dir: tempfile::TempDir,
    yaml: std::path::PathBuf,
    state: std::path::PathBuf,
    marker: std::path::PathBuf,
}

/// A fully converged two-resource stack — the state in which every planned
/// change is a NoOp and `--force` re-runs them all.
fn converged() -> Fx {
    let dir = tempfile::tempdir().expect("tempdir");
    let marker = dir.path().join("marker.txt");
    let marker2 = dir.path().join("marker2.txt");
    let yaml = write_yaml(dir.path(), &marker, &marker2);
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();

    let first = apply_raw(&yaml, &state, &[]);
    assert!(
        first.status.success(),
        "setup apply: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    Fx {
        _dir: dir,
        yaml,
        state,
        marker,
    }
}

/// The load-bearing assertions, shared by the `-r` and `-g` shapes: a run that
/// converged exactly one of two resources reports exactly one forced no-op.
fn assert_one_forced_noop(out: &std::process::Output, shape: &str) {
    assert!(
        out.status.code().is_some(),
        "{shape}: forjar died on a signal (SIGABRT from the contract \
         debug_assert), so there is no summary to read at all"
    );
    assert!(
        !out.stdout.is_empty(),
        "{shape}: --json produced no stdout — the process aborted before printing"
    );
    let s = summary(out);
    let forced = u(&s, "forced_noop_count");
    let converged = u(&s, "total_converged");
    assert!(
        forced <= converged,
        "{shape}: forced_noop ({forced}) > converged ({converged}) — the count \
         describes a shadow plan, not the run"
    );
    assert_eq!(converged, 1, "{shape}: exactly one resource was selected");
    assert_eq!(
        forced, 1,
        "{shape}: the resource the run skipped was never forced"
    );
    assert_eq!(
        u(&s, "actual_changes"),
        0,
        "{shape}: nothing genuinely changed"
    );
}

/// RED-1: `-r` narrows the run but not the shadow plan.
#[test]
fn forced_noop_never_exceeds_converged_under_a_resource_selector() {
    let fx = converged();
    let out = apply_raw(&fx.yaml, &fx.state, &["--force", "-r", "marker"]);
    assert_one_forced_noop(&out, "--force -r marker");
}

/// RED-2: `-g` goes through a different executor branch (`resource_filtered_out`)
/// from `-r` (`should_skip_single`'s early return), so one test does not cover
/// both.
#[test]
fn forced_noop_never_exceeds_converged_under_a_group_selector() {
    let fx = converged();
    let out = apply_raw(&fx.yaml, &fx.state, &["--force", "-g", "alpha"]);
    assert_one_forced_noop(&out, "--force -g alpha");
}

/// RED-3: the second escape from `converged` — a resource that FAILS is not a
/// forced no-op either.
///
/// `libc::geteuid` is not available (no `libc` in `[dev-dependencies]`), so
/// root is detected by asking the filesystem the same question: root can still
/// open a 0444 file for writing.
#[test]
fn a_forced_resource_that_fails_is_not_a_forced_noop() {
    let fx = converged();

    let mut perms = std::fs::metadata(&fx.marker).expect("stat").permissions();
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o444);
    }
    std::fs::set_permissions(&fx.marker, perms).expect("chmod 0444");
    if std::fs::OpenOptions::new()
        .write(true)
        .open(&fx.marker)
        .is_ok()
    {
        // Running as root: the resource would converge, so the shape under test
        // does not exist here.
        return;
    }

    let out = apply_raw(&fx.yaml, &fx.state, &["--force"]);
    assert!(
        out.status.code().is_some(),
        "forjar died on a signal instead of reporting a failed resource"
    );
    assert!(
        !out.stdout.is_empty(),
        "--json produced no stdout — the process aborted before printing"
    );
    let s = summary(&out);
    assert_eq!(
        u(&s, "total_failed"),
        1,
        "the read-only file must fail: {s}"
    );
    assert_eq!(u(&s, "total_converged"), 1, "the other resource converges");
    assert_eq!(
        u(&s, "forced_noop_count"),
        1,
        "a resource that failed was not re-run as a no-op"
    );
    assert_eq!(u(&s, "actual_changes"), 0);
}

/// GREEN GUARD: with no selector and no failure, every resource is still a
/// forced no-op. Reconciling the count must not deflate the honest case — this
/// is FJ-129 shape 3, and it must keep reading 2/2/0.
#[test]
fn an_unfiltered_force_over_a_converged_stack_still_counts_every_resource() {
    let fx = converged();
    let out = apply_raw(&fx.yaml, &fx.state, &["--force"]);
    assert!(out.status.success(), "apply --force must succeed");
    let s = summary(&out);
    assert_eq!(u(&s, "total_converged"), 2);
    assert_eq!(u(&s, "forced_noop_count"), 2);
    assert_eq!(u(&s, "actual_changes"), 0);
}
