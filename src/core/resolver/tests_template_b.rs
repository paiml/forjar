//! Template tests — secrets resolution and edge cases.

#![allow(unused_imports)]
use super::template::{resolve_secret, resolve_template};
use super::*;
use std::collections::HashMap;

#[test]
fn test_fj132_resolve_secret_missing() {
    // Use a key that won't exist in any CI/local env
    let result = resolve_secret("zzz-nonexistent-key-12345", &SecretsConfig::default());
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("FORJAR_SECRET_ZZZ_NONEXISTENT_KEY_12345"));
    assert!(err.contains("not found"));
}

#[test]
fn test_fj132_resolve_secret_env_key_format() {
    // Verify the env key derivation: hyphens → underscores, uppercase
    let result = resolve_secret("my-db-pass", &SecretsConfig::default());
    // Will fail because env var doesn't exist, but error message shows the derived key
    let err = result.unwrap_err();
    assert!(
        err.contains("FORJAR_SECRET_MY_DB_PASS"),
        "should derive FORJAR_SECRET_MY_DB_PASS from 'my-db-pass'"
    );
}

#[test]
fn test_resolve_secret_file_provider() {
    let dir = tempfile::tempdir().unwrap();
    let secret_path = dir.path().join("db-pass");
    std::fs::write(&secret_path, "s3cret_val\n").unwrap();
    let cfg = SecretsConfig {
        provider: Some("file".into()),
        path: Some(dir.path().to_string_lossy().to_string()),
    };
    let result = resolve_secret("db-pass", &cfg).unwrap();
    assert_eq!(result, "s3cret_val");
}

#[test]
fn test_resolve_secret_file_provider_missing() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = SecretsConfig {
        provider: Some("file".into()),
        path: Some(dir.path().to_string_lossy().to_string()),
    };
    let result = resolve_secret("nonexistent", &cfg);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_fj132_unclosed_template() {
    let params = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("hello {{params.name", &params, &machines);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unclosed template"));
}

#[test]
fn test_fj132_resolve_template_secret_missing_error() {
    let params = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("token={{secrets.zzz-missing-99}}", &params, &machines);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("FORJAR_SECRET_ZZZ_MISSING_99"));
}

#[test]
fn test_resolve_template_nested_braces() {
    let mut params = HashMap::new();
    params.insert(
        "x".to_string(),
        serde_yaml_ng::Value::String("value".to_string()),
    );
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("{{params.x}}_suffix", &params, &machines).unwrap();
    assert_eq!(result, "value_suffix");
}

#[test]
fn test_resolve_template_multiple() {
    let mut params = HashMap::new();
    params.insert(
        "a".to_string(),
        serde_yaml_ng::Value::String("hello".to_string()),
    );
    params.insert(
        "b".to_string(),
        serde_yaml_ng::Value::String("world".to_string()),
    );
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("{{params.a}}-{{params.b}}", &params, &machines).unwrap();
    assert_eq!(result, "hello-world");
}

// ── FJ-2300: Secret provider tests ──

#[test]
fn test_secret_file_provider() {
    let dir = tempfile::tempdir().unwrap();
    let secret_file = dir.path().join("db_password");
    std::fs::write(&secret_file, "s3cret\n").unwrap();

    let result = super::template::resolve_secret_with_provider(
        "db_password",
        Some("file"),
        Some(dir.path().to_str().unwrap()),
    );
    assert_eq!(result.unwrap(), "s3cret"); // trims trailing newline
}

#[test]
fn test_secret_file_provider_missing() {
    let result = super::template::resolve_secret_with_provider(
        "nonexistent",
        Some("file"),
        Some("/tmp/forjar-test-no-such-dir"),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_secret_env_provider_explicit() {
    std::env::set_var("FORJAR_SECRET_TEST_KEY_2300", "env-secret");
    let result = super::template::resolve_secret_with_provider("test_key_2300", Some("env"), None);
    assert_eq!(result.unwrap(), "env-secret");
    std::env::remove_var("FORJAR_SECRET_TEST_KEY_2300");
}

#[test]
fn test_redact_secrets() {
    let text = "password is s3cret and token is abc123";
    let secrets = vec!["s3cret".to_string(), "abc123".to_string()];
    let redacted = super::template::redact_secrets(text, &secrets);
    assert_eq!(redacted, "password is *** and token is ***");
}

#[test]
fn test_redact_secrets_empty() {
    let text = "no secrets here";
    let redacted = super::template::redact_secrets(text, &[]);
    assert_eq!(redacted, "no secrets here");
}

#[test]
fn test_redact_secrets_empty_value() {
    let text = "keep me";
    let secrets = vec!["".to_string()];
    let redacted = super::template::redact_secrets(text, &secrets);
    assert_eq!(redacted, "keep me"); // empty secrets are skipped
}
