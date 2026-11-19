//! forjar#881: the three light PR jobs (lockfile-preflight, examples-validate,
//! file-health) opt into a burst seat through their OWN repository variable,
//! `BURST_OPTIN_LIGHT`, so each class of job can be switched on or off without
//! moving the others. The older trio (no-default-features, doctests,
//! dogfood-guards) keeps `BURST_OPTIN`.
//!
//! With the variable unset each job gets
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

/// The whole expected `runs-on` string for one variable and one fallback array.
fn expected(var: &str, fallback: &str) -> String {
    format!("${{{{ ({PR} && vars.{var} == '1') && {BURST} || fromJSON('{fallback}') }}}}")
}

#[test]
fn the_light_jobs_read_burst_optin_light_and_keep_their_else_branch() {
    let ci = load(".github/workflows/ci.yml");
    let lint = load(".github/workflows/lint.yml");
    let x64 = r#"["self-hosted","clean-room","X64"]"#;
    let plain = r#"["self-hosted","clean-room"]"#;
    let cases = [
        (&ci, "lockfile", x64),
        (&ci, "examples-validate", x64),
        (&lint, "file-health", plain),
    ];
    let want = |f| expected("BURST_OPTIN_LIGHT", f);
    for (doc, job, fallback) in cases {
        // Exact string: kills a flipped `&&`/`||`, a wider event term, an extra
        // `|| vars.X` and a trailing `|| true` in one assert.
        assert_eq!(runs_on(doc, job), want(fallback), "{job}: runs-on changed");
        let j = &doc["jobs"][job];
        let mut perm = serde_yaml_ng::Mapping::new();
        perm.insert("contents".into(), "read".into());
        assert_eq!(j["permissions"], Value::Mapping(perm), "{job}: permissions");
        let text = serde_yaml_ng::to_string(j).unwrap();
        assert!(
            !text.contains("continue-on-error"),
            "{job}: continue-on-error"
        );
        assert_eq!(
            text.matches("persist-credentials: false").count(),
            1,
            "{job}"
        );
        // No job-level `if` beyond the one examples-validate already had.
        let guard = (job == "examples-validate").then_some("needs.classify.outputs.code == 'true'");
        assert_eq!(j["if"].as_str(), guard, "{job}: if changed");
    }
}

#[test]
fn the_older_trio_keeps_burst_optin() {
    let ci = load(".github/workflows/ci.yml");
    for job in ["no-default-features", "doctests", "dogfood-guards"] {
        let want = expected("BURST_OPTIN", r#"["self-hosted","clean-room","X64"]"#);
        assert_eq!(runs_on(&ci, job), want, "{job}");
    }
}

#[test]
fn this_test_is_in_the_examples_validate_target_list() {
    let ci = load(".github/workflows/ci.yml");
    let job = serde_yaml_ng::to_string(&ci["jobs"]["examples-validate"]).unwrap();
    let me = "--test falsification_ci_light_jobs_read_their_own_burst_variable";
    assert!(job.contains(me), "{me} is not run by examples-validate");
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
