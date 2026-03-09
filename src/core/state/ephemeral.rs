//! FJ-3300: Ephemeral value redaction for state persistence.
//!
//! When `secrets.ephemeral: true`, resolved secret values are replaced with
//! BLAKE3 hashes before writing to state. This prevents cleartext secrets
//! from appearing in state files while still enabling drift detection.

/// Marker prefix for ephemeral (hashed) values in state files.
pub const EPHEMERAL_PREFIX: &str = "EPHEMERAL[blake3:";

/// Marker suffix for ephemeral values.
pub const EPHEMERAL_SUFFIX: &str = "]";

/// Replace a secret value with its BLAKE3 hash marker.
///
/// The marker format is `EPHEMERAL[blake3:<64-hex-chars>]`.
/// This allows drift detection: re-resolve the secret and compare hashes.
pub fn redact_to_hash(value: &str) -> String {
    let hash = blake3::hash(value.as_bytes());
    format!("{EPHEMERAL_PREFIX}{}{EPHEMERAL_SUFFIX}", hash.to_hex())
}

/// Check if a value is an ephemeral hash marker.
pub fn is_ephemeral_marker(value: &str) -> bool {
    value.starts_with(EPHEMERAL_PREFIX) && value.ends_with(EPHEMERAL_SUFFIX)
}

/// Extract the BLAKE3 hash from an ephemeral marker, if valid.
pub fn extract_hash(marker: &str) -> Option<&str> {
    marker
        .strip_prefix(EPHEMERAL_PREFIX)
        .and_then(|s| s.strip_suffix(EPHEMERAL_SUFFIX))
}

/// Verify that a secret value matches a stored ephemeral marker.
///
/// Returns `true` if the BLAKE3 hash of `current_value` matches the hash
/// in the stored marker. Used for drift detection on ephemeral secrets.
pub fn verify_drift(current_value: &str, stored_marker: &str) -> bool {
    if let Some(stored_hash) = extract_hash(stored_marker) {
        let current_hash = blake3::hash(current_value.as_bytes()).to_hex().to_string();
        current_hash == stored_hash
    } else {
        false
    }
}

/// Redact all values in a resolved output map that look like secrets.
///
/// A value is treated as a secret if its key contains "secret", "password",
/// "token", "key", or "credential" (case-insensitive). This heuristic
/// catches common naming patterns without requiring explicit annotation.
///
/// When `force_all` is true (from `secrets.ephemeral: true`), ALL values
/// are redacted regardless of key name.
pub fn redact_outputs(
    outputs: &indexmap::IndexMap<String, String>,
    force_all: bool,
) -> indexmap::IndexMap<String, String> {
    outputs
        .iter()
        .map(|(k, v)| {
            if force_all || is_secret_key(k) {
                (k.clone(), redact_to_hash(v))
            } else {
                (k.clone(), v.clone())
            }
        })
        .collect()
}

/// Heuristic: does this key name look like it holds a secret?
fn is_secret_key(key: &str) -> bool {
    let lower = key.to_lowercase();
    lower.contains("secret")
        || lower.contains("password")
        || lower.contains("token")
        || lower.contains("key")
        || lower.contains("credential")
}

/// Compute a BLAKE3 keyed hash (HMAC-like) for encrypted state integrity.
///
/// Uses BLAKE3's keyed hash mode with a 32-byte key derived from the
/// identity passphrase. This detects tampering of encrypted state files.
pub fn keyed_hash(data: &[u8], key: &[u8; 32]) -> String {
    let hash = blake3::keyed_hash(key, data);
    hash.to_hex().to_string()
}

/// Derive a 32-byte key from a passphrase using BLAKE3.
pub fn derive_key(passphrase: &str) -> [u8; 32] {
    let hash = blake3::hash(passphrase.as_bytes());
    *hash.as_bytes()
}

/// Verify a keyed hash against data and key.
pub fn verify_keyed_hash(data: &[u8], key: &[u8; 32], expected: &str) -> bool {
    keyed_hash(data, key) == expected
}

/// Write BLAKE3 keyed hash sidecar for an encrypted state file.
///
/// The sidecar is written to `<path>.b3hmac` and contains the keyed hash
/// of the ciphertext.
pub fn write_hmac_sidecar(encrypted_path: &std::path::Path, key: &[u8; 32]) -> Result<(), String> {
    let data = std::fs::read(encrypted_path)
        .map_err(|e| format!("cannot read {}: {}", encrypted_path.display(), e))?;
    let hmac = keyed_hash(&data, key);
    let sidecar = encrypted_path.with_extension("b3hmac");
    std::fs::write(&sidecar, &hmac)
        .map_err(|e| format!("cannot write {}: {}", sidecar.display(), e))?;
    Ok(())
}

