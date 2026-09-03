//! Discrimination controls: proof that the guard in `main.rs` can go RED.
//!
//! A structural guard is only evidence if it is capable of failing. Each test
//! here drives `scan_workflow` with a synthetic workflow and asserts BOTH
//! directions — the defect is rejected, and the fix is accepted — because a
//! matcher that flags everything deletes four working caches, and one that
//! flags nothing watched #386 happen.

use crate::scan::{is_rust_build_dir, mentions_reduced_debuginfo, scan_workflow, Scan, Violation};

#[test]
fn the_checker_rejects_the_defect_and_accepts_the_fix() {
    let defective = r#"
jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - name: Cache cargo
        uses: actions/cache@v6
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: coverage-Linux-abc
      - run: cargo llvm-cov --summary-only --fail-under-lines 95
"#;
    let mut red = Scan::default();
    scan_workflow(
        "defective.yml",
        &serde_yaml_ng::from_str(defective).expect("fixture must parse"),
        &mut red,
    );
    assert_eq!(
        red.violations,
        vec![Violation {
            workflow: "defective.yml".into(),
            job: "coverage".into(),
            runs_on: "ubuntu-latest".into(),
            path: "target".into(),
        }],
        "the checker did not reject the exact configuration that killed the \
         Coverage lane in #386 — it is not capable of failing, so its green \
         verdict on this repo means nothing"
    );

    let fixed = r#"
jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - name: Cache cargo registry
        uses: actions/cache@v6
        with:
          path: |
            ~/.cargo/registry/index
            ~/.cargo/registry/cache
            ~/.cargo/git/db
          key: coverage-cargo-Linux-abc
      - run: cargo llvm-cov --summary-only --fail-under-lines 95
"#;
    let mut green = Scan::default();
    scan_workflow(
        "fixed.yml",
        &serde_yaml_ng::from_str(fixed).expect("fixture must parse"),
        &mut green,
    );
    assert!(
        green.violations.is_empty(),
        "the checker rejects the FIXED shape too, so it is not discriminating \
         between them — it would refuse every cache, including the registry \
         cache the lane needs: {:#?}",
        green.violations
    );
    assert_eq!(
        green.cached_paths, 3,
        "the accepting pass inspected {} path(s), not 3 — it accepted the fixed \
         shape by never looking at it",
        green.cached_paths
    );
}

/// Over-correction control. A hosted job that caches `target` WITHOUT
/// instrumenting is not this defect: measured, `bench`/`lint`/`msrv` cache
/// 750-880 MiB and restore fine. The first draft of this guard flagged all four
/// and would have deleted four working caches to fix one broken one. This test
/// is what stops that draft from coming back.
#[test]
fn a_hosted_job_that_caches_target_without_instrumenting_is_not_flagged() {
    let ordinary = r#"
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/cache@v6
        with:
          path: |
            ~/.cargo/registry
            target
          key: lint-Linux-abc
      - run: cargo clippy --all-targets -- -D warnings
"#;
    let mut scan = Scan::default();
    scan_workflow(
        "lint.yml",
        &serde_yaml_ng::from_str(ordinary).expect("fixture must parse"),
        &mut scan,
    );
    assert_eq!(scan.hosted_jobs, 1, "the job was not seen at all");
    assert!(
        scan.violations.is_empty(),
        "an ordinary (uninstrumented) hosted job was flagged: {:#?}. Its tree is \
         under a gigabyte compressed and its cache demonstrably saves and \
         restores; banning it is an over-correction, not a fix",
        scan.violations
    );
}

/// The SAME over-correction control, for the broadened matcher.
///
/// mutation.yml, quorum.yml, convergence.yml and behavior.yml all run
/// `Swatinem/rust-cache@v2` on `ubuntu-latest`, and their live caches are
/// 142-583 MiB and restore fine. Broadening `is_cache_action` without keeping
/// the `is_instrumented_coverage_job` gate would flag all four and quietly
/// delete four working caches to fix one broken lane. That is the exact shape
/// the original PR already rejected once; this is what stops it returning
/// through the new arm.
#[test]
fn a_hosted_job_using_rust_cache_without_instrumenting_is_not_flagged() {
    let ordinary = r#"
jobs:
  mutation:
    runs-on: ubuntu-latest
    steps:
      - uses: Swatinem/rust-cache@v2
      - run: cargo mutants --check
"#;
    let mut scan = Scan::default();
    scan_workflow(
        "mutation.yml",
        &serde_yaml_ng::from_str(ordinary).expect("fixture must parse"),
        &mut scan,
    );
    assert_eq!(scan.hosted_jobs, 1, "the job was not seen at all");
    assert_eq!(
        scan.hosted_coverage_jobs, 0,
        "an uninstrumented job was classified as instrumented coverage"
    );
    assert!(
        scan.violations.is_empty(),
        "an uninstrumented hosted job using rust-cache was flagged: {:#?}. Four \
         jobs in this repo do exactly this and their caches save and restore; \
         banning them is an over-correction, not a fix",
        scan.violations
    );
}

