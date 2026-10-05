//! Stress Tests must be able to finish inside its own timeout.
//!
//! forjar#673. Each leg ran `cargo test --lib -- --test-threads=1` ten times
//! inside `timeout-minutes: 30`. One pass measured 6–9 minutes on intel (run
//! 37300806114: 372.95s, 460.82s, 515.53s, then the timeout), so a leg could
//! finish three passes at most and the lane had no green run in five weeks —
//! while every pass that ran was green. A lane that cannot pass reports nothing
//! about flakiness. And with `fail-fast` defaulting to true, the timed-out leg
//! cancelled its siblings before they got a runner.

use serde_yaml_ng::Value;

/// The slowest single-threaded pass measured (515.53s), rounded up.
const PASS_MINUTES: u64 = 9;
/// Headroom for the build that the first pass performs, cold cache included.
const BUILD_MINUTES: u64 = 15;

fn stress() -> (Value, String) {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/stress.yml");
    let text = std::fs::read_to_string(p).expect("stress.yml is readable");
    (
        serde_yaml_ng::from_str(&text).expect("stress.yml parses"),
        text,
    )
}

fn job(wf: &Value) -> &Value {
    &wf["jobs"]["stress"]
}

fn loop_step(wf: &Value) -> &Value {
    job(wf)["steps"]
        .as_sequence()
        .expect("stress has steps")
        .iter()
        .find(|s| {
            s["run"]
                .as_str()
                .is_some_and(|r| r.contains("--test-threads=1"))
        })
        .expect("a step runs the single-threaded suite")
}

/// The pass count of `for run in $(seq 1 N)`.
fn passes(run: &str) -> u64 {
    let at = run.find("seq 1 ").expect("the loop is `seq 1 N`") + "seq 1 ".len();
    run[at..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .expect("N is a number")
}

#[test]
fn every_pass_of_a_leg_fits_the_step_timeout() {
    let (wf, _) = stress();
    let step = loop_step(&wf);
    let n = passes(step["run"].as_str().unwrap());
    let timeout = step["timeout-minutes"]
        .as_u64()
        .expect("the loop step carries timeout-minutes");
    let need = n * PASS_MINUTES + BUILD_MINUTES;
    assert!(
        need <= timeout,
        "{n} passes x {PASS_MINUTES} min + {BUILD_MINUTES} min build = {need} min, \
         but the step times out at {timeout} min: the lane can never go green (#673)"
    );
}

#[test]
fn a_leg_that_times_out_does_not_cancel_its_siblings() {
    let (wf, _) = stress();
    assert_eq!(
        job(&wf)["strategy"]["fail-fast"].as_bool(),
        Some(false),
        "stress.yml must set fail-fast: false so each leg reports for itself (#673)"
    );
}

#[test]
fn the_loop_still_runs_more_than_one_pass_per_leg() {
    // A stress lane of one pass is the ordinary test lane under another name.
    let (wf, _) = stress();
    let n = passes(loop_step(&wf)["run"].as_str().unwrap());
    let legs = job(&wf)["strategy"]["matrix"]["iteration"]
        .as_sequence()
        .map_or(1, Vec::len) as u64;
    assert!(n >= 2, "each leg must repeat the suite, found {n} pass(es)");
    assert!(
        legs * n >= 9,
        "{legs} legs x {n} passes is under 9 passes a week"
    );
}

#[test]
fn the_echoed_denominator_matches_the_loop() {
    let (wf, _) = stress();
    let run = loop_step(&wf)["run"].as_str().unwrap();
    let n = passes(run);
    assert!(
        run.contains(&format!("Run $run/{n}")),
        "the progress line must say /{n}, or the log lies about how many passes remain"
    );
}
