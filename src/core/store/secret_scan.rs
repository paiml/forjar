//! FJ-1356: Secret scanning framework.
//!
//! Detects plaintext secrets in config YAML fields via regex patterns.
//! All sensitive values must use `ENC[age,...]` encryption — any plaintext
//! match is a validation error.

use std::sync::OnceLock;

/// A detected secret in configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct SecretFinding {
    /// Pattern that matched (e.g. "aws_access_key", "private_key_pem")
    pub pattern_name: String,
    /// Redacted matched text (first 8 chars + "...")
    pub matched_text: String,
    /// Location in config (e.g. "resource:nginx.pre_apply" or "params.db_password")
    pub location: String,
}

/// Result of scanning a config for secrets.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanResult {
    /// Detected secret findings.
    pub findings: Vec<SecretFinding>,
    /// Number of YAML fields scanned.
    pub scanned_fields: usize,
    /// True if no secrets were found.
    pub clean: bool,
}

/// A compiled secret detection pattern.
struct SecretPattern {
    name: &'static str,
    regex: regex::Regex,
}

fn compiled_patterns() -> &'static Vec<SecretPattern> {
    static PATTERNS: OnceLock<Vec<SecretPattern>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        let defs: Vec<(&str, &str)> = vec![
            ("aws_access_key", r"AKIA[0-9A-Z]{16}"),
            (
                "aws_secret_key",
                r"(?i)aws_secret_access_key\s*[=:]\s*\S{20,}",
            ),
            (
                "private_key_pem",
                r"-----BEGIN (RSA|EC|DSA|OPENSSH) PRIVATE KEY-----",
            ),
            ("github_token", r"gh[ps]_[A-Za-z0-9_]{36,}"),
            (
                "generic_api_key",
                r"(?i)(api[_\-]?key|apikey)\s*[=:]\s*['\x22]?\S{20,}",
            ),
            (
                "generic_secret",
                r"(?i)(secret|password|passwd|token)\s*[=:]\s*['\x22]?\S{8,}",
            ),
            (
                "jwt_token",
                r"eyJ[A-Za-z0-9_\-]{10,}\.eyJ[A-Za-z0-9_\-]{10,}",
            ),
            (
                "slack_webhook",
                r"https://hooks\.slack\.com/services/T[A-Z0-9]+/B[A-Z0-9]+/[a-zA-Z0-9]+",
            ),
            ("gcp_service_key", r#""type":\s*"service_account""#),
            ("stripe_key", r"[sr]k_(live|test)_[A-Za-z0-9]{20,}"),
            (
                "database_url_pass",
                r"(?i)(mysql|postgres|mongodb)://[^:]+:[^@]+@",
            ),
            (
                "base64_private",
                r"(?i)private.key.*=\s*[A-Za-z0-9+/]{40,}={0,2}",
            ),
            ("hex_secret_32", r"(?i)(secret|key)\s*[=:]\s*[0-9a-f]{32,}"),
            ("ssh_password", r"(?i)sshpass\s+-p\s+\S+"),
            ("age_plaintext", r"AGE-SECRET-KEY-1[A-Z0-9]{58}"),
        ];
        defs.into_iter()
            .map(|(name, pattern)| SecretPattern {
                name,
                regex: regex::Regex::new(pattern).unwrap_or_else(|e| {
                    panic!("invalid secret pattern '{name}': {e}");
                }),
            })
            .collect()
    })
}

/// Check if a value is properly encrypted with age.
pub fn is_encrypted(value: &str) -> bool {
    value.contains("ENC[age,")
}

/// Scan a text string for secret patterns.
///
/// Returns `(pattern_name, redacted_match)` for each match.
/// Skips values that are age-encrypted.
pub fn scan_text(text: &str) -> Vec<(String, String)> {
    if is_encrypted(text) {
        return Vec::new();
    }

    let mut findings = Vec::new();
    for pattern in compiled_patterns() {
        for m in pattern.regex.find_iter(text) {
            let matched = m.as_str();
            let redacted = redact(matched);
            findings.push((pattern.name.to_string(), redacted));
        }
    }
    findings
}

/// Scan all string fields in a config YAML value for secrets.
///
/// Walks the YAML tree recursively, scanning every string value.
pub fn scan_yaml_value(
    value: &serde_yaml_ng::Value,
    path: &str,
    findings: &mut Vec<SecretFinding>,
    scanned: &mut usize,
) {
    match value {
        serde_yaml_ng::Value::String(s) => {
            *scanned += 1;
            for (pattern_name, matched_text) in scan_text(s) {
                findings.push(SecretFinding {
                    pattern_name,
                    matched_text,
                    location: path.to_string(),
                });
            }
        }
        serde_yaml_ng::Value::Mapping(map) => {
            for (k, v) in map {
                let key_str = match k {
                    serde_yaml_ng::Value::String(s) => s.clone(),
                    _ => format!("{k:?}"),
                };
                let child_path = if path.is_empty() {
                    key_str
                } else {
                    format!("{path}.{key_str}")
                };
                scan_yaml_value(v, &child_path, findings, scanned);
            }
        }
        serde_yaml_ng::Value::Sequence(seq) => {
            for (i, v) in seq.iter().enumerate() {
                scan_yaml_value(v, &format!("{path}[{i}]"), findings, scanned);
            }
        }
        _ => {}
    }
}

/// Scan a YAML string for secrets.
pub fn scan_yaml_str(yaml: &str) -> ScanResult {
    let value: serde_yaml_ng::Value = match serde_yaml_ng::from_str(yaml) {
        Ok(v) => v,
        Err(_) => {
            // If YAML parse fails, scan as raw text
            let text_findings = scan_text(yaml);
            let findings = text_findings
                .into_iter()
                .map(|(pattern_name, matched_text)| SecretFinding {
                    pattern_name,
                    matched_text,
                    location: "raw_text".to_string(),
                })
                .collect::<Vec<_>>();
            let clean = findings.is_empty();
            return ScanResult {
                findings,
                scanned_fields: 1,
                clean,
            };
        }
    };

    let mut findings = Vec::new();
    let mut scanned = 0;
    scan_yaml_value(&value, "", &mut findings, &mut scanned);

    let clean = findings.is_empty();
    ScanResult {
        findings,
        scanned_fields: scanned,
        clean,
    }
}

/// Redact a matched secret: show first 8 chars + "..."
fn redact(s: &str) -> String {
    if s.len() <= 8 {
        format!("{s}...")
    } else {
        format!("{}...", s.get(..8).unwrap_or(s))
    }
}