/// Poka-yoke for the assertion above: prove the debug-info detector can go RED,
/// and that it is not simply returning `true` for everything.
#[test]
fn the_debug_info_detector_rejects_full_dwarf_and_accepts_each_reduction() {
    let full_dwarf = r#"
jobs:
  coverage:
    runs-on: ubuntu-latest
    env:
      MIN_FREE_GIB: "40"
      CARGO_INCREMENTAL: "0"
    steps:
      - run: cargo llvm-cov --summary-only --fail-under-lines 95
"#;
    let mut red = Scan::default();
    scan_workflow(
        "fulldwarf.yml",
        &serde_yaml_ng::from_str(full_dwarf).expect("fixture must parse"),
        &mut red,
    );
    assert_eq!(
        red.full_dwarf_jobs,
        vec!["fulldwarf.yml:coverage".to_string()],
        "a coverage job carrying only MIN_FREE_GIB and CARGO_INCREMENTAL was \
         accepted — neither shrinks the tree, so the detector cannot fail and \
         its green verdict on this repo means nothing"
    );

    for (label, yaml) in [
        (
            "env profile knob",
            "jobs:\n  c:\n    runs-on: ubuntu-latest\n    env:\n      \
             CARGO_PROFILE_DEV_DEBUG: line-tables-only\n    steps:\n      \
             - run: cargo llvm-cov\n",
        ),
        (
            "numeric profile knob",
            "jobs:\n  c:\n    runs-on: ubuntu-latest\n    env:\n      \
             CARGO_PROFILE_DEV_DEBUG: 0\n    steps:\n      - run: cargo llvm-cov\n",
        ),
        (
            "RUSTFLAGS",
            "jobs:\n  c:\n    runs-on: ubuntu-latest\n    env:\n      \
             RUSTFLAGS: \"-C debuginfo=line-tables-only\"\n    steps:\n      \
             - run: cargo llvm-cov\n",
        ),
    ] {
        let mut green = Scan::default();
        scan_workflow(
            "reduced.yml",
            &serde_yaml_ng::from_str(yaml).expect("fixture must parse"),
            &mut green,
        );
        assert_eq!(
            green.hosted_coverage_jobs, 1,
            "{label}: the fixture's coverage job was not reached"
        );
        assert!(
            green.full_dwarf_jobs.is_empty(),
            "{label}: a job that DOES reduce debug info was reported as full \
             DWARF, so the detector would refuse the fix as well as the defect"
        );
    }

    assert!(
        !mentions_reduced_debuginfo("-C debuginfo=2"),
        "`debuginfo=2` is FULL debug info and must not satisfy the rule"
    );
}

/// A self-hosted job caching `target` is CORRECT (sovereign-ci mounts a
/// persistent per-PR `CARGO_TARGET_DIR` on the clean-room runners), so the
/// exemption must be real and not an accident of string matching.
#[test]
fn self_hosted_coverage_jobs_are_exempt() {
    let sovereign = r#"
jobs:
  coverage:
    runs-on: [self-hosted, clean-room]
    steps:
      - uses: actions/cache@v6
        with:
          path: |
            target
          key: cov
      - run: cargo llvm-cov --summary-only
"#;
    let mut scan = Scan::default();
    scan_workflow(
        "sovereign.yml",
        &serde_yaml_ng::from_str(sovereign).expect("fixture must parse"),
        &mut scan,
    );
    assert!(
        scan.violations.is_empty(),
        "a self-hosted job was flagged: {:#?}. The clean-room runners have real \
         disks; this rule is about runners whose disk we do not own",
        scan.violations
    );
    assert_eq!(
        scan.hosted_jobs, 0,
        "`[self-hosted, clean-room]` was classified as GitHub-hosted"
    );
}

/// `Swatinem/rust-cache` caches the build directory too — and it is already the
/// house idiom on four hosted jobs here (mutation, quorum, convergence,
/// behavior). Switching `coverage.yml` to it is the obvious "speed this lane
/// up" edit, and against the first draft of this guard that edit reproduced
/// #386 with every assertion still green, because `is_cache_action` required
/// the `uses` string to start with `actions/cache`.
///
/// rust-cache takes no `with.path`: what it caches is implicit, so the scan has
/// to supply the denominator itself or the violation stays invisible even once
/// the step is recognised.
#[test]
fn a_hosted_coverage_job_using_rust_cache_is_flagged() {
    let rustcache = r#"
jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: Swatinem/rust-cache@v2
      - run: cargo llvm-cov --summary-only --fail-under-lines 95
"#;
    let mut scan = Scan::default();
    scan_workflow(
        "rustcache.yml",
        &serde_yaml_ng::from_str(rustcache).expect("fixture must parse"),
        &mut scan,
    );
    assert_eq!(
        scan.hosted_coverage_jobs, 1,
        "the instrumented hosted job was not even reached"
    );
    assert_eq!(
        scan.cache_steps, 1,
        "`Swatinem/rust-cache@v2` was not counted as a cache step, so the guard \
         scans a hosted coverage job and inspects none of its caching"
    );
    assert_eq!(
        scan.violations,
        vec![Violation {
            workflow: "rustcache.yml".into(),
            job: "coverage".into(),
            runs_on: "ubuntu-latest".into(),
            path: "target".into(),
        }],
        "rust-cache on an instrumented hosted job was not flagged. It caches the \
         Rust build directory — the 66 of 70.70 GiB in `target/debug/deps` that \
         IS #386 — and it is already used by four hosted jobs in this repo, so \
         the migration that reproduces the outage is one line away"
    );
}

#[test]
fn a_rust_build_dir_is_recognised_by_component_not_substring() {
    for hit in [
        "target",
        "./target",
        "**/target",
        "target/llvm-cov-target",
        "/home/runner/work/forjar/forjar/target",
    ] {
        assert!(is_rust_build_dir(hit), "{hit} must be recognised");
    }
    for miss in [
        "~/.cargo/registry",
        "~/.cargo/registry/index",
        "~/.cargo/git/db",
        "~/.rustup/toolchains",
        // Substring matches that must NOT trip the rule.
        "docs/targeting.md",
        "src/target_triple.rs",
        "~/.cache/retargeted",
    ] {
        assert!(!is_rust_build_dir(miss), "{miss} must NOT be recognised");
    }
}
