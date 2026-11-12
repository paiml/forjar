//! Secrets management.

use super::commands::SecretsCmd;
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

/// Write one line to stdout without `println!`: that panics when stdout fails
/// (ENOSPC, EPIPE), and with panic=abort the core dump holds the plaintext.
#[cfg(feature = "encryption")]
fn emit_line(line: &str) -> Result<(), String> {
    emit_to(&mut std::io::stdout().lock(), line)
}

/// `emit_line` over any writer, so a failing sink is testable.
#[cfg(feature = "encryption")]
fn emit_to<W: std::io::Write>(out: &mut W, line: &str) -> Result<(), String> {
    out.write_all(line.as_bytes())
        .and_then(|()| out.write_all(b"\n"))
        .and_then(|()| out.flush())
        .map_err(|_| "cannot write to stdout".to_string())
}

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
    emit_line(&encrypted)
}

#[cfg(not(feature = "encryption"))]
pub(crate) fn cmd_secrets_encrypt(_value: &str, _recipients: &[String]) -> Result<(), String> {
    no_encryption_error()
}

#[cfg(feature = "encryption")]
pub(crate) fn cmd_secrets_decrypt(value: &str, identity_path: Option<&Path>) -> Result<(), String> {
    let identities = secrets::load_identities(identity_path)?;
    let plaintext = secrets::decrypt_marker(value, &identities)?;
    emit_line(&plaintext)
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
            emit_line(key_str.expose_secret())?;
        }
        Some(path) => {
            let body = format!(
                "# created: {created}\n# public key: {recipient}\n{}\n",
                key_str.expose_secret()
            );
            write_identity_0600(path, &body)?;
            eprintln!("# public key: {recipient}");
            eprintln!("# identity written (mode 0600)");
        }
    }
    Ok(())
}

/// Index of the subcommand word: the first argv word after the program name
/// that does not start with `-`. Every earlier word is a global flag or an
/// unknown flag clap would quote (`--verbose=S`, `--S`), so none counts.
fn subcommand_index<S: AsRef<str>>(argv: &[S]) -> Option<usize> {
    (1..argv.len()).find(|&i| !argv[i].as_ref().starts_with('-'))
}

/// True when this argv is a `secrets ... encrypt` invocation, wherever global
/// flags sit between the two words. clap quotes the offending argument in its
/// usage errors, and for `secrets encrypt` that argument can be the secret, so
/// the binary prints a fixed message instead (see `main.rs`). Only the
/// subcommand position counts: `apply -f secrets encrypt` is not this.
pub fn argv_is_secrets_encrypt<S: AsRef<str>>(argv: &[S]) -> bool {
    subcommand_index(argv).is_some_and(|i| {
        argv[i].as_ref() == "secrets" && argv[i + 1..].iter().any(|a| a.as_ref() == "encrypt")
    })
}

/// True when the SUBCOMMAND word is `secrets` or starts with `secret` (a
/// secret can be glued onto it: `secrets=S`, `secretsS`, `secret=S`), so every
/// clap usage error for the family is replaced by a fixed message (see
/// `main.rs`). A `secrets*.yaml` argument to another command does not count,
/// so `apply -f secrets.yaml --bogus` keeps clap's real error.
pub fn argv_mentions_secrets<S: AsRef<str>>(argv: &[S]) -> bool {
    subcommand_index(argv).is_some_and(|i| {
        let w = argv[i].as_ref().to_ascii_lowercase();
        // clap's built-in `help` quotes whatever follows it.
        w.starts_with("secret") || w == "help"
    })
}

/// True when the subcommand word is clap's built-in `help` (case-insensitive).
pub fn argv_is_help<S: AsRef<str>>(argv: &[S]) -> bool {
    subcommand_index(argv).is_some_and(|i| argv[i].as_ref().eq_ignore_ascii_case("help"))
}

/// True when a LATER word is `secrets` or starts with `secret`. Used for an
/// InvalidSubcommand error: `--verbose S secrets encrypt` makes the secret
/// take the subcommand slot, and clap then quotes it.
pub fn argv_has_later_secrets_word<S: AsRef<str>>(argv: &[S]) -> bool {
    subcommand_index(argv).is_some_and(|i| {
        argv[i + 1..]
            .iter()
            .any(|a| a.as_ref().to_ascii_lowercase().starts_with("secret"))
    })
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
        .map_err(|e| format!("cannot create the identity output path: {:?}", e.kind()))?;
    f.write_all(body.as_bytes())
        .map_err(|e| format!("cannot write the identity output path: {:?}", e.kind()))
}

