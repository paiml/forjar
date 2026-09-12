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
//! `runner=GitHub Actions <n>` is a GitHub-hosted runner. Thirty-three runner
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
//! here, and the pin is a count rather than a prohibition, because the seven
//! legs the fleet CANNOT serve have to stay visible instead of being argued
//! about once and forgotten.
//!
//! SEVEN, not six. `grep` prints six lines because `lint.yml` contributes one
//! line and two legs; the map below was right and the prose around it was
//! wrong, in the receipt, the log, the commit message and this comment, until
//! the legs were counted from the parsed YAML.
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
///
/// Case-INSENSITIVE: GitHub accepts `macOS-latest` and `Ubuntu-latest`, and a
/// case-sensitive prefix check would let either through. Matched by prefix
/// rather than against a fixed list because GitHub adds images (`ubuntu-26.04`,
/// `macos-27`) faster than a list here would be updated, and a list that is
/// behind fails open.
fn hosted_label(v: &str) -> bool {
    let v = v.trim().to_ascii_lowercase();
    v.starts_with("ubuntu-") || v.starts_with("macos-") || v.starts_with("windows-")
}

/// Append every string in `v` — a bare label, a list like
/// `[self-hosted, clean-room]`, or the OBJECT form GitHub also accepts:
/// `runs-on: { group: <g>, labels: [ubuntu-latest] }`. The object form is the
/// one a hosted runner can hide in, because it is rare enough that a reader
/// scanning for `runs-on: ubuntu-latest` will not see it.
fn push_labels(out: &mut Vec<String>, v: &Value) {
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
fn matrix_labels(job: &Value) -> Vec<String> {
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
fn runner_labels(job: &Value) -> Vec<String> {
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
fn hosted_sites() -> Vec<(String, String, String)> {
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
fn hosted_in(doc: &Value) -> Vec<(String, String)> {
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
fn fixture(yaml: &str) -> Value {
    serde_yaml_ng::from_str(yaml).expect("the fixture must parse")
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
    // MEASURED at 48 on this branch. A floor of 20 left 28 labels of slack, so
    // more than half the fleet jobs could have been deleted before this noticed;
    // 40 keeps the guard honest while leaving room to retire a workflow.
    assert!(
        fleet >= 40,
        "the parser found only {fleet} `self-hosted` labels; it is not reading \
         runners and every other case in this file would pass over anything"
    );
}

/// A job that calls a REUSABLE workflow declares no `runs-on` of its own, so
/// every case in this file passes over it — the runner is chosen by a file in
/// another repository that this test cannot read.
///
/// Found by a review lane, which is the only reason it is here: the parser was
/// silently skipping these and nothing said so. They cannot be checked from
/// here, so they are COUNTED, exactly like the macOS legs. Two of these three
/// were measured on run 34685410794 landing on `intel-clean-room-*`
/// (`ci / lint`, `ci / coverage`, `ci / test` all reported
/// `labels=self-hosted,clean-room`), which is evidence about
/// `sovereign-ci.yml` and not a guarantee about its future.
#[test]
fn the_jobs_that_delegate_their_runner_are_exactly_these() {
    let mut delegated = Vec::new();
    for (file, doc) in workflows() {
        let Some(Value::Mapping(jobs)) = doc.get("jobs") else {
            continue;
        };
        for (name, job) in jobs {
            if job.get("runs-on").is_some() {
                continue;
            }
            let Some(uses) = job.get("uses").and_then(Value::as_str) else {
                continue;
            };
            let name = name.as_str().unwrap_or("?");
            delegated.push(format!("{file}:{name} -> {uses}"));
        }
    }
    delegated.sort();

    let expected = vec![
        "ci.yml:ci -> paiml/.github/.github/workflows/sovereign-ci.yml@main".to_string(),
        "nightly-bench.yml:bench -> paiml/.github/.github/workflows/sovereign-ci.yml@main"
            .to_string(),
        "pr-gate.yml:authorize -> paiml/.github/.github/workflows/pr-gate.yml@main".to_string(),
    ];
    assert_eq!(
        delegated, expected,
        "the set of jobs whose runner is chosen by another repository changed. \
         No case in this file can see where those jobs run, so each one is a \
         hosted runner this repository cannot rule out. Adding one is a \
         decision; removing one should be written down."
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

// ---------------------------------------------------------------------------
// Controls. Every one of these is a shape a review lane claimed could hide a
// hosted runner from this file's parser. Each is a fixture rather than an
// argument, because the four cases above are only worth what the parser under
// them is worth, and "I considered that shape" is not a measurement.
// ---------------------------------------------------------------------------

/// `runs-on` also takes an OBJECT: `{ group: …, labels: [ubuntu-latest] }`.
///
/// This is the shape that hides best. A reader scanning for
/// `runs-on: ubuntu-latest` does not see it, and neither did the first draft of
/// `push_labels`, which matched only `String` and `Sequence`.
#[test]
fn the_object_form_of_runs_on_cannot_hide_a_hosted_runner() {
    let doc = fixture(
        r#"
jobs:
  build:
    runs-on:
      group: ubuntu-runners
      labels: [ubuntu-latest]
"#,
    );
    assert_eq!(
        hosted_in(&doc),
        vec![("build".to_string(), "ubuntu-latest".to_string())],
        "the object form of `runs-on` hid a hosted runner from the parser"
    );
}

/// GitHub accepts `macOS-latest` and `Ubuntu-Latest`. A case-sensitive prefix
/// check lets both through while looking correct.
#[test]
fn a_hosted_label_in_another_case_is_still_a_hosted_label() {
    let doc = fixture(
        r#"
jobs:
  a:
    runs-on: macOS-Latest
  b:
    runs-on: Ubuntu-Latest
"#,
    );
    let found: Vec<String> = hosted_in(&doc).into_iter().map(|(_, l)| l).collect();
    assert_eq!(
        found,
        vec!["macOS-Latest".to_string(), "Ubuntu-Latest".to_string()],
        "a hosted label spelled in another case was not recognised"
    );
}

/// The matrix key need not be called `runner` or `os`.
#[test]
fn a_matrix_key_by_any_name_is_still_read() {
    let doc = fixture(
        r#"
jobs:
  build:
    strategy:
      matrix:
        machine: [ubuntu-latest, self-hosted]
    runs-on: ${{ matrix.machine }}
"#,
    );
    let found: Vec<String> = hosted_in(&doc).into_iter().map(|(_, l)| l).collect();
    assert_eq!(
        found,
        vec!["ubuntu-latest".to_string()],
        "a matrix key not named `runner` or `os` hid a hosted runner"
    );
}

/// An expression nested inside a `runs-on` LIST still reaches the matrix.
#[test]
fn an_expression_inside_a_runs_on_list_still_reaches_the_matrix() {
    let doc = fixture(
        r#"
jobs:
  build:
    strategy:
      matrix:
        pool: [macos-latest]
    runs-on: [self-hosted, "${{ matrix.pool }}"]
"#,
    );
    let found: Vec<String> = hosted_in(&doc).into_iter().map(|(_, l)| l).collect();
    assert_eq!(
        found,
        vec!["macos-latest".to_string()],
        "an expression inside a runs-on list did not reach the matrix"
    );
}

/// And the controls do not fire on a job that is genuinely on the fleet — a
/// parser that called everything hosted would pass every case above.
#[test]
fn a_fleet_job_is_not_mistaken_for_a_hosted_one() {
    let doc = fixture(
        r#"
jobs:
  a:
    runs-on: [self-hosted, clean-room]
  b:
    runs-on:
      group: fleet
      labels: [self-hosted, build]
"#,
    );
    assert!(
        hosted_in(&doc).is_empty(),
        "a fleet job was reported as hosted: {:?}",
        hosted_in(&doc)
    );
}
