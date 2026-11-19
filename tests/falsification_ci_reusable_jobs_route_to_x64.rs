//! forjar#1199: the routing of the reusable workflow's jobs is the parsed
//! `jobs.ci.with.runs_on` value, never a comment. It is the X64 clean-room pool.
//!
//! mutation: change `X64` to `ARM64` in ci.yml's `runs_on` -> the first test goes RED.

use serde_yaml_ng::Value;

const WANT: &str = r#"["self-hosted","clean-room","X64"]"#;

/// The `jobs.ci.with.runs_on` string of a workflow text, if present.
fn runs_on(text: &str) -> Option<String> {
    let doc: Value = serde_yaml_ng::from_str(text).expect("workflow parses as YAML");
    let v = doc.get("jobs")?.get("ci")?.get("with")?.get("runs_on")?;
    v.as_str().map(String::from)
}

#[test]
fn the_reusable_workflow_runs_on_the_x64_clean_room_pool() {
    let path = format!("{}/.github/workflows/ci.yml", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    assert_eq!(runs_on(&text).as_deref(), Some(WANT));
}

#[test]
fn the_predicate_refuses_other_routes_and_a_missing_value() {
    let wf = |v: &str| format!("jobs:\n  ci:\n    with:\n      runs_on: '{v}'\n");
    assert_eq!(runs_on(&wf(WANT)).as_deref(), Some(WANT));
    assert_ne!(
        runs_on(&wf(r#"["self-hosted","clean-room","ARM64"]"#)).as_deref(),
        Some(WANT)
    );
    assert_eq!(runs_on("jobs:\n  ci:\n    with: {}\n"), None);
}
