//! FJ-3304: `forjar state encrypt/decrypt` CLI handler.
//!
//! Encrypts and decrypts state files using BLAKE3 key derivation
//! and integrity verification.

use crate::core::state_encryption::*;
use std::path::Path;

/// Encrypt a state file in-place with BLAKE3 integrity metadata.
///
/// The file is XOR-masked with a BLAKE3-derived key stream (lightweight
/// encryption for at-rest protection). Full age encryption is Phase 2.
pub fn cmd_state_encrypt(state_dir: &Path, passphrase: &str, json: bool) -> Result<(), String> {
    let key = derive_key(passphrase);

    let lock_files = find_lock_files(state_dir)?;
    if lock_files.is_empty() {
        if json {
            println!("{{\"encrypted\": 0, \"skipped\": 0}}");
        } else {
            println!("No state files found in {}", state_dir.display());
        }
        return Ok(());
    }

    let mut encrypted = 0;
    let mut skipped = 0;

    for file in &lock_files {
        if is_encrypted(file) {
            skipped += 1;
            continue;
        }

        let plaintext = std::fs::read(file).map_err(|e| format!("read {}: {e}", file.display()))?;

        // XOR-mask with BLAKE3 key stream
        let ciphertext = xor_mask(&plaintext, &key);

        // Write encrypted content
        std::fs::write(file, &ciphertext).map_err(|e| format!("write {}: {e}", file.display()))?;

        // Write metadata sidecar
        let meta = create_metadata(&plaintext, &ciphertext, &key);
        write_metadata(file, &meta)?;

        encrypted += 1;
    }

    if json {
        println!("{{\"encrypted\": {encrypted}, \"skipped\": {skipped}}}");
    } else {
        println!("Encrypted {encrypted} file(s), skipped {skipped} already-encrypted");
    }

    Ok(())
}

/// Decrypt state files encrypted with `forjar state encrypt`.
pub fn cmd_state_decrypt(state_dir: &Path, passphrase: &str, json: bool) -> Result<(), String> {
    let key = derive_key(passphrase);

    let lock_files = find_lock_files(state_dir)?;
    let mut decrypted = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for file in &lock_files {
        if !is_encrypted(file) {
            skipped += 1;
            continue;
        }

        // Verify integrity
        let meta = read_metadata(file)?;
        let ciphertext =
            std::fs::read(file).map_err(|e| format!("read {}: {e}", file.display()))?;

        if !verify_metadata(&meta, &ciphertext, &key) {
            errors += 1;
            if !json {
                println!("  INTEGRITY FAIL: {}", file.display());
            }
            continue;
        }

        // XOR-unmask (symmetric)
        let plaintext = xor_mask(&ciphertext, &key);

        // Verify plaintext hash
        if hash_data(&plaintext) != meta.plaintext_hash {
            errors += 1;
            if !json {
                println!("  HASH MISMATCH: {}", file.display());
            }
            continue;
        }

        // Write decrypted content
        std::fs::write(file, &plaintext).map_err(|e| format!("write {}: {e}", file.display()))?;

        // Remove metadata sidecar
        let meta_path = file.as_os_str().to_owned();
        let mut meta_file = std::path::PathBuf::from(meta_path);
        let name = meta_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        meta_file.set_file_name(format!("{name}.enc.meta.json"));
        let _ = std::fs::remove_file(&meta_file);

        decrypted += 1;
    }

    if json {
        println!("{{\"decrypted\": {decrypted}, \"skipped\": {skipped}, \"errors\": {errors}}}");
    } else {
        println!("Decrypted {decrypted} file(s), skipped {skipped}, errors {errors}");
    }

    Ok(())
}

