//! FJ-3304: `forjar state encrypt/decrypt` CLI handler.
//!
//! Encrypts and decrypts state files using age passphrase encryption
//! with BLAKE3 integrity verification.

use crate::core::state_encryption::*;
use std::path::Path;

/// Encrypt state files with age passphrase encryption.
///
/// Each lock file is encrypted using the `age` crate with a user passphrase.
/// A BLAKE3-derived HMAC sidecar provides integrity verification.
pub fn cmd_state_encrypt(state_dir: &Path, passphrase: &str, json: bool) -> Result<(), String> {
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

        encrypt_state_file(file, passphrase)?;
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
    let lock_files = find_lock_files(state_dir)?;
    let mut decrypted = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for file in &lock_files {
        if !is_encrypted(file) {
            skipped += 1;
            continue;
        }

        match decrypt_state_file(file, passphrase) {
            Ok(_) => decrypted += 1,
            Err(e) => {
                errors += 1;
                if !json {
                    println!("  DECRYPT FAIL: {}: {e}", file.display());
                }
            }
        }
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
/// Decrypts each encrypted file with the old passphrase, then re-encrypts
/// with the new one. Non-encrypted files are encrypted with the new passphrase directly.
pub fn cmd_state_rekey(
    state_dir: &Path,
    old_passphrase: &str,
    new_passphrase: &str,
    json: bool,
) -> Result<(), String> {
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
            match rekey_decrypt(file, old_passphrase) {
                Ok(p) => p,
                Err(e) => {
                    errors += 1;
                    if !json {
                        println!("  REKEY FAIL: {}: {e}", file.display());
                    }
                    continue;
                }
            }
        } else {
            std::fs::read(file).map_err(|e| format!("read {}: {e}", file.display()))?
        };

        // Re-encrypt with new passphrase
        let ciphertext = encrypt_data(&plaintext, new_passphrase)?;
        std::fs::write(file, &ciphertext).map_err(|e| format!("write {}: {e}", file.display()))?;

        let new_key = derive_key(new_passphrase);
        let meta = create_metadata(&plaintext, &ciphertext, &new_key);
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

/// Decrypt a file during rekey (returns plaintext without writing to disk).
fn rekey_decrypt(file: &Path, passphrase: &str) -> Result<Vec<u8>, String> {
    let ciphertext = std::fs::read(file).map_err(|e| format!("read {}: {e}", file.display()))?;
    let key = derive_key(passphrase);
    let meta = read_metadata(file)?;

    if !verify_metadata(&meta, &ciphertext, &key) {
        return Err("integrity check failed".into());
    }

    let plaintext = decrypt_data(&ciphertext, passphrase)?;

    if hash_data(&plaintext) != meta.plaintext_hash {
        return Err("plaintext hash mismatch".into());
    }

    Ok(plaintext)
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

#[cfg(all(test, feature = "encryption"))]
mod tests_encryption {
    use super::*;

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
    fn decrypt_wrong_passphrase() {
        let dir = tempfile::tempdir().unwrap();
        let lock = dir.path().join("test.lock.yaml");
        std::fs::write(&lock, "state data").unwrap();

        cmd_state_encrypt(dir.path(), "correct-pass", false).unwrap();

        // Decrypt with wrong passphrase — should fail integrity check
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
}
