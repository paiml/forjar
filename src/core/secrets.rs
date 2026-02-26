//! FJ-200: Age-encrypted secret values.
//!
//! Secrets stored as `ENC[age,<base64-ciphertext>]` markers in forjar.yaml,
//! decrypted at resolve time. Identity from `FORJAR_AGE_KEY` env var or
//! `--identity` flag. Coexists with `{{secrets.key}}` env-var resolution.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use std::io::{Read, Write};

/// Marker prefix for age-encrypted values.
const ENC_PREFIX: &str = "ENC[age,";
/// Marker suffix for age-encrypted values.
const ENC_SUFFIX: &str = "]";

/// Check if a string contains any `ENC[age,...]` markers.
pub fn has_encrypted_markers(s: &str) -> bool {
    s.contains(ENC_PREFIX)
}

/// Encrypt a plaintext value with one or more age X25519 recipients.
///
/// Returns `ENC[age,<base64>]` string suitable for embedding in YAML.
pub fn encrypt(plaintext: &str, recipient_strs: &[&str]) -> Result<String, String> {
    if recipient_strs.is_empty() {
        return Err("at least one recipient required".to_string());
    }
    let recipients: Vec<Box<dyn age::Recipient + Send>> = recipient_strs
        .iter()
        .map(|r| {
            r.parse::<age::x25519::Recipient>()
                .map(|r| Box::new(r) as Box<dyn age::Recipient + Send>)
                .map_err(|e| format!("invalid recipient '{}': {}", r, e))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let encryptor = age::Encryptor::with_recipients(
        recipients.iter().map(|r| r.as_ref() as &dyn age::Recipient),
    )
    .map_err(|e| format!("encryptor init failed: {}", e))?;

    let mut encrypted = vec![];
    let mut writer = encryptor
        .wrap_output(&mut encrypted)
        .map_err(|e| format!("encryption error: {}", e))?;
    writer
        .write_all(plaintext.as_bytes())
        .map_err(|e| format!("write error: {}", e))?;
    writer
        .finish()
        .map_err(|e| format!("encryption finish error: {}", e))?;

    let encoded = B64.encode(&encrypted);
    Ok(format!("{}{}{}", ENC_PREFIX, encoded, ENC_SUFFIX))
}

/// Decrypt a single `ENC[age,<base64>]` marker.
pub fn decrypt_marker(
    marker: &str,
    identities: &[age::x25519::Identity],
) -> Result<String, String> {
    let inner = marker
        .strip_prefix(ENC_PREFIX)
        .and_then(|s| s.strip_suffix(ENC_SUFFIX))
        .ok_or_else(|| "invalid ENC marker: does not match ENC[age,...] format".to_string())?;

    let encrypted = B64
        .decode(inner)
        .map_err(|e| format!("base64 decode error: {}", e))?;

    let decryptor =
        age::Decryptor::new(&encrypted[..]).map_err(|e| format!("age decryptor error: {}", e))?;

    let mut reader = decryptor
        .decrypt(identities.iter().map(|i| i as &dyn age::Identity))
        .map_err(|e| format!("decryption failed: {}", e))?;
    let mut plaintext = String::new();
    reader
        .read_to_string(&mut plaintext)
        .map_err(|e| format!("read error: {}", e))?;
    Ok(plaintext)
}

/// Find and decrypt all `ENC[age,...]` markers in a string.
///
/// Returns the string with all markers replaced by their plaintext values.
/// If no markers are found, returns the string unchanged.
pub fn decrypt_all(s: &str, identities: &[age::x25519::Identity]) -> Result<String, String> {
    if !has_encrypted_markers(s) {
        return Ok(s.to_string());
    }

    let mut result = s.to_string();
    // Process markers from right to left to preserve positions
    let markers = find_markers(&result);
    for (start, end) in markers.into_iter().rev() {
        let marker = &result[start..end];
        let plaintext = decrypt_marker(marker, identities)?;
        result.replace_range(start..end, &plaintext);
    }
    Ok(result)
}

/// Find all `ENC[age,...]` marker positions in a string.
///
/// Returns `(start, end)` byte positions for each marker.
fn find_markers(s: &str) -> Vec<(usize, usize)> {
    let mut markers = Vec::new();
    let mut start = 0;
    while let Some(pos) = s[start..].find(ENC_PREFIX) {
        let abs_start = start + pos;
        // Find the matching closing bracket, handling nested brackets
        let after_prefix = abs_start + ENC_PREFIX.len();
        if let Some(end_pos) = s[after_prefix..].find(ENC_SUFFIX) {
            let abs_end = after_prefix + end_pos + ENC_SUFFIX.len();
            markers.push((abs_start, abs_end));
            start = abs_end;
        } else {
            break; // No closing bracket found
        }
    }
    markers
}

/// Load age identity from `FORJAR_AGE_KEY` env var.
///
/// The env var should contain the age secret key string
/// (e.g., `AGE-SECRET-KEY-1...`).
pub fn load_identity_from_env() -> Result<age::x25519::Identity, String> {
    let key_str = std::env::var("FORJAR_AGE_KEY")
        .map_err(|_| "FORJAR_AGE_KEY not set (required for ENC[age,...] decryption)".to_string())?;
    parse_identity(&key_str)
}

/// Load age identity from a file path.
///
/// The file should contain an age identity (one `AGE-SECRET-KEY-1...` line).
pub fn load_identity_file(path: &std::path::Path) -> Result<age::x25519::Identity, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read identity file '{}': {}", path.display(), e))?;
    // Find the AGE-SECRET-KEY line (skip comments)
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("AGE-SECRET-KEY-") {
            return parse_identity(trimmed);
        }
    }
    Err(format!("no AGE-SECRET-KEY found in '{}'", path.display()))
}

