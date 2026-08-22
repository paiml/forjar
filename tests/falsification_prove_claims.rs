//! GH-248: what `forjar prove` NAMES must be what `forjar prove` CHECKS.
//!
//! `prove` shipped a proof called `hash-determinism`, documented in the book as
//! "BLAKE3 hashes are deterministic (same resource → same hash)". What it did
//! was emit one resource's `state_query` script twice in the same process and
//! compare the text. That is a real property — it catches `HashMap` iteration
//! order leaking into generated shell — but it is a property of *forjar's
//! codegen*, not of the user's build. A task with a genuinely non-deterministic
//! generator passed `hash-determinism` and then produced different artifact
//! bytes on the next `apply`.
//!
//! These tests run the real binary and read the real output, because the claim
//! under test is the one a user acts on: the printed proof name and the
//! `N/N proofs passed` aggregate.

use std::process::Command;

fn prove_output(json: bool) -> String {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("forjar.yaml");
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(
        &config,
        r#"version: '1.0'
name: prove-claims
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  pkgs:
    type: package
    provider: apt
    machine: local
    packages:
      - curl
  conf:
    type: file
    machine: local
    path: /etc/forjar-prove-claims.conf
    content: "hello"
    depends_on:
      - pkgs
"#,
    )
    .unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_forjar"));
    cmd.arg("prove")
        .arg("-f")
        .arg(&config)
        .arg("--state-dir")
        .arg(&state);
    if json {
        cmd.arg("--json");
    }
    let out = cmd.output().expect("forjar prove must run");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn prove_no_longer_claims_hash_determinism() {
    // The regression itself. If this name comes back, so does the false claim.
    let out = prove_output(false);
    assert!(
        !out.contains("hash-determinism"),
        "`hash-determinism` implies build reproducibility, which nothing in \
         `prove` checks (GH-248). Output was:\n{out}"
    );
}

#[test]
fn prove_reports_codegen_determinism_by_its_real_name() {
    let out = prove_output(false);
    assert!(
        out.contains("codegen-determinism"),
        "the determinism proof must still run, under the name of what it \
         actually checks. Output was:\n{out}"
    );
}

#[test]
fn the_determinism_proof_states_what_it_does_not_prove() {
    // The aggregate line ("N/N proofs passed") is what a reader trusts. The
    // per-proof detail is the only place the scope can be narrowed, so it has
    // to carry the disclaimer rather than leaving it to the name alone.
    let out = prove_output(false);
    let line = out
        .lines()
        .find(|l| l.contains("codegen-determinism"))
        .unwrap_or_else(|| panic!("no codegen-determinism line in:\n{out}"));
    assert!(
        line.contains("does NOT prove") || line.contains("does not prove"),
        "detail must disclaim build reproducibility: {line}"
    );
}

#[test]
fn the_determinism_proof_covers_check_and_apply_not_only_state_query() {
    // check and apply are hashed into desired state alongside state_query, so
    // nondeterminism in either produces the same phantom-drift failure. The
    // original sampled state_query only.
    let out = prove_output(false);
    let line = out
        .lines()
        .find(|l| l.contains("codegen-determinism"))
        .unwrap_or_else(|| panic!("no codegen-determinism line in:\n{out}"));
    for phase in ["check", "apply", "state_query"] {
        assert!(
            line.contains(phase),
            "determinism proof must cover the {phase} phase: {line}"
        );
    }
}

#[test]
fn prove_json_output_uses_the_same_name() {
    // A renamed proof that kept the old key in JSON would leave every
    // machine-readable consumer on the false claim.
    let out = prove_output(true);
    assert!(
        !out.contains("hash-determinism"),
        "JSON output still carries the old name:\n{out}"
    );
    assert!(
        out.contains("codegen-determinism"),
        "JSON output is missing the determinism proof:\n{out}"
    );
}
