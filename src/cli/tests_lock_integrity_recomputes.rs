//! CB-2010: the four lock *integrity* commands must MEASURE integrity.
//!
//! `lock-verify`, `lock-integrity`, `lock-validate` and `lock-audit` all answer one
//! question — "does this lock file still hash to the BLAKE3 `.b3` sidecar `save_lock`
//! wrote beside it?" Before this module they answered it without ever opening the
//! sidecar: `verify_machine_lock` tested only `rl.hash.is_empty()`, and the other
//! three never called `verify_state_integrity()`. Every hash they consulted lives
//! INSIDE the lock body, which is exactly what a tamperer rewrites.
//!
//! `tests/falsification_lock_verify_recomputes.rs` pins the same five scenarios at
//! the process boundary, where the exit code is what a CI gate reads. These pin them
//! at the function boundary as well, because the integration test's verdict is only
//! as good as the binary cargo hands it — and this repo builds into a target
//! directory shared by every worktree, where `debug/forjar` is a single file that a
//! concurrent build can replace. A lib test links the code under test directly and
//! cannot go stale that way.

use super::lock_audit::cmd_lock_audit;
use super::lock_core::{cmd_lock_integrity, cmd_lock_validate};
use super::lock_ops::cmd_lock_verify;
use crate::core::state::integrity;
use std::path::{Path, PathBuf};

/// Structurally valid for all four commands: schema "1", a `forjar` generator, and a
/// well-formed 64-hex `blake3:` hash. Only the sidecar can tell it apart from a
/// tampered copy.
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

fn lock_path(state_dir: &Path) -> PathBuf {
    state_dir.join("web1").join("state.lock.yaml")
}

fn sidecar_path(state_dir: &Path) -> PathBuf {
    state_dir.join("web1").join("state.lock.yaml.b3")
}

/// Write a lock and seal it the way `save_lock` does.
fn seal_state(state_dir: &Path, body: &str) {
    let lock = lock_path(state_dir);
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    std::fs::write(&lock, body).unwrap();
    integrity::write_b3_sidecar(&lock).unwrap();
}

/// Every integrity command must reject `state_dir`.
fn assert_all_reject(state_dir: &Path, scenario: &str) {
    for (name, result) in [
        ("lock-verify", cmd_lock_verify(state_dir, false)),
        ("lock-integrity", cmd_lock_integrity(state_dir, false)),
        ("lock-validate", cmd_lock_validate(state_dir, false)),
        ("lock-audit", cmd_lock_audit(state_dir, false)),
    ] {
        assert!(
            result.is_err(),
            "{name} returned Ok for {scenario} — a result it did not measure"
        );
    }
}

/// Every integrity command must accept `state_dir`.
fn assert_all_accept(state_dir: &Path, scenario: &str) {
    for (name, result) in [
        ("lock-verify", cmd_lock_verify(state_dir, false)),
        ("lock-integrity", cmd_lock_integrity(state_dir, false)),
        ("lock-validate", cmd_lock_validate(state_dir, false)),
        ("lock-audit", cmd_lock_audit(state_dir, false)),
    ] {
        assert!(
            result.is_ok(),
            "{name} rejected {scenario}: {:?}",
            result.err()
        );
    }
}

/// Baseline. Without it the four rejection tests could pass by always failing.
#[test]
fn sealed_lock_verifies() {
    let d = tempfile::tempdir().unwrap();
    seal_state(d.path(), LOCK_YAML);
    assert_all_accept(d.path(), "a sealed, untampered lock");
}

/// The body changes but stays valid YAML and structurally valid — recomputing BLAKE3
/// against the sidecar is the only way to see it.
#[test]
fn tampered_lock_body_is_rejected() {
    let d = tempfile::tempdir().unwrap();
    seal_state(d.path(), LOCK_YAML);
    let tampered = LOCK_YAML.replace("blake3:1111", "blake3:2222");
    assert_ne!(tampered, LOCK_YAML, "the tamper must change bytes");
    std::fs::write(lock_path(d.path()), tampered).unwrap();
    assert_all_reject(d.path(), "a lock body tampered after sealing");
}

