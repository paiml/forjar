//! Tests for FJ-1356: Secret scanning framework.

use super::secret_scan::*;

// ── Pattern Detection Tests ─────────────────────────────────────────

#[test]
fn test_aws_access_key() {
    let findings = scan_text("AKIAIOSFODNN7EXAMPLE");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].0, "aws_access_key");
}

#[test]
fn test_aws_secret_key() {
    let findings = scan_text("aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].0, "aws_secret_key");
}

#[test]
fn test_private_key_pem() {
    let findings = scan_text("-----BEGIN RSA PRIVATE KEY-----");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].0, "private_key_pem");
}

#[test]
fn test_private_key_pem_ec() {
    let findings = scan_text("-----BEGIN EC PRIVATE KEY-----");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].0, "private_key_pem");
}

#[test]
fn test_private_key_pem_openssh() {
    let findings = scan_text("-----BEGIN OPENSSH PRIVATE KEY-----");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].0, "private_key_pem");
}

#[test]
fn test_github_token() {
    let token = format!("ghp_{}", "A".repeat(36));
    let findings = scan_text(&token);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].0, "github_token");
}

#[test]
fn test_generic_api_key() {
    let findings = scan_text("api_key: sk-proj-ABCDEFGHIJKLMNOPQRSTUVWX");
    assert!(!findings.is_empty());
    assert!(findings.iter().any(|(name, _)| name == "generic_api_key"));
}

#[test]
fn test_generic_secret() {
    let findings = scan_text("password: SuperSecretPassword123");
    assert!(!findings.is_empty());
    assert!(findings.iter().any(|(name, _)| name == "generic_secret"));
}

#[test]
fn test_jwt_token() {
    let findings = scan_text("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].0, "jwt_token");
}

#[test]
fn test_slack_webhook() {
    // Construct dynamically to avoid GitHub push protection triggering on test data
    let url = format!(
        "https://hooks.slack.com/services/T{}/B{}/{}",
        "A".repeat(9),
        "B".repeat(9),
        "x".repeat(24)
    );
    let findings = scan_text(&url);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].0, "slack_webhook");
}

#[test]
fn test_gcp_service_key() {
    let findings = scan_text(r#""type": "service_account""#);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].0, "gcp_service_key");
}

#[test]
fn test_stripe_key() {
    // Use sk_test_ prefix to avoid GitHub push protection on sk_live_
    let key = format!("sk_test_{}", "a".repeat(24));
    let findings = scan_text(&key);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].0, "stripe_key");
}

#[test]
fn test_database_url_pass() {
    let findings = scan_text("postgres://admin:secretpass@db.internal:5432/mydb");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].0, "database_url_pass");
}

#[test]
fn test_hex_secret_32() {
    let hex = "a".repeat(32);
    let findings = scan_text(&format!("secret: {hex}"));
    assert!(!findings.is_empty());
    assert!(findings.iter().any(|(name, _)| name == "hex_secret_32"));
}

#[test]
fn test_ssh_password() {
    let findings = scan_text("sshpass -p mysecretpassword ssh user@host");
    assert!(!findings.is_empty());
    assert!(findings.iter().any(|(name, _)| name == "ssh_password"));
}

#[test]
fn test_age_plaintext_key() {
    let key = format!("AGE-SECRET-KEY-1{}", "A".repeat(58));
    let findings = scan_text(&key);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].0, "age_plaintext");
}

// ── False Positive Resistance ───────────────────────────────────────

#[test]
fn test_encrypted_value_skipped() {
    let findings = scan_text("ENC[age,password: SuperSecretPassword123]");
    assert!(findings.is_empty(), "encrypted values should be skipped");
}

#[test]
fn test_clean_text_no_findings() {
    let findings = scan_text("apt install nginx curl wget");
    assert!(findings.is_empty());
}

#[test]
fn test_normal_yaml_no_findings() {
    let findings = scan_text("name: my-server\npackages:\n  - nginx\n  - curl");
    assert!(findings.is_empty());
}

// ── is_encrypted Tests ──────────────────────────────────────────────

#[test]
fn test_is_encrypted_true() {
    assert!(is_encrypted("ENC[age,abc123...]"));
}

#[test]
fn test_is_encrypted_false() {
    assert!(!is_encrypted("plaintext-value"));
}

// ── Redaction Tests ─────────────────────────────────────────────────

#[test]
fn test_redaction_in_findings() {
    let findings = scan_text("AKIAIOSFODNN7EXAMPLE");
    assert!(!findings.is_empty());
    let redacted = &findings[0].1;
    assert!(redacted.ends_with("..."), "should be redacted: {redacted}");
    assert!(redacted.len() < "AKIAIOSFODNN7EXAMPLE".len());
}

// ── YAML Scanning Tests ────────────────────────────────────────────

#[test]
fn test_scan_yaml_with_secrets() {
    // Construct YAML with secrets dynamically to avoid GitHub push protection
    let stripe_key = format!("sk_test_{}", "A".repeat(28));
    let yaml = format!(
        r#"
params:
  db_password: SuperSecretPassword123
  api_key: {stripe_key}
resources:
  nginx:
    pre_apply: "echo AKIAIOSFODNN7EXAMPLE"
"#
    );
    let result = scan_yaml_str(&yaml);
    assert!(!result.clean, "config with secrets should not be clean");
    assert!(
        result.findings.len() >= 2,
        "expected at least 2 findings, got {}: {:?}",
        result.findings.len(),
        result.findings
    );
    assert!(result.scanned_fields > 0);
}

#[test]
fn test_scan_yaml_clean_config() {
    let yaml = r#"
version: "1.0"
name: my-config
machines:
  web:
    hostname: web.example.com
    addr: 10.0.1.1
resources:
  nginx:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
"#;
    let result = scan_yaml_str(yaml);
    assert!(result.clean);
    assert!(result.findings.is_empty());
    assert!(result.scanned_fields > 0);
}

#[test]
fn test_scan_yaml_encrypted_secrets_pass() {
    let yaml = r#"
params:
  db_password: "ENC[age,password: encrypted_value_here]"
  api_key: "ENC[age,key: encrypted_key_here]"
"#;
    let result = scan_yaml_str(yaml);
    assert!(result.clean);
}

#[test]
fn test_scan_yaml_location_tracking() {
    let yaml = r#"
params:
  db_url: "postgres://admin:secret@db:5432/app"
"#;
    let result = scan_yaml_str(yaml);
    assert!(!result.clean);
    let finding = &result.findings[0];
    assert!(
        finding.location.contains("params"),
        "location should contain path: {}",
        finding.location
    );
}

#[test]
fn test_scan_yaml_nested_sequence() {
    let yaml = r#"
hooks:
  - "echo safe command"
  - "sshpass -p mysecret ssh root@host"
"#;
    let result = scan_yaml_str(yaml);
    assert!(!result.clean);
    assert!(result
        .findings
        .iter()
        .any(|f| f.pattern_name == "ssh_password"));
}
