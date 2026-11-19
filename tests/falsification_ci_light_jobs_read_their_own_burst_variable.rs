//! forjar#881: the three light PR jobs (lockfile-preflight, examples-validate,
//! file-health) opt into a burst seat through their OWN repository variable,
//! `BURST_OPTIN_LIGHT`, so each class of job can be switched on or off without
//! moving the others. The older trio (no-default-features, doctests,
//! dogfood-guards) keeps `BURST_OPTIN`.
//!
//! Only `runs-on` may differ from today: with the variable unset each job gets
//! exactly its previous label array, and `gate` still needs both ci.yml jobs.

use serde_yaml_ng::Value;
use std::path::Path;

const BURST: &str = r#"fromJSON('["self-hosted","burst-x64"]')"#;
const PR: &str = "github.event_name == 'pull_request'";

fn load(rel: &str) -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    let text = std::fs::read_to_string(&p).expect("workflow is readable");
    serde_yaml_ng::from_str(&text).expect("workflow parses")
}

fn runs_on(doc: &Value, job: &str) -> String {
    doc["jobs"][job]["runs-on"]
        .as_str()
        .unwrap_or_else(|| panic!("{job}: runs-on is not an expression string"))
        .to_string()
}

/// The label array after the final `||`, i.e. what the job runs on when the
/// variable is not `1`.
fn else_branch(expr: &str) -> String {
    expr.rsplit_once("|| ")
        .map(|(_, e)| e.trim_end_matches("}}").trim().to_string())
        .unwrap_or_default()
}

#[test]
fn the_light_jobs_read_burst_optin_light_and_keep_their_else_branch() {
    let ci = load(".github/workflows/ci.yml");
    let lint = load(".github/workflows/lint.yml");
    let x64 = r#"fromJSON('["self-hosted","clean-room","X64"]')"#;
    let plain = r#"fromJSON('["self-hosted","clean-room"]')"#;
    let cases = [
        (&ci, "lockfile", x64),
        (&ci, "examples-validate", x64),
        (&lint, "file-health", plain),
    ];
    for (doc, job, else_label) in cases {
        let e = runs_on(doc, job);
        assert!(e.contains("vars.BURST_OPTIN_LIGHT == '1'"), "{job}: {e}");
        assert!(
            !e.contains("vars.BURST_OPTIN =="),
            "{job} reads BURST_OPTIN: {e}"
        );
        assert!(e.contains(PR) && e.contains(BURST), "{job}: {e}");
        assert_eq!(else_branch(&e), else_label, "{job}: else-branch changed");
    }
}

#[test]
fn the_older_trio_keeps_burst_optin() {
    let ci = load(".github/workflows/ci.yml");
    for job in ["no-default-features", "doctests", "dogfood-guards"] {
        let e = runs_on(&ci, job);
        assert!(e.contains("vars.BURST_OPTIN == '1'"), "{job}: {e}");
        assert!(!e.contains("BURST_OPTIN_LIGHT"), "{job}: {e}");
    }
}

#[test]
fn gate_still_needs_the_two_light_jobs_of_ci_yml() {
    let ci = load(".github/workflows/ci.yml");
    let needs = serde_yaml_ng::to_string(&ci["jobs"]["gate"]["needs"]).unwrap();
    for job in ["lockfile", "examples-validate"] {
        assert!(
            needs.lines().any(|l| l.trim_start_matches("- ") == job),
            "{needs}"
        );
    }
}
