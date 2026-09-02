//! E14 (paiml/forjar#416) — claims outrun behaviour.
//!
//! 1. `forjar prove` must exit non-zero when any obligation is UNKNOWN and must not render UNKNOWN as [PASS].
//! 2. `forjar provenance` must not label an unsigned, non-conformant attestation "SLSA Level 3".
//! 3. `tripwire::chain` is never built and `lock-audit-trail` reports on it, so it is withdrawn.
//!
//! RED under:
//!   - reverting the I9 Unknown bug fix in prove
//!   - reverting the provenance string label
//!   - restoring `lock-audit-trail`
//!
//! We verify that these three behaviours hold true and `forjar` fails or succeeds correctly.

use std::process::{Command, Output};

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

fn run(args: &[&str]) -> Output {
    forjar().args(args).output().expect("spawn forjar")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[test]
fn prove_exits_nonzero_on_unknown() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = dir.path().join("config.yaml");
    std::fs::write(&config, "version: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: local\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: [m1]\n    provider: apt
    packages: [ripgrep]\n").expect("write config");

    let out = run(&["prove", "--file", &config.to_string_lossy()]);
    let out_str = stdout(&out);
    let err_str = stderr(&out);

    assert!(
        !out.status.success(),
        "forjar prove exited 0 with UNKNOWN invariant.\nstdout: {out_str}\nstderr: {err_str}"
    );

    assert!(
        out_str.contains("[UNKNOWN]"),
        "did not print UNKNOWN.\nstdout: {out_str}"
    );

    for line in out_str.lines() {
        if line.contains("UNKNOWN") {
            assert!(
                !line.contains("[PASS]"),
                "UNKNOWN was rendered as PASS on line: {line}"
            );
        }
    }
}

#[test]
fn provenance_does_not_claim_slsa_level_3() {
    let dir = tempfile::tempdir().expect("tempdir");
    let state = dir.path().join("state");
    std::fs::create_dir(&state).expect("mkdir state");

    let config = dir.path().join("config.yaml");
    std::fs::write(&config, "version: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: local\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: [m1]\n    provider: apt
    packages: [ripgrep]\n").expect("write config");

    let lock_path = state.join("m1.lock.yaml");
    std::fs::write(&lock_path, "resources: {}\n").expect("write lock");

    let out = run(&[
        "provenance",
        "--file",
        &config.to_string_lossy(),
        "--state-dir",
        &state.to_string_lossy(),
        "-m",
        "m1",
    ]);
    let out_str = stdout(&out);
    let err_str = stderr(&out);

    assert!(
        out.status.success(),
        "forjar provenance failed.\nstdout: {out_str}\nstderr: {err_str}"
    );

    assert!(
        !out_str.contains("SLSA Level 3"),
        "provenance output still claims SLSA Level 3!\nstdout: {out_str}"
    );
}

#[test]
fn lock_audit_trail_is_withdrawn() {
    let out = run(&["lock-audit-trail"]);
    let err_str = stderr(&out);

    assert!(
        !out.status.success(),
        "lock-audit-trail command succeeded, but it should be withdrawn."
    );

    assert!(
        err_str.contains("unrecognized subcommand") || err_str.contains("Usage:"),
        "Expected unrecognized subcommand error for lock-audit-trail, got: {err_str}"
    );
}
