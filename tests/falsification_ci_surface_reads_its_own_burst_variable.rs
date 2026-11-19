//! forjar#881: `dogfood-surface` is 70% of forjar's clean-room seat-minutes, so
//! it has its own opt-in variable, `BURST_OPTIN_SURFACE`, separate from
//! `BURST_OPTIN` (which the trio of light jobs read).
//!
//! The rule: in ci.yml the job's `runs-on` is a pull_request-only conditional
//! on that variable; the burst branch is `["self-hosted","burst-x64"]`; the else
//! branch is exactly the labels the job had before; and `gate` still needs it.

use serde_yaml_ng::Value;
use std::path::Path;

const BURST: &str = r#"fromJSON('["self-hosted","burst-x64"]')"#;
const TODAY: &str = r#"fromJSON('["self-hosted","clean-room","X64"]')"#;

fn ci() -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    serde_yaml_ng::from_str(&std::fs::read_to_string(p).expect("ci.yml readable"))
        .expect("ci.yml parses")
}

fn norm(s: &str) -> String {
    s.split_whitespace().collect()
}

/// Check one `runs-on` expression against the shape the rule demands.
fn problems(expr: &str) -> Vec<String> {
    let e = norm(expr);
    let mut out = Vec::new();
    if !e.contains(&norm("github.event_name=='pull_request'")) {
        out.push("burst is not limited to pull_request".to_string());
    }
    if !e.contains(&norm("vars.BURST_OPTIN_SURFACE=='1'")) {
        out.push("does not read vars.BURST_OPTIN_SURFACE == '1'".to_string());
    }
    let rest = e.as_str();
    let want = format!("{}||{}", norm(BURST), norm(TODAY));
    if !rest.contains(&want) {
        out.push("branches are not burst-x64 then today's labels".to_string());
    }
    out
}

#[test]
fn dogfood_surface_reads_its_own_burst_variable() {
    let doc = ci();
    let job = &doc["jobs"]["dogfood-surface"];
    let expr = job["runs-on"].as_str().expect("runs-on is an expression");
    let found = problems(expr);
    assert!(
        found.is_empty(),
        "dogfood-surface runs-on: {found:?}\n{expr}"
    );
    assert!(
        !expr.contains("vars.BURST_OPTIN ") && !expr.contains("BURST_OPTIN=="),
        "dogfood-surface must not read the shared BURST_OPTIN: {expr}"
    );
}

#[test]
fn the_other_jobs_keep_the_variable_they_had() {
    let doc = ci();
    for j in ["examples-validate", "lockfile"] {
        let expr = doc["jobs"][j]["runs-on"].as_str().unwrap_or("");
        assert!(
            !expr.contains("BURST_OPTIN_SURFACE"),
            "{j} must not read the surface variable"
        );
    }
}

#[test]
fn gate_still_needs_dogfood_surface_and_checks_its_result() {
    let doc = ci();
    let needs = serde_yaml_ng::to_string(&doc["jobs"]["gate"]["needs"]).unwrap();
    assert!(needs.contains("dogfood-surface"), "{needs}");
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml"),
    )
    .unwrap();
    assert!(text.contains("needs.dogfood-surface.result"));
}

#[test]
fn the_checker_rejects_the_wrong_shapes() {
    let good = format!(
        "${{{{ (github.event_name == 'pull_request' && vars.BURST_OPTIN_SURFACE == '1') && {BURST} || {TODAY} }}}}"
    );
    assert!(problems(&good).is_empty(), "{:?}", problems(&good));
    assert!(!problems(&good.replace("BURST_OPTIN_SURFACE", "BURST_OPTIN")).is_empty());
    assert!(!problems(&good.replace("github.event_name == 'pull_request' && ", "")).is_empty());
    assert!(!problems(&good.replace("burst-x64", "burst-x65")).is_empty());
    assert!(!problems(&good.replace("X64", "ARM64")).is_empty());
}
