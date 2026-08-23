//! Falsification: the four lock *integrity* commands must actually MEASURE integrity.
//!
//! `lock-verify`, `lock-integrity`, `lock-validate` and `lock-audit` all exist to
//! answer one question — "does this lock file still match the BLAKE3 `.b3` sidecar
//! written beside it by `save_lock`?" Before this test they answered it without ever
//! reading the sidecar: `verify_machine_lock` only checked `rl.hash.is_empty()`, and
//! `lock-integrity` / `lock-validate` / `lock-audit` never called
//! `verify_state_integrity()` at all. All four printed a green result for a tampered
//! lock body, a deleted sidecar and a deleted lock file.
//!
//! Each command is driven as the REAL BINARY so the assertion is on the process exit
//! code — the thing a caller or a CI gate actually observes — not on an internal
//! return value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A lock body that is structurally valid for every one of the four commands:
/// schema "1", a `forjar` generator, and a well-formed 64-hex `blake3:` hash.
const LOCK_YAML: &str = r#"schema: "1"
machine: web1
hostname: web1.local
generated_at: "2026-01-01T00:00:00Z"
generator: forjar 1.16.0
blake3_version: "1.5"
resources:
  nginx:
    type: package
    status: converged
    applied_at: "2026-01-01T00:00:00Z"
    duration_seconds: 2.5
    hash: blake3:1111111111111111111111111111111111111111111111111111111111111111
"#;

/// The four commands whose entire purpose is integrity.
const INTEGRITY_COMMANDS: [&str; 4] = [
    "lock-verify",
    "lock-integrity",
    "lock-validate",
    "lock-audit",
];

fn lock_path(state_dir: &Path) -> PathBuf {
    state_dir.join("web1").join("state.lock.yaml")
}

fn sidecar_path(state_dir: &Path) -> PathBuf {
    state_dir.join("web1").join("state.lock.yaml.b3")
}

/// Build a state dir holding one machine lock plus the BLAKE3 sidecar that
/// `save_lock` would have written for it.
fn seal_state(state_dir: &Path, body: &str) {
    let lock = lock_path(state_dir);
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    std::fs::write(&lock, body).unwrap();
    // Dogfood the real sidecar writer rather than reimplementing it in the test.
    forjar::core::state::integrity::write_b3_sidecar(&lock).unwrap();
}

/// Run one forjar subcommand against `state_dir`; return (success, combined output).
fn run(subcommand: &str, state_dir: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .arg(subcommand)
        .arg("--state-dir")
        .arg(state_dir)
        .env("NO_COLOR", "1")
        .output()
        .expect("forjar binary should run");
    let mut combined = String::from_utf8_lossy(&out.stdout).to_string();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), combined)
}

fn assert_all_fail(state_dir: &Path, scenario: &str) {
    for cmd in INTEGRITY_COMMANDS {
        let (ok, output) = run(cmd, state_dir);
        assert!(
            !ok,
            "`forjar {cmd}` exited 0 for {scenario} — it reported a result it did not \
             measure. Output:\n{output}"
        );
    }
}

fn assert_all_pass(state_dir: &Path, scenario: &str) {
    for cmd in INTEGRITY_COMMANDS {
        let (ok, output) = run(cmd, state_dir);
        assert!(
            ok,
            "`forjar {cmd}` exited non-zero for {scenario} — a sealed, untampered lock \
             must verify. Output:\n{output}"
        );
    }
}

/// Baseline: a lock whose sidecar matches must pass all four commands.
/// Without this the other four tests could pass by always failing.
#[test]
fn untampered_lock_verifies() {
    let td = tempfile::tempdir().unwrap();
    seal_state(td.path(), LOCK_YAML);
    assert_all_pass(td.path(), "an untampered, correctly sealed lock");
}

/// Tampering with the lock BODY after sealing must be caught by recomputing BLAKE3.
/// The body stays valid YAML and structurally valid — only the bytes changed, so the
/// sidecar is the only witness.
#[test]
fn tampered_lock_body_is_caught() {
    let td = tempfile::tempdir().unwrap();
    seal_state(td.path(), LOCK_YAML);

    let tampered = LOCK_YAML.replace(
        "hash: blake3:1111111111111111111111111111111111111111111111111111111111111111",
        "hash: blake3:2222222222222222222222222222222222222222222222222222222222222222",
    );
    assert_ne!(tampered, LOCK_YAML, "the tamper must actually change bytes");
    std::fs::write(lock_path(td.path()), &tampered).unwrap();

    assert_all_fail(td.path(), "a lock body tampered after sealing");
}

/// A zeroed resource hash is both an empty-hash defect AND a body change, so every
/// command has two independent ways to see it. None may exit 0.
#[test]
fn zeroed_resource_hash_is_caught() {
    let td = tempfile::tempdir().unwrap();
    seal_state(td.path(), LOCK_YAML);

    let zeroed = LOCK_YAML.replace(
        "hash: blake3:1111111111111111111111111111111111111111111111111111111111111111",
        r#"hash: """#,
    );
    assert_ne!(zeroed, LOCK_YAML, "the tamper must actually change bytes");
    std::fs::write(lock_path(td.path()), &zeroed).unwrap();

    assert_all_fail(td.path(), "a lock with a zeroed resource hash");
}

/// Deleting the sidecar removes the only instrument these commands have. An absent
/// verifier is a NO-GO, never a pass.
#[test]
fn deleted_sidecar_is_caught() {
    let td = tempfile::tempdir().unwrap();
    seal_state(td.path(), LOCK_YAML);
    std::fs::remove_file(sidecar_path(td.path())).unwrap();

    assert_all_fail(td.path(), "a lock whose BLAKE3 sidecar was deleted");
}

/// Deleting the lock while its sidecar survives is proof a lock existed and is gone.
/// Walking only the locks that still exist makes this scenario invisible.
#[test]
fn deleted_lock_is_caught() {
    let td = tempfile::tempdir().unwrap();
    seal_state(td.path(), LOCK_YAML);
    std::fs::remove_file(lock_path(td.path())).unwrap();
    assert!(
        sidecar_path(td.path()).exists(),
        "the orphaned sidecar must survive for this scenario to mean anything"
    );

    assert_all_fail(td.path(), "a deleted lock whose sidecar survives");
}
