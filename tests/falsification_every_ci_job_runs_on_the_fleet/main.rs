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
//! Since then the org gained one macOS runner, `mini-m4` (labels `macOS`,
//! `ARM64`, `mini`). every darwin leg of lint.yml, nightly.yml and
//! release.yml builds there (labels `macOS`, `mini-builder`; nightly uses `mini`).
//! Only nightly.yml's windows-latest leg is still GitHub-hosted.
//!
//! THIS TEST PARSES THE WORKFLOWS rather than grepping them, so a hosted label
//! inside a comment is prose and a hosted label inside a matrix is a finding.

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

/// Jobs that are deliberately architecture-neutral (forjar#722): they run rustup,
/// cargo check/clippy/fmt, git and wc only, build no shipped artifact and name no
/// target triple, so an aarch64 runner is as good as an x86_64 one and pinning
/// `X64` only queues them behind the busiest runners. Naming a job here is a
/// reviewed decision (it may sit on `ARM64` since forjar#826, the cache-key guard
/// still applies), and a stale entry fails the test below.
const ARCH_NEUTRAL_JOBS: [(&str, &str); 3] = [
    ("lint.yml", "clippy"),
    ("lint.yml", "file-health"),
    ("msrv.yml", "msrv"),
];

/// The commit of paiml/.github that every delegated reusable workflow is pinned
/// to (forjar#970).
const REUSABLE_WORKFLOW_SHA: &str = "fbee3788a0ed92d09974ac21ea9bc97d373af6f0";

/// The commit `ci.yml` pins: one commit newer, whose `gate` roll-up job runs on
/// the light gate seat. Every other delegated call stays on the commit above.
const CI_WORKFLOW_SHA: &str = "8adbee1d793df71a31867eb875829b55e04a7c02";

/// Jobs on the `gate-light` pool (forjar#1005): the cheap classify/gate/authorize
/// jobs. They run no cargo and build no artifact, so they name no architecture
/// and are exempt from the `X64`/`ARM64` pin. Exempted by THIS exact list and not
/// by `ARCH_NEUTRAL_JOBS`, whose entries the cache-key guard in
/// `falsification_722_cache_key_arch.rs` treats as cargo jobs. The list is
/// bounded in both directions: a listed job that stops being `gate-light` fails,
/// and a `gate-light` job that is not listed fails.
const GATE_LIGHT_JOBS: [(&str, &str); 5] = [
    ("bench.yml", "classify"),
    ("ci.yml", "classify"),
    ("ci.yml", "gate"),
    ("pr-gate.yml", "no-pull-request-target"),
    ("proofs.yml", "classify"),
];

/// The seat probe: it runs on the one label `pool-canary`, which no real job carries, so it
/// cannot name an architecture or a second pool. Exempt by this exact (file, job) AND that label.
const SEAT_PROBE_JOBS: [(&str, &str); 1] = [("pool-canary.yml", "canary")];

/// True only for the listed (file, job) AND exactly the single label `[pool-canary]` (CANARY=1 seats carry no `self-hosted`): the
/// canary names no architecture and no second pool, and no other job may carry its label.
fn is_seat_probe(file: &str, job: &str, labels: &[String]) -> bool {
    let mut l: Vec<String> = labels.iter().map(|x| x.to_ascii_lowercase()).collect();
    l.sort();
    listed_at(&SEAT_PROBE_JOBS, file, job).is_some() && l == ["pool-canary"]
}

/// Check one self-hosted job against `GATE_LIGHT_JOBS`. Marks a listed job as
/// seen and returns a finding when the job is listed but not on `gate-light`,
/// or on `gate-light` but not listed.
fn gate_light_finding(
    file: &str,
    job_name: &str,
    labels: &[String],
    seen: &mut [bool; GATE_LIGHT_JOBS.len()],
) -> Option<String> {
    let on_pool = labels.iter().any(|l| l == "gate-light");
    let listed = GATE_LIGHT_JOBS
        .iter()
        .position(|(f, j)| *f == file && *j == job_name);
    if let Some(i) = listed {
        seen[i] = true;
    }
    match (listed.is_some(), on_pool) {
        (true, false) => Some(format!(
            "  {file}  job `{job_name}` is listed in GATE_LIGHT_JOBS but runs-on {labels:?}"
        )),
        (false, true) => Some(format!(
            "  {file}  job `{job_name}` runs on gate-light but is not in GATE_LIGHT_JOBS"
        )),
        _ => None,
    }
}

/// `GATE_LIGHT_JOBS` entries that matched no self-hosted job.
fn unseen_gate_light(seen: &[bool; GATE_LIGHT_JOBS.len()]) -> Vec<String> {
    GATE_LIGHT_JOBS
        .iter()
        .zip(seen)
        .filter(|(_, s)| !**s)
        .map(|((f, j), _)| {
            format!("  {f}  job `{j}` is listed in GATE_LIGHT_JOBS but is not a self-hosted job")
        })
        .collect()
}
/// Index of `(file, job)` in a `(file, job)` list.
fn listed_at(list: &[(&str, &str)], file: &str, job: &str) -> Option<usize> {
    list.iter().position(|(f, j)| *f == file && *j == job)
}

/// Accumulated findings of one pass over every self-hosted job.
struct PinScan {
    unpinned: Vec<String>,
    neutral_seen: [bool; ARCH_NEUTRAL_JOBS.len()],
    gate_light_seen: [bool; GATE_LIGHT_JOBS.len()],
    gate_light_findings: Vec<String>,
}

