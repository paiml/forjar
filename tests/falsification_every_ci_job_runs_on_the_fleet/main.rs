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

mod controls;
mod scan;

use scan::{hosted_sites, runner_labels, workflows};
use serde_yaml_ng::Value;
use std::collections::BTreeMap;

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

/// A fleet job names the ARCHITECTURE it needs.
///
/// MEASURED 2026-09-12, `gh api orgs/paiml/actions/runner-groups`: the label
/// `clean-room` spans BOTH architectures. Group 1 "Default" holds sixteen
/// `intel-clean-room-*`, every one `X64`; group 3 "gpu-nodes" holds four gx10
/// boxes that also carry `clean-room` and are every one **ARM64**.
///
/// Groups 3 and 5 are `visibility=selected` and forjar is not in them today, so
/// `[self-hosted, clean-room]` reaches only X64 boxes and nothing is currently
/// wrong. That is an access-control accident, not a property of the label: the
/// day forjar is added to `gpu-nodes` — one checkbox — a job building
/// `x86_64-unknown-linux-gnu` could be handed an ARM64 runner, and the artifact
/// it uploads would be wrong rather than absent.
///
/// Every runner forjar can reach is `X64`, so naming it costs no capacity and
/// states the assumption instead of relying on someone not clicking the box.
#[test]
fn a_fleet_job_names_the_architecture_it_needs() {
    let arches = ["X64", "ARM64"];
    let mut unpinned = Vec::new();
    for (file, doc) in workflows() {
        let Some(Value::Mapping(jobs)) = doc.get("jobs") else {
            continue;
        };
        for (name, job) in jobs {
            let labels = runner_labels(job);
            if !labels.iter().any(|l| l == "self-hosted") {
                continue;
            }
            if labels.iter().any(|l| arches.contains(&l.as_str())) {
                continue;
            }
            unpinned.push(format!(
                "  {file}  job `{}`  runs-on {labels:?}",
                name.as_str().unwrap_or("?")
            ));
        }
    }
    unpinned.sort();
    assert!(
        unpinned.is_empty(),
        "these fleet jobs name no architecture, and `clean-room` spans X64 and \
         ARM64:\n{}\n\nAdd `X64` (or `ARM64`, deliberately). Every runner this \
         repository can reach today is X64, so this costs nothing and removes \
         the chance that a checkbox on a runner group turns an x86_64 release \
         binary into an ARM one.",
        unpinned.join("\n")
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
