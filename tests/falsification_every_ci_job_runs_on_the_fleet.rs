//! Every job in this repository runs on the paiml fleet.
//!
//! # What was measured
//!
//! PR #545, workflow run 34685410794. Three of this repository's own CI jobs,
//! read back from the GitHub API rather than from the workflow file:
//!
//! ```text
//! dogfood-surface  runner=GitHub Actions 1000366553  labels=ubuntu-latest  success
//! classify         runner=GitHub Actions 1000366551  labels=ubuntu-latest  success
//! doctests         runner=GitHub Actions 1000366556  labels=ubuntu-latest  success
//! ```
//!
//! `runner=GitHub Actions <n>` is a GitHub-hosted runner. Thirty-two runner
//! declarations across seventeen workflow files named one, and every pull
//! request spent them.
//!
//! # Why a test and not a review note
//!
//! A hosted-runner label is one word, it is the default everyone reaches for,
//! and nothing about a green check says which machine produced it — the job
//! passes identically either way. That is exactly the shape that comes back:
//! the next workflow, or the next job added to an existing one, is written
//! `runs-on: ubuntu-latest` by habit and no reviewer sees it. So it is pinned
//! here, and the pin is a count rather than a prohibition, because the six
//! legs the fleet CANNOT serve have to stay visible instead of being argued
//! about once and forgotten.
//!
//! # The fleet
//!
//! `gh api orgs/paiml/actions/runners`, 2026-09-12: 25 runners. Labels —
//! `self-hosted` 25, `build` 23, `clean-room` 20, `intel` 16, `yoga` 5,
//! `gx10` 4. **Zero macOS and zero Windows.** So a macOS or Windows leg has
//! nowhere on the fleet to go, and those legs are enumerated below rather than
//! silently deleted: dropping them would drop this project's coverage of those
//! platforms, which is a decision, not a cleanup.
//!
//! THIS TEST PARSES THE WORKFLOWS rather than grepping them, so a hosted label
//! inside a comment is prose and a hosted label inside a matrix is a finding.

use serde_yaml_ng::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn workflow_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows")
}

fn workflows() -> Vec<(String, Value)> {
    let dir = workflow_dir();
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("the workflow directory must be readable") {
        let path = entry.expect("a readable directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("yml") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("a utf-8 filename")
            .to_string();
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
        let doc: Value =
            serde_yaml_ng::from_str(&text).unwrap_or_else(|e| panic!("{name} must parse: {e}"));
        out.push((name, doc));
    }
    assert!(
        out.len() >= 15,
        "expected the workflow directory to hold the repository's workflows, found {}",
        out.len()
    );
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// A GitHub-HOSTED runner label. The fleet's labels (`self-hosted`,
/// `clean-room`, `build`, …) are not in this shape and never match.
fn hosted_label(v: &str) -> bool {
    let v = v.trim();
    v.starts_with("ubuntu-") || v.starts_with("macos-") || v.starts_with("windows-")
}

/// Append every string in `v` — a bare label, or a list like
/// `[self-hosted, clean-room]`.
fn push_labels(out: &mut Vec<String>, v: &Value) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Sequence(seq) => {
            for item in seq {
                if let Value::String(s) = item {
                    out.push(s.clone());
                }
            }
        }
        _ => {}
    }
}

/// The labels a `runs-on: ${{ matrix.<key> }}` expression can take, read out of
/// the matrix itself: the `include:` legs first, then any top-level list.
fn matrix_labels(job: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let Some(matrix) = job.get("strategy").and_then(|s| s.get("matrix")) else {
        return out;
    };
    if let Some(Value::Sequence(include)) = matrix.get("include") {
        for leg in include {
            for key in ["runner", "os"] {
                if let Some(v) = leg.get(key) {
                    push_labels(&mut out, v);
                }
            }
        }
    }
    for key in ["runner", "os"] {
        if let Some(v) = matrix.get(key) {
            push_labels(&mut out, v);
        }
    }
    out
}

/// Every literal runner label a job can end up on: its own `runs-on` when that
/// is a literal, and — when `runs-on` is a `${{ matrix.* }}` expression — the
/// values that expression can take.
fn runner_labels(job: &Value) -> Vec<String> {
    let Some(runs_on) = job.get("runs-on") else {
        return Vec::new();
    };
    let is_expr = matches!(runs_on, Value::String(s) if s.contains("${{"));
    if is_expr {
        return matrix_labels(job);
    }
    let mut out = Vec::new();
    push_labels(&mut out, runs_on);
    out
}

