//! A GitHub-hosted coverage job must not cache its build directory.
//!
//! forjar#386. The `Coverage` lane died on **main**, not only on PRs, and the
//! job carried no logs at all: `Set up job`, checkout, `Install Rust toolchain`
//! and `Install cargo-llvm-cov` were `success`, and `Cache cargo`,
//! `Generate coverage` and both post-steps were `null`. The only surviving
//! evidence was the check-run annotation, and it named the cause exactly:
//!
//!   System.IO.IOException: No space left on device :
//!     '/home/runner/actions-runner/cached/2.336.0/_diag/Worker_20260830-091631-utc.log'
//!      at GitHub.Runner.Worker.Worker.RunAsync(String pipeIn, String pipeOut)
//!
//! The runner's own Worker process died because it could not write its own diag
//! log. That is why the job has no logs: the process that uploads them was the
//! process that was killed.
//!
//! `coverage.yml` cached `~/.cargo/registry`, `~/.cargo/git` and **`target`**.
//! `cargo llvm-cov` builds this crate's 242 integration test binaries plus the
//! lib and bin unit-test binaries, all instrumented and all carrying full DWARF.
//! MEASURED, that tree is **70.70 GiB in 19,070 files**, 66 GiB of it in
//! debug/deps — already the size of a hosted runner's disk, so the build alone
//! sat on the line. The cache step then asked for a SECOND copy of the same
//! tree, compressed, on the SAME filesystem:
//!
//!   [command]/usr/bin/tar --posix -cf cache.tzst --exclude cache.tzst -P \
//!     -C /home/runner/work/forjar/forjar --files-from manifest.txt \
//!     --use-compress-program zstdmt
//!   zstd: error 70 : Write error : cannot write block : No space left on device
//!   ##[warning]Failed to save: "/usr/bin/tar" failed with error: ...
//!
//! which is the second half of the defect, and the reason it survived so long:
//! `actions/cache` downgrades a failed SAVE to a warning. Every green run of
//! this lane had already filled the runner's disk and said so in a `##[warning]`
//! nobody reads. When the margin was a little tighter the ENOSPC arrived during
//! the build instead, and took the Worker — and the logs — with it.
//!
//! And because the save never once completed, the cache never once existed:
//! every run whose logs survive reports `Cache not found for input keys`, on
//! consecutive runs sharing an identical `Cargo.lock` hash. Zero restores, ever.
//! The step's entire measured contribution to this repo was to fill the disk.
//!
//! # Why this rule is about COVERAGE jobs and not about `target`
//!
//! The first draft of this guard banned caching `target` from any hosted job,
//! and it flagged four more: `bench`, `lint`, `msrv`, `stress`. Those are NOT
//! this defect, and deleting their caches would have been an over-correction
//! dressed up as a fix. Measured, from the repo's live cache list:
//!
//!   bench-Linux    815 MiB   lint-Linux    826 MiB   msrv-Linux  800 MiB
//!   lint-macOS     752 MiB   lint-Linux-encryption   877 MiB
//!
//! They save successfully and are restored on every run. An ordinary `cargo
//! build`/`clippy` tree is under a gigabyte compressed; the INSTRUMENTED tree is
//! 70.70 GiB. That is not a difference of degree, and `-C instrument-coverage`
//! is what separates them. So the rule is drawn where the measurement draws it.
//!
//! THIS TEST PARSES THE WORKFLOWS rather than grepping them, so the pin cannot
//! be satisfied by `target` appearing in a comment, and it PRINTS ITS
//! DENOMINATOR — a structural guard that scanned zero coverage jobs would
//! otherwise pass loudly while checking nothing.

use serde_yaml_ng::Value;
use std::path::{Path, PathBuf};

/// A cache step that this guard rejects.
#[derive(Debug, PartialEq, Eq)]
struct Violation {
    workflow: String,
    job: String,
    runs_on: String,
    path: String,
}

fn workflow_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows")
}

