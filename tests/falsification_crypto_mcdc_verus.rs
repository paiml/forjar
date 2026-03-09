//! FJ-3303/051: State encryption and MC/DC analysis.
//! Usage: cargo test --test falsification_crypto_mcdc_verus

use forjar::core::mcdc::{build_decision, generate_mcdc_and, generate_mcdc_or};
use forjar::core::state_encryption::*;
use std::path::Path;

// ============================================================================
// FJ-3303: hash_data
// ============================================================================

#[test]
fn hash_data_deterministic() {
    let h1 = hash_data(b"forjar state data");
    let h2 = hash_data(b"forjar state data");
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64); // BLAKE3 hex = 64 chars
}

#[test]
fn hash_data_different_inputs() {
    assert_ne!(hash_data(b"alpha"), hash_data(b"beta"));
}

#[test]
fn hash_data_empty() {
    let h = hash_data(b"");
    assert_eq!(h.len(), 64);
}

// ============================================================================
// FJ-3303: keyed_hash / verify_keyed_hash
// ============================================================================

#[test]
fn keyed_hash_roundtrip() {
    let key = derive_key("test-passphrase");
    let hmac = keyed_hash(b"state content", &key);
    assert_eq!(hmac.len(), 64);
    assert!(verify_keyed_hash(b"state content", &key, &hmac));
}

#[test]
fn keyed_hash_tamper_detected() {
    let key = derive_key("passphrase");
    let hmac = keyed_hash(b"original", &key);
    assert!(!verify_keyed_hash(b"tampered", &key, &hmac));
}

#[test]
fn keyed_hash_wrong_key() {
    let key1 = derive_key("key1");
    let key2 = derive_key("key2");
    let hmac = keyed_hash(b"data", &key1);
    assert!(!verify_keyed_hash(b"data", &key2, &hmac));
}

// ============================================================================
// FJ-3303: derive_key
// ============================================================================

#[test]
fn derive_key_deterministic() {
    let k1 = derive_key("my-secret");
    let k2 = derive_key("my-secret");
    assert_eq!(k1, k2);
}

#[test]
fn derive_key_different_passphrases() {
    assert_ne!(derive_key("alpha"), derive_key("beta"));
}

#[test]
fn derive_key_length() {
    let key = derive_key("x");
    assert_eq!(key.len(), 32);
}

// ============================================================================
// FJ-3303: create_metadata / verify_metadata
// ============================================================================

#[test]
fn metadata_create_verify() {
    let key = derive_key("encryption-key");
    let plaintext = b"resource state yaml here";
    let ciphertext = b"encrypted bytes";
    let meta = create_metadata(plaintext, ciphertext, &key);

    assert_eq!(meta.version, 1);
    assert_eq!(meta.plaintext_hash, hash_data(plaintext));
    assert!(verify_metadata(&meta, ciphertext, &key));
}

#[test]
fn metadata_verify_wrong_ciphertext() {
    let key = derive_key("key");
    let meta = create_metadata(b"plain", b"cipher", &key);
    assert!(!verify_metadata(&meta, b"wrong-cipher", &key));
}

#[test]
fn metadata_verify_wrong_key() {
    let key1 = derive_key("key1");
    let key2 = derive_key("key2");
    let meta = create_metadata(b"plain", b"cipher", &key1);
    assert!(!verify_metadata(&meta, b"cipher", &key2));
}

#[test]
fn metadata_serde_roundtrip() {
    let key = derive_key("test");
    let meta = create_metadata(b"p", b"c", &key);
    let json = serde_json::to_string(&meta).unwrap();
    let parsed: EncryptionMeta = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.version, meta.version);
    assert_eq!(parsed.plaintext_hash, meta.plaintext_hash);
    assert_eq!(parsed.ciphertext_hmac, meta.ciphertext_hmac);
}

// ============================================================================
// FJ-3303: write_metadata / read_metadata / is_encrypted
// ============================================================================

#[test]
fn write_read_metadata_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("state.lock.yaml");
    let key = derive_key("test");
    let meta = create_metadata(b"data", b"enc", &key);
    write_metadata(&file, &meta).unwrap();
    let loaded = read_metadata(&file).unwrap();
    assert_eq!(loaded.plaintext_hash, meta.plaintext_hash);
}

#[test]
fn read_metadata_missing() {
    assert!(read_metadata(Path::new("/nonexistent/path.yaml")).is_err());
}

#[test]
fn is_encrypted_false() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("state.yaml");
    std::fs::write(&file, "data").unwrap();
    assert!(!is_encrypted(&file));
}

#[test]
fn is_encrypted_true() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("state.yaml");
    std::fs::write(&file, "data").unwrap();
    let key = derive_key("k");
    write_metadata(&file, &create_metadata(b"p", b"c", &key)).unwrap();
    assert!(is_encrypted(&file));
}

// ============================================================================
// FJ-3303: list_encrypted
// ============================================================================

#[test]
fn list_encrypted_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    assert!(list_encrypted(dir.path()).is_empty());
}

