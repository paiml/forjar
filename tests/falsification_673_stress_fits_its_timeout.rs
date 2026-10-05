//! Refs #673: the Stress Tests lane must be ABLE to go green.
//!
//! THE FLAW THIS CLOSES. Each leg of `.github/workflows/stress.yml` looped
//! `cargo test --lib -- --test-threads=1` ten times inside a 30-minute step
//! timeout. One pass measured 373-516 s on intel-clean-room-13 (run
//! 37300806114), so a leg finished three passes and timed out, every week, with
//! every pass green. With the default `fail-fast` that timeout also cancelled
//! the other two legs before a runner was assigned. No run succeeded in five
//! weeks, and no red run said anything about the suite.
//!
//! WHAT THIS TEST HOLDS. It reads the stress step's pass count and timeout
//! from the workflow and asserts that the passes fit:
//! `passes x PASS_MIN + BUILD_MIN <= timeout-minutes`, and that the matrix sets
//! `fail-fast: false`. Raising the loop back to 10 or dropping the timeout to
//! 30 goes red here, not one Monday at 4am.

use serde_yaml_ng::Value;

/// Worst single-threaded `cargo test --lib` pass measured, rounded up (516 s).
const PASS_MIN: u64 = 9;
/// Allowance for the first pass's build on a warm cache.
const BUILD_MIN: u64 = 10;

fn stress_job() -> Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/stress.yml");
    let text = std::fs::read_to_string(&p).expect("read stress.yml");
    let doc: Value = serde_yaml_ng::from_str(&text).expect("parse stress.yml");
    doc["jobs"]["stress"].clone()
}

fn stress_step(job: &Value) -> Value {
    let steps = job["steps"].as_sequence().expect("stress.steps");
    let found: Vec<&Value> = steps
        .iter()
        .filter(|s| {
            s["run"]
                .as_str()
                .is_some_and(|r| r.contains("cargo test --lib -- --test-threads=1"))
        })
        .collect();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one single-threaded stress step, found {}",
        found.len()
    );
    found[0].clone()
}

/// The pass count: `STRESS_PASSES` from the step env when the loop reads it,
/// else a literal `seq 1 N`. Anything else cannot be measured, which fails.
fn passes(step: &Value) -> u64 {
    let run = step["run"].as_str().unwrap_or_default();
    if run.contains("seq 1 \"$STRESS_PASSES\"") || run.contains("seq 1 $STRESS_PASSES") {
        return step["env"]["STRESS_PASSES"]
            .as_u64()
            .expect("the loop reads STRESS_PASSES but the step env does not set a number");
    }
    let lit = run
        .split("seq 1 ")
        .nth(1)
        .and_then(|rest| rest.split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|n| n.parse().ok());
    lit.unwrap_or_else(|| panic!("cannot measure the pass count of the stress loop:\n{run}"))
}

#[test]
fn every_stress_leg_fits_its_passes_inside_its_timeout() {
    let step = stress_step(&stress_job());
    let n = passes(&step);
    let timeout = step["timeout-minutes"]
        .as_u64()
        .expect("the stress step has no numeric timeout-minutes");
    assert!(n >= 1, "a stress loop of {n} passes measures nothing");
    let need = n * PASS_MIN + BUILD_MIN;
    assert!(
        need <= timeout,
        "{n} pass(es) x {PASS_MIN} min + {BUILD_MIN} min build = {need} min > timeout-minutes {timeout}: \
         the leg cannot finish, so the lane cannot go green (#673)"
    );
}

#[test]
fn one_red_leg_does_not_cancel_its_siblings() {
    let job = stress_job();
    assert_eq!(
        job["strategy"]["fail-fast"].as_bool(),
        Some(false),
        "stress matrix must set fail-fast: false, so a red leg means a test failed in that leg (#673)"
    );
}
