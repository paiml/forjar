//! forjar#412 (CRUX audit E09): TWO schedulers, drifted apart.
//!
//! WHAT WAS OBSERVABLY WRONG. `apply` has a sequential loop
//! (`executor/machine.rs::execute_sequential` -> `resource_ops.rs`) and a
//! parallel wave loop (`executor/machine_wave.rs`), and the same fixture goes
//! through different code depending on one flag. Measured on 1.24.0 with two
//! independent `type: file` resources whose hooks append one line each to a
//! counter file, the ONLY difference between the runs being `--parallel`:
//!
//! ```text
//!   apply             -> pre-alpha 1  post-alpha 1  pre-beta 1  post-beta 1
//!   apply --parallel  -> pre-alpha 1  post-alpha 2  pre-beta 1  post-beta 2
//! ```
//!
//! `post_apply` ran TWICE per resource on the wave path: once inside the
//! spawned thread (`run_post_hook_if_success`) and once again in the record
//! phase through `output_verify::post_apply_failure`. A `post_apply` that
//! restarts a service, appends to a ledger or bumps a counter did it twice.
//!
//! Four more behaviours existed on ONE path only, all of them user-visible:
//!
//! ```text
//!   --retry N              sequential retried, --parallel did not
//!   runs/<id>/meta.yaml    sequential wrote `resources:`, --parallel wrote {}
//!   --trace                sequential printed the script, --parallel did not
//!   [n/total] progress     sequential printed it, --parallel did not
//! ```
//!
//! and a `post_apply` failure was REPORTED differently by each path: the
//! sequential path recorded the verify-failure text naming the resource's run
//! log, the wave path recorded `transport error: ...` plus the claim
//! "no exit code, no output and no run log exist for this resource" — for a
//! resource whose apply had in fact succeeded and whose transcript existed.
//!
//! WHY THESE ASSERTIONS. Each one is a COUNT or a BYTE COMPARISON of the two
//! paths over the SAME fixture in the SAME directories, so neither can pass
//! vacuously and neither depends on timing:
//!
//!   * `hooks_fire_exactly_once_per_resource_on_both_paths` — the counter file
//!     is written by the hooks themselves. 2 is 2 whatever the scheduler.
//!   * `the_lock_is_identical_between_the_two_paths` — bytes of
//!     `state.lock.yaml`, timestamps/durations/run-ids scrubbed. The failing
//!     fixture is used deliberately: `details.error` is where the two failure
//!     texts diverged.
//!   * `the_event_stream_and_the_run_log_agree_between_the_two_paths` — events
//!     are compared as a SORTED SET, because a wave genuinely interleaves two
//!     concurrent resources and demanding one order would be asserting that
//!     parallelism does not happen. The run log's `meta.yaml` is compared
//!     per-resource, which is where the wave path recorded nothing at all.
//!   * `a_failure_is_attributed_to_the_resource_that_failed` — the failing
//!     resource is the SECOND of three in one wave, so an index-0
//!     misattribution (the wave path's thread-panic arm hard-codes index 0)
//!     lands on `alpha` and is visible in the lock.
//!   * `retry_reruns_the_failed_resource_the_same_number_of_times` — attempts
//!     are counted by the resource's own command.
//!   * `trace_prints_the_generated_script_on_both_paths` — a literal marker
//!     in stderr.

#![allow(unused_imports)]
#[path = "common/scheduler_parity.rs"]
mod parity;
use parity::*;
use std::fs;

/// One scheduler means one hook per hook, whichever width the wave has.
#[test]
fn hooks_fire_exactly_once_per_resource_on_both_paths() {
    let fx = hooked_pair();

    let seq_run = fx.run(&[]);
    let sequential = fx.counts("hooks.log");
    let par_run = fx.run(&["--parallel"]);
    let parallel = fx.counts("hooks.log");

    assert_eq!(
        (seq_run.code, par_run.code),
        (0, 0),
        "fixture must converge on both paths.\nseq:\n{}\npar:\n{}",
        seq_run.stderr,
        par_run.stderr
    );
    assert_eq!(
        sequential.len(),
        4,
        "fixture is inert: expected 4 distinct hook lines, got {sequential:?}"
    );
    for (line, n) in &sequential {
        assert_eq!(*n, 1, "sequential ran '{line}' {n} times, expected once");
    }
    assert_eq!(
        parallel, sequential,
        "the two schedulers ran the hooks a different number of times.\n\
         sequential: {sequential:?}\n--parallel: {parallel:?}"
    );
}