/// Every `.yml`/`.yaml` under `.github/workflows`, sorted for a stable report.
fn workflow_files() -> Vec<PathBuf> {
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
fn runs_on(job: &Value) -> Option<String> {
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
fn is_github_hosted(runs_on: &str) -> bool {
    let lowered = runs_on.to_ascii_lowercase();
    !lowered.contains("self-hosted") && !lowered.contains("group")
}

/// Whether one cached path names a Rust build directory.
///
/// Compares whole path COMPONENTS, so `~/.cargo/registry` is not a hit while
/// `target`, `./target`, `**/target` and `target/llvm-cov-target` all are.
fn is_rust_build_dir(path: &str) -> bool {
    path.split('/')
        .map(str::trim)
        .any(|c| c == "target" || c == "llvm-cov-target")
}

/// Whether a step invokes `actions/cache` in any of its forms.
fn is_cache_action(step: &Value) -> bool {
    step.get("uses")
        .and_then(Value::as_str)
        .is_some_and(|u| u.starts_with("actions/cache"))
}

/// The `with.path` of a cache step, one entry per line of the block scalar.
fn cached_paths(step: &Value) -> Vec<String> {
    step.get("with")
        .and_then(|w| w.get("path"))
        .and_then(Value::as_str)
        .map(|p| {
            p.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Whether a job compiles with LLVM source-based coverage instrumentation.
///
/// This is the property that separates a ~0.8 GiB build tree from a 70.70 GiB
/// one — see the module docs. `cargo llvm-cov` is how this repo asks for it;
/// setting `-C instrument-coverage` by hand is the other way, and counts too.
fn is_instrumented_coverage_job(job: &Value) -> bool {
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

/// What the scan looked at, so a pass can state its denominator.
#[derive(Debug, Default, PartialEq, Eq)]
struct Scan {
    workflows: usize,
    jobs: usize,
    hosted_jobs: usize,
    hosted_coverage_jobs: usize,
    cache_steps: usize,
    cached_paths: usize,
    violations: Vec<Violation>,
}

/// Scan one already-parsed workflow. Kept separate from the filesystem so the
/// discrimination controls below can drive it with synthetic inputs.
fn scan_workflow(name: &str, doc: &Value, scan: &mut Scan) {
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
fn scan_repo() -> Scan {
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

#[test]
fn no_hosted_coverage_job_caches_its_build_directory() {
    let scan = scan_repo();

    // PRINT THE DENOMINATOR. "0 violations" over 0 inspected paths is the
    // failure mode this line exists to make impossible to mistake for a pass.
    println!(
        "scanned {} workflow(s), {} job(s), {} on GitHub-hosted runners, \
         {} of those instrumented-coverage, {} actions/cache step(s) in them, \
         {} cached path(s)",
        scan.workflows,
        scan.jobs,
        scan.hosted_jobs,
        scan.hosted_coverage_jobs,
        scan.cache_steps,
        scan.cached_paths
    );

    assert!(
        scan.violations.is_empty(),
        "a GitHub-hosted coverage job caches its build directory: {:#?}\n\n\
         `cargo llvm-cov` over this crate's 242 integration test binaries \
         produces a 70.70 GiB tree, which is already the size of a hosted \
         runner's disk. Caching it asks tar+zstd for a second copy on the same \
         filesystem, which cannot fit: the save ENOSPCs — and `actions/cache` \
         downgrades that to a `##[warning]`, so the job stays green and the \
         cache is never written — while a tighter margin puts the ENOSPC in the \
         middle of the build, where it kills the runner's Worker process and \
         takes the job's logs with it (#386). Cache the cargo registry, never \
         the instrumented build directory.",
        scan.violations
    );
}

/// The scan must actually have something to scan. A refactor that stops finding
/// workflows, stops parsing `runs-on`, or stops recognising the coverage job
/// would leave the assertion above green while checking nothing.
#[test]
fn the_scan_reaches_the_real_coverage_job() {
    let scan = scan_repo();
    assert!(
        scan.workflows >= 10,
        "found only {} workflow(s) — the scan is not reading .github/workflows",
        scan.workflows
    );
    assert!(
        scan.hosted_jobs > 0,
        "found {} GitHub-hosted job(s) in {} job(s) — `runs-on` parsing is broken, \
         so every job is being skipped before the check runs",
        scan.hosted_jobs,
        scan.jobs
    );
    assert!(
        scan.hosted_coverage_jobs > 0,
        "found 0 instrumented-coverage jobs among {} hosted job(s). coverage.yml \
         defines one, so `is_instrumented_coverage_job` no longer recognises it \
         and the guard above is passing over an empty set",
        scan.hosted_jobs
    );
}

/// Poka-yoke: prove the checker can go RED.
///
/// The guard above is only evidence if it is capable of failing. This drives the
/// same `scan_workflow` with the exact `coverage.yml` shape that caused #386 and
/// asserts it is rejected — and with the fixed shape, and asserts it is
/// accepted. A change that neuters the checker fails HERE, loudly, instead of
/// turning the real assertion into a green no-op.
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
