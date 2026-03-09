//! FJ-3303: State encryption falsification.
//!
//! Popperian rejection criteria for:
//! - BLAKE3 hash determinism and collision resistance
//! - Keyed HMAC computation and verification
//! - Key derivation from passphrase
//! - Encryption metadata creation and verification
//! - Sidecar file read/write
//! - Encrypted file detection and listing
//! - EncryptionStatus properties
//!
//! Usage: cargo test --test falsification_state_encryption

use forjar::core::state_encryption::{
    create_metadata, derive_key, hash_data, is_encrypted, keyed_hash, list_encrypted,
    read_metadata, verify_keyed_hash, verify_metadata, write_metadata, EncryptionMeta,
    EncryptionStatus,
};
use std::path::Path;

// ============================================================================
// FJ-3303: BLAKE3 Hash
// ============================================================================

#[test]
fn hash_deterministic() {
    let h1 = hash_data(b"test data");
    let h2 = hash_data(b"test data");
    assert_eq!(h1, h2);
}

#[test]
fn hash_length_64_hex() {
    let h = hash_data(b"any data");
    assert_eq!(h.len(), 64, "BLAKE3 hash should be 64 hex chars");
}

#[test]
fn hash_different_inputs_differ() {
    let h1 = hash_data(b"data-a");
    let h2 = hash_data(b"data-b");
    assert_ne!(h1, h2);
}

#[test]
fn hash_empty_input() {
    let h = hash_data(b"");
    assert_eq!(h.len(), 64);
    // BLAKE3 of empty input is a known value
    let h2 = hash_data(b"");
    assert_eq!(h, h2);
}

#[test]
fn hash_large_input() {
    let data = vec![0xABu8; 1024 * 1024]; // 1MB
    let h = hash_data(&data);
    assert_eq!(h.len(), 64);
}

// ============================================================================
// FJ-3303: Keyed HMAC
// ============================================================================

#[test]
fn keyed_hash_deterministic() {
    let key = derive_key("passphrase");
    let h1 = keyed_hash(b"data", &key);
    let h2 = keyed_hash(b"data", &key);
    assert_eq!(h1, h2);
}

#[test]
fn keyed_hash_length_64() {
    let key = derive_key("key");
    let h = keyed_hash(b"data", &key);
    assert_eq!(h.len(), 64);
}

#[test]
fn keyed_hash_different_keys_differ() {
    let k1 = derive_key("key-a");
    let k2 = derive_key("key-b");
    let h1 = keyed_hash(b"same data", &k1);
    let h2 = keyed_hash(b"same data", &k2);
    assert_ne!(h1, h2);
}

#[test]
fn keyed_hash_different_data_differ() {
    let key = derive_key("same-key");
    let h1 = keyed_hash(b"data-a", &key);
    let h2 = keyed_hash(b"data-b", &key);
    assert_ne!(h1, h2);
}

#[test]
fn verify_keyed_hash_valid() {
    let key = derive_key("pass");
    let h = keyed_hash(b"data", &key);
    assert!(verify_keyed_hash(b"data", &key, &h));
}

#[test]
fn verify_keyed_hash_tampered_data() {
    let key = derive_key("pass");
    let h = keyed_hash(b"original", &key);
    assert!(!verify_keyed_hash(b"tampered", &key, &h));
}

#[test]
fn verify_keyed_hash_wrong_key() {
    let k1 = derive_key("key-a");
    let k2 = derive_key("key-b");
    let h = keyed_hash(b"data", &k1);
    assert!(!verify_keyed_hash(b"data", &k2, &h));
}

#[test]
fn verify_keyed_hash_wrong_hmac() {
    let key = derive_key("pass");
    assert!(!verify_keyed_hash(
        b"data",
        &key,
        "0000000000000000000000000000000000000000000000000000000000000000"
    ));
}

// ============================================================================
// FJ-3303: Key Derivation
// ============================================================================

