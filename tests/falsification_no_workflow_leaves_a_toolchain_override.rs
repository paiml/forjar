//! forjar#566: no CI job may leave a rustup DIRECTORY OVERRIDE behind in a
//! runner's workspace.
//!
//! # The defect
//!
//! `msrv.yml` ran `rustup override set 1.89.0`. A directory override is stored
//! against the workspace PATH and never expires, and the `data/actions-runner-N`
//! workspaces are reused across jobs. A directory override also OUTRANKS
//! `rust-toolchain.toml`. So after the MSRV job ran on a runner, every later
//! forjar job on that runner silently used 1.89.0 instead of the pinned 1.93.0.
//!
//! Most of them passed — the code compiles on its MSRV — which is exactly what
//! kept it invisible for as long as it existed. The two that could not pass
//! were the only symptoms anyone saw: `lint` (1.89.0 carries no clippy,
//! forjar#567) and `mutation` (cargo-mutants 27.1.0 needs rustc 1.91). Both
//! were first diagnosed as something else — "some runners lack the component",
//! "the fleet's 1.89 toolchain" — because the cause lived in a third workflow.
//! Measured on main at 7bfb5720:
//!
//! ```text
//! info: default toolchain set to stable -- rustc 1.98.1
//! info: note that the toolchain '1.89.0' is currently in use
//!       (directory override for '/home/noah/data/actions-runner-2/_work/forjar/forjar')
//! ```
//!
//! Ephemeral `eph-build*` runners get a fresh workspace per job, which is why
//! the failures looked intermittent rather than tied to one workflow.
//!
//! # mutations — each measured to redden exactly its own test
//!
//! 1. Restore `rustup override set 1.89.0` in msrv.yml's install step →
//!    `no_workflow_writes_a_directory_override` goes RED alone.
//! 2. Change msrv.yml's `RUSTUP_TOOLCHAIN` to `1.88.0` →
//!    `msrv_selects_its_toolchain_per_job_and_matches_rust_version` goes RED alone.
//! 3. Delete `rustup override unset` from mutation.yml's install step →
//!    `mutation_clears_a_stale_override_before_installing` goes RED alone.

use serde_yaml_ng::Value;

fn workflows_dir() -> String {
    format!("{}/.github/workflows", env!("CARGO_MANIFEST_DIR"))
}

fn workflow(name: &str) -> Value {
    let path = format!("{}/{name}", workflows_dir());
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_yaml_ng::from_str(&text).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

/// Every `run:` block of a job, one string per step.
fn runs(job: &Value) -> Vec<&str> {
    job.get("steps")
        .and_then(Value::as_sequence)
        .map(|s| {
            s.iter()
                .filter_map(|st| st.get("run").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default()
}

/// The non-comment shell lines of a `run:` block, trimmed.
fn commands(run: &str) -> impl Iterator<Item = &str> {
    run.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
}

#[test]
fn no_workflow_writes_a_directory_override() {
    // A comment explaining the old command must not trip this, and a mention
    // inside an echo must not either: only a line that IS the command counts.
    let mut scanned = 0;
    for entry in std::fs::read_dir(workflows_dir()).expect("workflows dir") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("yml") {
            continue;
        }
        let file = path.file_name().unwrap().to_string_lossy().to_string();
        let wf = workflow(&file);
        let Some(jobs) = wf.get("jobs").and_then(Value::as_mapping) else {
            continue;
        };
        for (id, job) in jobs {
            scanned += 1;
            for run in runs(job) {
                for line in commands(run) {
                    assert!(
                        !line.starts_with("rustup override set"),
                        "{file} job `{}` writes a rustup DIRECTORY OVERRIDE (`{line}`), which \
                         outlives the job on a reused workspace and outranks rust-toolchain.toml for \
                         every job that follows on that runner (forjar#566). Select the toolchain for \
                         the job with RUSTUP_TOOLCHAIN or `cargo +<toolchain>` instead.",
                        id.as_str().unwrap_or("?")
                    );
                }
            }
        }
    }
    assert!(
        scanned > 10,
        "scanned only {scanned} jobs; the scan may be reading nothing"
    );
}

#[test]
fn msrv_selects_its_toolchain_per_job_and_matches_rust_version() {
    let wf = workflow("msrv.yml");
    let job = wf
        .get("jobs")
        .and_then(|j| j.get("msrv"))
        .expect("msrv.yml has an `msrv` job");
    let env_tc = job
        .get("env")
        .and_then(|e| e.get("RUSTUP_TOOLCHAIN"))
        .and_then(|v| {
            v.as_str()
                .map(str::to_string)
                .or_else(|| v.as_f64().map(|f| f.to_string()))
        })
        .expect(
            "the msrv job does not set RUSTUP_TOOLCHAIN, so it has to select its toolchain some \
             other way -- and the other way it used was a persistent directory override (forjar#566)",
        );

    let manifest = std::fs::read_to_string(format!("{}/Cargo.toml", env!("CARGO_MANIFEST_DIR")))
        .expect("Cargo.toml");
    let rust_version = manifest
        .lines()
        .find_map(|l| l.trim().strip_prefix("rust-version = \""))
        .and_then(|rest| rest.split('"').next())
        .expect("Cargo.toml declares rust-version");

    assert_eq!(
        env_tc, rust_version,
        "msrv.yml tests toolchain {env_tc} but Cargo.toml declares rust-version = {rust_version}; \
         the MSRV job is not testing the MSRV this crate claims"
    );
}

#[test]
fn mutation_clears_a_stale_override_before_installing() {
    // Runners that ran the MSRV job before it stopped writing an override still
    // carry one. mutation must clear it before `cargo install`, or cargo-mutants
    // builds under 1.89.0 and fails, as it did on main at 7bfb5720.
    let wf = workflow("mutation.yml");
    let job = wf
        .get("jobs")
        .and_then(|j| j.get("mutation"))
        .expect("mutation.yml has a `mutation` job");
    let install = runs(job)
        .into_iter()
        .find(|r| commands(r).any(|l| l.starts_with("cargo install cargo-mutants")))
        .expect("mutation.yml installs cargo-mutants in some step");
    let lines: Vec<&str> = commands(install).collect();
    let unset = lines.iter().position(|l| *l == "rustup override unset");
    let cargo = lines
        .iter()
        .position(|l| l.starts_with("cargo install cargo-mutants"));
    assert!(
        matches!((unset, cargo), (Some(u), Some(c)) if u < c),
        "mutation.yml's install step does not run `rustup override unset` before \
         `cargo install cargo-mutants`, so a stale override pins it to 1.89.0 (forjar#566):\n{install}"
    );
}
