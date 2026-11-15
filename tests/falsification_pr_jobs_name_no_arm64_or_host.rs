//! forjar#1090: a job that a pull request triggers must be served by a pool,
//! never by one architecture's few runners or by one named host.
//!
//! On 2026-11-15 the reusable `ci` job (`with: runs_on:`) and `examples-validate`
//! (`runs-on`) asked for ARM64 with no ARM64 runner online: every PR queued for
//! hours. The aarch64 smoke now runs in `nightly.yml`.
//!
//! The rule: in a PR-triggered workflow no job runner (`runs-on`, its matrix, a `with:`
//! runner input) names an ARM architecture or a host label, bar [`KNOWN_HOST_PINS`].

use serde_yaml_ng::Value;
use std::path::Path;

/// Labels that select one machine (or two or three of one kind), not a pool.
/// `.github/actionlint.yaml` documents them. Compared without case.
const HOST_LABELS: &str = "arm64 aarch64 gx10 intel yoga yoga-build mac-server t2yoga kfocus \
                           framework framework16 t2fw13 lambda mini mini-builder \
                           macos perf-solo ada blackwell gb10";

/// (workflow, job, label): the macOS legs of `lint.yml` have one seat, the
/// self-hosted mac. A reviewed exception; a stale entry fails the test.
const KNOWN_HOST_PINS: [(&str, &str, &str); 2] = [
    ("lint.yml", "clippy", "mini-builder"),
    ("lint.yml", "clippy", "macos"),
];

fn pr_triggered(doc: &Value) -> bool {
    let on = doc.get("on").or_else(|| doc.get(Value::Bool(true)));
    let has = |k: &str| match on {
        Some(Value::Mapping(m)) => m.contains_key(Value::from(k)),
        Some(Value::Sequence(s)) => s.iter().any(|v| v.as_str() == Some(k)),
        Some(Value::String(s)) => s == k,
        _ => false,
    };
    has("pull_request") || has("merge_group") || has("pull_request_target")
}

/// Every label a job can end up on, as lower-case tokens: `runs-on`, its
/// `strategy` (matrix legs) and any `with:` input that names a runner.
fn labels_of(job: &Value) -> Vec<String> {
    let mut parts: Vec<&Value> = ["runs-on", "strategy"]
        .iter()
        .filter_map(|k| job.get(*k))
        .collect();
    for (k, v) in job
        .get("with")
        .and_then(Value::as_mapping)
        .into_iter()
        .flatten()
    {
        let key = k
            .as_str()
            .unwrap_or("")
            .to_ascii_lowercase()
            .replace('-', "_");
        if key.contains("runs_on") || key.contains("runner") {
            parts.push(v);
        }
    }
    let text: String = parts
        .iter()
        .map(|v| serde_yaml_ng::to_string(v).expect("yaml"))
        .collect();
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .filter(|t| !t.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn offending(labels: &[String]) -> Vec<String> {
    let hit = |l: &&String| HOST_LABELS.split_whitespace().any(|h| h == l.as_str());
    labels.iter().filter(hit).cloned().collect()
}

#[test]
fn no_pull_request_job_names_arm64_or_a_host() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows");
    let (mut pr_workflows, mut findings, mut pins_seen) = (0, Vec::new(), Vec::new());
    for entry in std::fs::read_dir(&dir).expect("workflow directory is readable") {
        let path = entry.expect("readable entry").path();
        if !matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("yml" | "yaml")
        ) {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&path).expect("workflow is readable");
        let doc: Value = serde_yaml_ng::from_str(&text).expect("workflow parses");
        if !pr_triggered(&doc) {
            continue;
        }
        pr_workflows += 1;
        for (job, body) in doc
            .get("jobs")
            .and_then(Value::as_mapping)
            .into_iter()
            .flatten()
        {
            let job = job.as_str().unwrap_or("?");
            for label in offending(&labels_of(body)) {
                match KNOWN_HOST_PINS
                    .iter()
                    .find(|p| **p == (name.as_str(), job, label.as_str()))
                {
                    Some(pin) => pins_seen.push(*pin),
                    None => findings.push(format!("{name}:{job}: {label}")),
                }
            }
        }
    }
    assert!(pr_workflows >= 4, "found only {pr_workflows} PR workflows");
    assert!(
        findings.is_empty(),
        "a pull-request job names an ARM architecture or a host label; move it to a pool or \
         to nightly.yml:\n{}",
        findings.join("\n")
    );
    for pin in KNOWN_HOST_PINS {
        assert!(
            pins_seen.contains(&pin),
            "stale KNOWN_HOST_PINS entry: {pin:?}"
        );
    }
}

#[test]
fn the_scanner_sees_every_form_a_runner_is_named_in() {
    let matrix: Value = serde_yaml_ng::from_str(
        "runs-on: ${{ matrix.runner }}\nstrategy:\n  matrix:\n    include:\n      \
         - runner: [self-hosted, clean-room, ARM64]\n      - runner: [self-hosted, Intel]\n",
    )
    .unwrap();
    assert_eq!(offending(&labels_of(&matrix)), ["arm64", "intel"]);
    // The form that queued every PR: a reusable workflow's runner input.
    let reusable: Value = serde_yaml_ng::from_str(
        "uses: o/r/.github/workflows/w.yml@abc\nwith:\n  \
         runs_on: '[\"self-hosted\",\"clean-room\",\"ARM64\"]'\n  note: intel\n",
    )
    .unwrap();
    assert_eq!(offending(&labels_of(&reusable)), ["arm64"]);
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
    assert!(labels_of(smoke).iter().any(|l| l == "arm64"));
    let run = serde_yaml_ng::to_string(&smoke["steps"]).unwrap();
    assert!(run.contains("--test examples_validate"), "{run}");
    let needs = serde_yaml_ng::to_string(&doc["jobs"]["release"]["needs"]).unwrap();
    assert!(
        !needs.contains("arm64-smoke"),
        "release must not wait on it"
    );
}
