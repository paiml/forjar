//! forjar#629: the nightly must rebuild whenever the published `nightly` tag
//! is not HEAD — not whenever "a commit landed in the last 24 hours".
//!
//! # The defect
//!
//! `nightly.yml`'s `check-activity` job gated build and release on
//! `git log --since="24 hours ago"`. A commit that lands after the day's 04:00
//! run and is followed by a quiet day is never built, and a nightly whose
//! build failed is never retried: every later scheduled run is green with
//! build and release skipped. Measured by infra PMAT-1055: forjar's nightly
//! sat 12 days behind main, copia's 33.
//!
//! # Why this EXECUTES the gate instead of reading it
//!
//! The gate is a shell script, and what matters is the verdict it writes to
//! `$GITHUB_OUTPUT` on a given history. The test lifts the parsed
//! `steps[id=check].run` out of the workflow, builds a synthetic repository
//! whose commits are all 30 days old (a quiet day, forever), and runs the
//! script there under the shell GitHub uses for `run:` steps. A comment in the
//! workflow cannot satisfy it; only the script's behaviour can.
//!
//! # The second half: where the tag lands
//!
//! A tag-vs-HEAD gate is only honest if the release job tags the commit the
//! run BUILT. `softprops/action-gh-release` with no `target_commitish` asks
//! GitHub to create the tag at the default branch's tip at API-call time, so
//! a commit landing mid-build would read as built and never be built. That is
//! a parsed-field assertion on the release step.
//!
//! # mutations — each measured to redden exactly its own test
//!
//! 1. Restore the 24h gate (`RECENT=$(git log --oneline --since="24 hours
//!    ago" | wc -l)` / `[ "$RECENT" -gt 0 ]`) →
//!    `a_nightly_behind_head_on_a_quiet_day_is_rebuilt` goes RED (the old gate
//!    answers `false` for history with nothing in the last 24h), and so does
//!    `a_repo_with_no_nightly_tag_builds`.
//! 2. Make the gate unconditional (`has_commits=true` always) →
//!    `a_nightly_at_head_is_not_rebuilt` goes RED: a gate that always builds
//!    detects nothing.
//! 3. Delete `target_commitish` from the release step →
//!    `the_nightly_tag_is_pinned_to_the_built_commit` goes RED.

use serde_yaml_ng::Value;
use std::path::Path;
use std::process::Command;

fn nightly() -> Value {
    let path = format!(
        "{}/.github/workflows/nightly.yml",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_yaml_ng::from_str(&text).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

fn steps<'a>(wf: &'a Value, job: &str) -> &'a [Value] {
    wf.get("jobs")
        .and_then(|j| j.get(job))
        .and_then(|j| j.get("steps"))
        .and_then(Value::as_sequence)
        .unwrap_or_else(|| panic!("nightly.yml has no jobs.{job}.steps"))
}

/// The gate script, with the one expression GitHub would have substituted.
fn gate_script(event: &str) -> String {
    let wf = nightly();
    let step = steps(&wf, "check-activity")
        .iter()
        .find(|s| s.get("id").and_then(Value::as_str) == Some("check"))
        .expect("check-activity has no step with id: check (build's `if:` reads it)");
    let run = step
        .get("run")
        .and_then(Value::as_str)
        .expect("check step has no run:");
    run.replace("${{ github.event_name }}", event)
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(["-c", "tag.gpgsign=false", "-c", "core.hooksPath=/dev/null"])
        .args(args)
        // Every commit is 30 days old: nothing is "in the last 24 hours".
        .env("GIT_AUTHOR_DATE", "@1000000000 +0000")
        .env("GIT_COMMITTER_DATE", "@1000000000 +0000")
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A repo with two quiet commits; `tag_at` is where `nightly` points, if anywhere.
fn repo(tag_at: Option<&str>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = dir.path();
    git(p, &["init", "-q", "-b", "main"]);
    git(p, &["commit", "-q", "--allow-empty", "-m", "first"]);
    git(p, &["commit", "-q", "--allow-empty", "-m", "second"]);
    if let Some(rev) = tag_at {
        // Annotated, as a GitHub release tag may be: the gate must peel it.
        git(p, &["tag", "-a", "-m", "nightly", "nightly", rev]);
    }
    dir
}

/// Run the gate in `dir` the way GitHub runs a `run:` step; return has_commits.
fn verdict(dir: &Path, event: &str) -> String {
    let out_file = dir.join("github_output");
    std::fs::write(&out_file, "").expect("create GITHUB_OUTPUT");
    let out = Command::new("bash")
        .args([
            "--noprofile",
            "--norc",
            "-eo",
            "pipefail",
            "-c",
            &gate_script(event),
        ])
        .current_dir(dir)
        .env("GITHUB_OUTPUT", &out_file)
        .output()
        .expect("spawn bash");
    assert!(
        out.status.success(),
        "gate script failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let written = std::fs::read_to_string(&out_file).expect("read GITHUB_OUTPUT");
    let values: Vec<&str> = written
        .lines()
        .filter_map(|l| l.strip_prefix("has_commits="))
        .collect();
    assert_eq!(
        values.len(),
        1,
        "gate must write has_commits exactly once, wrote: {written:?}"
    );
    values[0].to_string()
}

#[test]
fn a_nightly_behind_head_on_a_quiet_day_is_rebuilt() {
    let dir = repo(Some("HEAD~1"));
    assert_eq!(
        verdict(dir.path(), "schedule"),
        "true",
        "tag behind HEAD must build"
    );
}

#[test]
fn a_nightly_at_head_is_not_rebuilt() {
    let dir = repo(Some("HEAD"));
    assert_eq!(
        verdict(dir.path(), "schedule"),
        "false",
        "tag at HEAD must skip"
    );
    // ...unless someone asked for it by hand.
    assert_eq!(
        verdict(dir.path(), "workflow_dispatch"),
        "true",
        "a manual run must build"
    );
}

#[test]
fn a_repo_with_no_nightly_tag_builds() {
    let dir = repo(None);
    assert_eq!(
        verdict(dir.path(), "schedule"),
        "true",
        "no nightly yet must build"
    );
}

#[test]
fn the_nightly_tag_is_pinned_to_the_built_commit() {
    let wf = nightly();
    let publish: Vec<&Value> = steps(&wf, "release")
        .iter()
        .filter(|s| {
            s.get("uses")
                .and_then(Value::as_str)
                .is_some_and(|u| u.starts_with("softprops/action-gh-release@"))
        })
        .collect();
    assert_eq!(
        publish.len(),
        1,
        "expected exactly one release-publishing step"
    );
    let with = publish[0].get("with").expect("release step has no with:");
    assert_eq!(
        with.get("tag_name").and_then(Value::as_str),
        Some("nightly")
    );
    assert_eq!(
        with.get("target_commitish").and_then(Value::as_str),
        Some("${{ github.sha }}"),
        "unset, GitHub tags the branch tip at API time and a mid-build commit reads as built"
    );
    // The checkout the gate compares against must carry tags.
    let checkout = steps(&wf, "check-activity")
        .iter()
        .find(|s| {
            s.get("uses")
                .and_then(Value::as_str)
                .is_some_and(|u| u.starts_with("actions/checkout@"))
        })
        .expect("check-activity has no checkout");
    assert_eq!(
        checkout
            .get("with")
            .and_then(|w| w.get("fetch-depth"))
            .and_then(Value::as_i64),
        Some(0),
        "without fetch-depth: 0 the nightly tag is absent and every run rebuilds"
    );
}
