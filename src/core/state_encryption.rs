//! FJ-3303: State file encryption with age.
//!
//! Provides at-rest encryption for state files using age (sovereign,
//! no cloud KMS dependency). BLAKE3 HMAC ensures integrity.

use std::path::Path;

/// Encryption metadata stored alongside encrypted state files.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptionMeta {
    /// Encryption scheme version.
    pub version: u32,
    /// BLAKE3 hash of the plaintext state before encryption.
    pub plaintext_hash: String,
    /// BLAKE3 HMAC of the ciphertext for integrity verification.
    pub ciphertext_hmac: String,
    /// Timestamp when encryption was performed.
    pub encrypted_at: String,
}

/// Compute BLAKE3 hash of data.
pub fn hash_data(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

/// Compute BLAKE3 keyed hash (HMAC) for integrity verification.
pub fn keyed_hash(data: &[u8], key: &[u8; 32]) -> String {
    blake3::keyed_hash(key, data).to_hex().to_string()
}

/// Verify a BLAKE3 keyed hash.
pub fn verify_keyed_hash(data: &[u8], key: &[u8; 32], expected: &str) -> bool {
    keyed_hash(data, key) == expected
}

/// Derive a 32-byte key from a passphrase using BLAKE3.
pub fn derive_key(passphrase: &str) -> [u8; 32] {
    blake3::derive_key("forjar state encryption v1", passphrase.as_bytes())
}

/// Create encryption metadata for state data.
pub fn create_metadata(plaintext: &[u8], ciphertext: &[u8], key: &[u8; 32]) -> EncryptionMeta {
    EncryptionMeta {
        version: 1,
        plaintext_hash: hash_data(plaintext),
        ciphertext_hmac: keyed_hash(ciphertext, key),
        encrypted_at: crate::tripwire::eventlog::now_iso8601(),
    }
}

/// Verify encryption metadata against ciphertext.
pub fn verify_metadata(meta: &EncryptionMeta, ciphertext: &[u8], key: &[u8; 32]) -> bool {
    verify_keyed_hash(ciphertext, key, &meta.ciphertext_hmac)
}

/// Read encryption metadata from a sidecar file (.enc.meta.json).
pub fn read_metadata(path: &Path) -> Result<EncryptionMeta, String> {
    let meta_path = meta_path_for(path);
    let content = std::fs::read_to_string(&meta_path)
        .map_err(|e| format!("read {}: {e}", meta_path.display()))?;
    serde_json::from_str(&content).map_err(|e| format!("parse metadata: {e}"))
}

/// Write encryption metadata to a sidecar file.
pub fn write_metadata(path: &Path, meta: &EncryptionMeta) -> Result<(), String> {
    let meta_path = meta_path_for(path);
    let json = serde_json::to_string_pretty(meta).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&meta_path, json).map_err(|e| format!("write {}: {e}", meta_path.display()))
}

/// Get the sidecar metadata file path for an encrypted file.
fn meta_path_for(path: &Path) -> std::path::PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(".enc.meta.json");
    std::path::PathBuf::from(p)
}

/// Check if a state file has an encryption sidecar.
pub fn is_encrypted(path: &Path) -> bool {
    meta_path_for(path).exists()
}

/// List encrypted state files in a directory.
pub fn list_encrypted(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.ends_with(".enc.meta.json") {
                    // This is a metadata sidecar, get the base file
                    let base = name.strip_suffix(".enc.meta.json").unwrap_or("");
                    if !base.is_empty() {
                        files.push(dir.join(base));
                    }
                }
            }
        }
    }
    files.sort();
    files
}

/// Summary of state encryption status.
#[derive(Debug, Clone)]
pub struct EncryptionStatus {
    /// Total state files checked.
    pub total_files: usize,
    /// Files with encryption metadata.
    pub encrypted_count: usize,
    /// Files without encryption.
    pub unencrypted_count: usize,
    /// Files with failed integrity checks.
    pub integrity_failures: usize,
}

