//! `plan` must say WHERE it read state from, and when it found none.
//!
//! THE FLAW THIS CLOSES (paiml/forjar#273).
//!
//! `--state-dir` defaults to the literal `state` — a CWD-RELATIVE path. So the
//! same command, naming the same config by the same `-f` path, answers
//! differently depending on which directory you stand in:
//!
//!   $ forjar plan -f machines/intel/forjar.yaml
//!   Plan: 0 to add, 1 to change, 0 to destroy, 100 unchanged.
//!
//!   $ cd /home/noah/src/infra/.claude/worktrees/wt   # git worktree, same commit
//!   $ forjar plan -f machines/intel/forjar.yaml
//!   Plan: 101 to add, 0 to change, 0 to destroy, 0 unchanged.
//!
//! Same host, provably converged and serving 16 CI runners. The worktree simply
//! had no `./state/intel`, and `state::load_lock` returns None for that with no
//! comment. "101 to add" then reads as catastrophic host drift, which invites
//! exactly the wrong response — and acting on it would re-create every resource
//! on a live machine.
//!
//! Worse than the misreading: applying from the wrong directory WRITES state
//! there too, so the canonical state and the stray copy diverge silently.
//!
//! The default is not changed here. Making it config-relative would silently
//! relocate paiml/infra's state (config at `machines/<m>/forjar.yaml`, state at
//! the repo root) and break every existing caller. The defect is that the
//! choice is invisible, so the fix is to make it visible.
//!
//! WHAT THIS TEST MUST NOT BECOME. Asserting that the summary says "to add"
//! would pass today. Every case here asserts that the RESOLVED state directory
//! is named in the output, and that finding no prior state is stated rather
//! than implied.

use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

fn config(dir: &std::path::Path) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        r#"version: "1.0"
name: state-visibility
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    transport: local
resources:
  a-dir:
    type: file
    machine: local
    state: directory
    path: /tmp/forjar-state-visibility-probe
"#,
    )
    .unwrap();
    cfg
}

#[test]
fn plan_names_the_state_directory_it_resolved() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let out = forjar()
        .current_dir(dir.path())
        .args(["plan", "-f", cfg.to_str().unwrap()])
        .output()
        .unwrap();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("state:"),
        "plan never said which state directory it read. A reader seeing \
         'N to add' has no way to tell a drifted host from a wrong CWD.\n{combined}"
    );
}

#[test]
fn plan_says_when_it_found_no_prior_state() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let out = forjar()
        .current_dir(dir.path())
        .args(["plan", "-f", cfg.to_str().unwrap()])
        .output()
        .unwrap();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("no prior state"),
        "plan treated 'never applied' and 'converged' as indistinguishable. \
         Absent state must be stated, not implied by a large add count.\n{combined}"
    );
}

/// The counter-case: when state IS present for the machine, plan must not cry
/// wolf. Without this, always printing the warning would pass the test above.
#[test]
fn plan_does_not_claim_missing_state_when_state_exists() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    // Apply once so a real lock exists in this directory's state.
    let _ = forjar()
        .current_dir(dir.path())
        .args([
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--yes",
            "--no-tripwire",
        ])
        .output()
        .unwrap();

    let out = forjar()
        .current_dir(dir.path())
        .args(["plan", "-f", cfg.to_str().unwrap()])
        .output()
        .unwrap();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !combined.contains("no prior state"),
        "plan reported missing state for a machine that has a lock — a warning \
         that always fires is one people learn to ignore.\n{combined}"
    );
}
