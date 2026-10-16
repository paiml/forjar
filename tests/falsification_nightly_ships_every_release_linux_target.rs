//! forjar#624: the nightly must publish every Linux target a release publishes.
//!
//! # The defect
//!
//! `release.yml` builds static `*-unknown-linux-musl` binaries, and fleet-bins
//! installs those, because the `-gnu` legs build on the fleet host (glibc 2.39)
//! and die on lambda (glibc 2.35) with ``version `GLIBC_2.39' not found``.
//! `nightly.yml` built the gnu legs only, so on lambda every nightly was a
//! WONT-RUN failure, and the fleet-bins andon flapped on it every poll.
//!
//! # Why this compares the two matrices instead of naming the targets
//!
//! A list of expected targets would be a third hand-maintained list next to
//! the two workflows. The release matrix is the source; the nightly has to
//! cover it, so a Linux target added to releases and not to the nightly is RED.
//!
//! # mutations — each measured to redden this test
//!
//! 1. Delete the `x86_64-unknown-linux-musl` leg from `nightly.yml` → RED.
//! 2. Delete the `aarch64-unknown-linux-musl` leg from `nightly.yml` → RED.

use serde_yaml_ng::Value;

fn matrix_targets(file: &str, job: &str) -> Vec<String> {
    let path = format!("{}/.github/workflows/{file}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let wf: Value = serde_yaml_ng::from_str(&text).unwrap_or_else(|e| panic!("parse {path}: {e}"));
    wf.get("jobs")
        .and_then(|j| j.get(job))
        .and_then(|j| j.get("strategy"))
        .and_then(|s| s.get("matrix"))
        .and_then(|m| m.get("include"))
        .and_then(Value::as_sequence)
        .unwrap_or_else(|| panic!("{file} has no jobs.{job}.strategy.matrix.include"))
        .iter()
        .filter_map(|leg| leg.get("target").and_then(Value::as_str).map(str::to_owned))
        .collect()
}

#[test]
fn nightly_builds_every_linux_target_a_release_builds() {
    let release: Vec<String> = matrix_targets("release.yml", "build-binaries")
        .into_iter()
        .filter(|t| t.contains("-linux-"))
        .collect();
    // Vacuity guard: the comparison means nothing over an empty or gnu-only set.
    assert!(
        release.iter().any(|t| t.ends_with("-musl")),
        "release.yml build-binaries has no musl target; read {release:?}"
    );
    let nightly = matrix_targets("nightly.yml", "build");
    let missing: Vec<&String> = release.iter().filter(|t| !nightly.contains(t)).collect();
    assert!(
        missing.is_empty(),
        "nightly.yml does not build {missing:?}, which release.yml ships (forjar#624: \
         gnu nightlies need GLIBC_2.39 and do not run on lambda); nightly builds {nightly:?}"
    );
}