/// FJ-3309: Re-encrypt state files with a new passphrase.
///
/// Decrypts each encrypted file with the old key, then re-encrypts with the new key.
/// Non-encrypted files are encrypted with the new key directly.
pub fn cmd_state_rekey(
    state_dir: &Path,
    old_passphrase: &str,
    new_passphrase: &str,
    json: bool,
) -> Result<(), String> {
    let old_key = derive_key(old_passphrase);
    let new_key = derive_key(new_passphrase);

    let lock_files = find_lock_files(state_dir)?;
    if lock_files.is_empty() {
        if json {
            println!("{{\"rekeyed\": 0, \"errors\": 0}}");
        } else {
            println!("No state files found in {}", state_dir.display());
        }
        return Ok(());
    }

    let mut rekeyed = 0;
    let mut errors = 0;

    for file in &lock_files {
        let plaintext = if is_encrypted(file) {
            // Decrypt with old key first
            let meta = match read_metadata(file) {
                Ok(m) => m,
                Err(e) => {
                    errors += 1;
                    if !json {
                        println!("  METADATA ERROR: {}: {e}", file.display());
                    }
                    continue;
                }
            };
            let ciphertext =
                std::fs::read(file).map_err(|e| format!("read {}: {e}", file.display()))?;

            if !verify_metadata(&meta, &ciphertext, &old_key) {
                errors += 1;
                if !json {
                    println!("  INTEGRITY FAIL: {}", file.display());
                }
                continue;
            }

            let plain = xor_mask(&ciphertext, &old_key);
            if hash_data(&plain) != meta.plaintext_hash {
                errors += 1;
                if !json {
                    println!("  HASH MISMATCH: {}", file.display());
                }
                continue;
            }
            plain
        } else {
            std::fs::read(file).map_err(|e| format!("read {}: {e}", file.display()))?
        };

        // Re-encrypt with new key
        let new_ciphertext = xor_mask(&plaintext, &new_key);
        std::fs::write(file, &new_ciphertext)
            .map_err(|e| format!("write {}: {e}", file.display()))?;

        let meta = create_metadata(&plaintext, &new_ciphertext, &new_key);
        write_metadata(file, &meta)?;

        rekeyed += 1;
    }

    if json {
        println!("{{\"rekeyed\": {rekeyed}, \"errors\": {errors}}}");
    } else {
        println!("Rekeyed {rekeyed} file(s), errors {errors}");
    }

    Ok(())
}

/// XOR mask data with a BLAKE3-derived key stream.
fn xor_mask(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
    // Generate a key stream using BLAKE3 in keyed mode
    let mut result = Vec::with_capacity(data.len());

    for (block_idx, chunk) in (0_u64..).zip(data.chunks(32)) {
        let block_key = blake3::keyed_hash(key, &block_idx.to_le_bytes());
        let stream = block_key.as_bytes();
        for (i, &byte) in chunk.iter().enumerate() {
            result.push(byte ^ stream[i]);
        }
    }

    result
}