#[test]
fn list_encrypted_with_sidecar() {
    let dir = tempfile::tempdir().unwrap();
    let sidecar = dir.path().join("lock.yaml.enc.meta.json");
    std::fs::write(&sidecar, "{}").unwrap();
    let files = list_encrypted(dir.path());
    assert_eq!(files.len(), 1);
}

// ============================================================================
// FJ-3303: EncryptionStatus
// ============================================================================

#[test]
fn encryption_status_fully_encrypted() {
    let s = EncryptionStatus {
        total_files: 5,
        encrypted_count: 5,
        unencrypted_count: 0,
        integrity_failures: 0,
    };
    assert!(s.fully_encrypted());
}

#[test]
fn encryption_status_not_full() {
    let s = EncryptionStatus {
        total_files: 3,
        encrypted_count: 2,
        unencrypted_count: 1,
        integrity_failures: 0,
    };
    assert!(!s.fully_encrypted());
}

#[test]
fn encryption_status_integrity_failure() {
    let s = EncryptionStatus {
        total_files: 3,
        encrypted_count: 3,
        unencrypted_count: 0,
        integrity_failures: 1,
    };
    assert!(!s.fully_encrypted());
}

// ============================================================================
// FJ-051: MC/DC — build_decision
// ============================================================================

#[test]
fn mcdc_build_decision() {
    let d = build_decision("a && b", &["a", "b"]);
    assert_eq!(d.name, "a && b");
    assert_eq!(d.conditions.len(), 2);
    assert_eq!(d.conditions[0].name, "a");
    assert_eq!(d.conditions[0].index, 0);
    assert_eq!(d.conditions[1].index, 1);
}

// ============================================================================
// FJ-051: MC/DC — generate_mcdc_and
// ============================================================================

#[test]
fn mcdc_and_two_conditions() {
    let d = build_decision("a && b", &["a", "b"]);
    let report = generate_mcdc_and(&d);
    assert_eq!(report.pairs.len(), 2);
    assert_eq!(report.min_tests_needed, 3); // n+1
    assert!(report.coverage_achievable);
    assert_eq!(report.num_conditions, 2);
}

#[test]
fn mcdc_and_three_conditions() {
    let d = build_decision("a && b && c", &["a", "b", "c"]);
    let report = generate_mcdc_and(&d);
    assert_eq!(report.pairs.len(), 3);
    assert_eq!(report.min_tests_needed, 4);
}

#[test]
fn mcdc_and_single() {
    let d = build_decision("a", &["a"]);
    let report = generate_mcdc_and(&d);
    assert_eq!(report.pairs.len(), 1);
    assert_eq!(report.min_tests_needed, 2);
}

#[test]
fn mcdc_and_pair_structure() {
    let d = build_decision("x && y", &["x", "y"]);
    let report = generate_mcdc_and(&d);
    // For AND: true case = all true, false case = one false
    let pair = &report.pairs[0];
    assert!(pair.true_case.iter().all(|&v| v));
    assert!(!pair.false_case.iter().all(|&v| v));
}

// ============================================================================
// FJ-051: MC/DC — generate_mcdc_or
// ============================================================================

#[test]
fn mcdc_or_two_conditions() {
    let d = build_decision("a || b", &["a", "b"]);
    let report = generate_mcdc_or(&d);
    assert_eq!(report.pairs.len(), 2);
    assert!(report.coverage_achievable);
}

#[test]
fn mcdc_or_pair_structure() {
    let d = build_decision("x || y", &["x", "y"]);
    let report = generate_mcdc_or(&d);
    // For OR: true case = one true (rest false), false case = all false
    let pair = &report.pairs[0];
    assert!(pair.true_case.iter().any(|&v| v));
    assert!(!pair.false_case.iter().any(|&v| v)); // all false
}

#[test]
fn mcdc_report_serde() {
    let d = build_decision("p && q", &["p", "q"]);
    let report = generate_mcdc_and(&d);
    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains("coverage_achievable"));
    assert!(json.contains("\"decision\":\"p && q\""));
}

// ============================================================================
// FJ-051: MC/DC — five conditions
// ============================================================================

#[test]
fn mcdc_and_five_conditions() {
    let d = build_decision("a && b && c && d && e", &["a", "b", "c", "d", "e"]);
    let report = generate_mcdc_and(&d);
    assert_eq!(report.pairs.len(), 5);
    assert_eq!(report.min_tests_needed, 6); // n+1
    assert!(report.coverage_achievable);
}

#[test]
fn mcdc_or_three_conditions() {
    let d = build_decision("a || b || c", &["a", "b", "c"]);
    let report = generate_mcdc_or(&d);
    assert_eq!(report.pairs.len(), 3);
    assert_eq!(report.min_tests_needed, 4);
    assert!(report.coverage_achievable);
}

#[test]
fn mcdc_and_pair_condition_names() {
    let d = build_decision("p && q && r", &["p", "q", "r"]);
    let report = generate_mcdc_and(&d);
    let names: Vec<&str> = report.pairs.iter().map(|p| p.condition.as_str()).collect();
    assert!(names.contains(&"p"));
    assert!(names.contains(&"q"));
    assert!(names.contains(&"r"));
}
