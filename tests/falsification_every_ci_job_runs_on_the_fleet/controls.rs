//! Controls: one fixture per shape a review lane claimed could hide a hosted
//! runner from this suite's parser, plus one that a fleet job is not mistaken
//! for a hosted one.
//!
//! Split out of `main.rs` when that file reached this repository's 500-line
//! gate.

use crate::scan::{fixture, hosted_in, job_runs_cargo, linux_label};

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

/// A mixed-case Linux label is a LINUX label, and the case that owns the rule
/// is the one that must see it.
///
/// Measured before `linux_label` existed: `runs-on: Ubuntu-latest` in audit.yml
/// left `no_linux_job_asks_github_for_a_runner` — "this is the whole ticket" —
/// GREEN, and failed `the_platforms_the_fleet_cannot_serve_are_exactly_these`
/// instead, which reported a Linux runner as a platform the fleet cannot serve.
/// The suite caught it, so nothing shipped; it caught it in the wrong place with
/// a message that named the wrong problem, which is how a green invariant comes
/// to mean nothing. Found by a review lane.
#[test]
fn a_mixed_case_linux_label_is_still_a_linux_label() {
    assert!(linux_label("Ubuntu-latest"), "Ubuntu-latest is Linux");
    assert!(linux_label("UBUNTU-24.04"), "UBUNTU-24.04 is Linux");
    assert!(
        linux_label(" ubuntu-latest "),
        "surrounding space is not a platform"
    );
    assert!(!linux_label("macos-latest"), "macos-latest is not Linux");
    assert!(
        !linux_label("self-hosted"),
        "a fleet label must not be read as a hosted Linux one"
    );
}

/// forjar#572: a job whose only step is a repository script that runs cargo
/// internally is a cargo job, even though the step text never says `cargo`.
/// The receipt job in quorum.yml was invisible to the private-`CARGO_HOME`
/// guard for exactly that reason.
#[test]
fn a_step_that_runs_a_cargo_script_makes_the_job_a_cargo_job() {
    let doc = fixture(
        r#"
jobs:
  receipt:
    runs-on: [self-hosted, clean-room]
    steps:
      - run: ./scripts/quorum-gate.sh --remote-ref origin/main
  surface:
    runs-on: [self-hosted, clean-room]
    steps:
      - run: bash scripts/dogfood/surface.sh
"#,
    );
    for job in ["receipt", "surface"] {
        assert!(
            job_runs_cargo(doc.get("jobs").and_then(|j| j.get(job)).unwrap()),
            "job `{job}` runs a script that runs cargo, but job_runs_cargo did not see it"
        );
    }
}

/// And a job whose steps name neither cargo nor such a script is still not one.
#[test]
fn a_job_with_no_cargo_and_no_cargo_script_is_not_a_cargo_job() {
    let doc = fixture(
        r#"
jobs:
  plain:
    runs-on: [self-hosted, clean-room]
    steps:
      - run: echo hello
      - run: bash scripts/other.sh
"#,
    );
    assert!(!job_runs_cargo(
        doc.get("jobs").and_then(|j| j.get("plain")).unwrap()
    ));
}

#[test]
fn non_probe_pool_canary_job_is_rejected_even_without_self_hosted() {
    let probe = fixture("jobs:\n  canary:\n    runs-on: [pool-canary]\n");
    let rogue = fixture("jobs:\n  sneaky:\n    runs-on: [pool-canary]\n");
    let (seen, bare) = crate::fleet_pool_findings(&[
        ("pool-canary.yml".to_string(), probe.clone()),
        ("other.yml".to_string(), rogue),
    ]);
    assert!(seen, "real seat probe should be seen");
    assert_eq!(
        bare.len(),
        1,
        "expected rogue [pool-canary]-only job to fail: {bare:?}"
    );
    assert!(bare[0].contains("other.yml") && bare[0].contains("pool-canary"));

    let mixed = fixture("jobs:\n  sneaky:\n    runs-on: [self-hosted, Pool-Canary]\n");
    let (seen_mixed, bare_mixed) = crate::fleet_pool_findings(&[("other.yml".to_string(), mixed)]);
    assert!(!seen_mixed, "non-probe doc must not report probe_seen");
    assert_eq!(
        bare_mixed.len(),
        1,
        "expected [self-hosted, Pool-Canary] to fail: {bare_mixed:?}"
    );
    assert!(
        bare_mixed[0].contains("carries `pool-canary`"),
        "Pool-Canary must trigger the pool-canary check, not fall through to bare self-hosted: {bare_mixed:?}"
    );

    for (label_desc, yaml) in [
        (
            "[Pool-Canary] without self-hosted",
            "jobs:\n  sneaky:\n    runs-on: [Pool-Canary]\n",
        ),
        (
            "[self-hosted, clean-room, Pool-Canary]",
            "jobs:\n  sneaky:\n    runs-on: [self-hosted, clean-room, Pool-Canary]\n",
        ),
    ] {
        let doc = fixture(yaml);
        let (seen_case, bare_case) = crate::fleet_pool_findings(&[("other.yml".to_string(), doc)]);
        assert!(!seen_case, "{label_desc} must not satisfy probe_seen");
        assert_eq!(
            bare_case.len(),
            1,
            "expected {label_desc} to be rejected case-insensitively: {bare_case:?}"
        );
        assert!(bare_case[0].contains("pool-canary"));
    }

    let (seen_ok, bare_ok) = crate::fleet_pool_findings(&[("pool-canary.yml".to_string(), probe)]);
    assert!(
        seen_ok && bare_ok.is_empty(),
        "valid seat probe must pass: {bare_ok:?}"
    );
}

