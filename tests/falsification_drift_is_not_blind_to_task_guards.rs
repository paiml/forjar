//! forjar#380 / paiml/infra#380: `forjar drift` said nothing about the guards.
//!
//! paiml/infra declares its fleet guards as forjar resources — `completion_check`
//! is the assertion, `command` reports the violation — and `forjar drift` never
//! executed one unless the lock happened to carry an observed digest for it. It
//! also never said how many resources it had inspected, so
//! `No drift detected.` read identically whether it had checked sixty-two
//! resources or none. forjar's own dogfood ledger has carried this since 1.12.3
//! as `drift-and-plan-blind-to-failing-task-completion-check`
//! (`docs/dogfood-1.12.3-cli-defects.json`).
//!
//! DRIVEN THROUGH THE REAL BINARY. Both halves of the defect are things the
//! operator READS, so what the operator sees is the thing under test.

use std::fs;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

struct Sandbox {
    dir: std::path::PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-380-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("sandbox");
        Self { dir }
    }

    /// The file the guard asserts. Deleting it is the violation.
    fn marker(&self) -> std::path::PathBuf {
        self.dir.join("runner.registered")
    }

    /// One guard resource: the `completion_check` is the claim, and `command`
    /// is what runs when the claim is false.
    ///
    /// `command` restores the marker rather than merely reporting, because a
    /// report-only guard (`exit 1`, which is what paiml/infra writes) cannot be
    /// converged into a lock at all today — see
    /// `a_guard_the_lock_never_heard_of_is_counted_as_uninspected`.
    fn write_config(&self) {
        let cfg = format!(
            "version: \"1.0\"\nname: drift-380\nmachines:\n  sandbox:\n\
             \x20   hostname: sandbox\n    addr: 127.0.0.1\nresources:\n  runner-registered:\n\
             \x20   type: task\n    machine: sandbox\n\
             \x20   command: \"touch '{marker}'\"\n\
             \x20   completion_check: \"[ -f '{marker}' ]\"\n",
            marker = self.marker().display()
        );
        fs::write(self.dir.join("forjar.yaml"), cfg).expect("config");
    }

    /// The same guard, but report-only — the assertion shape a fleet uses when
    /// forjar has no token and the fix is a human's job.
    fn write_report_only_config(&self) {
        let cfg = format!(
            "version: \"1.0\"\nname: drift-380\nmachines:\n  sandbox:\n\
             \x20   hostname: sandbox\n    addr: 127.0.0.1\nresources:\n  runner-registered:\n\
             \x20   type: task\n    machine: sandbox\n\
             \x20   command: |\n      echo 'the runner is not registered' >&2\n      exit 1\n\
             \x20   completion_check: \"[ -f '{}' ]\"\n",
            self.marker().display()
        );
        fs::write(self.dir.join("forjar.yaml"), cfg).expect("config");
    }

    fn run(&self, args: &[&str]) -> (String, bool) {
        let out = Command::new(FORJAR)
            .args(args)
            .arg("-f")
            .arg(self.dir.join("forjar.yaml"))
            .arg("--state-dir")
            .arg(self.dir.join("state"))
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

    /// Apply with the guard already satisfied, so the resource converges
    /// without its `command` ever running — the state every fleet guard is in.
    fn converge(&self) {
        fs::write(self.marker(), "registered").expect("marker");
        let (out, ok) = self.run(&["apply", "--yes"]);
        assert!(ok, "the guard should converge while satisfied:\n{out}");
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// THE FALSIFICATION. A converged guard whose assertion no longer holds is
/// drift, and `--tripwire` must exit non-zero over it.
#[test]
fn a_violated_task_guard_is_drift() {
    let sb = Sandbox::new("violated");
    sb.write_config();
    sb.converge();
    fs::remove_file(sb.marker()).expect("violate the guard");

    let (out, ok) = sb.run(&["drift", "-m", "sandbox", "--tripwire"]);

    assert!(
        !ok,
        "drift exited 0 over a guard whose completion_check fails:\n{out}"
    );
    assert!(
        !out.contains("No drift detected."),
        "drift reported clean over a violated guard:\n{out}"
    );
    assert!(
        out.contains("runner-registered") && out.contains("completion_check"),
        "the finding must name the resource AND the assertion that failed:\n{out}"
    );
}

/// THE CONTROL. The same run, with the guard satisfied, must stay green — a
/// detector that reports drift unconditionally is worth no more than one that
/// never reports it.
#[test]
fn a_satisfied_task_guard_is_not_drift() {
    let sb = Sandbox::new("satisfied");
    sb.write_config();
    sb.converge();

    let (out, ok) = sb.run(&["drift", "-m", "sandbox", "--tripwire"]);

    assert!(ok, "a satisfied guard must not report drift:\n{out}");
    assert!(out.contains("No drift detected."), "{out}");
}

/// THE DENOMINATOR. Every run states what it inspected and what it skipped —
/// including the clean runs, which are the ones the number is for.
#[test]
fn drift_reports_what_it_inspected() {
    let sb = Sandbox::new("denominator");
    sb.write_config();
    sb.converge();

    let (out, ok) = sb.run(&["drift", "-m", "sandbox"]);

    assert!(ok, "{out}");
    assert!(
        out.contains("inspected 1 of 1 resource(s) in scope: task 1"),
        "a clean run must still say what it looked at:\n{out}"
    );
}

/// THE DENOMINATOR, MACHINE-READABLE. A `--json` consumer was as blind as a
/// human reading the text output.
#[test]
fn the_json_report_carries_the_denominator() {
    let sb = Sandbox::new("json");
    sb.write_config();
    sb.converge();

    let (out, ok) = sb.run(&["drift", "-m", "sandbox", "--json"]);

    assert!(ok, "{out}");
    let report: serde_json::Value = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("drift --json emitted invalid JSON ({e}):\n{out}"));
    assert_eq!(report["drift_count"], 0, "{out}");
    assert_eq!(report["resources_inspected"], 1, "{out}");
    assert_eq!(report["census"][0]["inspected_by_type"]["task"], 1, "{out}");
}

/// THE OPT-OUT IS NOT A SILENCER. `--no-task-checks` may decline the work; it
/// may not pretend the work was done. Without the census line this flag would
/// reintroduce the exact defect the rest of this file exists to prevent.
#[test]
fn no_task_checks_reports_what_it_declined() {
    let sb = Sandbox::new("optout");
    sb.write_config();
    sb.converge();
    fs::remove_file(sb.marker()).expect("violate the guard");

    let (out, ok) = sb.run(&["drift", "-m", "sandbox", "--tripwire", "--no-task-checks"]);

    assert!(ok, "--no-task-checks must not execute the check:\n{out}");
    assert!(
        out.contains("--no-task-checks"),
        "the skipped population must name the flag that skipped it:\n{out}"
    );
    assert!(
        out.contains("inspected 0 of 1"),
        "the run must admit it inspected nothing:\n{out}"
    );
}

/// THE GX10 SHAPE, AND THE LIMIT OF THIS FIX. A report-only guard — `command`
/// exits 1 by design — cannot reach the lock as converged: with an empty lock
/// the planner plans `create` and the command fails, and `--refresh` finds the
/// check already satisfied and plans nothing, seeding an entry that lives only
/// in the planner's copy of the lock (`executor::refresh_seed`). Drift walks
/// the LOCK, so it never sees the resource at all.
///
/// That is a plan-side gap, not a drift verdict: "never applied here" is not
/// "changed since it was applied". What drift owes the operator is the COUNT,
/// so a clean bill of health cannot be mistaken for coverage.
#[test]
fn a_guard_the_lock_never_heard_of_is_counted_as_uninspected() {
    let sb = Sandbox::new("unlocked");
    sb.write_report_only_config();
    fs::write(sb.marker(), "registered").expect("marker");
    let (apply_out, _) = sb.run(&["apply", "--refresh", "--yes"]);
    assert!(
        apply_out.contains("unchanged"),
        "--refresh should have found the guard satisfied:\n{apply_out}"
    );

    let (out, ok) = sb.run(&["drift", "-m", "sandbox"]);

    assert!(ok, "{out}");
    assert!(
        out.contains("skipped 1: declared here, absent from the lock 1"),
        "a declared guard the lock has never heard of must be COUNTED, not \
         folded into a clean verdict:\n{out}"
    );
    assert!(
        out.contains("0 resource(s) inspected, 1 not inspected."),
        "the closing verdict must carry the denominator:\n{out}"
    );
}