/// A zeroed resource hash is both an empty-hash defect and a body change.
#[test]
fn zeroed_resource_hash_is_rejected() {
    let d = tempfile::tempdir().unwrap();
    seal_state(d.path(), LOCK_YAML);
    let zeroed = LOCK_YAML.replace(
        "hash: blake3:1111111111111111111111111111111111111111111111111111111111111111",
        r#"hash: """#,
    );
    assert_ne!(zeroed, LOCK_YAML, "the tamper must change bytes");
    std::fs::write(lock_path(d.path()), zeroed).unwrap();
    assert_all_reject(d.path(), "a lock with a zeroed resource hash");
}

/// No sidecar means no instrument. An absent verifier is a NO-GO, never a pass.
#[test]
fn deleted_sidecar_is_rejected() {
    let d = tempfile::tempdir().unwrap();
    seal_state(d.path(), LOCK_YAML);
    std::fs::remove_file(sidecar_path(d.path())).unwrap();
    assert_all_reject(d.path(), "a lock whose BLAKE3 sidecar was deleted");
}

/// A surviving sidecar proves a lock existed. Walking only the locks that still
/// exist finds nothing to check and calls that success.
#[test]
fn deleted_lock_is_rejected() {
    let d = tempfile::tempdir().unwrap();
    seal_state(d.path(), LOCK_YAML);
    std::fs::remove_file(lock_path(d.path())).unwrap();
    assert!(
        sidecar_path(d.path()).exists(),
        "the orphaned sidecar must survive for this scenario to mean anything"
    );
    assert_all_reject(d.path(), "a deleted lock whose sidecar survives");
}

/// The mismatch must be reported with both digests, not just flagged — an operator
/// who cannot see what the sidecar expected cannot tell tampering from a stale seal.
#[test]
fn mismatch_names_expected_and_actual() {
    let d = tempfile::tempdir().unwrap();
    seal_state(d.path(), LOCK_YAML);
    std::fs::write(
        lock_path(d.path()),
        LOCK_YAML.replace("blake3:1111", "blake3:2222"),
    )
    .unwrap();

    let results = integrity::verify_state_integrity(d.path());
    let reasons = results
        .iter()
        .filter_map(integrity::failure_reason)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        reasons.contains("BLAKE3 mismatch") && reasons.contains("sidecar says"),
        "expected a mismatch naming both digests, got:\n{reasons}"
    );
}

/// `apply` tolerates a missing sidecar (pre-FJ-1270 state dirs still converge), so
/// widening the strict check must not have widened `has_errors` with it.
#[test]
fn apply_still_tolerates_a_missing_sidecar() {
    let d = tempfile::tempdir().unwrap();
    seal_state(d.path(), LOCK_YAML);
    std::fs::remove_file(sidecar_path(d.path())).unwrap();

    let results = integrity::verify_state_integrity(d.path());
    assert!(
        !integrity::has_errors(&results),
        "a missing sidecar must stay a warning for apply, not a hard error"
    );
    assert!(
        !integrity::failure_reasons(&results).is_empty(),
        "…but the integrity commands must still see it as a failure"
    );
}

/// A deleted lock IS a hard error for `apply`: state vanished under a live seal, and
/// converging from "everything missing" would reapply the world.
#[test]
fn apply_refuses_a_lock_deleted_under_its_sidecar() {
    let d = tempfile::tempdir().unwrap();
    seal_state(d.path(), LOCK_YAML);
    std::fs::remove_file(lock_path(d.path())).unwrap();

    let results = integrity::verify_state_integrity(d.path());
    assert!(
        integrity::has_errors(&results),
        "a lock deleted under a surviving sidecar must be a hard error"
    );
}
