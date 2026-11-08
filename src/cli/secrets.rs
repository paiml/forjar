//! Secrets management.

#[cfg(feature = "encryption")]
use crate::core::secrets;
use std::path::Path;

// ─── FJ-200: Secrets commands ─────────────────────────────────────

#[cfg(feature = "encryption")]
use super::helpers_time::*;
#[cfg(feature = "encryption")]
use crate::core::types::ProvenanceEvent;
#[cfg(feature = "encryption")]
use crate::tripwire::eventlog;

#[cfg(not(feature = "encryption"))]
fn no_encryption_error() -> Result<(), String> {
    Err(
        "forjar was compiled without encryption support. Rebuild with `--features encryption`."
            .to_string(),
    )
}

#[cfg(feature = "encryption")]
pub(crate) fn cmd_secrets_encrypt(value: &str, recipients: &[String]) -> Result<(), String> {
    let recipient_refs: Vec<&str> = recipients.iter().map(|r| r.as_str()).collect();
    let encrypted = secrets::encrypt(value, &recipient_refs)?;
    println!("{encrypted}");
    Ok(())
}

#[cfg(not(feature = "encryption"))]
pub(crate) fn cmd_secrets_encrypt(_value: &str, _recipients: &[String]) -> Result<(), String> {
    no_encryption_error()
}

#[cfg(feature = "encryption")]
pub(crate) fn cmd_secrets_decrypt(value: &str, identity_path: Option<&Path>) -> Result<(), String> {
    let identities = secrets::load_identities(identity_path)?;
    let plaintext = secrets::decrypt_marker(value, &identities)?;
    println!("{plaintext}");
    Ok(())
}

#[cfg(not(feature = "encryption"))]
pub(crate) fn cmd_secrets_decrypt(
    _value: &str,
    _identity_path: Option<&Path>,
) -> Result<(), String> {
    no_encryption_error()
}

#[cfg(feature = "encryption")]
pub(crate) fn cmd_secrets_keygen(output: Option<&Path>) -> Result<(), String> {
    use age::secrecy::ExposeSecret;
    let identity = secrets::generate_identity();
    let recipient = secrets::identity_to_recipient(&identity);
    let key_str = identity.to_string();
    let created = chrono_date();
    match output {
        None => {
            eprintln!("# created: {created}");
            eprintln!("# public key: {recipient}");
            println!("{}", key_str.expose_secret());
        }
        Some(path) => {
            let body = format!(
                "# created: {created}\n# public key: {recipient}\n{}\n",
                key_str.expose_secret()
            );
            write_identity_0600(path, &body)?;
            eprintln!("# public key: {recipient}");
            eprintln!("# identity written to {} (mode 0600)", path.display());
        }
    }
    Ok(())
}

/// Create `path` with mode 0600 from the first syscall (O_CREAT|O_EXCL), so the
/// secret is never readable by anyone else, even briefly. Refuses to overwrite.
#[cfg(feature = "encryption")]
fn write_identity_0600(path: &Path, body: &str) -> Result<(), String> {
    use std::io::Write;
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts
        .open(path)
        .map_err(|e| format!("cannot create '{}': {}", path.display(), e))?;
    f.write_all(body.as_bytes())
        .map_err(|e| format!("cannot write '{}': {}", path.display(), e))
}

/// Resolve the plaintext for `secrets encrypt`: `--from-file PATH`, else stdin
/// (one trailing newline removed). A positional value is refused WITHOUT
/// echoing it: argv leaks via `ps` and /proc/<pid>/cmdline.
pub(crate) fn resolve_encrypt_input(
    from_file: Option<&Path>,
    rest: &[String],
) -> Result<String, String> {
    if !rest.is_empty() {
        return Err(
            "a secret must not be passed as an argument (it would be visible in \
             `ps`); pipe it on stdin (`forjar secrets encrypt -r R < file`) or use \
             --from-file PATH"
                .to_string(),
        );
    }
    let mut buf = match from_file {
        Some(p) => {
            std::fs::read_to_string(p).map_err(|e| format!("cannot read '{}': {e}", p.display()))?
        }
        None => {
            use std::io::Read;
            let mut s = String::new();
            std::io::stdin()
                .read_to_string(&mut s)
                .map_err(|e| format!("cannot read stdin: {e}"))?;
            s
        }
    };
    if buf.ends_with('\n') {
        buf.pop();
        if buf.ends_with('\r') {
            buf.pop();
        }
    }
    if buf.is_empty() {
        return Err("no plaintext given: pipe it on stdin or use --from-file PATH".to_string());
    }
    Ok(buf)
}

#[cfg(not(feature = "encryption"))]
pub(crate) fn cmd_secrets_keygen(_output: Option<&Path>) -> Result<(), String> {
    no_encryption_error()
}

#[cfg(feature = "encryption")]
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

#[cfg(not(feature = "encryption"))]
pub(crate) fn cmd_secrets_view(_file: &Path, _identity_path: Option<&Path>) -> Result<(), String> {
    no_encryption_error()
}

#[cfg(feature = "encryption")]
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

    let mut result = content.clone();
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

#[cfg(not(feature = "encryption"))]
pub(crate) fn cmd_secrets_rekey(
    _file: &Path,
    _identity_path: Option<&Path>,
    _new_recipients: &[String],
) -> Result<(), String> {
    no_encryption_error()
}

#[cfg(feature = "encryption")]
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

#[cfg(not(feature = "encryption"))]
pub(crate) fn cmd_secrets_rotate(
    _file: &Path,
    _identity_path: Option<&Path>,
    _new_recipients: &[String],
    _re_encrypt: bool,
    _state_dir: &Path,
) -> Result<(), String> {
    no_encryption_error()
}

#[cfg(any(feature = "encryption", test))]
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