/// `secrets encrypt` entry point: resolve the plaintext (file or stdin), then encrypt.
pub(crate) fn cmd_secrets_encrypt_input(
    from_file: Option<&Path>,
    rest: &[String],
    recipients: &[String],
) -> Result<(), String> {
    let plaintext = resolve_encrypt_input(from_file, rest)?;
    cmd_secrets_encrypt(&plaintext, recipients)
}

/// Largest plaintext `secrets encrypt` accepts (stdin or `--from-file`).
/// Reading is bounded so input size alone cannot reach an OOM abort, whose
/// core dump would hold the plaintext.
pub(crate) const MAX_PLAINTEXT_BYTES: usize = 16 * 1024 * 1024;

/// Read at most `cap` bytes of UTF-8 text; one byte more is refused with a
/// fixed message (nothing of the input is echoed).
pub(crate) fn read_capped<R: std::io::Read>(r: R, cap: usize) -> Result<String, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    r.take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("cannot read the plaintext: {:?}", e.kind()))?;
    if bytes.len() > cap {
        return Err("plaintext exceeds the 16 MiB limit".to_string());
    }
    String::from_utf8(bytes).map_err(|_| "plaintext is not valid UTF-8".to_string())
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
            // The path is not echoed: a secret can land in this slot.
            let f = std::fs::File::open(p)
                .map_err(|e| format!("cannot read the --from-file path: {:?}", e.kind()))?;
            read_capped(f, MAX_PLAINTEXT_BYTES)?
        }
        None => read_capped(std::io::stdin().lock(), MAX_PLAINTEXT_BYTES)?,
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

/// Read a secrets file; the path is not echoed (only the io error kind).
#[cfg(feature = "encryption")]
fn read_secrets_file(file: &Path) -> Result<String, String> {
    std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read the secrets file: {:?}", e.kind()))
}

#[cfg(feature = "encryption")]
pub(crate) fn cmd_secrets_view(file: &Path, identity_path: Option<&Path>) -> Result<(), String> {
    let content = read_secrets_file(file)?;
    if !secrets::has_encrypted_markers(&content) {
        return emit_line(&content);
    }
    let identities = secrets::load_identities(identity_path)?;
    let decrypted = secrets::decrypt_all(&content, &identities)?;
    emit_line(&decrypted)
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
    let content = read_secrets_file(file)?;
    if !secrets::has_encrypted_markers(&content) {
        emit_line("no ENC[age,...] markers found")?;
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
        .map_err(|e| format!("cannot write the secrets file: {:?}", e.kind()))?;
    let count = find_enc_markers(&result).len();
    emit_line(&format!("re-encrypted {count} secret(s)"))?;
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

    let content = read_secrets_file(file)?;
    if !secrets::has_encrypted_markers(&content) {
        emit_line("no ENC[age,...] markers found")?;
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
        .map_err(|e| format!("cannot write the secrets file: {:?}", e.kind()))?;

    let event = ProvenanceEvent::SecretRotated {
        file: file.display().to_string(),
        marker_count: marker_count as u32,
        new_recipients: new_recipients.to_vec(),
    };
    let _ = eventlog::append_event(state_dir, "__secrets__", event);

    emit_line(&format!(
        "rotated {} secret(s) to {} recipient(s)",
        marker_count,
        new_recipients.len()
    ))?;
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

/// Dispatch `forjar secrets <sub>`.
pub(crate) fn dispatch_secrets(sub: SecretsCmd) -> Result<(), String> {
    match sub {
        SecretsCmd::Encrypt {
            from_file,
            rest,
            recipient,
            ..
        } => cmd_secrets_encrypt_input(from_file.as_deref(), &rest, &recipient),
        SecretsCmd::Decrypt { value, identity } => cmd_secrets_decrypt(&value, identity.as_deref()),
        SecretsCmd::Keygen { output } => cmd_secrets_keygen(output.as_deref()),
        SecretsCmd::View { file, identity } => cmd_secrets_view(&file, identity.as_deref()),
        SecretsCmd::Rekey {
            file,
            identity,
            recipient,
        } => cmd_secrets_rekey(&file, identity.as_deref(), &recipient),
        SecretsCmd::Rotate {
            file,
            identity,
            recipient,
            re_encrypt,
            state_dir,
        } => cmd_secrets_rotate(
            &file,
            identity.as_deref(),
            &recipient,
            re_encrypt,
            &state_dir,
        ),
    }
}

#[cfg(test)]
#[path = "secrets_input_tests.rs"]
mod input_tests;
