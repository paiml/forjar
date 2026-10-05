//! forjar#435: `apply --canary-machine` skipped `apply_pre_checks`.
//!
//! `apply_early_exits` returned `cmd_apply_canary_machine(...)` before
//! `apply_pre_checks` ran, and `cmd_apply_canary_machine` calls `cmd_apply` per
//! machine directly. So
//!
//! ```text
//!   forjar apply --canary-machine sandbox --pre-script gate.sh --yes
//! ```
//!
//! converged the canary and then the fleet with `gate.sh` never run, and no
//! message saying so. The flags parsed, the run succeeded, the knobs did
//! nothing. The hooks now run once per rollout, before the canary leg.
//!
//! Everything below drives the shipped binary, for the reason the #374 suite
//! gives: the hole is in which dispatcher branch reaches the hooks.
#[path = "common/canary_authz.rs"]
mod harness;
use harness::*;
use std::path::PathBuf;

/// A pre-script that appends one line to `marker` per run and exits `rc`.
fn pre_script(sb: &Sandbox, rc: i32) -> (PathBuf, PathBuf) {
    let marker = sb.dir.join("pre-script.ran");
    let script = sb.dir.join(format!("gate-{rc}.sh"));
    std::fs::write(
        &script,
        format!("echo ran >> '{}'\nexit {rc}\n", marker.display()),
    )
    .expect("write pre-script");
    (script, marker)
}

fn runs(marker: &PathBuf) -> usize {
    std::fs::read_to_string(marker)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

/// Without this, "the canary refuses on a failing pre-script" could mean the
/// fixture never converges at all.
#[test]
fn control_canary_without_hooks_converges_the_fleet() {
    let sb = Sandbox::fleet("435-control");
    let out = sb.canary(Some("alice"), &["--yes"]);
    assert!(
        out.status.success(),
        "control canary failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(sb.canary_file().exists() && sb.prod_file().exists());
}

#[test]
fn canary_runs_the_pre_script_once_per_rollout() {
    let sb = Sandbox::fleet("435-runs");
    let (script, marker) = pre_script(&sb, 0);
    let out = sb.canary(
        Some("alice"),
        &["--yes", "--pre-script", script.to_str().unwrap()],
    );
    assert!(
        out.status.success(),
        "canary with a passing pre-script failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        runs(&marker),
        1,
        "--pre-script must run exactly once for a canary rollout (0 = ignored)"
    );
    assert!(sb.canary_file().exists() && sb.prod_file().exists());
}

#[test]
fn canary_refuses_when_the_pre_script_fails() {
    let sb = Sandbox::fleet("435-pre-script");
    let (script, marker) = pre_script(&sb, 3);
    let out = sb.canary(
        Some("alice"),
        &["--yes", "--pre-script", script.to_str().unwrap()],
    );
    assert_eq!(runs(&marker), 1, "the failing pre-script never ran");
    assert!(
        !out.status.success(),
        "a canary whose pre-script exited 3 reported success"
    );
    assert!(
        sb.nothing_was_written(),
        "a failing --pre-script still converged the canary or the fleet"
    );
}

#[test]
fn canary_refuses_when_the_pre_flight_fails() {
    let sb = Sandbox::fleet("435-pre-flight");
    let out = sb.canary(Some("alice"), &["--yes", "--pre-flight", "exit 1"]);
    assert!(
        !out.status.success(),
        "a canary whose --pre-flight failed reported success"
    );
    assert!(
        sb.nothing_was_written(),
        "a failing --pre-flight still converged the canary or the fleet"
    );
}