impl EncryptionStatus {
    /// Whether all files are encrypted and verified.
    pub fn fully_encrypted(&self) -> bool {
        self.unencrypted_count == 0 && self.integrity_failures == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_data_deterministic() {
        let h1 = hash_data(b"test data");
        let h2 = hash_data(b"test data");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn hash_data_different_inputs() {
        let h1 = hash_data(b"data-a");
        let h2 = hash_data(b"data-b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn keyed_hash_works() {
        let key = derive_key("passphrase");
        let h = keyed_hash(b"data", &key);
        assert_eq!(h.len(), 64);
        assert!(verify_keyed_hash(b"data", &key, &h));
    }

    #[test]
    fn keyed_hash_tamper_detection() {
        let key = derive_key("passphrase");
        let h = keyed_hash(b"original", &key);
        assert!(!verify_keyed_hash(b"tampered", &key, &h));
    }

    #[test]
    fn derive_key_deterministic() {
        let k1 = derive_key("my-passphrase");
        let k2 = derive_key("my-passphrase");
        assert_eq!(k1, k2);
    }

    #[test]
    fn derive_key_different_passphrases() {
        let k1 = derive_key("passphrase-a");
        let k2 = derive_key("passphrase-b");
        assert_ne!(k1, k2);
    }

    #[test]
    fn create_and_verify_metadata() {
        let key = derive_key("test");
        let plaintext = b"state data here";
        let ciphertext = b"encrypted bytes";
        let meta = create_metadata(plaintext, ciphertext, &key);

        assert_eq!(meta.version, 1);
        assert_eq!(meta.plaintext_hash, hash_data(plaintext));
        assert!(verify_metadata(&meta, ciphertext, &key));
        assert!(!verify_metadata(&meta, b"wrong data", &key));
    }

    #[test]
    fn write_and_read_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("state.lock.yaml");
        let key = derive_key("test");
        let meta = create_metadata(b"plain", b"cipher", &key);

        write_metadata(&file_path, &meta).unwrap();
        let loaded = read_metadata(&file_path).unwrap();
        assert_eq!(loaded.plaintext_hash, meta.plaintext_hash);
        assert_eq!(loaded.ciphertext_hmac, meta.ciphertext_hmac);
    }

    #[test]
    fn meta_path_for_correct() {
        let p = meta_path_for(Path::new("/state/lock.yaml"));
        assert_eq!(p.to_str().unwrap(), "/state/lock.yaml.enc.meta.json");
    }

    #[test]
    fn is_encrypted_false() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("state.lock.yaml");
        std::fs::write(&file, "data").unwrap();
        assert!(!is_encrypted(&file));
    }

    #[test]
    fn is_encrypted_true() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("state.lock.yaml");
        std::fs::write(&file, "data").unwrap();
        let key = derive_key("test");
        let meta = create_metadata(b"p", b"c", &key);
        write_metadata(&file, &meta).unwrap();
        assert!(is_encrypted(&file));
    }

    #[test]
    fn list_encrypted_empty() {
        let dir = tempfile::tempdir().unwrap();
        let files = list_encrypted(dir.path());
        assert!(files.is_empty());
    }

    #[test]
    fn list_encrypted_with_files() {
        let dir = tempfile::tempdir().unwrap();
        // Create a metadata sidecar
        let meta_path = dir.path().join("lock.yaml.enc.meta.json");
        std::fs::write(&meta_path, "{}").unwrap();
        let files = list_encrypted(dir.path());
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn encryption_status_fully_encrypted() {
        let status = EncryptionStatus {
            total_files: 3,
            encrypted_count: 3,
            unencrypted_count: 0,
            integrity_failures: 0,
        };
        assert!(status.fully_encrypted());
    }

    #[test]
    fn encryption_status_not_fully() {
        let status = EncryptionStatus {
            total_files: 3,
            encrypted_count: 2,
            unencrypted_count: 1,
            integrity_failures: 0,
        };
        assert!(!status.fully_encrypted());
    }

    #[test]
    fn read_metadata_missing_file() {
        let result = read_metadata(Path::new("/nonexistent/file.yaml"));
        assert!(result.is_err());
    }
}
