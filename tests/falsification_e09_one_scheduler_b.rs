//! forjar#412 (CRUX audit E09), second file: retry, --trace, --progress and
//! the task input cache must behave identically on both schedulers. Split
//! from `falsification_e09_one_scheduler.rs` at the 500-line budget; same
//! harness (`tests/common/scheduler_parity.rs`), same binary.

#![allow(unused_imports)]
#[path = "common/scheduler_parity.rs"]
mod parity;
use parity::*;
use std::fs;

/// `--retry` is a promise about attempts, not about which scheduler ran.
#[test]
fn retry_reruns_the_failed_resource_the_same_number_of_times() {
    let fx = retryable_pair();

    fx.run(&["--retry", "1"]);
    let sequential = fx.counts("attempts.log");
    fx.run(&["--retry", "1", "--parallel"]);
    let parallel = fx.counts("attempts.log");

    assert_eq!(
        sequential.get("attempt").copied(),
        Some(2),
        "fixture is inert: sequential --retry 1 did not produce 2 attempts: {sequential:?}"
    );
    assert_eq!(
        parallel, sequential,
        "--retry 1 ran the failing resource {parallel:?} times under --parallel \
         and {sequential:?} times without it"
    );
}

/// `--trace` prints the script that is about to run — on either scheduler.
#[test]
fn trace_prints_the_generated_script_on_both_paths() {
    let fx = hooked_pair();

    let seq = fx.run(&["--trace"]);
    let par = fx.run(&["--trace", "--parallel"]);

    for (label, err) in [("sequential", &seq.stderr), ("--parallel", &par.stderr)] {
        for id in ["alpha", "beta"] {
            assert!(
                err.contains(&format!("[TRACE] {id} script:")),
                "{label}: --trace printed no script for {id}:\n{err}"
            );
        }
    }
}

/// `--progress` is a promise about the console, not about the scheduler.
#[test]
fn progress_reports_every_resource_on_both_paths() {
    let fx = hooked_pair();

    let seq = fx.run(&["--progress"]);
    let par = fx.run(&["--progress", "--parallel"]);

    for (label, err) in [("sequential", &seq.stderr), ("--parallel", &par.stderr)] {
        for line in ["[1/2] alpha converged", "[2/2] beta converged"] {
            assert!(
                err.contains(line),
                "{label}: --progress never reported '{line}':\n{err}"
            );
        }
    }
}

/// FJ-2701: a cached task whose inputs are unchanged does not re-run — on
/// either wave width.
#[test]
fn the_input_cache_holds_on_both_paths() {
    for (label, extra) in [("sequential", vec![]), ("--parallel", vec!["--parallel"])] {
        let fx = Fixture::new(&cached_task_config("one"));
        fs::create_dir_all(fx.path("work")).expect("create work dir");
        // `task_inputs` are relative to the state dir's PARENT, which is the
        // fixture root — not to `working_dir`.
        fs::create_dir_all(fx.path("in")).expect("create input dir");
        fs::write(fx.path("in/seed.txt"), "seed\n").expect("write input");

        let first = fx.run_keeping_state(&extra);
        assert_eq!(
            first.code, 0,
            "{label}: the first apply must converge:\n{}",
            first.stderr
        );
        assert_eq!(
            fx.counts("attempts.log").get("build").copied(),
            Some(1),
            "{label}: fixture is inert — the task did not build once"
        );

        // Same declared input, different command text: the planner must want to
        // re-run it, and only the input cache may stop it.
        fx.rewrite_config(&cached_task_config("two"));
        let second = fx.run_keeping_state(&extra);
        assert_eq!(
            second.code, 0,
            "{label}: the second apply must converge:\n{}",
            second.stderr
        );
        assert_eq!(
            fx.counts("attempts.log").get("build").copied(),
            Some(1),
            "{label}: the cached task re-ran although its declared inputs were \
             unchanged:\n{}",
            second.stderr
        );
    }
}
