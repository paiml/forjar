//! Secrets management.

use super::helpers_time::*;
use crate::core::secrets;
use crate::core::types::ProvenanceEvent;
use crate::tripwire::eventlog;
use std::path::Path;

// ─── FJ-200: Secrets commands ─────────────────────────────────────

pub(crate) fn cmd_secrets_encrypt(value: &str, recipients: &[String]) -> Result<(), String> {
    let recipient_refs: Vec<&str> = recipients.iter().map(|r| r.as_str()).collect();
    let encrypted = secrets::encrypt(value, &recipient_refs)?;
    println!("{encrypted}");
    Ok(())
}

pub(crate) fn cmd_secrets_decrypt(value: &str, identity_path: Option<&Path>) -> Result<(), String> {
    let identities = secrets::load_identities(identity_path)?;
    let plaintext = secrets::decrypt_marker(value, &identities)?;
    println!("{plaintext}");
    Ok(())
}

pub(crate) fn cmd_secrets_keygen() -> Result<(), String> {
    use age::secrecy::ExposeSecret;
    let identity = secrets::generate_identity();
    let recipient = secrets::identity_to_recipient(&identity);
    let key_str = identity.to_string();
    eprintln!("# created: {}", chrono_date());
    eprintln!("# public key: {recipient}");
    println!("{}", key_str.expose_secret());
    Ok(())
}

pub(crate) fn cmd_secrets_view(file: &Path, identity_path: Option<&Path>) -> Result<(), String> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read '{}': {}", file.display(), e))?;
    if !secrets::has_encrypted_markers(&content) {
        println!("{content}");
        return Ok(());
    }
    let identities = secrets::load_identities(identity_path)?;
    let decrypted = secrets::decrypt_all(&content, &identities)?;
    println!("{decrypted}");
    Ok(())
}

pub(crate) fn cmd_secrets_rekey(
    file: &Path,
    identity_path: Option<&Path>,
    new_recipients: &[String],
) -> Result<(), String> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read '{}': {}", file.display(), e))?;
    if !secrets::has_encrypted_markers(&content) {
        println!("no ENC[age,...] markers found in {}", file.display());
        return Ok(());
    }

    let identities = secrets::load_identities(identity_path)?;
    let recipient_refs: Vec<&str> = new_recipients.iter().map(|r| r.as_str()).collect();

    // Find all markers, decrypt each, re-encrypt with new recipients
    let mut result = content.clone();
    // Process from right to left to preserve positions
    let markers = find_enc_markers(&result);
    for (start, end) in markers.into_iter().rev() {
        let marker = &result[start..end].to_string();
        let plaintext = secrets::decrypt_marker(marker, &identities)?;
        let re_encrypted = secrets::encrypt(&plaintext, &recipient_refs)?;
        result.replace_range(start..end, &re_encrypted);
    }

    std::fs::write(file, &result)
        .map_err(|e| format!("cannot write '{}': {}", file.display(), e))?;
    let count = find_enc_markers(&result).len();
    println!("re-encrypted {} secret(s) in {}", count, file.display());
    Ok(())
}

pub(crate) fn cmd_secrets_rotate(
    file: &Path,
    identity_path: Option<&Path>,
    new_recipients: &[String],
    re_encrypt: bool,
    state_dir: &Path,
) -> Result<(), String> {
    if !re_encrypt {
        return Err(
            "pass --re-encrypt to confirm secret rotation (destructive operation)".to_string(),
        );
    }

    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read '{}': {}", file.display(), e))?;
    if !secrets::has_encrypted_markers(&content) {
        println!("no ENC[age,...] markers found in {}", file.display());
        return Ok(());
    }

    let identities = secrets::load_identities(identity_path)?;
    let recipient_refs: Vec<&str> = new_recipients.iter().map(|r| r.as_str()).collect();

    // Find all markers, decrypt each, re-encrypt with new recipients
    let mut result = content.clone();
    let markers = find_enc_markers(&result);
    let marker_count = markers.len();

    for (start, end) in markers.into_iter().rev() {
        let marker = result[start..end].to_string();
        let plaintext = secrets::decrypt_marker(&marker, &identities)?;
        let re_encrypted = secrets::encrypt(&plaintext, &recipient_refs)?;
        result.replace_range(start..end, &re_encrypted);
    }

    std::fs::write(file, &result)
        .map_err(|e| format!("cannot write '{}': {}", file.display(), e))?;

    // FJ-201: Audit log the rotation event
    let event = ProvenanceEvent::SecretRotated {
        file: file.display().to_string(),
        marker_count: marker_count as u32,
        new_recipients: new_recipients.to_vec(),
    };
    let _ = eventlog::append_event(state_dir, "__secrets__", event);

    println!(
        "rotated {} secret(s) in {} to {} recipient(s)",
        marker_count,
        file.display(),
        new_recipients.len()
    );
    Ok(())
}

pub(crate) fn find_enc_markers(s: &str) -> Vec<(usize, usize)> {
    let prefix = "ENC[age,";
    let suffix = "]";
    let mut markers = Vec::new();
    let mut start = 0;
    while let Some(pos) = s[start..].find(prefix) {
        let abs_start = start + pos;
        let after_prefix = abs_start + prefix.len();
        if let Some(end_pos) = s[after_prefix..].find(suffix) {
            let abs_end = after_prefix + end_pos + suffix.len();
            markers.push((abs_start, abs_end));
            start = abs_end;
        } else {
            break;
        }
    }
    markers
}
