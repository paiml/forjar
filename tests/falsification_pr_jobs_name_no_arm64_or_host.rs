//! forjar#1090: a job that a pull request triggers must be served by a pool,
//! never by one architecture's few runners or by one named host.
//!
//! On 2026-11-15 the required PR job `examples-validate` asked for
//! `[self-hosted, clean-room, ARM64]`. No ARM64 runner was online, the job
//! stayed queued, and no pull request could merge for hours. The aarch64 smoke
//! now runs in `nightly.yml`, which no pull request triggers.
//!
//! The rule: in every workflow triggered by `pull_request` or `merge_group`, no
//! job's `runs-on` (or the matrix it reads its runner from) names `ARM64` or a
//! host label.

use serde_yaml_ng::Value;
use std::path::Path;

/// Labels that select one machine (or one machine class of two or three),
/// not a pool.
const HOST_LABELS: [&str; 9] = [
    "gx10",
    "intel",
    "yoga-build",
    "mac-server",
    "t2yoga",
    "kfocus",
    "framework16",
    "t2fw13",
    "lambda",
];

fn pr_triggered(doc: &Value) -> bool {
    // `on` parses as a string key under YAML 1.2 and as `true` under YAML 1.1.
    let on = doc.get("on").or_else(|| doc.get(Value::Bool(true)));
    let has = |k: &str| match on {
        Some(Value::Mapping(m)) => m.contains_key(Value::from(k)),
        Some(Value::Sequence(s)) => s.iter().any(|v| v.as_str() == Some(k)),
        Some(Value::String(s)) => s == k,
        _ => false,
    };
    has("pull_request") || has("merge_group") || has("pull_request_target")
}

/// Every label a job can end up on: `runs-on` and its `strategy` (matrix legs),
/// as text, split into label tokens.
fn labels_of(job: &Value) -> Vec<String> {
    let mut text = String::new();
    for key in ["runs-on", "strategy"] {
        if let Some(v) = job.get(key) {
            text.push_str(&serde_yaml_ng::to_string(v).expect("yaml value serialises"));
            text.push('\n');
        }
    }
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

fn offending(labels: &[String]) -> Vec<String> {
    labels
        .iter()
        .filter(|l| l.eq_ignore_ascii_case("ARM64") || HOST_LABELS.contains(&l.as_str()))
        .cloned()
        .collect()
}

#[test]
fn no_pull_request_job_names_arm64_or_a_host() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows");
    let mut pr_workflows = 0;
    let mut findings = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("workflow directory is readable") {
        let path = entry.expect("readable entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("yml") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&path).expect("workflow is readable");
        let doc: Value = serde_yaml_ng::from_str(&text).expect("workflow parses");
        if !pr_triggered(&doc) {
            continue;
        }
        pr_workflows += 1;
        let jobs = doc.get("jobs").and_then(Value::as_mapping);
        for (job, body) in jobs.into_iter().flatten() {
            for label in offending(&labels_of(body)) {
                findings.push(format!("{name}:{}: {label}", job.as_str().unwrap_or("?")));
            }
        }
    }
    // Vacuity guard: ci.yml, lint.yml, audit.yml and pr-gate.yml at least.
    assert!(pr_workflows >= 4, "found only {pr_workflows} PR workflows");
    assert!(
        findings.is_empty(),
        "a pull-request job names ARM64 or a host label; move it to a pool or \
         to nightly.yml:\n{}",
        findings.join("\n")
    );
}

#[test]
fn the_scanner_sees_arm64_and_host_labels() {
    let job: Value = serde_yaml_ng::from_str(
        "runs-on: ${{ matrix.runner }}\nstrategy:\n  matrix:\n    include:\n      \
         - runner: [self-hosted, clean-room, ARM64]\n      - runner: [self-hosted, intel]\n",
    )
    .unwrap();
    assert_eq!(offending(&labels_of(&job)), ["ARM64", "intel"]);
    let pool: Value = serde_yaml_ng::from_str("runs-on: [self-hosted, clean-room, X64]").unwrap();
    assert!(offending(&labels_of(&pool)).is_empty());
    let pr: Value = serde_yaml_ng::from_str("on:\n  pull_request:\n  push:\n").unwrap();
    assert!(pr_triggered(&pr));
    let nightly: Value =
        serde_yaml_ng::from_str("on:\n  schedule:\n    - cron: '0 4 * * *'\n").unwrap();
    assert!(!pr_triggered(&nightly));
}

#[test]
fn the_aarch64_smoke_runs_nightly_and_does_not_hold_the_release() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/nightly.yml");
    let doc: Value = serde_yaml_ng::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert!(
        !pr_triggered(&doc),
        "nightly.yml must not run on pull requests"
    );
    let smoke = &doc["jobs"]["arm64-smoke"];
    assert!(labels_of(smoke).iter().any(|l| l == "ARM64"));
    let run = serde_yaml_ng::to_string(&smoke["steps"]).unwrap();
    assert!(run.contains("--test examples_validate"), "{run}");
    let needs = serde_yaml_ng::to_string(&doc["jobs"]["release"]["needs"]).unwrap();
    assert!(
        !needs.contains("arm64-smoke"),
        "release must not wait on it"
    );
}