impl PinScan {
    fn visit(&mut self, file: &str, job_name: &str, labels: &[String]) {
        let arches = ["X64", "ARM64"];
        let neutral = listed_at(&ARCH_NEUTRAL_JOBS, file, job_name);
        if let Some(i) = neutral {
            self.neutral_seen[i] = true;
        }
        let listed = listed_at(&GATE_LIGHT_JOBS, file, job_name);
        let finding = gate_light_finding(file, job_name, labels, &mut self.gate_light_seen);
        self.gate_light_findings.extend(finding);
        let exempt = listed.is_some() && labels.iter().any(|l| l == "gate-light");
        let probe = is_seat_probe(file, job_name, labels);
        let exempt = exempt || probe;
        let pinned = labels.iter().any(|l| arches.contains(&l.as_str()));
        if !pinned && neutral.is_none() && !exempt {
            self.unpinned
                .push(format!("  {file}  job `{job_name}`  runs-on {labels:?}"));
        }
    }
}

/// Scan every self-hosted job: the ones that name no architecture, which
/// `ARCH_NEUTRAL_JOBS` entries still point at a self-hosted job, and the `gate-light` findings: jobs on
/// `gate-light` that `GATE_LIGHT_JOBS` does not list, and listed jobs that are
/// absent or no longer `gate-light`.
fn scan_architecture_pins_with_gate_light(
) -> (Vec<String>, [bool; ARCH_NEUTRAL_JOBS.len()], Vec<String>) {
    let mut scan = PinScan {
        unpinned: Vec::new(),
        neutral_seen: [false; ARCH_NEUTRAL_JOBS.len()],
        gate_light_seen: [false; GATE_LIGHT_JOBS.len()],
        gate_light_findings: Vec::new(),
    };
    for (file, doc) in workflows() {
        let Some(Value::Mapping(jobs)) = doc.get("jobs") else {
            continue;
        };
        for (name, job) in jobs {
            let labels = runner_labels(job);
            if labels.iter().any(|l| l == "self-hosted") {
                scan.visit(&file, name.as_str().unwrap_or("?"), &labels);
            }
        }
    }
    let missing = unseen_gate_light(&scan.gate_light_seen);
    scan.gate_light_findings.extend(missing);
    (scan.unpinned, scan.neutral_seen, scan.gate_light_findings)
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
/// Naming the architecture (`X64` or `ARM64`, deliberately) states the
/// assumption instead of relying on someone not clicking the box.
#[test]
fn a_fleet_job_names_the_architecture_it_needs() {
    let (mut unpinned, neutral_seen, gate_light_findings) =
        scan_architecture_pins_with_gate_light();
    assert!(
        gate_light_findings.is_empty(),
        "GATE_LIGHT_JOBS must list exactly the jobs on the gate-light pool:\n{}",
        gate_light_findings.join("\n")
    );
    let stale: Vec<_> = ARCH_NEUTRAL_JOBS
        .iter()
        .zip(neutral_seen)
        .filter(|(_, seen)| !seen)
        .map(|(e, _)| format!("{e:?}"))
        .collect();
    assert!(
        stale.is_empty(),
        "ARCH_NEUTRAL_JOBS names jobs that no longer exist, are not self-hosted, or are not self-hosted: {stale:?}"
    );
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

    // forjar#970 pinned every reusable-workflow call to one commit of
    // paiml/.github; `@main` would let another repository change what runs here.
    let sha = REUSABLE_WORKFLOW_SHA;
    let expected = vec![
        format!("ci.yml:ci -> paiml/.github/.github/workflows/sovereign-ci.yml@{CI_WORKFLOW_SHA}"),
        format!(
            "nightly-bench.yml:bench -> paiml/.github/.github/workflows/sovereign-ci.yml@{sha}"
        ),
        format!("pr-gate.yml:authorize -> paiml/.github/.github/workflows/pr-gate.yml@{sha}"),
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

    let expected: BTreeMap<String, usize> = [("nightly.yml:windows-latest", 1)]
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
        // the light classify/gate pool (forjar#1005); which jobs may use it is
        // pinned exactly by GATE_LIGHT_JOBS above
        "gate-light",
        "gpu",
        "cuda",
        // the self-hosted mac (mini-m4) that the nightly darwin legs build on
        "mini",
        "mini-builder",
    ];
    let mut bare = Vec::new();
    let mut probe_seen = false;
    for (file, doc) in workflows() {
        let Some(Value::Mapping(jobs)) = doc.get("jobs") else {
            continue;
        };
        for (name, job) in jobs {
            let labels = runner_labels(job);
            if is_seat_probe(&file, name.as_str().unwrap_or("?"), &labels) {
                probe_seen = true;
                continue;
            }
            if !labels.iter().any(|l| l == "self-hosted") {
                continue;
            }
            if labels.iter().any(|l| l.eq_ignore_ascii_case("pool-canary")) {
                bare.push(format!(
                    "  {file}  job `{}` carries `pool-canary` but is not the seat probe: {labels:?}",
                    name.as_str().unwrap_or("?")
                ));
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
        probe_seen,
        "SEAT_PROBE_JOBS names a job that is not a [pool-canary]-only job"
    );
    assert!(
        bare.is_empty(),
        "these jobs say `self-hosted` without naming a pool, so they can land on \
         any of the 25 runners including the GPU boxes:\n{}",
        bare.join("\n")
    );
}
