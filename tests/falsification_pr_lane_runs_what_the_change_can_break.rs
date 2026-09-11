//! PMAT-237: A PR RUNS WHAT ITS CHANGE CAN BREAK, AND THE RELEASE RUNS
//! EVERYTHING.
//!
//! Measured on the ten most recently merged PRs: eight touched no `src/` at
//! all, and every one still spent about 131 job-minutes on jobs that build the
//! binary — ledger-replay 34, dogfood 26, bench 18, the workspace suite 14,
//! coverage 12. The release gate (`make dogfood-release`) is untouched and
//! still runs all of it; what changes is what a PR pays for what it changed.
//!
//! `scripts/ci/changed-class.sh` is the single decision, and it is an
//! ALLOW-LIST of the harmless rather than a deny-list of the heavy: a path
//! nobody classified must read as code, because being wrong that way costs CI
//! minutes and being wrong the other way ships untested code. These cases pin
//! both directions, and three of them are the paths that LOOK harmless and are
//! not — `README.md`, which gate D executes; `docs/audits/surface_audit.csv`,
//! which gate C diffs the live surface against; and `contracts/**`, which gate
//! G validates.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/ci/changed-class.sh")
}

/// The classifier's verdict for a file list.
fn class_of(files: &[&str]) -> String {
    let mut child = Command::new("bash")
        .arg(script())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bash must run");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(files.join("\n").as_bytes())
        .expect("write the file list");
    let out = child.wait_with_output().expect("classifier must finish");
    assert!(
        out.status.success(),
        "the classifier exited {:?}: {}{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn assert_code(files: &[&str], why: &str) {
    assert_eq!(
        class_of(files),
        "code=true",
        "PMAT-237: {why}. A change touching {files:?} must run every heavy job; \
         reading it as harmless skips the suite over it."
    );
}

fn assert_harmless(files: &[&str], why: &str) {
    assert_eq!(
        class_of(files),
        "code=false",
        "PMAT-237: {why}. A change touching only {files:?} cannot alter what any \
         heavy job measures, and paying forty minutes for it is the waste this \
         ticket exists to remove."
    );
}

#[test]
fn source_and_tests_and_the_manifests_are_code() {
    assert_code(&["src/lib.rs"], "a source file is code");
    assert_code(
        &["crates/forjar-contracts/src/lib.rs"],
        "a workspace member is code",
    );
    assert_code(&["tests/falsification_something.rs"], "a test is code");
    assert_code(&["benches/store_bench.rs"], "a benchmark is code");
    assert_code(&["Cargo.toml"], "the manifest is code");
    assert_code(&["Cargo.lock"], "the lockfile is code");
}

#[test]
fn the_things_that_run_the_gates_are_code() {
    assert_code(&["scripts/dogfood/surface.sh"], "a gate script is code");
    assert_code(
        &[".github/workflows/ci.yml"],
        "the workflow that runs the jobs is code",
    );
    assert_code(
        &["Makefile"],
        "the target the gates are invoked through is code",
    );
    assert_code(
        &["scripts/ci/changed-class.sh"],
        "the classifier itself is code",
    );
}

/// The three that look harmless because of where they live.
#[test]
fn the_paths_that_look_harmless_and_are_not() {
    assert_code(
        &["README.md"],
        "gate D runs every fenced forjar block in the README",
    );
    assert_code(
        &["docs/audits/surface_audit.csv"],
        "gate C diffs the live surface against that ledger",
    );
    assert_code(
        &["contracts/forjar-dogfood-coverage-v1.yaml"],
        "gate G validates the corpus",
    );
}

#[test]
fn the_record_is_harmless() {
    assert_harmless(
        &["docs/roadmaps/roadmap.yaml"],
        "a roadmap row changes no behaviour",
    );
    assert_harmless(
        &["docs/roadmaps/releases.yaml"],
        "the release ledger changes no behaviour",
    );
    assert_harmless(
        &["docs/audits/impl-PMAT-237-receipt.md"],
        "a receipt changes no behaviour",
    );
    assert_harmless(
        &[".quorum/PMAT-237-x.json"],
        "a quorum artifact changes no behaviour",
    );
    assert_harmless(
        &["CHANGELOG.md"],
        "the changelog is read by gate H, which is a release gate",
    );
    assert_harmless(
        &[
            "docs/roadmaps/roadmap.yaml",
            "docs/audits/logs/PMAT-237-x.log",
            ".quorum/evidence/x.md",
        ],
        "a whole triage-shaped change is harmless",
    );
}

/// The direction the classifier must fail in.
#[test]
fn an_unclassified_path_is_code() {
    assert_code(
        &["some-new-top-level-thing"],
        "a path nobody classified must read as code",
    );
    assert_code(&["rust-toolchain.toml"], "the toolchain is code");
    assert_code(&[".cargo/config.toml"], "cargo's own configuration is code");
    assert_code(
        &["docs/roadmaps/roadmap.yaml", "src/lib.rs"],
        "one code file among a hundred harmless ones is still code",
    );
}

#[test]
fn an_empty_file_list_is_code() {
    assert_code(
        &[],
        "a diff that could not be read is not a licence to skip anything",
    );
}

// ---------------------------------------------------------------------
// The wiring. The classifier decides nothing if no job asks it, and a job
// that skips without the gate re-checking the class is a hole rather than a
// saving. These rules read the workflow YAML at `env!("CARGO_MANIFEST_DIR")`,
// the same bytes GitHub Actions parses.
// ---------------------------------------------------------------------

fn workflow(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p:?}: {e}"))
}

