//! `gate` must enforce the complexity limits the pre-commit hook says it does.
//!
//! forjar#692. The generated pre-commit hook calls itself FEEDBACK and says
//! `ci / gate` enforces the thresholds on the merge path. No job in ci.yml did.
//! A commit made where the hook is not installed therefore reached main with
//! `toolchain_gaps` at Cognitive 39 > 25 (#690), and the next PR to merge main
//! had its merge commit refused by the hook for a function it never wrote
//! (#693 repaired the function, not the hole).
//!
//! These tests pin the wiring, PARSED rather than grepped, so a comment naming
//! the script cannot satisfy them:
//!
//! * `gate` needs the job that runs scripts/ci/complexity-diff-scope.sh, and
//!   its own script reads that job's result;
//! * that job hands the script the base `classify` compared against, from a
//!   full-history checkout (a shallow clone has no merge base);
//! * the limits live in pmat.toml `[quality]` -- the file `pmat hooks`
//!   generates the hook from -- and the script carries no number of its own.
//!
//! What the script DECIDES is proven by running it, not here: red on #690's
//! merge (Cognitive 39 > 25), green on #693's, and green on a PR that merges
//! #690 in, because the diff starts at the merge base.

use serde_yaml_ng::{Mapping, Value};
use std::path::Path;

const SCRIPT: &str = "scripts/ci/complexity-diff-scope.sh";

fn repo_file(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{rel} must be readable: {e}"))
}

fn ci_workflow() -> Value {
    serde_yaml_ng::from_str(&repo_file(".github/workflows/ci.yml"))
        .expect("ci.yml must parse as YAML")
}

fn jobs(w: &Value) -> &Mapping {
    w.get("jobs")
        .and_then(Value::as_mapping)
        .expect("ci.yml must define `jobs`")
}

fn steps(job: &Value) -> &[Value] {
    job.get("steps")
        .and_then(Value::as_sequence)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn step_runs(step: &Value) -> &str {
    step.get("run").and_then(Value::as_str).unwrap_or_default()
}

/// The id of the job with a step whose `run:` invokes the complexity script.
fn complexity_job_id(w: &Value) -> Option<String> {
    jobs(w)
        .iter()
        .filter(|(_, job)| steps(job).iter().any(|s| step_runs(s).contains(SCRIPT)))
        .find_map(|(id, _)| id.as_str().map(str::to_string))
}

fn needs(job: &Value) -> Vec<String> {
    match job.get("needs") {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Sequence(seq)) => seq
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

#[test]
fn gate_needs_the_job_that_runs_the_complexity_script_and_reads_its_result() {
    let w = ci_workflow();
    let id = complexity_job_id(&w).expect("no ci.yml job runs scripts/ci/complexity-diff-scope.sh");
    let gate = jobs(&w).get("gate").expect("ci.yml must define `gate`");
    assert!(
        needs(gate).contains(&id),
        "`gate` does not need `{id}`, so a red complexity job cannot turn the gate red"
    );
    let reads = format!("needs.{id}.result");
    assert!(
        steps(gate).iter().any(|s| step_runs(s).contains(&reads)),
        "`gate` needs `{id}` but its script never reads {reads}; with `if: always()` an unread need is decoration"
    );
}

#[test]
fn the_complexity_job_measures_from_the_base_classify_compared_against() {
    let w = ci_workflow();
    let id = complexity_job_id(&w).expect("no ci.yml job runs the complexity script");
    let job = &jobs(&w)[id.as_str()];

    let full_history = steps(job).iter().any(|s| {
        s.get("uses")
            .and_then(Value::as_str)
            .is_some_and(|u| u.starts_with("actions/checkout@"))
            && s.get("with")
                .and_then(|with| with.get("fetch-depth"))
                .and_then(Value::as_u64)
                == Some(0)
    });
    assert!(
        full_history,
        "`{id}` must check out with fetch-depth: 0 -- a shallow clone has no merge base"
    );

    let step = steps(job)
        .iter()
        .find(|s| step_runs(s).contains(SCRIPT))
        .expect("the step that runs the script");
    let base = step
        .get("env")
        .and_then(|e| e.get("BASE"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert_eq!(
        base, "${{ needs.classify.outputs.base }}",
        "the script must be handed classify's base, not a second definition of it"
    );
    assert!(
        step_runs(step).contains("\"$BASE\""),
        "the step must pass $BASE to the script"
    );
    assert!(
        needs(job).iter().any(|n| n == "classify"),
        "`{id}` reads classify's output, so it must need classify"
    );

    let classify = &jobs(&w)["classify"];
    let out = classify
        .get("outputs")
        .and_then(|o| o.get("base"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        out.contains("steps.class.outputs.base"),
        "classify must export the base its changed-class step measured against"
    );
    let action = repo_file(".github/actions/changed-class/action.yml");
    assert!(
        action.contains("echo \"base=$(git rev-parse \"$BASE^{commit}\")\" >> \"$GITHUB_OUTPUT\""),
        "the changed-class action must write `base=` from the same BASE it diffed against"
    );
}

#[test]
fn the_limits_live_in_pmat_toml_and_nowhere_else() {
    let table: toml::Table = repo_file("pmat.toml")
        .parse()
        .expect("pmat.toml must parse");
    let quality = table
        .get("quality")
        .and_then(toml::Value::as_table)
        .expect("pmat.toml must have [quality] -- `pmat hooks` reads the hook's limits from it");
    for key in ["max_complexity", "max_cognitive_complexity"] {
        let v = quality.get(key).and_then(toml::Value::as_integer);
        assert!(
            v.is_some_and(|v| v > 0),
            "pmat.toml [quality] {key} must be a positive integer, got {v:?}"
        );
    }

    let script = repo_file(SCRIPT);
    assert!(
        script.contains("pmat.toml"),
        "the script must read pmat.toml"
    );
    for flag in ["--max-cyclomatic", "--max-cognitive"] {
        let line = script
            .lines()
            .find(|l| l.contains(flag))
            .unwrap_or_else(|| panic!("the script must pass {flag}"));
        let value = line.split(flag).nth(1).unwrap_or_default().trim_start();
        assert!(
            value.starts_with("\"$max_"),
            "{flag} must take the value read from pmat.toml, not a literal: {line}"
        );
    }
    assert!(
        script.contains("--diff-scope"),
        "the script must ask the hook's question (--diff-scope), or it bills a change for debt it did not write"
    );
}
