//! Extended coverage for secret_scan.rs — scan_yaml_value branches,
//! scan_yaml_str error path, redact edge cases, DSA key pattern.

use super::secret_scan::*;

// ── scan_yaml_value: non-string/non-map/non-seq YAML values ─────

#[test]
fn scan_yaml_value_null() {
    let value = serde_yaml_ng::Value::Null;
    let mut findings = Vec::new();
    let mut scanned = 0;
    scan_yaml_value(&value, "root", &mut findings, &mut scanned);
    assert!(findings.is_empty());
    assert_eq!(scanned, 0);
}

#[test]
fn scan_yaml_value_bool() {
    let value = serde_yaml_ng::Value::Bool(true);
    let mut findings = Vec::new();
    let mut scanned = 0;
    scan_yaml_value(&value, "root", &mut findings, &mut scanned);
    assert!(findings.is_empty());
    assert_eq!(scanned, 0);
}

#[test]
fn scan_yaml_value_number() {
    let value = serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(42));
    let mut findings = Vec::new();
    let mut scanned = 0;
    scan_yaml_value(&value, "root", &mut findings, &mut scanned);
    assert!(findings.is_empty());
    assert_eq!(scanned, 0);
}

// ── scan_yaml_value: mapping with non-string key ────────────────

#[test]
fn scan_yaml_value_mapping_non_string_key() {
    let mut map = serde_yaml_ng::Mapping::new();
    map.insert(
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(1)),
        serde_yaml_ng::Value::String("safe value".to_string()),
    );
    let value = serde_yaml_ng::Value::Mapping(map);
    let mut findings = Vec::new();
    let mut scanned = 0;
    scan_yaml_value(&value, "", &mut findings, &mut scanned);
    assert_eq!(scanned, 1);
    assert!(findings.is_empty());
}

// ── scan_yaml_value: nested mapping with root path ──────────────

#[test]
fn scan_yaml_value_nested_path_construction() {
    let mut inner = serde_yaml_ng::Mapping::new();
    inner.insert(
        serde_yaml_ng::Value::String("key".to_string()),
        serde_yaml_ng::Value::String("AKIAIOSFODNN7EXAMPLE".to_string()),
    );
    let mut outer = serde_yaml_ng::Mapping::new();
    outer.insert(
        serde_yaml_ng::Value::String("params".to_string()),
        serde_yaml_ng::Value::Mapping(inner),
    );
    let value = serde_yaml_ng::Value::Mapping(outer);
    let mut findings = Vec::new();
    let mut scanned = 0;
    scan_yaml_value(&value, "", &mut findings, &mut scanned);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].location, "params.key");
}

// ── scan_yaml_value: sequence with index tracking ───────────────

#[test]
fn scan_yaml_value_sequence_indices() {
    let seq = serde_yaml_ng::Value::Sequence(vec![
        serde_yaml_ng::Value::String("safe".to_string()),
        serde_yaml_ng::Value::String("-----BEGIN RSA PRIVATE KEY-----".to_string()),
    ]);
    let mut findings = Vec::new();
    let mut scanned = 0;
    scan_yaml_value(&seq, "hooks", &mut findings, &mut scanned);
    assert_eq!(scanned, 2);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].location, "hooks[1]");
}

// ── scan_yaml_str: invalid YAML falls back to text scan ─────────

#[test]
fn scan_yaml_str_invalid_yaml_with_secret() {
    let bad_yaml = "key: [broken\nAKIAIOSFODNN7EXAMPLE";
    let result = scan_yaml_str(bad_yaml);
    assert!(!result.clean);
    assert_eq!(result.scanned_fields, 1);
    assert!(result
        .findings
        .iter()
        .any(|f| f.pattern_name == "aws_access_key"));
    assert_eq!(result.findings[0].location, "raw_text");
}

#[test]
fn scan_yaml_str_invalid_yaml_no_secret() {
    let bad_yaml = "key: [broken yaml here";
    let result = scan_yaml_str(bad_yaml);
    assert!(result.clean);
    assert_eq!(result.scanned_fields, 1);
}

// ── scan_text: DSA key pattern ──────────────────────────────────

#[test]
fn scan_text_dsa_private_key() {
    let findings = scan_text("-----BEGIN DSA PRIVATE KEY-----");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].0, "private_key_pem");
}

// ── scan_text: multiple patterns in same text ───────────────────

#[test]
fn scan_text_multiple_secrets() {
    let text = "AKIAIOSFODNN7EXAMPLE password: SuperSecretPassword123";
    let findings = scan_text(text);
    assert!(findings.len() >= 2);
    let names: Vec<&str> = findings.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"aws_access_key"));
    assert!(names.contains(&"generic_secret"));
}

// ── redact: short string (<=8 chars) ────────────────────────────

#[test]
fn redact_short_string() {
    let findings = scan_text("-----BEGIN RSA PRIVATE KEY-----");
    // The matched text is ≥8 chars so will be truncated
    assert!(!findings.is_empty());
    assert!(findings[0].1.ends_with("..."));
}

// ── is_encrypted: edge cases ────────────────────────────────────

#[test]
fn is_encrypted_partial_match() {
    assert!(!is_encrypted("ENC[aes,data]"));
}

#[test]
fn is_encrypted_empty() {
    assert!(!is_encrypted(""));
}

// ── scan_yaml_str: clean nested structures ──────────────────────

#[test]
fn scan_yaml_clean_deeply_nested() {
    let yaml = r#"
level1:
  level2:
    level3:
      - safe: value
      - also: clean
    array: [one, two, three]
"#;
    let result = scan_yaml_str(yaml);
    assert!(result.clean);
    assert!(result.scanned_fields >= 5);
}

// ── scan_text: base64 private key pattern ───────────────────────

#[test]
fn scan_text_base64_private_key() {
    let b64 = "A".repeat(44);
    let text = format!("private.key = {b64}");
    let findings = scan_text(&text);
    assert!(
        findings.iter().any(|(n, _)| n == "base64_private"),
        "expected base64_private pattern, got {findings:?}"
    );
}

// ── ScanResult struct fields ────────────────────────────────────

#[test]
fn scan_result_clean_flag() {
    let result = scan_yaml_str("key: value\n");
    assert!(result.clean);
    assert!(result.findings.is_empty());
    assert_eq!(result.scanned_fields, 1);
}

// ── SecretFinding equality ──────────────────────────────────────

#[test]
fn secret_finding_equality() {
    let a = SecretFinding {
        pattern_name: "test".to_string(),
        matched_text: "abc...".to_string(),
        location: "params.key".to_string(),
    };
    let b = a.clone();
    assert_eq!(a, b);
}