/// The lock is the artefact everything downstream reads. It must not depend on
/// which scheduler produced it — including `details.error` on a failure.
#[test]
fn the_lock_is_identical_between_the_two_paths() {
    let fx = failing_middle();

    fx.run(&[]);
    let sequential = scrub_yaml(&fx.lock_text());
    fx.run(&["--parallel"]);
    let parallel = scrub_yaml(&fx.lock_text());

    assert!(
        sequential.contains("boom") && sequential.contains("status: failed"),
        "fixture is inert: the sequential lock records no failure:\n{sequential}"
    );
    assert_eq!(
        sequential, parallel,
        "state.lock.yaml differs between the schedulers.\n\
         --- sequential ---\n{sequential}\n--- --parallel ---\n{parallel}"
    );
}

/// The event stream and the run log are the audit surface. Both paths must
/// emit the same events and record the same per-resource run metadata.
#[test]
fn the_event_stream_and_the_run_log_agree_between_the_two_paths() {
    let fx = hooked_pair();

    fx.run(&[]);
    let seq_events = scrub_events(&fx.events_text());
    let seq_meta = meta_resources(&fx.meta_text());
    fx.run(&["--parallel"]);
    let par_events = scrub_events(&fx.events_text());
    let par_meta = meta_resources(&fx.meta_text());

    assert!(
        seq_events.iter().any(|e| e.contains("resource_converged")),
        "fixture is inert: no converge events at all:\n{seq_events:#?}"
    );
    assert_eq!(
        seq_events, par_events,
        "the event streams differ (sorted, timestamps and run ids scrubbed).\n\
         --- sequential ---\n{seq_events:#?}\n--- --parallel ---\n{par_events:#?}"
    );

    assert_eq!(
        seq_meta.len(),
        2,
        "fixture is inert: sequential meta.yaml names {} resources: {seq_meta:?}",
        seq_meta.len()
    );
    assert_eq!(
        seq_meta, par_meta,
        "runs/<id>/meta.yaml differs between the schedulers.\n\
         --- sequential ---\n{seq_meta:#?}\n--- --parallel ---\n{par_meta:#?}"
    );
}

/// A failure belongs to the resource that failed, and reads the same either way.
#[test]
fn a_failure_is_attributed_to_the_resource_that_failed() {
    let fx = failing_middle();

    let seq = fx.run(&[]);
    let seq_lock = fx.lock_text();
    let par = fx.run(&["--parallel"]);
    let par_lock = fx.lock_text();

    assert_eq!(
        (seq.code, par.code),
        (1, 1),
        "a failing post_apply must fail the apply on both paths"
    );
    for (label, lock) in [("sequential", &seq_lock), ("--parallel", &par_lock)] {
        let boom = lock.split("  boom:").nth(1).unwrap_or_default();
        let alpha = lock.split("  alpha:").nth(1).unwrap_or_default();
        assert!(
            boom.contains("status: failed"),
            "{label}: boom is not recorded failed:\n{lock}"
        );
        assert!(
            alpha.contains("status: converged"),
            "{label}: the failure was attributed to alpha, the resource at \
             index 0, which converged:\n{lock}"
        );
    }
    for (label, err) in [("sequential", &seq.stderr), ("--parallel", &par.stderr)] {
        assert!(
            err.contains("local/boom failed"),
            "{label}: the failure line does not name boom:\n{err}"
        );
        assert!(
            err.contains("post_apply hook failed (exit 3)"),
            "{label}: the failure text loses the hook's own verdict:\n{err}"
        );
        assert!(
            !err.contains("no run log exist"),
            "{label}: the failure text claims no run log exists for a resource \
             whose apply succeeded and whose transcript was written:\n{err}"
        );
    }
}
