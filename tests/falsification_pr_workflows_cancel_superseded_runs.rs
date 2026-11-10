//! Refs fleet-sup 10:16Z (Noah order 10:14Z): every workflow triggered by
//! `pull_request` carries a top-level `concurrency` block, so a new push
//! cancels that PR's older queued or running runs, and ONLY a PR run is ever
//! cancelled.
//!
//! The required shape:
//!
//! ```yaml
//! concurrency:
//!   group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.run_id }}
//!   cancel-in-progress: ${{ github.event_name == 'pull_request' }}
//! ```
//!
//! Why `run_id` and not `ref` for the non-PR fallback: a group that two
//! non-PR runs share replaces the PENDING one even with
//! `cancel-in-progress: false`, so two quick pushes to main would silently
//! skip a main run. A unique group per non-PR run is never cancelled and
//! never replaced. Push, schedule, merge_group, release and train/** runs all
//! take that path (train/** runs are push events since #971, not pull_request).
//!
//! The rule is a pure function (`violations`) so the mutants below can prove
//! the test would catch each way of breaking it.

use serde_yaml_ng::Value;
use std::path::Path;

const CANCEL: &str = "${{ github.event_name == 'pull_request' }}";

fn trigger_names(doc: &Value) -> Vec<String> {
    // YAML 1.1 reads the key `on` as the boolean true; accept either spelling.
    let on = doc
        .get("on")
        .or_else(|| doc.get(Value::Bool(true)))
        .cloned()
        .unwrap_or(Value::Null);
    match on {
        Value::String(s) => vec![s],
        Value::Sequence(v) => v
            .iter()
            .filter_map(|x| x.as_str().map(String::from))
            .collect(),
        Value::Mapping(m) => m
            .keys()
            .filter_map(|k| k.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    }
}

/// Every way `text` (a workflow file) breaks the cancel-superseded rule.
fn violations(text: &str) -> Vec<String> {
    let doc: Value = serde_yaml_ng::from_str(text).expect("workflow parses as YAML");
    if !trigger_names(&doc).iter().any(|t| t == "pull_request") {
        return Vec::new();
    }
    let Some(c) = doc.get("concurrency") else {
        return vec!["no top-level `concurrency` block".into()];
    };
    let mut out = Vec::new();
    let group = c.get("group").and_then(Value::as_str).unwrap_or("");
    if !group.contains("github.workflow") || !group.contains("github.event.pull_request.number") {
        out.push(format!(
            "group `{group}` is not per workflow and per PR number"
        ));
    }
    if group.contains("github.ref") && !group.contains("github.event.pull_request.number") {
        out.push("group is per ref only".into());
    }
    match c.get("cancel-in-progress") {
        Some(Value::String(s)) if s == CANCEL => {}
        other => out.push(format!(
            "cancel-in-progress is {other:?}, want `{CANCEL}` (a bare true would cancel main/train runs)"
        )),
    }
    out
}

const GOOD: &str = "on:\n  push:\n    branches: [main]\n  pull_request:\nconcurrency:\n  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.run_id }}\n  cancel-in-progress: ${{ github.event_name == 'pull_request' }}\njobs: {}\n";

#[test]
fn every_pull_request_workflow_cancels_superseded_runs() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows");
    let mut seen = Vec::new();
    let mut bad = Vec::new();
    for e in std::fs::read_dir(&dir).expect("read .github/workflows") {
        let p = e.expect("dir entry").path();
        if !p.extension().is_some_and(|x| x == "yml" || x == "yaml") {
            continue;
        }
        let text = std::fs::read_to_string(&p).expect("read workflow");
        let v = violations(&text);
        if trigger_names(&serde_yaml_ng::from_str(&text).expect("yaml"))
            .iter()
            .any(|t| t == "pull_request")
        {
            seen.push(p.file_stem().unwrap().to_string_lossy().into_owned());
        }
        for m in v {
            bad.push(format!("{}: {m}", p.file_name().unwrap().to_string_lossy()));
        }
    }
    // After #953/#962/#971 only these workflows run on pull_request (the rest moved to
    // push main+train/** or workflow_dispatch). The old floor of 9 counted workflows that no
    // longer exist on the PR path; name the real set so losing one still fails discovery.
    for want in ["audit", "ci", "lint"] {
        assert!(
            seen.iter().any(|s| s == want),
            "discovery lost pull_request workflow {want}; found {seen:?}"
        );
    }
    assert!(
        bad.is_empty(),
        "superseded runs are not cancelled:\n{}",
        bad.join("\n")
    );
}

#[test]
fn the_rule_accepts_the_required_shape() {
    assert!(violations(GOOD).is_empty());
}

#[test]
fn mutant_missing_block_is_caught() {
    let m = GOOD.replace("concurrency:\n  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.run_id }}\n  cancel-in-progress: ${{ github.event_name == 'pull_request' }}\n", "");
    assert_eq!(
        violations(&m),
        vec!["no top-level `concurrency` block".to_string()]
    );
}

#[test]
fn mutant_unconditional_cancel_is_caught() {
    let m = GOOD.replace("${{ github.event_name == 'pull_request' }}", "true");
    assert!(violations(&m)
        .iter()
        .any(|v| v.contains("cancel-in-progress")));
}

#[test]
fn mutant_group_per_ref_only_is_caught() {
    let m = GOOD.replace(
        "${{ github.event.pull_request.number || github.run_id }}",
        "${{ github.ref }}",
    );
    assert!(!violations(&m).is_empty());
}

#[test]
fn a_workflow_without_a_pull_request_trigger_is_exempt() {
    let m = "on:\n  push:\n    branches: [main]\njobs: {}\n";
    assert!(violations(m).is_empty());
}

/// A PR gate makes no network fetch and uses no floating toolchain (Noah, 2026-11-10).
#[test]
fn pr_workflows_install_nothing_over_the_network() {
    let banned = [
        "dtolnay/rust-toolchain",
        "toolchain install",
        "cargo install",
    ];
    for name in ["behavior", "convergence", "mutation", "quorum"] {
        let path = format!(
            "{}/.github/workflows/{name}.yml",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read_to_string(&path).unwrap();
        for (n, line) in text.lines().enumerate() {
            if line.trim_start().starts_with('#') {
                continue;
            }
            for b in banned {
                assert!(
                    !line.contains(b),
                    "{name}.yml:{} fetches over the network: {line}",
                    n + 1
                );
            }
        }
    }
}
