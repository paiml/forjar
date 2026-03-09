//! Additional coverage tests for pq_signing.rs — uncovered cmd branches.

use super::pq_signing::*;

// ── cmd_dual_sign — sign mode ────────────────────────────────────────

#[test]
fn cmd_sign_text_output() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("recipe.yaml");
    std::fs::write(&f, "hello world").unwrap();
    let result = cmd_dual_sign(&f, false, Some("ci-bot"), false);
    assert!(result.is_ok());
    // sig file should exist
    assert!(f.with_extension("dual-sig.json").exists());
}

#[test]
fn cmd_sign_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("config.yaml");
    std::fs::write(&f, "version: 1").unwrap();
    let result = cmd_dual_sign(&f, false, Some("ci"), true);
    assert!(result.is_ok());
}

#[test]
fn cmd_sign_default_signer() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("data.yaml");
    std::fs::write(&f, "test content").unwrap();
    // None signer defaults to "local"
    let result = cmd_dual_sign(&f, false, None, false);
    assert!(result.is_ok());
}

// ── cmd_dual_sign — verify-only mode ─────────────────────────────────

#[test]
fn cmd_verify_only_text_valid() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("signed.yaml");
    std::fs::write(&f, "original").unwrap();
    dual_sign(&f, "signer").unwrap();
    let result = cmd_dual_sign(&f, true, None, false);
    assert!(result.is_ok());
}

#[test]
fn cmd_verify_only_json_valid() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("signed.yaml");
    std::fs::write(&f, "original").unwrap();
    dual_sign(&f, "signer").unwrap();
    let result = cmd_dual_sign(&f, true, None, true);
    assert!(result.is_ok());
}

#[test]
fn cmd_verify_only_text_tampered() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("tampered.yaml");
    std::fs::write(&f, "original").unwrap();
    dual_sign(&f, "signer").unwrap();
    std::fs::write(&f, "tampered!").unwrap();
    let result = cmd_dual_sign(&f, true, None, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("dual verification failed"));
}

#[test]
fn cmd_verify_only_json_tampered() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("tampered.yaml");
    std::fs::write(&f, "original").unwrap();
    dual_sign(&f, "signer").unwrap();
    std::fs::write(&f, "tampered!").unwrap();
    let result = cmd_dual_sign(&f, true, None, true);
    assert!(result.is_err());
}

#[test]
fn cmd_verify_only_no_sig_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("unsigned.yaml");
    std::fs::write(&f, "no sig here").unwrap();
    let result = cmd_dual_sign(&f, true, None, false);
    assert!(result.is_err());
}

// ── dual_sign / dual_verify edge cases ───────────────────────────────

#[test]
fn sign_nonexistent_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("missing.yaml");
    let result = dual_sign(&f, "signer");
    assert!(result.is_err());
}

#[test]
fn sign_empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("empty.yaml");
    std::fs::write(&f, "").unwrap();
    let sig = dual_sign(&f, "signer").unwrap();
    assert_eq!(sig.blake3_hash.len(), 64);
    assert_eq!(sig.classical_sig.len(), 64);
    assert_eq!(sig.pq_sig.len(), 64);
}

#[test]
fn verify_roundtrip_same_signer() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("stable.yaml");
    std::fs::write(&f, "stable content").unwrap();
    let sig1 = dual_sign(&f, "alice").unwrap();
    let result = dual_verify(&f).unwrap();
    assert!(result.both_valid);
    assert_eq!(result.reason, "both signatures valid");

    // Sign again with different signer — hash stays same but sigs differ
    let sig2 = dual_sign(&f, "bob").unwrap();
    assert_eq!(sig1.blake3_hash, sig2.blake3_hash);
    assert_ne!(sig1.classical_sig, sig2.classical_sig);
}

#[test]
fn dual_verify_result_fields() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("fields.yaml");
    std::fs::write(&f, "data").unwrap();
    let result = dual_verify(&f).unwrap();
    // No sig file — should fail
    assert!(!result.both_valid);
    assert!(!result.classical_valid);
    assert!(!result.pq_valid);
    assert_eq!(result.reason, "no dual signature file");
    assert!(result.path.contains("fields.yaml"));
}
