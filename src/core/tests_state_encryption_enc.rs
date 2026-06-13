//! Encryption-gated tests for `state_encryption.rs` (feature = "encryption").
//!
//! Extracted to a sibling file to keep `state_encryption.rs` under the 500-line
//! source-file limit.

use super::state_encryption::*;

#[test]
fn encrypt_decrypt_roundtrip() {
    let plaintext = b"hello world, this is state data!";
    let passphrase = "test-passphrase-42";
    let ciphertext = encrypt_data(plaintext, passphrase).unwrap();
    assert_ne!(&ciphertext, &plaintext[..]);
    assert!(ciphertext.len() > plaintext.len()); // age adds overhead
    let decrypted = decrypt_data(&ciphertext, passphrase).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn encrypt_decrypt_empty_data() {
    let plaintext = b"";
    let passphrase = "empty-test";
    let ciphertext = encrypt_data(plaintext, passphrase).unwrap();
    let decrypted = decrypt_data(&ciphertext, passphrase).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn decrypt_wrong_passphrase() {
    let plaintext = b"secret state data";
    let ciphertext = encrypt_data(plaintext, "correct-pass").unwrap();
    let result = decrypt_data(&ciphertext, "wrong-pass");
    assert!(result.is_err());
}

#[test]
fn decrypt_corrupted_data() {
    let result = decrypt_data(b"not valid age data", "pass");
    assert!(result.is_err());
}

#[test]
fn encrypt_state_file_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("state.lock.yaml");
    let original = "resources:\n  pkg:\n    state: converged\n";
    std::fs::write(&file, original).unwrap();

    let passphrase = "file-test-pass";

    // Encrypt
    let meta = encrypt_state_file(&file, passphrase).unwrap();
    assert!(is_encrypted(&file));
    assert_eq!(meta.version, 1);
    assert_eq!(meta.plaintext_hash, hash_data(original.as_bytes()));

    let encrypted_content = std::fs::read(&file).unwrap();
    assert_ne!(encrypted_content, original.as_bytes());

    // Decrypt
    let plaintext = decrypt_state_file(&file, passphrase).unwrap();
    assert_eq!(plaintext, original.as_bytes());
    assert!(!is_encrypted(&file)); // sidecar removed
}

#[test]
fn encrypt_state_file_metadata_written() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.lock.yaml");
    std::fs::write(&file, "data").unwrap();

    let meta = encrypt_state_file(&file, "pass").unwrap();
    let loaded = read_metadata(&file).unwrap();
    assert_eq!(loaded.plaintext_hash, meta.plaintext_hash);
    assert_eq!(loaded.ciphertext_hmac, meta.ciphertext_hmac);
}

#[test]
fn decrypt_state_file_wrong_passphrase() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("state.lock.yaml");
    std::fs::write(&file, "secret").unwrap();

    encrypt_state_file(&file, "right-pass").unwrap();
    let result = decrypt_state_file(&file, "wrong-pass");
    assert!(result.is_err());
}

/// FJ-154 (#10): encryption must write atomically (temp + rename), leaving no
/// in-place truncate window and no orphaned temp files.
#[test]
fn encrypt_state_file_uses_atomic_write_no_temp_left() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("state.lock.yaml");
    std::fs::write(&file, "resources: {}\n").unwrap();

    encrypt_state_file(&file, "atomic-test").unwrap();

    // The atomic write must clean up its temp file: no `.tmp` sibling and no
    // `.enc.meta.json.tmp` sidecar temp should remain after a successful run.
    let data_tmp = {
        let mut p = file.as_os_str().to_owned();
        p.push(".tmp");
        std::path::PathBuf::from(p)
    };
    assert!(!data_tmp.exists(), "data temp file leaked: {data_tmp:?}");
    let meta_tmp = {
        let mut p = meta_path_for(&file).as_os_str().to_owned();
        p.push(".tmp");
        std::path::PathBuf::from(p)
    };
    assert!(!meta_tmp.exists(), "meta temp file leaked: {meta_tmp:?}");

    // Metadata sidecar exists (written before the data swap).
    assert!(is_encrypted(&file));
    // And the on-disk file is real ciphertext (single atomic rename, not a
    // truncated in-place write).
    let on_disk = std::fs::read(&file).unwrap();
    assert_ne!(on_disk, b"resources: {}\n");
}

/// FJ-154 (#10): round-trip preserves content and metadata across the atomic
/// encrypt → decrypt path.
#[test]
fn encrypt_decrypt_roundtrip_preserves_content_and_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("state.lock.yaml");
    let original = "resources:\n  pkg:\n    state: converged\n";
    std::fs::write(&file, original).unwrap();

    let meta = encrypt_state_file(&file, "round-trip").unwrap();
    // Metadata persisted and matches the freshly computed plaintext hash.
    let loaded = read_metadata(&file).unwrap();
    assert_eq!(loaded.plaintext_hash, meta.plaintext_hash);
    assert_eq!(loaded.plaintext_hash, hash_data(original.as_bytes()));
    assert_eq!(loaded.ciphertext_hmac, meta.ciphertext_hmac);

    let plaintext = decrypt_state_file(&file, "round-trip").unwrap();
    assert_eq!(plaintext, original.as_bytes());
    assert!(!is_encrypted(&file)); // sidecar removed after decrypt
}

#[test]
fn decrypt_state_file_missing_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("state.lock.yaml");
    std::fs::write(&file, "data").unwrap();
    // No metadata sidecar
    let result = decrypt_state_file(&file, "pass");
    assert!(result.is_err());
}
