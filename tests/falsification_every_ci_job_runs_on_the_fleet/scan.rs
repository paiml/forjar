//! Reading a runner out of a workflow — the parser every case in this suite
//! stands on, and the fixture helper the controls drive it with.
//!
//! Split out of `main.rs` when that file reached this repository's 500-line
//! gate. Nothing here asserts anything.

use serde_yaml_ng::Value;
use std::path::{Path, PathBuf};

pub(crate) fn workflow_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows")
}

pub(crate) fn workflows() -> Vec<(String, Value)> {
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
///
/// Case-INSENSITIVE: GitHub accepts `macOS-latest` and `Ubuntu-latest`, and a
/// case-sensitive prefix check would let either through. Matched by prefix
/// rather than against a fixed list because GitHub adds images (`ubuntu-26.04`,
/// `macos-27`) faster than a list here would be updated, and a list that is
/// behind fails open.
pub(crate) fn hosted_label(v: &str) -> bool {
    let v = v.trim().to_ascii_lowercase();
    v.starts_with("ubuntu-") || v.starts_with("macos-") || v.starts_with("windows-")
}

/// Append every string in `v` — a bare label, a list like
/// `[self-hosted, clean-room]`, or the OBJECT form GitHub also accepts:
/// `runs-on: { group: <g>, labels: [ubuntu-latest] }`. The object form is the
/// one a hosted runner can hide in, because it is rare enough that a reader
/// scanning for `runs-on: ubuntu-latest` will not see it.
pub(crate) fn push_labels(out: &mut Vec<String>, v: &Value) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Sequence(seq) => {
            for item in seq {
                push_labels(out, item);
            }
        }
        Value::Mapping(map) => {
            // `labels` ONLY. `group` names a runner GROUP, not a runner: a
            // group called `ubuntu-runners` is not a hosted label, and reading
            // it made the fleet control below report a hosted runner that was
            // not there. The first draft of this arm read both.
            if let Some(inner) = map.get(Value::String("labels".to_string())) {
                push_labels(out, inner);
            }
        }
        _ => {}
    }
}

/// The labels a `runs-on: ${{ matrix.<key> }}` expression can take, read out of
/// the matrix itself: the `include:` legs first, then any top-level list.
pub(crate) fn matrix_labels(job: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let Some(matrix) = job.get("strategy").and_then(|s| s.get("matrix")) else {
        return out;
    };
    // EVERY key, not just `runner` and `os`. `runs-on: ${{ matrix.machine }}`
    // is as valid as `matrix.os`, and a scan that hardcoded two names would
    // pass over it. Reading them all can only over-collect, and over-collecting
    // a label that is not a runner is harmless: it is only ever compared
    // against the hosted-label shape.
    if let Some(Value::Sequence(include)) = matrix.get("include") {
        for leg in include {
            if let Value::Mapping(leg) = leg {
                for (_, v) in leg {
                    push_labels(&mut out, v);
                }
            }
        }
    }
    if let Value::Mapping(matrix) = matrix {
        for (key, v) in matrix {
            if key.as_str() == Some("include") || key.as_str() == Some("exclude") {
                continue;
            }
            push_labels(&mut out, v);
        }
    }
    out
}

/// Every literal runner label a job can end up on: its own `runs-on` when that
/// is a literal, and — when `runs-on` is a `${{ matrix.* }}` expression — the
/// values that expression can take.
pub(crate) fn runner_labels(job: &Value) -> Vec<String> {
    let Some(runs_on) = job.get("runs-on") else {
        return Vec::new();
    };
    // An expression anywhere in `runs-on` -- bare, or inside a list like
    // `runs-on: [self-hosted, "${{ matrix.pool }}"]` -- means the matrix is
    // also a source of labels. Both are read: the literals `runs-on` names AND
    // everything the matrix can substitute, because either can be hosted.
    let mut out = Vec::new();
    push_labels(&mut out, runs_on);
    if out.iter().any(|l| l.contains("${{")) {
        out.retain(|l| !l.contains("${{"));
        out.extend(matrix_labels(job));
    }
    out
}

/// Every `(workflow, job, label)` in the repository that names a hosted runner.
pub(crate) fn hosted_sites() -> Vec<(String, String, String)> {
    let mut sites = Vec::new();
    for (file, doc) in workflows() {
        for (job, label) in hosted_in(&doc) {
            sites.push((file.clone(), job, label));
        }
    }
    sites.sort();
    sites
}

/// Every `(job, hosted label)` in one parsed workflow. Split out of
/// [`hosted_sites`] so the controls below can drive it with a fixture: a claim
/// that some shape "would hide a hosted runner" is worth exactly as much as the
/// fixture that shows it does not.
pub(crate) fn hosted_in(doc: &Value) -> Vec<(String, String)> {
    let Some(Value::Mapping(jobs)) = doc.get("jobs") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (name, job) in jobs {
        let name = name.as_str().unwrap_or("<non-string job name>").to_string();
        for label in runner_labels(job) {
            if hosted_label(&label) {
                out.push((name.clone(), label));
            }
        }
    }
    out
}

/// Parse a workflow fixture, or fail naming it.
pub(crate) fn fixture(yaml: &str) -> Value {
    serde_yaml_ng::from_str(yaml).expect("the fixture must parse")
}

/// Does any step of this job invoke cargo — directly, or through an action
/// whose name says so? Read from `run:` and `uses:` together, because
/// `cargo-llvm-cov` arrives as a `uses:` and compiles just the same.
pub(crate) fn job_runs_cargo(job: &Value) -> bool {
    let Some(Value::Sequence(steps)) = job.get("steps") else {
        return false;
    };
    steps.iter().any(|step| {
        ["run", "uses"].iter().any(|k| {
            step.get(*k)
                .and_then(Value::as_str)
                .is_some_and(|v| v.contains("cargo"))
        })
    })
}