/// Parse an age X25519 identity from a key string.
fn parse_identity(key_str: &str) -> Result<age::x25519::Identity, String> {
    key_str
        .trim()
        .parse::<age::x25519::Identity>()
        .map_err(|e| format!("invalid age identity: {}", e))
}

/// Get the public key (recipient) string for an identity.
pub fn identity_to_recipient(identity: &age::x25519::Identity) -> String {
    identity.to_public().to_string()
}

/// Generate a new age X25519 identity (keypair).
pub fn generate_identity() -> age::x25519::Identity {
    age::x25519::Identity::generate()
}

/// Load identities for decryption.
///
/// Priority: `--identity` file path > `FORJAR_AGE_KEY` env var.
pub fn load_identities(
    identity_path: Option<&std::path::Path>,
) -> Result<Vec<age::x25519::Identity>, String> {
    if let Some(path) = identity_path {
        return Ok(vec![load_identity_file(path)?]);
    }
    Ok(vec![load_identity_from_env()?])
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use age::secrecy::ExposeSecret;

    fn test_keypair() -> (age::x25519::Identity, String) {
        let identity = generate_identity();
        let recipient = identity_to_recipient(&identity);
        (identity, recipient)
    }

    #[test]
    fn test_fj200_encrypt_decrypt_roundtrip() {
        let (identity, recipient) = test_keypair();
        let plaintext = "s3cret-db-passw0rd!";

        let encrypted = encrypt(plaintext, &[&recipient]).unwrap();
        assert!(encrypted.starts_with(ENC_PREFIX));
        assert!(encrypted.ends_with(ENC_SUFFIX));

        let decrypted = decrypt_marker(&encrypted, &[identity]).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_fj200_encrypt_empty_string() {
        let (identity, recipient) = test_keypair();
        let encrypted = encrypt("", &[&recipient]).unwrap();
        let decrypted = decrypt_marker(&encrypted, &[identity]).unwrap();
        assert_eq!(decrypted, "");
    }

    #[test]
    fn test_fj200_encrypt_unicode() {
        let (identity, recipient) = test_keypair();
        let plaintext = "日本語パスワード🔑";
        let encrypted = encrypt(plaintext, &[&recipient]).unwrap();
        let decrypted = decrypt_marker(&encrypted, &[identity]).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_fj200_encrypt_multiline() {
        let (identity, recipient) = test_keypair();
        let plaintext = "line1\nline2\nline3";
        let encrypted = encrypt(plaintext, &[&recipient]).unwrap();
        let decrypted = decrypt_marker(&encrypted, &[identity]).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_fj200_encrypt_no_recipients() {
        let err = encrypt("test", &[]).unwrap_err();
        assert!(err.contains("at least one recipient"));
    }

    #[test]
    fn test_fj200_encrypt_invalid_recipient() {
        let err = encrypt("test", &["not-a-valid-key"]).unwrap_err();
        assert!(err.contains("invalid recipient"));
    }

    #[test]
    fn test_fj200_decrypt_invalid_marker() {
        let (identity, _) = test_keypair();
        let err = decrypt_marker("not-a-marker", &[identity]).unwrap_err();
        assert!(err.contains("invalid ENC marker"));
    }

    #[test]
    fn test_fj200_decrypt_invalid_base64() {
        let (identity, _) = test_keypair();
        let err = decrypt_marker("ENC[age,!!!invalid!!!]", &[identity]).unwrap_err();
        assert!(err.contains("base64 decode error"));
    }

    #[test]
    fn test_fj200_decrypt_wrong_key() {
        let (_identity1, recipient1) = test_keypair();
        let (identity2, _recipient2) = test_keypair();

        let encrypted = encrypt("secret", &[&recipient1]).unwrap();
        let err = decrypt_marker(&encrypted, &[identity2]).unwrap_err();
        assert!(err.contains("decryption failed"));
    }

    #[test]
    fn test_fj200_multi_recipient() {
        let (identity1, recipient1) = test_keypair();
        let (identity2, recipient2) = test_keypair();
        let plaintext = "shared-secret";

        let encrypted = encrypt(plaintext, &[&recipient1, &recipient2]).unwrap();

        // Both recipients can decrypt
        let d1 = decrypt_marker(&encrypted, &[identity1]).unwrap();
        assert_eq!(d1, plaintext);
        let d2 = decrypt_marker(&encrypted, &[identity2]).unwrap();
        assert_eq!(d2, plaintext);
    }

    #[test]
    fn test_fj200_has_encrypted_markers() {
        assert!(has_encrypted_markers("foo ENC[age,abc] bar"));
        assert!(has_encrypted_markers("ENC[age,abc]"));
        assert!(!has_encrypted_markers("no markers here"));
        assert!(!has_encrypted_markers("ENC[other,abc]"));
        assert!(!has_encrypted_markers(""));
    }

    #[test]
    fn test_fj200_find_markers() {
        let s = "a=ENC[age,x] b=ENC[age,y]";
        let markers = find_markers(s);
        assert_eq!(markers.len(), 2);
        assert_eq!(&s[markers[0].0..markers[0].1], "ENC[age,x]");
        assert_eq!(&s[markers[1].0..markers[1].1], "ENC[age,y]");
    }

    #[test]
    fn test_fj200_find_markers_none() {
        assert!(find_markers("no markers here").is_empty());
    }

    #[test]
    fn test_fj200_find_markers_unclosed() {
        // Unclosed marker — should not match
        let markers = find_markers("ENC[age,abc");
        assert!(markers.is_empty());
    }

    #[test]
    fn test_fj200_decrypt_all() {
        let (identity, recipient) = test_keypair();
        let enc1 = encrypt("alpha", &[&recipient]).unwrap();
        let enc2 = encrypt("beta", &[&recipient]).unwrap();
        let input = format!("key1={} key2={}", enc1, enc2);

        let result = decrypt_all(&input, &[identity]).unwrap();
        assert_eq!(result, "key1=alpha key2=beta");
    }

    #[test]
    fn test_fj200_decrypt_all_no_markers() {
        let (identity, _) = test_keypair();
        let input = "plain text with no markers";
        let result = decrypt_all(input, &[identity]).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_fj200_decrypt_all_mixed_content() {
        let (identity, recipient) = test_keypair();
        let enc = encrypt("secret-value", &[&recipient]).unwrap();
        let input = format!("host: localhost\npassword: {}\nport: 5432", enc);

        let result = decrypt_all(&input, &[identity]).unwrap();
        assert_eq!(
            result,
            "host: localhost\npassword: secret-value\nport: 5432"
        );
    }

    #[test]
    fn test_fj200_parse_identity_valid() {
        let identity = generate_identity();
        let key_str = identity.to_string();
        let parsed = parse_identity(key_str.expose_secret()).unwrap();
        assert_eq!(
            identity_to_recipient(&parsed),
            identity_to_recipient(&identity)
        );
    }

    #[test]
    fn test_fj200_parse_identity_invalid() {
        let err = parse_identity("not-a-key").err().expect("expected error");
        assert!(err.contains("invalid age identity"));
    }

    #[test]
    fn test_fj200_identity_to_recipient() {
        let identity = generate_identity();
        let recipient = identity_to_recipient(&identity);
        assert!(recipient.starts_with("age1"));
    }

    #[test]
    fn test_fj200_load_identity_from_env_subprocess() {
        // Run a subprocess with FORJAR_AGE_KEY set to verify env-based loading
        let identity = generate_identity();
        let key_str = identity.to_string();

        let exe = std::env::current_exe().unwrap();
        let output = std::process::Command::new(exe)
            .env("FORJAR_AGE_KEY", key_str.expose_secret())
            .arg("--test-threads=1")
            .arg("--exact")
            .arg("core::secrets::tests::test_fj200_load_identity_env_inner")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "subprocess failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_fj200_load_identity_env_inner() {
        // Run via subprocess with FORJAR_AGE_KEY set
        if std::env::var("FORJAR_AGE_KEY").is_err() {
            return; // Skip when not run via subprocess
        }
        let loaded = load_identity_from_env().unwrap();
        let recipient = identity_to_recipient(&loaded);
        assert!(recipient.starts_with("age1"));
    }

    #[test]
    fn test_fj200_load_identity_from_env_error_message() {
        // Verify error message format (don't actually set env vars)
        let result = load_identity_from_env();
        if result.is_ok() {
            return; // FORJAR_AGE_KEY happens to be set in this environment
        }
        let err = result.err().expect("expected error");
        assert!(err.contains("FORJAR_AGE_KEY not set"));
    }

    #[test]
    fn test_fj200_load_identity_file() {
        let identity = generate_identity();
        let key_str = identity.to_string();
        let recipient_expected = identity_to_recipient(&identity);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.txt");
        std::fs::write(
            &path,
            format!(
                "# created: 2024-01-01\n# public key: {}\n{}\n",
                recipient_expected,
                key_str.expose_secret()
            ),
        )
        .unwrap();

        let loaded = load_identity_file(&path).unwrap();
        assert_eq!(identity_to_recipient(&loaded), recipient_expected);
    }

    #[test]
    fn test_fj200_load_identity_file_not_found() {
        let err = load_identity_file(std::path::Path::new("/nonexistent/key.txt"))
            .err()
            .expect("expected error");
        assert!(err.contains("cannot read identity file"));
    }

    #[test]
    fn test_fj200_load_identity_file_no_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.txt");
        std::fs::write(&path, "# just a comment\n").unwrap();

        let err = load_identity_file(&path).err().expect("expected error");
        assert!(err.contains("no AGE-SECRET-KEY found"));
    }

    #[test]
    fn test_fj200_deterministic_marker_format() {
        let (_identity, recipient) = test_keypair();
        let encrypted = encrypt("test", &[&recipient]).unwrap();

        // Verify format
        assert!(encrypted.starts_with("ENC[age,"));
        assert!(encrypted.ends_with(']'));

        // Extract base64 portion and verify it decodes
        let b64 = encrypted
            .strip_prefix(ENC_PREFIX)
            .unwrap()
            .strip_suffix(ENC_SUFFIX)
            .unwrap();
        assert!(B64.decode(b64).is_ok());
    }

    #[test]
    fn test_fj200_large_secret() {
        let (identity, recipient) = test_keypair();
        let plaintext = "x".repeat(100_000);
        let encrypted = encrypt(&plaintext, &[&recipient]).unwrap();
        let decrypted = decrypt_marker(&encrypted, &[identity]).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_fj200_special_chars() {
        let (identity, recipient) = test_keypair();
        let plaintext = r#"p@$$w0rd!@#$%^&*(){}[]|\"':;<>,.?/~`"#;
        let encrypted = encrypt(plaintext, &[&recipient]).unwrap();
        let decrypted = decrypt_marker(&encrypted, &[identity]).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
