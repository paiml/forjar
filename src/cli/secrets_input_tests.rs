//! Unit tests for the `secrets` input handling (PMAT-893): plaintext
//! resolution, the 0600 identity writer and the clap-error redaction predicate.

use super::*;

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn predicate_sees_encrypt_after_global_flags() {
    assert!(argv_is_secrets_encrypt(&args(&[
        "forjar", "secrets", "encrypt"
    ])));
    assert!(argv_is_secrets_encrypt(&args(&[
        "forjar",
        "secrets",
        "-v",
        "encrypt",
        "--stdin=X"
    ])));
    assert!(argv_is_secrets_encrypt(&args(&[
        "forjar",
        "--no-color",
        "secrets",
        "--no-color",
        "encrypt"
    ])));
}

#[test]
fn predicate_ignores_other_secrets_verbs_and_other_commands() {
    assert!(!argv_is_secrets_encrypt(&args(&[
        "forjar", "secrets", "keygen"
    ])));
    assert!(!argv_is_secrets_encrypt(&args(&["forjar", "encrypt"])));
    assert!(!argv_is_secrets_encrypt(&args(&[
        "forjar", "encrypt", "secrets"
    ])));
    assert!(!argv_is_secrets_encrypt::<&str>(&[]));
}

#[test]
fn resolve_refuses_positional_without_echoing_it() {
    let rest = args(&["-----BEGIN-topsecret"]);
    let err = resolve_encrypt_input(None, &rest).unwrap_err();
    assert!(err.contains("must not be passed as an argument"), "{err}");
    assert!(
        !err.contains("topsecret") && !err.contains("BEGIN"),
        "{err}"
    );
}

#[test]
fn resolve_reads_file_and_strips_one_trailing_newline() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("p");
    std::fs::write(&f, "line1\nline2\n").unwrap();
    assert_eq!(
        resolve_encrypt_input(Some(&f), &[]).unwrap(),
        "line1\nline2"
    );
    std::fs::write(&f, "crlf\r\n").unwrap();
    assert_eq!(resolve_encrypt_input(Some(&f), &[]).unwrap(), "crlf");
    std::fs::write(&f, "two\n\n").unwrap();
    assert_eq!(resolve_encrypt_input(Some(&f), &[]).unwrap(), "two\n");
}

#[test]
fn resolve_rejects_empty_plaintext_and_missing_file() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("empty");
    std::fs::write(&f, "\n").unwrap();
    let err = resolve_encrypt_input(Some(&f), &[]).unwrap_err();
    assert!(err.contains("no plaintext given"), "{err}");
    let err = resolve_encrypt_input(Some(&d.path().join("nope")), &[]).unwrap_err();
    assert!(err.contains("cannot read the --from-file path"), "{err}");
    assert!(!err.contains("nope"), "{err}");
}

#[cfg(all(feature = "encryption", unix))]
#[test]
fn identity_is_written_0600_and_never_overwritten() {
    use std::os::unix::fs::PermissionsExt;
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("id.txt");
    // Only discriminating under a umask that leaves group/other bits (022);
    // the integration test pins umask 022 in the child, this one cannot (no
    // unsafe/libc), so it is a plain check under whatever umask the harness has.
    write_identity_0600(&f, "AGE-SECRET-KEY-1\n").unwrap();
    let mode = std::fs::metadata(&f).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let err = write_identity_0600(&f, "other").unwrap_err();
    assert!(err.contains("cannot create"), "{err}");
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "AGE-SECRET-KEY-1\n");
}

#[test]
fn mentions_predicate_catches_glued_words() {
    assert!(argv_mentions_secrets(&args(&[
        "forjar",
        "secrets=S",
        "encrypt"
    ])));
    assert!(argv_mentions_secrets(&args(&["forjar", "secretsS"])));
    assert!(argv_mentions_secrets(&args(&["forjar", "secrets", "encX"])));
    assert!(!argv_mentions_secrets(&args(&["forjar", "apply"])));
    assert!(!argv_mentions_secrets::<&str>(&[]));
}
