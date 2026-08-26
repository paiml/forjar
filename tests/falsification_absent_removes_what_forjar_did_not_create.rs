//! `state: absent` must remove a file forjar did not create.
//!
//! forjar#339. A `state: absent` resource with no lock entry returned
//! `PlanAction::NoOp`. The reasoning, stated outright in `planner/why.rs`, was
//! "resource not in lock, nothing to destroy" — a claim about the LOCK, not
//! about the machine.
//!
//! The whole reason to declare a file absent is normally that it exists and
//! forjar did NOT create it: a legacy file, a leftover, a stale drop-in. Those
//! are exactly the resources with no lock entry. So `absent` worked only for
//! files forjar had made itself — the case where you would simply delete the
//! declaration instead.
//!
//! It surfaced removing a dormant `NOPASSWD: ALL` sudoers grant for a user
//! that does not exist, from the fleet controller (paiml/infra#317). A plain
//! apply printed `Apply complete` and left the file in place.
//!
//! DRIVEN THROUGH THE REAL BINARY, not the planner, for two reasons: the
//! planner's types are private, and the whole defect is that every
//! user-visible surface reported success. Testing what the user sees is the
//! point.

use std::fs;
use std::path::Path;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

struct Sandbox {
    dir: std::path::PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-339-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("sandbox");
        Self { dir }
    }

    fn victim(&self) -> std::path::PathBuf {
        self.dir.join("victim.txt")
    }

    fn write_config(&self) {
        let cfg = format!(
            "version: \"1.0\"\nname: absent-repro\nmachines:\n  sandbox:\n    hostname: sandbox\n\
             \x20   addr: 127.0.0.1\nresources:\n  kill-the-victim:\n    type: file\n\
             \x20   machine: sandbox\n    path: {}\n    state: absent\n",
            self.victim().display()
        );
        fs::write(self.dir.join("forjar.yaml"), cfg).expect("config");
    }

    /// Returns (combined output, exit success). The status matters: an early
    /// version of this test asserted `!out.contains("failed")`, which matched
    /// the word inside `0 failed` on a perfectly good run. A substring is not
    /// a verdict.
    fn run(&self, args: &[&str]) -> (String, bool) {
        let out = Command::new(FORJAR)
            .args(args)
            .arg("-f")
            .arg(self.dir.join("forjar.yaml"))
            .current_dir(&self.dir)
            .output()
            .expect("run forjar");
        (
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
            out.status.success(),
        )
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// THE DEFECT. A plain apply must remove a file declared absent that forjar
/// never created. "Apply complete" over a surviving file is how a
/// security-motivated removal becomes a green report and a live file.
#[test]
fn a_plain_apply_removes_a_file_forjar_never_created() {
    let sb = Sandbox::new("plain-apply");
    sb.write_config();
    fs::write(sb.victim(), "i should not survive an apply").expect("victim");
    assert!(sb.victim().exists(), "precondition: the file is there");

    let (out, _) = sb.run(&["apply", "--yes"]);

    assert!(
        !sb.victim().exists(),
        "a plain apply left a file declared absent on disk.\n{out}"
    );
}

/// IDEMPOTENCE, WHICH IS GH-229 AND MUST NOT REGRESS. A successful destroy
/// writes the resource back into the lock as `converged`. The broken NoOp
/// existed to stop that re-emitting Destroy forever — observed as a permanent
/// "N to destroy" on infra's lambda-labs, pending for days.
///
/// The fix must reach a fixed point after ONE apply, so this asserts the
/// second apply converges nothing.
#[test]
fn a_second_apply_reaches_a_fixed_point() {
    let sb = Sandbox::new("fixed-point");
    sb.write_config();
    fs::write(sb.victim(), "gone after the first apply").expect("victim");

    sb.run(&["apply", "--yes"]);
    assert!(!sb.victim().exists(), "first apply must remove it");

    let (second, _) = sb.run(&["apply", "--yes"]);
    assert!(
        second.contains("0 converged") || second.contains("unchanged"),
        "the second apply must no-op, or GH-229 is back:\n{second}"
    );

    let (plan, _) = sb.run(&["plan"]);
    assert!(
        !plan.contains("to destroy, ") || plan.contains("0 to destroy"),
        "plan must not keep proposing a destroy that already ran:\n{plan}"
    );
}

/// An absent resource whose target is ALREADY gone must not fail. `rm -rf` on
/// a missing path succeeds, and the run must be clean on a fresh machine.
#[test]
fn absent_on_an_already_missing_path_is_clean() {
    let sb = Sandbox::new("already-gone");
    sb.write_config();
    assert!(!sb.victim().exists(), "precondition: nothing to remove");

    let (out, ok) = sb.run(&["apply", "--yes"]);

    assert!(
        ok,
        "declaring an already-absent path absent must not fail:\n{out}"
    );
    assert!(!Path::new(&sb.victim()).exists());
}
