//! The three light PR jobs (lockfile, examples-validate, file-health) read
//! nothing but the repository, so each carries `permissions: contents: read`
//! and checks out with `persist-credentials: false`. This guard judges the
//! parsed YAML: permissions are exactly that map, no `continue-on-error`,
//! exactly one checkout step per job and it drops the token, the job-level
//! `if` is the one the job already had, `gate` still needs both ci.yml jobs,
//! and this target is run by examples-validate.

use serde_yaml_ng::Value;
use std::path::Path;

fn load(rel: &str) -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    let text = std::fs::read_to_string(&p).expect("workflow is readable");
    serde_yaml_ng::from_str(&text).expect("workflow parses")
}

fn light_jobs() -> Vec<(&'static str, Value)> {
    let ci = load(".github/workflows/ci.yml");
    let lint = load(".github/workflows/lint.yml");
    vec![
        ("lockfile", ci["jobs"]["lockfile"].clone()),
        ("examples-validate", ci["jobs"]["examples-validate"].clone()),
        ("file-health", lint["jobs"]["file-health"].clone()),
    ]
}

#[test]
fn light_jobs_hold_contents_read_and_nothing_else() {
    for (name, job) in light_jobs() {
        let mut want = serde_yaml_ng::Mapping::new();
        want.insert("contents".into(), "read".into());
        assert_eq!(
            job["permissions"],
            Value::Mapping(want),
            "{name}: permissions"
        );
        let text = serde_yaml_ng::to_string(&job).unwrap();
        assert!(
            !text.contains("continue-on-error"),
            "{name}: continue-on-error"
        );
    }
}

#[test]
fn light_jobs_checkout_drops_the_token() {
    for (name, job) in light_jobs() {
        let steps = job["steps"].as_sequence().expect("steps");
        let checkouts: Vec<&Value> = steps
            .iter()
            .filter(|s| {
                s["uses"]
                    .as_str()
                    .is_some_and(|u| u.starts_with("actions/checkout@"))
            })
            .collect();
        assert_eq!(checkouts.len(), 1, "{name}: checkout steps");
        assert_eq!(
            checkouts[0]["with"]["persist-credentials"],
            Value::Bool(false),
            "{name}: persist-credentials"
        );
    }
}

#[test]
fn light_jobs_keep_their_job_level_if() {
    for (name, job) in light_jobs() {
        let want = (name == "examples-validate").then_some("needs.classify.outputs.code == 'true'");
        assert_eq!(job["if"].as_str(), want, "{name}: if changed");
    }
}

#[test]
fn gate_needs_both_ci_jobs_and_this_target_runs_in_examples_validate() {
    let ci = load(".github/workflows/ci.yml");
    let needs = serde_yaml_ng::to_string(&ci["jobs"]["gate"]["needs"]).unwrap();
    for job in ["lockfile", "examples-validate"] {
        assert!(
            needs.lines().any(|l| l.trim_start_matches("- ") == job),
            "{needs}"
        );
    }
    let ev = serde_yaml_ng::to_string(&ci["jobs"]["examples-validate"]).unwrap();
    let me = "--test falsification_ci_light_jobs_are_hardened";
    assert!(ev.contains(me), "{me} is not run by examples-validate");
}