/// Every workflow that gates a heavy job, and the jobs it gates.
const GATED: &[(&str, &[&str])] = &[
    (
        ".github/workflows/ci.yml",
        &[
            "ci",
            "examples-validate",
            "no-default-features",
            "doctests",
            "dogfood",
        ],
    ),
    (".github/workflows/proofs.yml", &["ledger-replay"]),
    (".github/workflows/coverage.yml", &["coverage"]),
    (".github/workflows/bench.yml", &["benchmark"]),
];

#[test]
fn every_gated_workflow_computes_the_class_from_the_one_script() {
    for (rel, _) in GATED {
        let text = workflow(rel);
        assert!(
            text.contains("uses: ./.github/actions/changed-class"),
            "PMAT-237: {rel} gates a heavy job but computes the class some other \
             way. One decision, in one place, or the workflows drift apart and \
             the test below stops meaning anything."
        );
    }
    let action = workflow(".github/actions/changed-class/action.yml");
    assert!(
        action.contains("bash scripts/ci/changed-class.sh"),
        "PMAT-237: the composite action no longer calls the classifier script, so \
         the cases above test something the workflows do not run:\n{action}"
    );
}

#[test]
fn every_heavy_job_runs_only_when_the_change_can_reach_it() {
    for (rel, jobs) in GATED {
        let text = workflow(rel);
        for job in *jobs {
            let start = text
                .find(&format!("\n  {job}:\n"))
                .unwrap_or_else(|| panic!("PMAT-237: {rel} has no job `{job}`"))
                + 1;
            let rest = &text[start..];
            let end = rest[1..]
                .find("\n  ")
                .map(|i| i + 1)
                .and_then(|i| {
                    rest[i..]
                        .lines()
                        .next()
                        .filter(|l| l.starts_with("  ") && !l.starts_with("   "))
                        .map(|_| i)
                })
                .unwrap_or(rest.len());
            let block = &rest[..end];
            assert!(
                block.contains("needs.classify.outputs.code == 'true'"),
                "PMAT-237: {rel}'s `{job}` runs on every change. It is one of the \
                 jobs that builds the binary, and eight of the ten most recently \
                 merged PRs touched no code at all:\n{block}"
            );
        }
    }
}

#[test]
fn a_skip_is_refused_when_the_change_is_code() {
    let ci = workflow(".github/workflows/ci.yml");
    assert!(
        ci.contains("was skipped on a change the classifier called code"),
        "PMAT-237: ci.yml's gate accepts a skipped job without re-reading the \
         class. Trusting the skip is how a saving becomes a hole: the gate must \
         refuse a heavy job that did not run on a change that touches code."
    );
    let proofs = workflow(".github/workflows/proofs.yml");
    assert!(
        proofs.contains("skipped on a change the classifier called code"),
        "PMAT-237: proofs.yml's aggregator reads every skip as `not selected`. \
         ledger-replay is now skipped by CLASS, and a class-driven skip on a code \
         change must fail it."
    );
}
