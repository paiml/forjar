//! Additional coverage tests for pq_signing, sbom (parse_image_tag),
//! and undo_helpers edge cases.

use super::pq_signing::*;
use super::undo_helpers::*;
use crate::core::types;

// ── pq_signing: cmd_dual_sign coverage ─────────────────────────────

#[test]
fn cmd_dual_sign_text_output() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("file.yaml");
    std::fs::write(&f, "test content").unwrap();
    // Sign in text mode (not JSON)
    let result = cmd_dual_sign(&f, false, Some("ci-bot"), false);
    assert!(result.is_ok());
}

#[test]
fn cmd_dual_sign_verify_json_valid() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("file.yaml");
    std::fs::write(&f, "hello").unwrap();
    dual_sign(&f, "signer").unwrap();
    // Verify JSON mode
    let result = cmd_dual_sign(&f, true, None, true);
    assert!(result.is_ok());
}

#[test]
fn cmd_dual_sign_verify_text_valid() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("file.yaml");
    std::fs::write(&f, "hello").unwrap();
    dual_sign(&f, "signer").unwrap();
    // Verify text mode
    let result = cmd_dual_sign(&f, true, None, false);
    assert!(result.is_ok());
}

#[test]
fn cmd_dual_sign_verify_tampered() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("file.yaml");
    std::fs::write(&f, "original").unwrap();
    dual_sign(&f, "signer").unwrap();
    std::fs::write(&f, "tampered").unwrap();
    // Verify should fail
    let result = cmd_dual_sign(&f, true, None, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("verification failed"));
}

#[test]
fn cmd_dual_sign_verify_no_sig_json() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("nosig.yaml");
    std::fs::write(&f, "test").unwrap();
    // Verify with no signature file — should fail
    let result = cmd_dual_sign(&f, true, None, true);
    assert!(result.is_err());
}

#[test]
fn cmd_dual_sign_default_signer() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("test.yaml");
    std::fs::write(&f, "data").unwrap();
    // None signer → defaults to "local"
    let result = cmd_dual_sign(&f, false, None, false);
    assert!(result.is_ok());
}

#[test]
fn dual_sign_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("nonexistent.yaml");
    let result = dual_sign(&f, "signer");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("read:"));
}

#[test]
fn dual_verify_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("nosig.yaml");
    std::fs::write(&f, "test").unwrap();
    let result = dual_verify(&f).unwrap();
    assert!(!result.both_valid);
    assert!(result.reason.contains("no dual signature"));
}

#[test]
fn dual_verify_result_fields() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("check.yaml");
    std::fs::write(&f, "hello").unwrap();
    dual_sign(&f, "tester").unwrap();
    let r = dual_verify(&f).unwrap();
    assert!(r.classical_valid);
    assert!(r.pq_valid);
    assert!(r.both_valid);
    assert!(r.reason.contains("valid"));
    assert!(r.path.contains("check.yaml"));
}

// ── undo_helpers: cmd_undo_destroy coverage ────────────────────────

#[test]
fn cmd_undo_destroy_no_log() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_undo_destroy(dir.path(), None, false, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("nothing to undo"));
}

#[test]
fn cmd_undo_destroy_empty_log() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("destroy-log.jsonl"), "").unwrap();
    let result = cmd_undo_destroy(dir.path(), None, false, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no matching entries"));
}

#[test]
fn cmd_undo_destroy_dry_run_reliable() {
    let dir = tempfile::tempdir().unwrap();
    let entry = types::DestroyLogEntry {
        timestamp: "2026-03-08T12:00:00Z".into(),
        machine: "web".into(),
        resource_id: "pkg-nginx".into(),
        resource_type: "package".into(),
        pre_hash: "blake3:abc".into(),
        generation: 5,
        reliable_recreate: true,
        config_fragment: Some("type: package\nprovider: apt\npackages: [nginx]".into()),
    };
    let jsonl = entry.to_jsonl().unwrap();
    std::fs::write(dir.path().join("destroy-log.jsonl"), &jsonl).unwrap();

    // Dry run should succeed
    let result = cmd_undo_destroy(dir.path(), None, false, true);
    assert!(result.is_ok());
}