/// Every `(workflow, job, label)` in the repository that names a hosted runner.
fn hosted_sites() -> Vec<(String, String, String)> {
    let mut sites = Vec::new();
    for (file, doc) in workflows() {
        let Some(Value::Mapping(jobs)) = doc.get("jobs") else {
            continue;
        };
        for (name, job) in jobs {
            let name = name.as_str().unwrap_or("<non-string job name>").to_string();
            for label in runner_labels(job) {
                if hosted_label(&label) {
                    sites.push((file.clone(), name.clone(), label));
                }
            }
        }
    }
    sites.sort();
    sites
}

/// The measurement this whole test rests on: the parser can SEE a runner
/// label. A `runner_labels` that returned nothing would make every assertion
/// below vacuously true.
#[test]
fn the_parser_finds_the_runners_that_are_there() {
    let mut fleet = 0usize;
    for (_, doc) in workflows() {
        let Some(Value::Mapping(jobs)) = doc.get("jobs") else {
            continue;
        };
        for (_, job) in jobs {
            fleet += runner_labels(job)
                .iter()
                .filter(|l| l.as_str() == "self-hosted")
                .count();
        }
    }
    assert!(
        fleet >= 20,
        "the parser found only {fleet} `self-hosted` labels; it is not reading \
         runners and every other case in this file would pass over anything"
    );
}

/// No Linux job asks GitHub for a runner. This is the whole ticket.
#[test]
fn no_linux_job_asks_github_for_a_runner() {
    let linux: Vec<_> = hosted_sites()
        .into_iter()
        .filter(|(_, _, label)| label.starts_with("ubuntu-"))
        .collect();
    assert!(
        linux.is_empty(),
        "these jobs run Linux on GitHub-hosted runners, and the fleet has 25 Linux \
         runners that should have them:\n{}",
        linux
            .iter()
            .map(|(f, j, l)| format!("  {f}  job `{j}`  runs-on {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The legs the fleet cannot serve are exactly these, and adding one fails.
///
/// Not a prohibition: `gh api orgs/paiml/actions/runners` reports no macOS and
/// no Windows runner, so these legs have nowhere to go. Counting them keeps
/// the exception from quietly becoming the rule.
#[test]
fn the_platforms_the_fleet_cannot_serve_are_exactly_these() {
    let mut found: BTreeMap<String, usize> = BTreeMap::new();
    for (file, _, label) in hosted_sites() {
        if label.starts_with("ubuntu-") {
            continue; // the case above owns those
        }
        *found.entry(format!("{file}:{label}")).or_default() += 1;
    }

    let expected: BTreeMap<String, usize> = [
        ("lint.yml:macos-latest", 2),
        ("nightly.yml:macos-latest", 2),
        ("nightly.yml:windows-latest", 1),
        ("release.yml:macos-latest", 2),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();

    assert_eq!(
        found, expected,
        "the set of legs GitHub still runs for this repository changed.\n\
         Every one of them is a platform the fleet has no runner for. If a leg \
         was ADDED, it needs fleet hardware or a decision to drop that platform; \
         if one was REMOVED, say so here."
    );
}

/// A fleet job names `self-hosted` AND a pool, never `self-hosted` alone.
///
/// `runs-on: self-hosted` matches any of the 25 runners, including the GPU and
/// yoga boxes, which is how a lint job ends up holding a blackwell.
#[test]
fn a_fleet_job_names_a_pool_and_not_just_self_hosted() {
    let pools = [
        "clean-room",
        "build",
        "intel",
        "gx10",
        "yoga",
        "gpu",
        "cuda",
    ];
    let mut bare = Vec::new();
    for (file, doc) in workflows() {
        let Some(Value::Mapping(jobs)) = doc.get("jobs") else {
            continue;
        };
        for (name, job) in jobs {
            let labels = runner_labels(job);
            if !labels.iter().any(|l| l == "self-hosted") {
                continue;
            }
            if !labels.iter().any(|l| pools.contains(&l.as_str())) {
                bare.push(format!(
                    "  {file}  job `{}`  runs-on {labels:?}",
                    name.as_str().unwrap_or("?")
                ));
            }
        }
    }
    assert!(
        bare.is_empty(),
        "these jobs say `self-hosted` without naming a pool, so they can land on \
         any of the 25 runners including the GPU boxes:\n{}",
        bare.join("\n")
    );
}
