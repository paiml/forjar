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

mod controls;
mod scan;

use scan::scan_repo;

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
/// The knob that shrank the tree 70.70 GiB -> 23 GiB must stay set.
///
/// `MIN_FREE_GIB` is a pre-build floor and cannot substitute for it (see
/// `job_reduces_debug_info`). Deleting `CARGO_PROFILE_DEV_DEBUG:
/// line-tables-only` from coverage.yml — the natural "why is coverage slow /
/// why are my backtraces useless" edit — restores the pre-#388 arithmetic with
/// the cache rule still satisfied, because it caches no build directory at all.
#[test]
fn every_hosted_coverage_job_reduces_debug_info() {
    let scan = scan_repo();
    assert!(
        scan.hosted_coverage_jobs > 0,
        "0 instrumented-coverage jobs found — this assertion has no denominator"
    );
    assert!(
        scan.full_dwarf_jobs.is_empty(),
        "hosted instrumented-coverage job(s) build with full debug info: {:?}\n\n\
         MEASURED: the same `cargo llvm-cov` run is 70.70 GiB in 19,070 files \
         with full DWARF and 23 GiB with `CARGO_PROFILE_DEV_DEBUG: \
         line-tables-only`. A hosted runner's disk is ~145 GiB with ~84 GiB \
         free, so the full-DWARF build leaves ~13 GiB and the ENOSPC lands \
         mid-build, killing the runner's Worker and the job's logs with it \
         (#386). `MIN_FREE_GIB` is checked BEFORE the build and passes either \
         way — it cannot catch this.",
        scan.full_dwarf_jobs
    );
}