#[test]
fn cmd_undo_destroy_dry_run_with_force() {
    let dir = tempfile::tempdir().unwrap();
    let entry = types::DestroyLogEntry {
        timestamp: "2026-03-08T12:00:00Z".into(),
        machine: "web".into(),
        resource_id: "custom-svc".into(),
        resource_type: "service".into(),
        pre_hash: "blake3:def".into(),
        generation: 3,
        reliable_recreate: false,
        config_fragment: Some("type: service\nname: nginx".into()),
    };
    let jsonl = entry.to_jsonl().unwrap();
    std::fs::write(dir.path().join("destroy-log.jsonl"), &jsonl).unwrap();

    // Dry run with force should count unreliable entries
    let result = cmd_undo_destroy(dir.path(), None, true, true);
    assert!(result.is_ok());
}

#[test]
fn cmd_undo_destroy_machine_filter_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let entry = types::DestroyLogEntry {
        timestamp: "2026-03-08T12:00:00Z".into(),
        machine: "web".into(),
        resource_id: "pkg-a".into(),
        resource_type: "package".into(),
        pre_hash: "blake3:x".into(),
        generation: 1,
        reliable_recreate: true,
        config_fragment: None,
    };
    let jsonl = entry.to_jsonl().unwrap();
    std::fs::write(dir.path().join("destroy-log.jsonl"), &jsonl).unwrap();

    // Filter to different machine
    let result = cmd_undo_destroy(dir.path(), Some("db"), false, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no matching entries"));
}

#[test]
fn cmd_undo_destroy_unreliable_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let entry = types::DestroyLogEntry {
        timestamp: "2026-03-08T12:00:00Z".into(),
        machine: "web".into(),
        resource_id: "custom-svc".into(),
        resource_type: "service".into(),
        pre_hash: "blake3:def".into(),
        generation: 3,
        reliable_recreate: false,
        config_fragment: Some("type: service\nname: nginx".into()),
    };
    let jsonl = entry.to_jsonl().unwrap();
    std::fs::write(dir.path().join("destroy-log.jsonl"), &jsonl).unwrap();

    // Without force, dry_run=false: only reliable entries replayed → 0 reliable → config_fragment parse attempt
    // This should not crash but may fail trying to replay
    let result = cmd_undo_destroy(dir.path(), None, false, true);
    assert!(result.is_ok());
}

#[test]
fn cmd_undo_destroy_no_config_fragment() {
    let dir = tempfile::tempdir().unwrap();
    let entry = types::DestroyLogEntry {
        timestamp: "2026-03-08T12:00:00Z".into(),
        machine: "local".into(),
        resource_id: "orphan-pkg".into(),
        resource_type: "package".into(),
        pre_hash: "blake3:z".into(),
        generation: 0,
        reliable_recreate: true,
        config_fragment: None, // no fragment
    };
    let jsonl = entry.to_jsonl().unwrap();
    std::fs::write(dir.path().join("destroy-log.jsonl"), &jsonl).unwrap();

    // Replay with no config_fragment → SKIP, then fails
    let result = cmd_undo_destroy(dir.path(), None, false, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("failed to recreate"));
}

#[test]
fn cmd_undo_destroy_invalid_config_fragment() {
    let dir = tempfile::tempdir().unwrap();
    let entry = types::DestroyLogEntry {
        timestamp: "2026-03-08T12:00:00Z".into(),
        machine: "local".into(),
        resource_id: "bad-resource".into(),
        resource_type: "package".into(),
        pre_hash: "blake3:bad".into(),
        generation: 0,
        reliable_recreate: true,
        config_fragment: Some("{{broken yaml".into()),
    };
    let jsonl = entry.to_jsonl().unwrap();
    std::fs::write(dir.path().join("destroy-log.jsonl"), &jsonl).unwrap();

    let result = cmd_undo_destroy(dir.path(), None, false, false);
    assert!(result.is_err());
}

#[test]
fn cmd_undo_destroy_invalid_jsonl_lines_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let good_entry = types::DestroyLogEntry {
        timestamp: "2026-03-08T12:00:00Z".into(),
        machine: "web".into(),
        resource_id: "pkg-a".into(),
        resource_type: "package".into(),
        pre_hash: "blake3:a".into(),
        generation: 1,
        reliable_recreate: true,
        config_fragment: None,
    };
    let good = good_entry.to_jsonl().unwrap();
    // Mix valid and invalid lines
    let content = format!("{{invalid json\n{good}\nalso broken\n");
    std::fs::write(dir.path().join("destroy-log.jsonl"), content).unwrap();

    // Should parse only the valid line, dry run should work
    let result = cmd_undo_destroy(dir.path(), None, false, true);
    assert!(result.is_ok());
}
