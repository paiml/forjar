//! Refs #406: unit falsification for transcript redaction.
//!
//! The end-to-end proof lives in `tests/falsification_e04_run_log_secrets.rs`
//! (real binary, real apply, greps the whole state tree). These cases pin the
//! two properties that suite cannot isolate: that a base64 blob is redacted by
//! what it DECODES to, and that redaction leaves the rest of the script intact.

use super::redaction::{collect_secret_values, redact_transcript};
use crate::core::types::{Resource, ResourceType, SecretsConfig};

const SECRET: &str = "hunter2-Zq7x4Kv9Lm2Rw8Tn";

fn b64(s: &str) -> String {
    use crate::core::secrets::B64;
    use base64::Engine;
    B64.encode(s.as_bytes())
}

#[test]
fn literal_secret_is_struck_from_the_transcript() {
    let script = format!("printf '%s' 'literal={SECRET}' > /tmp/x");
    let out = redact_transcript(&script, &[SECRET.to_string()]);
    assert!(!out.contains(SECRET), "{out}");
    assert!(out.contains("printf '%s' 'literal=***'"), "{out}");
}

/// THE CASE `redact_secrets` ALONE CANNOT SEE. `codegen::file` emits
/// `echo '<base64 of the whole content>' | base64 -d > path`. The secret starts
/// at byte 10 of that content, so it is not 3-byte aligned and `base64(secret)`
/// is not a substring of the blob. Matching on the value therefore finds
/// nothing; the blob has to be decoded.
#[test]
fn base64_encoded_secret_is_struck_even_though_the_value_never_appears() {
    let content = format!("api_token={SECRET}\n");
    let blob = b64(&content);
    assert!(
        !blob.contains(&b64(SECRET)),
        "precondition: the secret must NOT be 3-byte aligned inside the blob"
    );
    let script =
        format!("echo '{blob}' | base64 -d > '/etc/app.conf'\nchmod '0600' '/etc/app.conf'");

    // The old path — value substitution only — leaves the blob untouched.
    let value_only = super::redact_secrets(&script, &[SECRET.to_string()]);
    assert!(
        value_only.contains(&blob),
        "value-only redaction is expected to miss the blob; if it stopped \
         missing it, this test no longer proves what it claims"
    );

    let out = redact_transcript(&script, &[SECRET.to_string()]);
    assert!(!out.contains(&blob), "blob survived redaction:\n{out}");
    assert!(out.contains("| base64 -d > '/etc/app.conf'"), "{out}");
    assert!(out.contains("chmod '0600'"), "{out}");
}

#[test]
fn innocent_base64_survives() {
    let blob = b64("nothing to see here at all");
    let script = format!("echo '{blob}' | base64 -d");
    let out = redact_transcript(&script, &[SECRET.to_string()]);
    assert_eq!(out, script);
}

#[test]
fn no_secrets_is_the_identity() {
    let script = "echo hello";
    assert_eq!(redact_transcript(script, &[]), script);
}

#[test]
fn empty_secret_never_shreds_the_transcript() {
    let script = "echo hello";
    assert_eq!(redact_transcript(script, &[String::new()]), script);
}

/// A `file`-provider config pointing at a temp directory: no process-wide env
/// mutation, so these cases are safe to run in parallel with the rest of the
/// suite (which is why they do not use `env::set_var`).
fn file_secrets(dir: &std::path::Path, key: &str, value: &str) -> SecretsConfig {
    std::fs::write(dir.join(key), value).unwrap();
    SecretsConfig {
        provider: Some("file".into()),
        path: Some(dir.to_string_lossy().into_owned()),
        ..Default::default()
    }
}

#[test]
fn collect_finds_the_secret_a_resource_interpolates() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = file_secrets(tmp.path(), "e04-unit", SECRET);
    let r = Resource {
        resource_type: ResourceType::File,
        path: Some("/etc/app.conf".into()),
        content: Some("api_token={{secrets.e04-unit}}\n".into()),
        ..Default::default()
    };
    assert_eq!(collect_secret_values(&r, &cfg), vec![SECRET.to_string()]);
}

/// Not a hand-maintained field list: `command:` is nowhere near `content:` in
/// `Resource`, and the collector must find both without being told about either.
#[test]
fn collect_finds_secrets_in_any_field() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = file_secrets(tmp.path(), "e04-cmd", SECRET);
    let r = Resource {
        resource_type: ResourceType::Task,
        command: Some("curl -H 'Auth: {{secrets.e04-cmd}}' https://x".into()),
        ..Default::default()
    };
    assert_eq!(collect_secret_values(&r, &cfg), vec![SECRET.to_string()]);
}

/// The same value referenced twice is collected once — a duplicate would make
/// the redactor do the same replacement twice for no gain.
#[test]
fn collect_deduplicates() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = file_secrets(tmp.path(), "e04-dup", SECRET);
    let r = Resource {
        resource_type: ResourceType::Task,
        command: Some("{{secrets.e04-dup}} {{secrets.e04-dup}}".into()),
        ..Default::default()
    };
    assert_eq!(collect_secret_values(&r, &cfg), vec![SECRET.to_string()]);
}

#[test]
fn collect_skips_keys_that_do_not_resolve() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = file_secrets(tmp.path(), "unrelated", SECRET);
    let r = Resource {
        resource_type: ResourceType::File,
        content: Some("{{secrets.e04-absent-key-nobody-set}}".into()),
        ..Default::default()
    };
    assert!(collect_secret_values(&r, &cfg).is_empty());
}

#[test]
fn collect_ignores_non_secret_templates() {
    let r = Resource {
        resource_type: ResourceType::File,
        content: Some("{{params.env}} {{machine.web.addr}}".into()),
        ..Default::default()
    };
    assert!(collect_secret_values(&r, &SecretsConfig::default()).is_empty());
}
