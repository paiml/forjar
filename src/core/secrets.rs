//! FJ-200: Age-encrypted secret values.
//!
//! Secrets stored as `ENC[age,<base64-ciphertext>]` markers in forjar.yaml,
//! decrypted at resolve time. Identity from `FORJAR_AGE_KEY` env var or
//! `--identity` flag. Coexists with `{{secrets.key}}` env-var resolution.

pub(crate) use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use std::io::{Read, Write};

/// Marker prefix for age-encrypted values.
pub(crate) const ENC_PREFIX: &str = "ENC[age,";
/// Marker suffix for age-encrypted values.
pub(crate) const ENC_SUFFIX: &str = "]";

/// Check if a string contains any real `ENC[age,<base64>]` markers.
///
/// PMAT-037: Validates that the content between markers is plausible base64
/// ciphertext (>= 20 chars, valid base64 alphabet). This prevents false
/// positives from comments like `# values use ENC[age,...] markers`.
pub fn has_encrypted_markers(s: &str) -> bool {
    for (start, end) in find_markers(s) {
        let inner = &s[start + ENC_PREFIX.len()..end - ENC_SUFFIX.len()];
        // Real age ciphertext is always >= 20 chars of base64
        if inner.len() >= 20 && B64.decode(inner).is_ok() {
            return true;
        }
    }
    false
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
                .map_err(|e| format!("invalid recipient '{r}': {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let encryptor = age::Encryptor::with_recipients(
        recipients.iter().map(|r| r.as_ref() as &dyn age::Recipient),
    )
    .map_err(|e| format!("encryptor init failed: {e}"))?;

    let mut encrypted = vec![];
    let mut writer = encryptor
        .wrap_output(&mut encrypted)
        .map_err(|e| format!("encryption error: {e}"))?;
    writer
        .write_all(plaintext.as_bytes())
        .map_err(|e| format!("write error: {e}"))?;
    writer
        .finish()
        .map_err(|e| format!("encryption finish error: {e}"))?;

    let encoded = B64.encode(&encrypted);
    Ok(format!("{ENC_PREFIX}{encoded}{ENC_SUFFIX}"))
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
        .map_err(|e| format!("base64 decode error: {e}"))?;

    let decryptor =
        age::Decryptor::new(&encrypted[..]).map_err(|e| format!("age decryptor error: {e}"))?;

    let mut reader = decryptor
        .decrypt(identities.iter().map(|i| i as &dyn age::Identity))
        .map_err(|e| format!("decryption failed: {e}"))?;
    let mut plaintext = String::new();
    reader
        .read_to_string(&mut plaintext)
        .map_err(|e| format!("read error: {e}"))?;
    Ok(plaintext)
}

/// Find and decrypt all `ENC[age,...]` markers in a string.
///
/// Returns the string with all markers replaced by their plaintext values.
/// If no markers are found, returns the string unchanged.
pub fn decrypt_all(s: &str, identities: &[age::x25519::Identity]) -> Result<String, String> {
    decrypt_all_counted(s, identities).map(|(s, _)| s)
}

/// Decrypt all markers and return `(decrypted_string, marker_count)`.
pub fn decrypt_all_counted(
    s: &str,
    identities: &[age::x25519::Identity],
) -> Result<(String, usize), String> {
    if !has_encrypted_markers(s) {
        return Ok((s.to_string(), 0));
    }

    let mut result = s.to_string();
    // Process markers from right to left to preserve positions
    let markers = find_markers(&result);
    let count = markers.len();
    for (start, end) in markers.into_iter().rev() {
        let marker = &result[start..end];
        let plaintext = decrypt_marker(marker, identities)?;
        result.replace_range(start..end, &plaintext);
    }
    Ok((result, count))
}

/// Find all `ENC[age,...]` marker positions in a string.
///
/// Returns `(start, end)` byte positions for each marker.
pub(crate) fn find_markers(s: &str) -> Vec<(usize, usize)> {
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
pub(crate) fn parse_identity(key_str: &str) -> Result<age::x25519::Identity, String> {
    key_str
        .trim()
        .parse::<age::x25519::Identity>()
        .map_err(|e| format!("invalid age identity: {e}"))
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
