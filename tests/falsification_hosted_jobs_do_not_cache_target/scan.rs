//! The scanner behind `falsification_hosted_jobs_do_not_cache_target`.
//!
//! Kept apart from the assertions so the discrimination controls can drive
//! `scan_workflow` with synthetic workflows, and so no one file here carries
//! both the rule and its own proof that the rule can fail.

use serde_yaml_ng::Value;
use std::path::{Path, PathBuf};

/// A cache step that this guard rejects.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Violation {
    pub(crate) workflow: String,
    pub(crate) job: String,
    pub(crate) runs_on: String,
    pub(crate) path: String,
}

pub(crate) fn workflow_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows")
}

/// Every `.yml`/`.yaml` under `.github/workflows`, sorted for a stable report.
pub(crate) fn workflow_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(workflow_dir())
        .expect(".github/workflows must be readable")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("yml") | Some("yaml")
            )
        })
        .collect();
    files.sort();
    files
}

/// The `runs-on` of a job, flattened to one string for reporting.
///
/// `runs-on` is a string (`ubuntu-latest`), a sequence
/// (`[self-hosted, clean-room]`), or a `group:`/`labels:` mapping. A job with
/// `uses:` (a reusable-workflow call) has no `runs-on` of its own.
pub(crate) fn runs_on(job: &Value) -> Option<String> {
    match job.get("runs-on")? {
        Value::String(s) => Some(s.clone()),
        Value::Sequence(items) => Some(
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", "),
        ),
        Value::Mapping(m) => m
            .get(Value::String("labels".into()))
            .map(|l| match l {
                Value::String(s) => s.clone(),
                Value::Sequence(items) => items
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", "),
                other => format!("{other:?}"),
            })
            .or_else(|| Some("group".to_string())),
        _ => None,
    }
}

/// Whether a `runs-on` selects a runner GitHub owns, i.e. one whose disk we do
/// not control and cannot enlarge.
///
/// Anything naming `self-hosted` is ours. So is anything selecting a runner
/// group, which on this fleet only ever resolves to self-hosted hardware.
pub(crate) fn is_github_hosted(runs_on: &str) -> bool {
    let lowered = runs_on.to_ascii_lowercase();
    !lowered.contains("self-hosted") && !lowered.contains("group")
}

/// Whether one cached path names a Rust build directory.
///
/// Compares whole path COMPONENTS, so `~/.cargo/registry` is not a hit while
/// `target`, `./target`, `**/target` and `target/llvm-cov-target` all are.
pub(crate) fn is_rust_build_dir(path: &str) -> bool {
    path.split('/')
        .map(str::trim)
        .any(|c| c == "target" || c == "llvm-cov-target")
}

/// Whether a step invokes `Swatinem/rust-cache`, which caches the Rust BUILD
/// directory rather than a path it is told to.
///
/// Matched separately from `actions/cache` because it has no `with.path` at all:
/// what it saves is implicit, so the scan has to name `target` on its behalf or
/// the step contributes a zero denominator and is silently exempt.
pub(crate) fn is_rust_cache_action(step: &Value) -> bool {
    step.get("uses")
        .and_then(Value::as_str)
        .is_some_and(|u| u.to_ascii_lowercase().starts_with("swatinem/rust-cache"))
}

/// Whether a step invokes a cache action of any kind this repo actually uses.
///
/// `actions/cache*` was the whole matcher until #400's sweep: it is blind to
/// `Swatinem/rust-cache`, which four hosted jobs here already use
/// (mutation.yml, quorum.yml, convergence.yml, behavior.yml). Migrating
/// `coverage.yml` to the house idiom is a one-line edit that reproduces #386 —
/// rust-cache keeps `target/debug/deps`, the 66 of 70.70 GiB that is the defect
/// — and the narrow matcher would have watched it happen.
pub(crate) fn is_cache_action(step: &Value) -> bool {
    let matches_actions_cache = step
        .get("uses")
        .and_then(Value::as_str)
        .is_some_and(|u| u.starts_with("actions/cache"));
    matches_actions_cache || is_rust_cache_action(step)
}

