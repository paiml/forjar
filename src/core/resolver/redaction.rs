//! Refs #406 (CRUX audit E04): keep resolved secrets out of run transcripts.
//!
//! The executor resolves `{{secrets.*}}` INTO the resource before codegen, so
//! by the time `run_capture` writes `<res>.script`, `<res>.<action>.log` and
//! `<res>.<action>.json` the plaintext is already in the script. `--auto-commit`
//! then runs `git add state`. `redact_secrets` had shipped since FJ-2300 with no
//! production caller at all.
//!
//! # Why a literal `str::replace` is not enough
//!
//! Measured on unfixed `main`, a `type: file` resource whose `content:` holds a
//! secret produces this transcript:
//!
//! ```text
//! echo 'YXBpX3Rva2VuPWUwNC1QTEFJTlRFWFQtWnE3eDRLdjlMbTJSdzhUbi1E…' | base64 -d > '…'
//! ```
//!
//! The plaintext is not in that line, so #406's own success criterion ("grep the
//! state tree for the plaintext — zero matches") passes VACUOUSLY against the
//! bug. And `redact_secrets(script, [secret])` finds nothing either: the blob
//! encodes `api_token=` + secret + `\n`, and since `api_token=` is 10 bytes the
//! secret does not begin on a 3-byte boundary, so `base64(secret)` is not a
//! substring of `base64(content)`.
//!
//! So redaction here works on what a blob DECODES to, not on how it is spelled:
//! every base64 run in the transcript is decoded, and any run whose plaintext
//! contains a secret is replaced wholesale. `sensitive: true` remains the answer
//! for the general case — a value forjar cannot name (derived on the host,
//! compressed, re-encoded) can never be redacted by matching.

use crate::core::secrets::B64;
use crate::core::types::{Resource, SecretsConfig};
use base64::Engine;

/// Shortest base64 run worth decoding. Four characters is one full base64
/// group; anything shorter cannot encode a byte.
const MIN_BLOB_LEN: usize = 4;

/// The plaintext values this resource's templates resolve to.
///
/// Takes the UNRESOLVED resource — the one still carrying `{{secrets.*}}` — and
/// re-resolves each referenced key through the same provider the executor used,
/// so the caller can strike those values out of the transcript.
///
/// A key that fails to resolve is skipped rather than fatal: this runs on the
/// reporting path, after the resource has already executed, and a redaction
/// pass must never be able to fail an apply that converged.
pub fn collect_secret_values(resource: &Resource, secrets_cfg: &SecretsConfig) -> Vec<String> {
    let Ok(yaml) = serde_yaml_ng::to_string(resource) else {
        return Vec::new();
    };
    let mut values: Vec<String> = Vec::new();
    for key in secret_keys(&yaml) {
        let Ok(value) = super::template::resolve_secret(&key, secrets_cfg) else {
            continue;
        };
        if !value.is_empty() && !values.contains(&value) {
            values.push(value);
        }
    }
    values
}

/// Every `secrets.*` key named by a `{{ … }}` span in `yaml`.
///
/// Scans the serialized resource rather than a hand-maintained field list:
/// `resolve_resource_templates_with_secrets` already needs such a list and has
/// its own completeness test guarding it, and a redactor that misses a field is
/// a leak rather than an unresolved template.
fn secret_keys(yaml: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut rest = yaml;
    while let Some(open) = rest.find("{{") {
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else { break };
        if let Some(key) = after[..close].trim().strip_prefix("secrets.") {
            keys.push(key.to_string());
        }
        rest = &after[close + 2..];
    }
    keys
}

/// Strike every recoverable form of `secrets` out of one transcript stream.
///
/// Two passes, because a secret reaches a transcript two ways: spliced literally
/// into a `command:`, and base64-encoded inside a `file` resource's content.
pub fn redact_transcript(text: &str, secrets: &[String]) -> String {
    if secrets.is_empty() || text.is_empty() {
        return text.to_string();
    }
    let literal = super::template::redact_secrets(text, secrets);
    redact_encoded(&literal, secrets)
}

/// Replace any base64 run whose DECODED bytes contain a secret.
fn redact_encoded(text: &str, secrets: &[String]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut run = String::new();
    for ch in text.chars() {
        if is_b64_char(ch) {
            run.push(ch);
        } else {
            flush_run(&mut out, &mut run, secrets);
            out.push(ch);
        }
    }
    flush_run(&mut out, &mut run, secrets);
    out
}

/// Emit `run` — as `***` when it decodes to something holding a secret.
fn flush_run(out: &mut String, run: &mut String, secrets: &[String]) {
    if run.len() >= MIN_BLOB_LEN && blob_reveals(run, secrets) {
        out.push_str("***");
    } else {
        out.push_str(run);
    }
    run.clear();
}

fn is_b64_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '+' || ch == '/' || ch == '='
}

/// Does this base64 run decode to text containing one of `secrets`?
fn blob_reveals(run: &str, secrets: &[String]) -> bool {
    let Ok(decoded) = B64.decode(run) else {
        return false;
    };
    let plain = String::from_utf8_lossy(&decoded);
    secrets
        .iter()
        .any(|s| !s.is_empty() && plain.contains(s.as_str()))
}
