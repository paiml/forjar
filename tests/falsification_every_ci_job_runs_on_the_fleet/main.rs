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

mod arch;
mod controls;
mod scan;

use scan::{hosted_sites, job_runs_cargo, linux_label, runner_labels, workflows};
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

/// A fleet job that runs cargo keeps its registry to itself.
///
/// infra#430. Sixteen clean-room runners share one `~/.cargo`, and an hourly
/// reaper deletes `registry/src` entries by mtime — under live builds. rustc
/// then reads a source file that has just vanished:
///
/// ```text
/// error: couldn't read .../registry/src/.../bashrs-6.68.0/src/formal/…rs:
///        No such file or directory (os error 2)
/// error: could not parse/generate dep info at: .../deps/tower-….d
/// ```
///
/// MEASURED on this branch: `msrv` and `coverage` both died that way the first
/// time PMAT-547 moved them off GitHub-hosted runners — where every job had a
/// private `~/.cargo` by construction, so the exposure arrived WITH the move to
/// the fleet. `proofs.yml:ledger-replay` had already hit it and already carried
/// the fix; this makes the fix the rule rather than one job's footnote.
///
/// The host-side fix is paiml/infra's. This is the repository-side one.
#[test]
fn a_fleet_job_that_runs_cargo_keeps_its_registry_to_itself() {
    let mut exposed = Vec::new();
    for (file, doc) in workflows() {
        let Some(Value::Mapping(jobs)) = doc.get("jobs") else {
            continue;
        };
        for (name, job) in jobs {
            let labels = runner_labels(job);
            if !labels.iter().any(|l| l == "self-hosted") {
                continue;
            }
            if !job_runs_cargo(job) {
                continue;
            }
            let has_home = job.get("env").and_then(|e| e.get("CARGO_HOME")).is_some();
            if !has_home {
                exposed.push(format!("  {file}  job `{}`", name.as_str().unwrap_or("?")));
            }
        }
    }
    exposed.sort();
    assert!(
        exposed.is_empty(),
        "these fleet jobs run cargo against the SHARED ~/.cargo that infra#430's \
         reaper sweeps under live builds:\n{}\n\nGive each one\n  env:\n    \
         CARGO_HOME: ${{{{ github.workspace }}}}/../cargo-home-${{{{ github.job }}}}\n\
         The path sits outside the workspace, so it survives the checkout clean \
         and is a cache rather than a cold download.",
        exposed.join("\n")
    );
}

/// Self-hosted jobs that DELIBERATELY name no architecture (PMAT-756).
///
/// Operator ruling: drop `X64` from jobs that do not depend on the chip, so the
/// gx10 ARM64 boxes can run them too. Each entry is `(workflow file, job id,
/// reason)`. Every job here is chip-neutral: it lints, classifies, audits or
/// tests without producing an architecture-specific artifact. Jobs that build
/// or upload a release/artifact binary keep `X64` and are NOT listed.
///
/// The list is two-sided (see the test below): a listed job must exist and must
/// carry no arch label, and an unlisted job without one fails.
const ARCH_NEUTRAL: &[(&str, &str, &str)] = &[
    (
        "audit.yml",
        "audit",
        "dependency/advisory audit, no artifact",
    ),
    (
        "behavior.yml",
        "behavior",
        "behavior spec checks, no artifact",
    ),
    (
        "bench.yml",
        "classify",
        "path classifier, emits only job outputs",
    ),
    (
        "ci.yml",
        "classify",
        "path classifier, emits only job outputs",
    ),
    ("ci.yml", "lockfile", "Cargo.lock consistency check"),
    (
        "ci.yml",
        "examples-validate",
        "validates example configs, chip-independent",
    ),
    (
        "ci.yml",
        "no-default-features",
        "feature-matrix compile check, no artifact",
    ),
    ("ci.yml", "doctests", "doctests run on any chip"),
    (
        "ci.yml",
        "dogfood-surface",
        "dogfood surface checks, no artifact",
    ),
    (
        "ci.yml",
        "dogfood-guards",
        "shell/forjar guards, chip-independent",
    ),
    (
        "ci.yml",
        "gate",
        "aggregates needs.*.result, builds nothing",
    ),
    (
        "convergence.yml",
        "convergence",
        "convergence checks, no artifact",
    ),
    (
        "lint.yml",
        "clippy",
        "lint on any chip; macOS legs are hosted",
    ),
    (
        "lint.yml",
        "file-health",
        "file-size/health scan, chip-independent",
    ),
    ("msrv.yml", "msrv", "MSRV compile check, no artifact"),
    (
        "nightly.yml",
        "check-activity",
        "git activity probe, emits only outputs",
    ),
    (
        "proofs.yml",
        "classify",
        "path classifier, emits only job outputs",
    ),
    ("proofs.yml", "proofs", "proof replay, chip-independent"),
    (
        "proofs.yml",
        "ledger-replay",
        "ledger replay over its own build, chip-independent",
    ),
    ("quorum.yml", "receipt", "quorum receipt verification"),
    (
        "release-audit.yml",
        "audit",
        "release metadata audit, no artifact",
    ),
];

/// A fleet job names the ARCHITECTURE it needs, or is listed in
/// [`ARCH_NEUTRAL`] as chip-independent on purpose.
///
/// MEASURED 2026-09-12, `gh api orgs/paiml/actions/runner-groups`: the label
/// `clean-room` spans BOTH architectures. Group 1 "Default" holds sixteen
/// `intel-clean-room-*`, every one `X64`; group 3 "gpu-nodes" holds four gx10
/// boxes that also carry `clean-room` and are every one **ARM64**.
///
/// A job that produces an architecture-specific artifact must pin `X64` or
/// `ARM64`, or a runner-group checkbox could hand it the wrong chip and the
/// uploaded artifact would be wrong rather than absent. A chip-neutral job may
/// instead be listed in `ARCH_NEUTRAL` so gx10 can run it. The silence has to
/// be written down: a job that merely LOSES its arch label is still caught.
#[test]
fn a_fleet_job_names_the_architecture_it_needs() {
    let neutral: Vec<(&str, &str)> = ARCH_NEUTRAL.iter().map(|(f, j, _)| (*f, *j)).collect();
    let docs = workflows();
    // Judged per matrix LEG (forjar#772): one leg's `X64` no longer vouches for
    // a sibling leg that names nothing.
    let mut unpinned: Vec<String> = docs
        .iter()
        .flat_map(|(file, doc)| arch::unpinned_in(file, doc, &neutral))
        .collect();
    unpinned.sort();
    assert!(
        unpinned.is_empty(),
        "these fleet jobs name no architecture and are not in ARCH_NEUTRAL, and \
         `clean-room` spans X64 and ARM64:\n{}\n\nAdd `X64` (or `ARM64`, \
         deliberately), or, if the job is chip-independent, add it to \
         ARCH_NEUTRAL with a reason.",
        unpinned.join("\n")
    );

    // The list cannot go stale: every entry exists, is on the fleet, and
    // carries no arch label.
    let stale = arch::stale_in(&docs, &neutral);
    assert!(
        stale.is_empty(),
        "ARCH_NEUTRAL has stale entries (a listed job must exist, run on a \
         self-hosted runner, and NOT name an architecture):\n{}",
        stale.join("\n")
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
        .filter(|(_, _, label)| linux_label(label))
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
        if linux_label(&label) {
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