/// The `with.path` of a cache step, one entry per line of the block scalar.
///
/// A rust-cache step declares no path; it caches the build directory by design,
/// so an absent `with.path` there means `target`, not "nothing".
pub(crate) fn cached_paths(step: &Value) -> Vec<String> {
    let declared: Vec<String> = step
        .get("with")
        .and_then(|w| w.get("path"))
        .and_then(Value::as_str)
        .map(|p| {
            p.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if declared.is_empty() && is_rust_cache_action(step) {
        return vec!["target".to_string()];
    }
    declared
}

/// Whether a job compiles with LLVM source-based coverage instrumentation.
///
/// This is the property that separates a ~0.8 GiB build tree from a 70.70 GiB
/// one — see the module docs. `cargo llvm-cov` is how this repo asks for it;
/// setting `-C instrument-coverage` by hand is the other way, and counts too.
pub(crate) fn is_instrumented_coverage_job(job: &Value) -> bool {
    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let in_a_run = steps
        .iter()
        .filter_map(|s| s.get("run"))
        .filter_map(Value::as_str)
        .any(|r| r.contains("llvm-cov") || r.contains("instrument-coverage"));
    let in_the_env = job
        .get("env")
        .and_then(Value::as_mapping)
        .is_some_and(|env| {
            env.values()
                .filter_map(Value::as_str)
                .any(|v| v.contains("instrument-coverage"))
        });
    in_a_run || in_the_env
}

/// A `debug` setting that keeps the instrumented tree small enough to build.
pub(crate) fn is_reduced_debug_setting(raw: &str) -> bool {
    matches!(
        raw.trim().trim_matches('"').to_ascii_lowercase().as_str(),
        "line-tables-only" | "none" | "0" | "false"
    )
}

/// Whether any `debuginfo=<x>` in this text asks for reduced debug info.
pub(crate) fn mentions_reduced_debuginfo(text: &str) -> bool {
    text.split("debuginfo=").skip(1).any(|rest| {
        let token: String = rest
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
            .collect();
        is_reduced_debug_setting(&token)
    })
}

/// Whether a job compiles its instrumented tree WITHOUT full DWARF.
///
/// This is the half of #388 that actually saved the lane, and until now nothing
/// pinned it. MEASURED: the same `cargo llvm-cov` run over this crate produced
/// **70.70 GiB in 19,070 files** with full debug info and **23 GiB** with
/// `CARGO_PROFILE_DEV_DEBUG: line-tables-only` +
/// `CARGO_PROFILE_TEST_DEBUG: line-tables-only` (run 33598453499, `23G
/// target/llvm-cov-target`, `/dev/root 145G 83G 63G 57% /`).
///
/// `MIN_FREE_GIB: 40` cannot stand in for it: that is a PRE-build floor, so on
/// the measured runner (84 GiB free) it passes, and a full-DWARF build then
/// leaves ~13 GiB — the exact pre-#388 arithmetic that killed the runner's
/// Worker mid-build and took the job's logs with it.
pub(crate) fn job_reduces_debug_info(job: &Value) -> bool {
    let env = job.get("env");
    let profile_knob = env
        .and_then(|e| e.get("CARGO_PROFILE_DEV_DEBUG"))
        .and_then(scalar_to_string)
        .is_some_and(|v| is_reduced_debug_setting(&v));
    let flags = env.and_then(Value::as_mapping).is_some_and(|m| {
        m.values()
            .filter_map(Value::as_str)
            .any(mentions_reduced_debuginfo)
    });
    let in_a_run = job
        .get("steps")
        .and_then(Value::as_sequence)
        .is_some_and(|steps| {
            steps
                .iter()
                .filter_map(|s| s.get("run"))
                .filter_map(Value::as_str)
                .any(mentions_reduced_debuginfo)
        });
    profile_knob || flags || in_a_run
}

/// A YAML scalar as a string. `debug: 0` parses as a number, `debug: "0"` as a
/// string, and both mean the same thing to cargo.
pub(crate) fn scalar_to_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// What the scan looked at, so a pass can state its denominator.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Scan {
    pub(crate) workflows: usize,
    pub(crate) jobs: usize,
    pub(crate) hosted_jobs: usize,
    pub(crate) hosted_coverage_jobs: usize,
    pub(crate) cache_steps: usize,
    pub(crate) cached_paths: usize,
    pub(crate) violations: Vec<Violation>,
    /// `workflow.yml:job` for every hosted instrumented-coverage job that builds
    /// with full debug info.
    pub(crate) full_dwarf_jobs: Vec<String>,
}

/// Scan one already-parsed workflow. Kept separate from the filesystem so the
/// discrimination controls below can drive it with synthetic inputs.
pub(crate) fn scan_workflow(name: &str, doc: &Value, scan: &mut Scan) {
    scan.workflows += 1;
    let Some(jobs) = doc.get("jobs").and_then(Value::as_mapping) else {
        return;
    };
    for (job_id, job) in jobs {
        scan.jobs += 1;
        let job_id = job_id.as_str().unwrap_or("<non-string job id>");
        let Some(on) = runs_on(job) else { continue };
        if !is_github_hosted(&on) {
            continue;
        }
        scan.hosted_jobs += 1;
        if !is_instrumented_coverage_job(job) {
            continue;
        }
        scan.hosted_coverage_jobs += 1;
        if !job_reduces_debug_info(job) {
            scan.full_dwarf_jobs.push(format!("{name}:{job_id}"));
        }
        let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
            continue;
        };
        for step in steps.iter().filter(|s| is_cache_action(s)) {
            scan.cache_steps += 1;
            for path in cached_paths(step) {
                scan.cached_paths += 1;
                if is_rust_build_dir(&path) {
                    scan.violations.push(Violation {
                        workflow: name.to_string(),
                        job: job_id.to_string(),
                        runs_on: on.clone(),
                        path,
                    });
                }
            }
        }
    }
}

/// Scan every workflow this repository ships.
pub(crate) fn scan_repo() -> Scan {
    let mut scan = Scan::default();
    for file in workflow_files() {
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unnamed>")
            .to_string();
        let text = std::fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("{name} must be readable: {e}"));
        let doc: Value = serde_yaml_ng::from_str(&text)
            .unwrap_or_else(|e| panic!("{name} must parse as YAML: {e}"));
        scan_workflow(&name, &doc, &mut scan);
    }
    scan
}
