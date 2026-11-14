//! PMAT-1008: THE FRAGMENT FALSIFIER RUNS IN A REQUIRED JOB, AND NOTHING
//! SOFTENS IT.
//!
//! `tests/changelog-fragments.sh` is the only check on the per-PR fragment
//! layout, so it is worth something only while CI runs it and a red there can
//! block the merge. That used to be asserted with `grep` over `ci.yml`, which
//! a commented-out step (`# run: bash tests/changelog-fragments.sh`) still
//! satisfied. Here the workflow is PARSED as YAML: the step must be a real
//! step of `dogfood-guards`, with no `if:` and no `continue-on-error` on the
//! step or the job, and `gate` must both need `dogfood-guards` and check its
//! result. Each mutation below is planted and must be caught.

use serde_yaml_ng::Value;
use std::path::Path;

const STEP_RUN: &str = "bash tests/changelog-fragments.sh";

fn ci_yml() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p:?}: {e}"))
}

/// Ok(()) when the wiring holds, else the first reason it does not.
fn wiring(text: &str) -> Result<(), String> {
    let doc: Value =
        serde_yaml_ng::from_str(text).map_err(|e| format!("ci.yml is not YAML: {e}"))?;
    let jobs = doc.get("jobs").ok_or("ci.yml has no jobs")?;
    let job = jobs.get("dogfood-guards").ok_or("no dogfood-guards job")?;
    if job.get("continue-on-error").is_some() {
        return Err("dogfood-guards has continue-on-error".into());
    }
    let steps = job
        .get("steps")
        .and_then(Value::as_sequence)
        .ok_or("dogfood-guards has no steps")?;
    let hits: Vec<&Value> = steps
        .iter()
        .filter(|s| s.get("run").and_then(Value::as_str).map(str::trim) == Some(STEP_RUN))
        .collect();
    let step = match hits.as_slice() {
        [one] => *one,
        [] => return Err(format!("dogfood-guards has no step running `{STEP_RUN}`")),
        _ => return Err(format!("dogfood-guards runs `{STEP_RUN}` more than once")),
    };
    if step.get("if").is_some() {
        return Err("the fragments step carries an if: condition".into());
    }
    if step.get("continue-on-error").is_some() {
        return Err("the fragments step has continue-on-error".into());
    }
    let gate = jobs.get("gate").ok_or("no gate job")?;
    let needs: Vec<&str> = match gate.get("needs") {
        Some(Value::Sequence(s)) => s.iter().filter_map(Value::as_str).collect(),
        Some(Value::String(s)) => vec![s.as_str()],
        _ => vec![],
    };
    if !needs.contains(&"dogfood-guards") {
        return Err("gate does not need dogfood-guards".into());
    }
    let checks = gate
        .get("steps")
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(|s| s.get("run").and_then(Value::as_str))
        .any(|r| r.contains("needs.dogfood-guards.result"));
    if !checks {
        return Err("gate never checks needs.dogfood-guards.result".into());
    }
    Ok(())
}

#[test]
fn this_tree_runs_the_fragment_falsifier_in_a_required_job() {
    wiring(&ci_yml()).unwrap_or_else(|e| panic!("{e}"));
}

/// Replace exactly one occurrence of `from`, or fail the test (a planted
/// mutation that does not apply would prove nothing).
fn plant(text: &str, from: &str, to: &str) -> String {
    assert_eq!(
        text.matches(from).count(),
        1,
        "mutation anchor {from:?} not unique in ci.yml"
    );
    text.replacen(from, to, 1)
}

fn assert_caught(mutated: &str, why: &str) {
    assert!(
        wiring(mutated).is_err(),
        "planted mutation not caught: {why}"
    );
}

#[test]
fn a_commented_out_step_is_caught() {
    let t = ci_yml();
    let m = plant(
        &t,
        &format!("        run: {STEP_RUN}\n"),
        &format!("        # run: {STEP_RUN}\n        run: 'true'\n"),
    );
    assert_caught(&m, "step commented out");
}

#[test]
fn a_conditional_or_soft_failing_step_is_caught() {
    let t = ci_yml();
    let anchor = format!("        run: {STEP_RUN}\n");
    assert_caught(
        &plant(&t, &anchor, &format!("{anchor}        if: false\n")),
        "step if: false",
    );
    assert_caught(
        &plant(
            &t,
            &anchor,
            &format!("{anchor}        continue-on-error: true\n"),
        ),
        "step continue-on-error",
    );
}

#[test]
fn a_gate_that_stops_needing_or_checking_dogfood_guards_is_caught() {
    let t = ci_yml();
    assert_caught(&plant(&t, ", dogfood-guards]", "]"), "gate needs dropped");
    assert_caught(
        &plant(&t, "${{ needs.dogfood-guards.result }}", "success"),
        "gate result check dropped",
    );
}