/// Verify BLAKE3 keyed hash for an encrypted state file.
pub fn verify_hmac_sidecar(
    encrypted_path: &std::path::Path,
    key: &[u8; 32],
) -> Result<bool, String> {
    let sidecar = encrypted_path.with_extension("b3hmac");
    if !sidecar.exists() {
        return Err(format!("HMAC sidecar not found: {}", sidecar.display()));
    }
    let expected = std::fs::read_to_string(&sidecar).map_err(|e| format!("read error: {e}"))?;
    let data = std::fs::read(encrypted_path)
        .map_err(|e| format!("cannot read {}: {}", encrypted_path.display(), e))?;
    Ok(verify_keyed_hash(&data, key, expected.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_to_hash_deterministic() {
        let h1 = redact_to_hash("my-secret-password");
        let h2 = redact_to_hash("my-secret-password");
        assert_eq!(h1, h2);
        assert!(h1.starts_with(EPHEMERAL_PREFIX));
        assert!(h1.ends_with(EPHEMERAL_SUFFIX));
    }

    #[test]
    fn redact_to_hash_different_values_differ() {
        let h1 = redact_to_hash("password-A");
        let h2 = redact_to_hash("password-B");
        assert_ne!(h1, h2);
    }

    #[test]
    fn is_ephemeral_marker_valid() {
        let marker = redact_to_hash("test");
        assert!(is_ephemeral_marker(&marker));
    }

    #[test]
    fn is_ephemeral_marker_invalid() {
        assert!(!is_ephemeral_marker("plaintext"));
        assert!(!is_ephemeral_marker("EPHEMERAL["));
        assert!(!is_ephemeral_marker("blake3:abc]"));
    }

    #[test]
    fn extract_hash_roundtrip() {
        let original = "secret-value";
        let marker = redact_to_hash(original);
        let hash = extract_hash(&marker).unwrap();
        assert_eq!(hash.len(), 64); // BLAKE3 hex is 64 chars
        let expected = blake3::hash(original.as_bytes()).to_hex().to_string();
        assert_eq!(hash, expected);
    }

    #[test]
    fn extract_hash_invalid() {
        assert!(extract_hash("not-a-marker").is_none());
    }

    #[test]
    fn verify_drift_matches() {
        let secret = "db-password-2026";
        let marker = redact_to_hash(secret);
        assert!(verify_drift(secret, &marker));
    }

    #[test]
    fn verify_drift_changed() {
        let marker = redact_to_hash("old-password");
        assert!(!verify_drift("new-password", &marker));
    }

    #[test]
    fn verify_drift_invalid_marker() {
        assert!(!verify_drift("value", "not-a-marker"));
    }

    #[test]
    fn redact_outputs_force_all() {
        let mut outputs = indexmap::IndexMap::new();
        outputs.insert("data_dir".into(), "/var/data".into());
        outputs.insert("app_port".into(), "8080".into());

        let redacted = redact_outputs(&outputs, true);
        assert!(is_ephemeral_marker(redacted.get("data_dir").unwrap()));
        assert!(is_ephemeral_marker(redacted.get("app_port").unwrap()));
    }

    #[test]
    fn redact_outputs_heuristic() {
        let mut outputs = indexmap::IndexMap::new();
        outputs.insert("data_dir".into(), "/var/data".into());
        outputs.insert("db_password".into(), "s3cret".into());
        outputs.insert("api_token".into(), "tok-123".into());
        outputs.insert("app_port".into(), "8080".into());

        let redacted = redact_outputs(&outputs, false);
        // Secret-looking keys are redacted
        assert!(is_ephemeral_marker(redacted.get("db_password").unwrap()));
        assert!(is_ephemeral_marker(redacted.get("api_token").unwrap()));
        // Non-secret keys are preserved
        assert_eq!(redacted.get("data_dir").unwrap(), "/var/data");
        assert_eq!(redacted.get("app_port").unwrap(), "8080");
    }

    #[test]
    fn is_secret_key_patterns() {
        assert!(is_secret_key("db_password"));
        assert!(is_secret_key("DB_PASSWORD"));
        assert!(is_secret_key("api_token"));
        assert!(is_secret_key("ssh_key"));
        assert!(is_secret_key("aws_secret_access_key"));
        assert!(is_secret_key("service_credential"));
        assert!(!is_secret_key("data_dir"));
        assert!(!is_secret_key("app_port"));
        assert!(!is_secret_key("hostname"));
    }

    #[test]
    fn keyed_hash_deterministic() {
        let key = derive_key("my-passphrase");
        let h1 = keyed_hash(b"state data", &key);
        let h2 = keyed_hash(b"state data", &key);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn keyed_hash_different_keys_differ() {
        let k1 = derive_key("key-A");
        let k2 = derive_key("key-B");
        let h1 = keyed_hash(b"same data", &k1);
        let h2 = keyed_hash(b"same data", &k2);
        assert_ne!(h1, h2);
    }

    #[test]
    fn verify_keyed_hash_pass() {
        let key = derive_key("test-key");
        let hash = keyed_hash(b"important data", &key);
        assert!(verify_keyed_hash(b"important data", &key, &hash));
    }

    #[test]
    fn verify_keyed_hash_tampered() {
        let key = derive_key("test-key");
        let hash = keyed_hash(b"original data", &key);
        assert!(!verify_keyed_hash(b"tampered data", &key, &hash));
    }

    #[test]
    fn derive_key_deterministic() {
        let k1 = derive_key("same-passphrase");
        let k2 = derive_key("same-passphrase");
        assert_eq!(k1, k2);
    }

    #[test]
    fn derive_key_different_passphrases() {
        let k1 = derive_key("phrase-1");
        let k2 = derive_key("phrase-2");
        assert_ne!(k1, k2);
    }

    #[test]
    fn hmac_sidecar_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("state.yaml.age");
        std::fs::write(&file, b"encrypted-state-data").unwrap();

        let key = derive_key("test-passphrase");
        write_hmac_sidecar(&file, &key).unwrap();

        // Verify passes
        assert!(verify_hmac_sidecar(&file, &key).unwrap());

        // Tamper with file
        std::fs::write(&file, b"tampered-data").unwrap();
        assert!(!verify_hmac_sidecar(&file, &key).unwrap());
    }

    #[test]
    fn hmac_sidecar_missing() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("state.yaml.age");
        std::fs::write(&file, b"data").unwrap();
        let key = derive_key("k");
        assert!(verify_hmac_sidecar(&file, &key).is_err());
    }
}
