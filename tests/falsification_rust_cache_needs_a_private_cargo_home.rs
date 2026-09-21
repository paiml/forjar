//! paiml/infra#775: `Swatinem/rust-cache`'s SAVE step deletes the registry
//! SOURCES, and on a self-hosted runner those sources are shared.
//!
//! `cache-bin: "false"` is the documented mitigation and it is not the fix. It
//! skips `cleanBin` and nothing else; the same save step runs `cleanRegistry`
//! unconditionally and removes `${CARGO_HOME}/registry/src` — the extracted
//! crate sources every OTHER job on the box is compiling from at that instant.
//! All sixteen `intel-clean-room-*` runner services share `HOME=/home/noah`.
//!
//! Measured on intel 2026-09-19: eight save steps in 100 minutes, every crate
//! directory under the shared registry carrying a birth time pinned to the
//! second one of them finished, and an infra `cargo kani` build killed
//! mid-crate with `could not parse/generate dep info at .../serde_core-<hash>.d:
//! No such file or directory`.
//!
//! THE ONLY EXONERATION IS A JOB-PRIVATE `CARGO_HOME`, and that is the whole
//! content of the rule below. `--target-dir`, `cargo install --root` and a
//! fresh `$HOME` all leave cargo reading sources from the shared
//! `$CARGO_HOME`, so none of them moves the thing the save step deletes.
//!
//! WHY THIS EXISTS AS A RUST TEST when `scripts/lint-rust-cache-guard.sh`
//! already asserts the same rule: the shell lint runs in `make policy`, and
//! this runs in `cargo test`. Two consumers, and the second is the one the
//! quorum gate can revert and observe. They are deliberately redundant — the
//! rule is worth more than the duplication, and each catches a push the other
//! can be configured out of.
//!
//! Read from the workflow YAML at `env!("CARGO_MANIFEST_DIR")`, the same bytes
//! GitHub Actions parses. The small helpers are duplicated from
//! `falsification_release_workflow_fixed_paths.rs` rather than lifted into
//! `tests/common/`, for the reason that file states.

use std::fs;
use std::path::{Path, PathBuf};

fn workflows_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows")
}

fn all_workflow_files() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(workflows_dir())
        .expect("read .github/workflows")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yml"))
        .collect();
    out.sort();
    out
}

/// Every top-level job block in one workflow: `(job name, its text)`.
///
/// A job block runs from its own 2-space-indented key to the next one, so an
/// `env:` or `steps:` belonging to a LATER job can never be read as this
/// job's. Getting that boundary wrong is how a scan reports a rule satisfied
/// by a neighbour.
fn jobs_of(text: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut starts: Vec<(usize, String)> = Vec::new();
    let mut in_jobs = false;
    for (i, line) in lines.iter().enumerate() {
        if *line == "jobs:" {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }
        // A top-level key at column 0 ends the `jobs:` mapping entirely.
        if !line.trim().is_empty() && !line.starts_with(' ') {
            break;
        }
        let is_job_key = line.starts_with("  ")
            && !line.starts_with("   ")
            && line.trim_end().ends_with(':')
            && !line.trim_start().starts_with('#');
        if is_job_key {
            starts.push((i, line.trim().trim_end_matches(':').to_string()));
        }
    }
    let mut out = Vec::new();
    for (n, (i, name)) in starts.iter().enumerate() {
        let end = starts.get(n + 1).map_or(lines.len(), |(j, _)| *j);
        out.push((name.clone(), lines[*i..end].join("\n")));
    }
    out
}

fn is_self_hosted(job: &str) -> bool {
    job.lines()
        .filter(|l| l.contains("runs-on:"))
        .any(|l| l.contains("self-hosted") || l.contains("clean-room"))
}

fn uses_rust_cache(job: &str) -> bool {
    // `actions-rust-lang/setup-rust-toolchain` WRAPS rust-cache and its
    // `cache:` input defaults to true, so naming only Swatinem would miss it.
    job.contains("Swatinem/rust-cache") || job.contains("actions-rust-lang/setup-rust-toolchain")
}

/// Does this job declare a CARGO_HOME that is NOT the shared one?
///
/// The shared spellings are the ones that resolve to the runner user's own
/// `~/.cargo`. Anything else — a workspace-relative path, a job-named
/// directory — is private to the job and is the exoneration.
fn has_private_cargo_home(job: &str) -> bool {
    job.lines().filter(|l| l.contains("CARGO_HOME:")).any(|l| {
        let value = l.split("CARGO_HOME:").nth(1).unwrap_or("").trim();
        let value = value.trim_matches(|c| c == '"' || c == '\'');
        let shared = [
            "~/.cargo",
            "$HOME/.cargo",
            "${HOME}/.cargo",
            "/home/",
            "/Users/",
        ]
        .iter()
        .any(|p| value.starts_with(p));
        !value.is_empty() && !shared
    })
}

#[test]
fn a_self_hosted_job_caching_cargo_must_own_its_cargo_home() {
    let files = all_workflow_files();

    // ANTI-VACUITY, first, because every assertion below is over a set this
    // scan built. `0 violations over 0 files` is indistinguishable from a
    // repository with no workflows, and it is the shape this fleet keeps
    // finding in its own guards.
    assert!(
        !files.is_empty(),
        "no .github/workflows/*.yml found — this test scanned nothing and would \
         pass for a repository that had deleted its CI"
    );

    let mut self_hosted = 0usize;
    let mut cachers = 0usize;
    let mut exonerated = 0usize;
    let mut violations: Vec<String> = Vec::new();

    for path in &files {
        let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        for (job, block) in jobs_of(&text) {
            if !is_self_hosted(&block) {
                continue;
            }
            self_hosted += 1;
            if !uses_rust_cache(&block) {
                continue;
            }
            cachers += 1;
            if has_private_cargo_home(&block) {
                exonerated += 1;
            } else {
                violations.push(format!(
                    "{name}: job `{job}` is self-hosted and caches cargo, with no job-private \
                     CARGO_HOME. Its save step will delete the SHARED registry/src that other \
                     jobs on the box are compiling from. `cache-bin: \"false\"` does not \
                     prevent this — it skips cleanBin alone (paiml/infra#775)."
                ));
            }
        }
    }

    assert!(
        self_hosted > 0,
        "scanned {} workflow file(s) and found no self-hosted job — the rule was \
         never exercised, so a pass here measures nothing",
        files.len()
    );

    // The rule must be LIVE, not merely unviolated. If nothing in the repo
    // caches cargo on a self-hosted runner, this test is green for the same
    // reason an empty fixture is: it never reached its subject. forjar has such
    // a job (`quorum receipt`), so this holds today and turns red the moment
    // someone removes the last one WITHOUT removing this test — which is the
    // conversation that should happen.
    assert!(
        cachers > 0,
        "scanned {} workflow file(s), {self_hosted} self-hosted job(s), and none \
         caches cargo — the exoneration rule was not exercised by anything",
        files.len()
    );

    assert!(
        violations.is_empty(),
        "{} self-hosted job(s) cache cargo into a SHARED CARGO_HOME \
         ({cachers} cacher(s), {exonerated} exonerated):\n  {}",
        violations.len(),
        violations.join("\n  ")
    );
}