#[test]
fn derive_key_deterministic() {
    let k1 = derive_key("my-passphrase");
    let k2 = derive_key("my-passphrase");
    assert_eq!(k1, k2);
}

#[test]
fn derive_key_different_passphrases_differ() {
    let k1 = derive_key("passphrase-a");
    let k2 = derive_key("passphrase-b");
    assert_ne!(k1, k2);
}

#[test]
fn derive_key_32_bytes() {
    let key = derive_key("test");
    assert_eq!(key.len(), 32);
}

#[test]
fn derive_key_empty_passphrase() {
    let k1 = derive_key("");
    let k2 = derive_key("");
    assert_eq!(k1, k2);
    assert_eq!(k1.len(), 32);
}

// ============================================================================
// FJ-3303: Encryption Metadata
// ============================================================================

#[test]
fn create_metadata_version_1() {
    let key = derive_key("test");
    let meta = create_metadata(b"plain", b"cipher", &key);
    assert_eq!(meta.version, 1);
}

#[test]
fn create_metadata_stores_plaintext_hash() {
    let key = derive_key("test");
    let plaintext = b"state data";
    let meta = create_metadata(plaintext, b"cipher", &key);
    assert_eq!(meta.plaintext_hash, hash_data(plaintext));
}

#[test]
fn create_metadata_stores_ciphertext_hmac() {
    let key = derive_key("test");
    let ciphertext = b"encrypted bytes";
    let meta = create_metadata(b"plain", ciphertext, &key);
    assert_eq!(meta.ciphertext_hmac, keyed_hash(ciphertext, &key));
}

#[test]
fn create_metadata_has_timestamp() {
    let key = derive_key("test");
    let meta = create_metadata(b"p", b"c", &key);
    assert!(!meta.encrypted_at.is_empty());
}

#[test]
fn verify_metadata_valid() {
    let key = derive_key("test");
    let ciphertext = b"encrypted";
    let meta = create_metadata(b"plain", ciphertext, &key);
    assert!(verify_metadata(&meta, ciphertext, &key));
}

#[test]
fn verify_metadata_tampered_ciphertext() {
    let key = derive_key("test");
    let meta = create_metadata(b"plain", b"original-cipher", &key);
    assert!(!verify_metadata(&meta, b"tampered-cipher", &key));
}

#[test]
fn verify_metadata_wrong_key() {
    let k1 = derive_key("key-a");
    let k2 = derive_key("key-b");
    let ciphertext = b"cipher";
    let meta = create_metadata(b"plain", ciphertext, &k1);
    assert!(!verify_metadata(&meta, ciphertext, &k2));
}

// ============================================================================
// FJ-3303: Metadata Serde
// ============================================================================

#[test]
fn metadata_serde_roundtrip() {
    let key = derive_key("test");
    let meta = create_metadata(b"plain", b"cipher", &key);
    let json = serde_json::to_string(&meta).unwrap();
    let parsed: EncryptionMeta = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.version, meta.version);
    assert_eq!(parsed.plaintext_hash, meta.plaintext_hash);
    assert_eq!(parsed.ciphertext_hmac, meta.ciphertext_hmac);
    assert_eq!(parsed.encrypted_at, meta.encrypted_at);
}

// ============================================================================
// FJ-3303: Sidecar File Operations
// ============================================================================

#[test]
fn write_and_read_metadata_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("state.lock.yaml");
    let key = derive_key("test");
    let meta = create_metadata(b"plain", b"cipher", &key);

    write_metadata(&file_path, &meta).unwrap();
    let loaded = read_metadata(&file_path).unwrap();
    assert_eq!(loaded.plaintext_hash, meta.plaintext_hash);
    assert_eq!(loaded.ciphertext_hmac, meta.ciphertext_hmac);
    assert_eq!(loaded.version, meta.version);
}

#[test]
fn read_metadata_missing_file() {
    let result = read_metadata(Path::new("/nonexistent/path/state.yaml"));
    assert!(result.is_err());
}