#[test]
fn seat_probe_requires_exact_file_job_and_single_pool_canary_label() {
    let (seen_empty, bare_empty) = crate::fleet_pool_findings(&[]);
    assert!(!seen_empty && bare_empty.is_empty());

    let no_jobs = fixture("on: push\n");
    let (seen_no_jobs, bare_no_jobs) =
        crate::fleet_pool_findings(&[("pool-canary.yml".to_string(), no_jobs)]);
    assert!(!seen_no_jobs && bare_no_jobs.is_empty());

    let same_job_other_file = fixture("jobs:\n  canary:\n    runs-on: [pool-canary]\n");
    let (seen_file, bare_file) =
        crate::fleet_pool_findings(&[("other.yml".to_string(), same_job_other_file)]);
    assert!(!seen_file && bare_file.len() == 1);

    let same_file_other_job = fixture("jobs:\n  other:\n    runs-on: [pool-canary]\n");
    let (seen_job, bare_job) =
        crate::fleet_pool_findings(&[("pool-canary.yml".to_string(), same_file_other_job)]);
    assert!(!seen_job && bare_job.len() == 1);

    for extra_yaml in [
        "jobs:\n  canary:\n    runs-on: [pool-canary, self-hosted]\n",
        "jobs:\n  canary:\n    runs-on: [pool-canary, X64]\n",
    ] {
        let doc = fixture(extra_yaml);
        let (seen_extra, bare_extra) =
            crate::fleet_pool_findings(&[("pool-canary.yml".to_string(), doc)]);
        assert!(
            !seen_extra && bare_extra.len() == 1,
            "probe with extra labels must be rejected: {bare_extra:?}"
        );
    }

    let fleet_on_probe_slot = fixture("jobs:\n  canary:\n    runs-on: [self-hosted, clean-room]\n");
    let (seen_fleet, bare_fleet) =
        crate::fleet_pool_findings(&[("pool-canary.yml".to_string(), fleet_on_probe_slot)]);
    assert!(
        !seen_fleet && bare_fleet.is_empty(),
        "probe slot without pool-canary label must not satisfy probe_seen: {bare_fleet:?}"
    );

    let mixed_case_probe = fixture("jobs:\n  canary:\n    runs-on: [Pool-Canary]\n");
    let (seen_mixed, bare_mixed) =
        crate::fleet_pool_findings(&[("pool-canary.yml".to_string(), mixed_case_probe)]);
    assert!(
        seen_mixed && bare_mixed.is_empty(),
        "seat probe must match pool-canary case-insensitively: {bare_mixed:?}"
    );
}

#[test]
fn bare_self_hosted_jobs_are_rejected_and_fleet_pools_pass() {
    for bare_yaml in [
        "jobs:\n  bare:\n    runs-on: [self-hosted]\n",
        "jobs:\n  arch_only:\n    runs-on: [self-hosted, X64]\n",
    ] {
        let doc = fixture(bare_yaml);
        let (seen, bare) = crate::fleet_pool_findings(&[("other.yml".to_string(), doc)]);
        assert!(!seen, "bare self-hosted job must not set probe_seen");
        assert_eq!(
            bare.len(),
            1,
            "bare self-hosted job must be flagged: {bare:?}"
        );
        assert!(bare[0].contains("other.yml"));
    }

    for non_self_hosted_yaml in [
        "jobs:\n  hosted:\n    runs-on: [ubuntu-latest]\n",
        "jobs:\n  pool_only:\n    runs-on: [clean-room]\n",
    ] {
        let doc = fixture(non_self_hosted_yaml);
        let (seen, bare) = crate::fleet_pool_findings(&[("other.yml".to_string(), doc)]);
        assert!(
            !seen && bare.is_empty(),
            "non-self-hosted job must not be flagged: {bare:?}"
        );
    }

    for pool in [
        "clean-room",
        "build",
        "intel",
        "gx10",
        "yoga",
        "gate-light",
        "gpu",
        "cuda",
        "mini",
        "mini-builder",
    ] {
        let doc = fixture(&format!(
            "jobs:\n  ok:\n    runs-on: [self-hosted, {pool}]\n"
        ));
        let (seen, bare) = crate::fleet_pool_findings(&[("other.yml".to_string(), doc)]);
        assert!(
            !seen && bare.is_empty(),
            "fleet pool `{pool}` must pass without bare self-hosted finding: {bare:?}"
        );
    }
}