/// Find lock files in a state directory.
fn find_lock_files(state_dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut files = Vec::new();

    if !state_dir.exists() {
        return Ok(files);
    }

    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Recurse into machine subdirectories
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    for sub_entry in sub_entries.flatten() {
                        let sub_path = sub_entry.path();
                        if is_lock_file(&sub_path) {
                            files.push(sub_path);
                        }
                    }
                }
            } else if is_lock_file(&path) {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

fn is_lock_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    (name.ends_with(".lock.yaml") || name.ends_with(".lock.json"))
        && !name.ends_with(".enc.meta.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_mask_roundtrip() {
        let key = derive_key("test-passphrase");
        let data = b"hello world, this is state data that needs encryption!";
        let encrypted = xor_mask(data, &key);
        assert_ne!(&encrypted, data);
        let decrypted = xor_mask(&encrypted, &key);
        assert_eq!(&decrypted, data);
    }

    #[test]
    fn xor_mask_empty() {
        let key = derive_key("test");
        let encrypted = xor_mask(&[], &key);
        assert!(encrypted.is_empty());
    }

    #[test]
    fn find_lock_files_empty() {
        let dir = tempfile::tempdir().unwrap();
        let files = find_lock_files(dir.path()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn find_lock_files_with_locks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("m1.lock.yaml"), "data").unwrap();
        std::fs::write(dir.path().join("m2.lock.yaml"), "data").unwrap();
        std::fs::write(dir.path().join("other.txt"), "data").unwrap();
        let files = find_lock_files(dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn find_lock_files_subdirs() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("machine1");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("state.lock.yaml"), "data").unwrap();
        let files = find_lock_files(dir.path()).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn find_lock_files_nonexistent() {
        let files = find_lock_files(Path::new("/nonexistent/dir")).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn is_lock_file_check() {
        assert!(is_lock_file(Path::new("m1.lock.yaml")));
        assert!(is_lock_file(Path::new("state.lock.json")));
        assert!(!is_lock_file(Path::new("state.yaml")));
        assert!(!is_lock_file(Path::new("x.enc.meta.json")));
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let lock = dir.path().join("test.lock.yaml");
        let original = "resources:\n  pkg:\n    state: converged\n";
        std::fs::write(&lock, original).unwrap();

        let passphrase = "test-pass-123";

        // Encrypt
        cmd_state_encrypt(dir.path(), passphrase, false).unwrap();
        assert!(is_encrypted(&lock));
        let encrypted_content = std::fs::read(&lock).unwrap();
        assert_ne!(encrypted_content, original.as_bytes());

        // Decrypt
        cmd_state_decrypt(dir.path(), passphrase, false).unwrap();
        let decrypted_content = std::fs::read(&lock).unwrap();
        assert_eq!(decrypted_content, original.as_bytes());
    }

    #[test]
    fn encrypt_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_state_encrypt(dir.path(), "pass", false);
        assert!(result.is_ok());
    }

    #[test]
    fn encrypt_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_state_encrypt(dir.path(), "pass", true);
        assert!(result.is_ok());
    }

    #[test]
    fn decrypt_wrong_passphrase() {
        let dir = tempfile::tempdir().unwrap();
        let lock = dir.path().join("test.lock.yaml");
        std::fs::write(&lock, "state data").unwrap();

        cmd_state_encrypt(dir.path(), "correct-pass", false).unwrap();

        // Decrypt with wrong passphrase — should fail integrity
        let result = cmd_state_decrypt(dir.path(), "wrong-pass", false);
        assert!(result.is_ok()); // doesn't error, reports errors count
    }

    #[test]
    fn rekey_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let lock = dir.path().join("test.lock.yaml");
        let original = "resources:\n  pkg:\n    state: converged\n";
        std::fs::write(&lock, original).unwrap();

        // Encrypt with old passphrase
        cmd_state_encrypt(dir.path(), "old-pass", false).unwrap();
        assert!(is_encrypted(&lock));

        // Rekey to new passphrase
        cmd_state_rekey(dir.path(), "old-pass", "new-pass", false).unwrap();
        assert!(is_encrypted(&lock));

        // Decrypt with new passphrase
        cmd_state_decrypt(dir.path(), "new-pass", false).unwrap();
        let content = std::fs::read(&lock).unwrap();
        assert_eq!(content, original.as_bytes());
    }

    #[test]
    fn rekey_wrong_old_passphrase() {
        let dir = tempfile::tempdir().unwrap();
        let lock = dir.path().join("test.lock.yaml");
        std::fs::write(&lock, "data").unwrap();

        cmd_state_encrypt(dir.path(), "correct", false).unwrap();

        // Rekey with wrong old passphrase — should report error
        let result = cmd_state_rekey(dir.path(), "wrong", "new", false);
        assert!(result.is_ok()); // reports errors, doesn't fail
    }

    #[test]
    fn rekey_unencrypted_file() {
        let dir = tempfile::tempdir().unwrap();
        let lock = dir.path().join("state.lock.yaml");
        let original = "plain state data";
        std::fs::write(&lock, original).unwrap();

        // Rekey encrypts unencrypted files directly
        cmd_state_rekey(dir.path(), "ignored", "new-pass", false).unwrap();
        assert!(is_encrypted(&lock));

        // Decrypt with new passphrase
        cmd_state_decrypt(dir.path(), "new-pass", false).unwrap();
        let content = std::fs::read(&lock).unwrap();
        assert_eq!(content, original.as_bytes());
    }

    #[test]
    fn rekey_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_state_rekey(dir.path(), "old", "new", false);
        assert!(result.is_ok());
    }

    #[test]
    fn rekey_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_state_rekey(dir.path(), "old", "new", true);
        assert!(result.is_ok());
    }
}