#[test]
fn write_metadata_creates_sidecar() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("state.lock.yaml");
    let key = derive_key("test");
    let meta = create_metadata(b"p", b"c", &key);

    write_metadata(&file_path, &meta).unwrap();
    let sidecar = dir.path().join("state.lock.yaml.enc.meta.json");
    assert!(sidecar.exists());
}

// ============================================================================
// FJ-3303: Encrypted File Detection
// ============================================================================

#[test]
fn is_encrypted_false_no_sidecar() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("state.lock.yaml");
    std::fs::write(&file, "data").unwrap();
    assert!(!is_encrypted(&file));
}

#[test]
fn is_encrypted_true_with_sidecar() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("state.lock.yaml");
    std::fs::write(&file, "data").unwrap();
    let key = derive_key("test");
    let meta = create_metadata(b"p", b"c", &key);
    write_metadata(&file, &meta).unwrap();
    assert!(is_encrypted(&file));
}

#[test]
fn is_encrypted_nonexistent_file() {
    assert!(!is_encrypted(Path::new("/nonexistent/file.yaml")));
}

// ============================================================================
// FJ-3303: List Encrypted Files
// ============================================================================

#[test]
fn list_encrypted_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let files = list_encrypted(dir.path());
    assert!(files.is_empty());
}

#[test]
fn list_encrypted_finds_sidecars() {
    let dir = tempfile::tempdir().unwrap();
    // Create metadata sidecar
    let meta_path = dir.path().join("lock.yaml.enc.meta.json");
    std::fs::write(&meta_path, "{}").unwrap();
    let files = list_encrypted(dir.path());
    assert_eq!(files.len(), 1);
}

#[test]
fn list_encrypted_multiple_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("state1.yaml.enc.meta.json"), "{}").unwrap();
    std::fs::write(dir.path().join("state2.yaml.enc.meta.json"), "{}").unwrap();
    let files = list_encrypted(dir.path());
    assert_eq!(files.len(), 2);
}

#[test]
fn list_encrypted_sorted() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("b.yaml.enc.meta.json"), "{}").unwrap();
    std::fs::write(dir.path().join("a.yaml.enc.meta.json"), "{}").unwrap();
    let files = list_encrypted(dir.path());
    assert_eq!(files.len(), 2);
    assert!(files[0] < files[1], "should be sorted");
}

#[test]
fn list_encrypted_ignores_non_sidecar() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("state.yaml"), "data").unwrap();
    std::fs::write(dir.path().join("config.json"), "{}").unwrap();
    let files = list_encrypted(dir.path());
    assert!(files.is_empty());
}

// ============================================================================
// FJ-3303: EncryptionStatus
// ============================================================================

#[test]
fn status_fully_encrypted() {
    let status = EncryptionStatus {
        total_files: 3,
        encrypted_count: 3,
        unencrypted_count: 0,
        integrity_failures: 0,
    };
    assert!(status.fully_encrypted());
}

#[test]
fn status_not_fully_unencrypted() {
    let status = EncryptionStatus {
        total_files: 3,
        encrypted_count: 2,
        unencrypted_count: 1,
        integrity_failures: 0,
    };
    assert!(!status.fully_encrypted());
}

#[test]
fn status_not_fully_integrity_failure() {
    let status = EncryptionStatus {
        total_files: 3,
        encrypted_count: 3,
        unencrypted_count: 0,
        integrity_failures: 1,
    };
    assert!(!status.fully_encrypted());
}

#[test]
fn status_empty() {
    let status = EncryptionStatus {
        total_files: 0,
        encrypted_count: 0,
        unencrypted_count: 0,
        integrity_failures: 0,
    };
    assert!(status.fully_encrypted());
}

#[test]
fn status_debug() {
    let status = EncryptionStatus {
        total_files: 5,
        encrypted_count: 4,
        unencrypted_count: 1,
        integrity_failures: 0,
    };
    let debug = format!("{status:?}");
    assert!(debug.contains("EncryptionStatus"));
}
