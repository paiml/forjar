//! The PR lane must compile this crate's doctests.
//!
//! forjar#318. paiml/forjar#315 passed EVERY PR check and then failed the
//! clean-room release gate:
//!
//!   Couldn't compile the test.
//!   failures:
//!       src/core/state/process_lock.rs - core::state::process_lock::
//!         locked_by_other_live_pid (line 231)
//!   test result: FAILED. 87 passed; 1 failed; 1 ignored
//!   GATE B3 FAILED (54s): doctests failed
//!
//! The defect itself was mundane — a four-space-indented transcript inside a
//! `///` block is a Rust code block, so rustdoc tried to compile it, and one
//! ```text fence fixed it. The gap is the interesting part: no PR job selected
//! the doc target at all.
//!
//!   ci                 delegates to sovereign-ci, whose test job is hard-scoped
//!                      to `cargo test --lib` and, with use_nextest: true, is run
//!                      by cargo-nextest, which cannot execute doctests in any
//!                      mode
//!   lockfile           `cargo package --no-verify` compiles nothing
//!   examples-validate  `--test examples_validate`, one integration target
//!
//! So all 87 doctests were first compiled a release cycle later, at the gate,
//! where each occurrence costs a full clean-room run.
//!
//! THIS TEST PARSES THE WORKFLOW rather than grepping it, so the pin cannot be
//! satisfied by the string `--doc` appearing in a comment.

use serde_yaml_ng::{Mapping, Value};
use std::path::Path;

fn ci_workflow() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    let text = std::fs::read_to_string(&path).expect("ci.yml must be readable");
    serde_yaml_ng::from_str(&text).expect("ci.yml must parse as YAML")
}

fn jobs(w: &Value) -> &Mapping {
    w.get("jobs")
        .and_then(Value::as_mapping)
        .expect("ci.yml must define `jobs`")
}

/// Every `run:` scalar of every step of one job, as separate strings.
fn run_lines(job: &Value) -> Vec<String> {
    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return Vec::new();
    };
    steps
        .iter()
        .filter_map(|s| s.get("run"))
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

/// The id of the first job that actually invokes the doc test target.
fn doctest_job_id(w: &Value) -> Option<String> {
    jobs(w).iter().find_map(|(id, job)| {
        let runs = run_lines(job);
        let selects_doc = runs
            .iter()
            .any(|r| r.contains("cargo test") && r.contains("--doc"));
        selects_doc.then(|| id.as_str().unwrap_or_default().to_string())
    })
}

fn gate(w: &Value) -> &Value {
    jobs(w)
        .get("gate")
        .expect("ci.yml must define the `gate` job")
}

fn gate_needs(w: &Value) -> Vec<String> {
    gate(w)
        .get("needs")
        .and_then(Value::as_sequence)
        .expect("`gate` must declare `needs`")
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

#[test]
fn a_pr_job_compiles_the_doctests() {
    let w = ci_workflow();
    assert!(
        doctest_job_id(&w).is_some(),
        "no PR job runs `cargo test --doc`. sovereign-ci is --lib-scoped and \
         nextest cannot run doctests, so every doctest in this crate is first \
         compiled at the clean-room release gate, one release cycle after the PR \
         that broke it (#318)"
    );
}

#[test]
fn the_doctest_job_blocks_the_gate() {
    let w = ci_workflow();
    let id = doctest_job_id(&w).expect("no doctest job to check (#318)");
    assert!(
        gate_needs(&w).contains(&id),
        "`{id}` is not in the gate's `needs`, so the required check passes \
         whatever it does"
    );
    let checked = run_lines(gate(&w)).join("\n");
    assert!(
        checked.contains(&format!("needs.{id}.result")),
        "`gate` never inspects `needs.{id}.result`. `gate` is `if: always()`, so \
         a job listed in `needs` whose result is not read cannot fail it — the \
         half-fix that looks green and pins nothing"
    );
}

#[test]
fn every_gate_dependency_is_a_real_job() {
    let w = ci_workflow();
    let defined = jobs(&w);
    let dangling: Vec<String> = gate_needs(&w)
        .into_iter()
        .filter(|n| !defined.contains_key(Value::String(n.clone())))
        .collect();
    assert!(
        dangling.is_empty(),
        "the gate depends on jobs that do not exist: {dangling:?}. A typo'd or \
         renamed dependency turns the aggregate into a no-op for that job"
    );
}
