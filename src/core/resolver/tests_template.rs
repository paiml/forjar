//! Template tests.

#![allow(unused_imports)]
use super::template::{resolve_secret, resolve_template};
use super::*;
use std::collections::HashMap;

#[test]
fn test_fj003_resolve_params() {
    let mut params = HashMap::new();
    params.insert(
        "name".to_string(),
        serde_yaml_ng::Value::String("world".to_string()),
    );
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("hello {{params.name}}", &params, &machines).unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn test_fj003_resolve_machine_addr() {
    let params = HashMap::new();
    let mut machines = indexmap::IndexMap::new();
    machines.insert(
        "lambda".to_string(),
        Machine {
            hostname: "lambda-box".to_string(),
            addr: "192.168.1.1".to_string(),
            user: "noah".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        },
    );
    let result = resolve_template("ssh {{machine.lambda.addr}}", &params, &machines).unwrap();
    assert_eq!(result, "ssh 192.168.1.1");
}

#[test]
fn test_fj003_resolve_unknown_param() {
    let params = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("{{params.missing}}", &params, &machines);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown param"));
}

#[test]
fn test_fj062_resolve_secret_missing() {
    // Use a unique key that definitely won't exist in CI/local env
    let params = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("{{secrets.zzz-test-nonexistent-9999}}", &params, &machines);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("FORJAR_SECRET_ZZZ_TEST_NONEXISTENT_9999"));
    assert!(err.contains("not found"));
}

#[test]
fn test_fj062_secret_key_normalization() {
    // Verify the env var name construction (hyphens → underscores, uppercase)
    let result = resolve_secret("db-password");
    // We can't set env vars safely, but we can verify the error message
    // contains the correctly-normalized key
    let err = result.unwrap_err();
    assert!(err.contains("FORJAR_SECRET_DB_PASSWORD"));
}

#[test]
fn test_fj062_secret_from_env_via_subprocess() {
    // Run a child process with the env var set to verify resolution works
    let exe = std::env::current_exe().unwrap();
    let output = std::process::Command::new(exe)
        .env("FORJAR_SECRET_TEST_KEY", "secret_value")
        .arg("--test-threads=1")
        .arg("--exact")
        .arg("core::resolver::tests::test_fj062_secret_inner")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "subprocess failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_fj062_secret_inner() {
    // This test is run via subprocess with FORJAR_SECRET_TEST_KEY set
    if std::env::var("FORJAR_SECRET_TEST_KEY").is_err() {
        return; // Skip when not run via subprocess
    }
    let params = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("val={{secrets.test-key}}", &params, &machines).unwrap();
    assert_eq!(result, "val=secret_value");
}

#[test]
fn test_fj003_resolve_multiple() {
    let mut params = HashMap::new();
    params.insert(
        "a".to_string(),
        serde_yaml_ng::Value::String("X".to_string()),
    );
    params.insert(
        "b".to_string(),
        serde_yaml_ng::Value::String("Y".to_string()),
    );
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("{{params.a}}-{{params.b}}", &params, &machines).unwrap();
    assert_eq!(result, "X-Y");
}

#[test]
fn test_fj003_resolve_unknown_template_var() {
    let params = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let err = resolve_template("{{bogus.var}}", &params, &machines);
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("unknown template variable"));
}

#[test]
fn test_fj003_unclosed_template() {
    let params = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let err = resolve_template("hello {{params.name", &params, &machines);
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("unclosed template"));
}

#[test]
fn test_fj003_no_template_passthrough() {
    let params = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("plain string no templates", &params, &machines).unwrap();
    assert_eq!(result, "plain string no templates");
}

#[test]
fn test_fj003_empty_string_passthrough() {
    let params = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("", &params, &machines).unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_fj003_mixed_template_types() {
    // Mix params and machine refs in one string
    let mut params = HashMap::new();
    params.insert(
        "port".to_string(),
        serde_yaml_ng::Value::String("8080".to_string()),
    );
    let mut machines = indexmap::IndexMap::new();
    machines.insert(
        "web".to_string(),
        Machine {
            hostname: "web-01".to_string(),
            addr: "10.0.0.1".to_string(),
            user: "deploy".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        },
    );
    let result = resolve_template(
        "http://{{machine.web.addr}}:{{params.port}}",
        &params,
        &machines,
    )
    .unwrap();
    assert_eq!(result, "http://10.0.0.1:8080");
}

#[test]
fn test_fj003_unknown_machine_name() {
    let params = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let err = resolve_template("{{machine.ghost.addr}}", &params, &machines);
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("unknown machine"));
}

#[test]
fn test_fj003_numeric_param_value() {
    let mut params = HashMap::new();
    params.insert(
        "count".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(42)),
    );
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("total={{params.count}}", &params, &machines).unwrap();
    assert_eq!(result, "total=42");
}

#[test]
fn test_fj003_boolean_param_value() {
    let mut params = HashMap::new();
    params.insert("flag".to_string(), serde_yaml_ng::Value::Bool(true));
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("debug={{params.flag}}", &params, &machines).unwrap();
    assert_eq!(result, "debug=true");
}

#[test]
fn test_fj003_template_with_whitespace() {
    // Templates with spaces around the key should still resolve
    let mut params = HashMap::new();
    params.insert(
        "name".to_string(),
        serde_yaml_ng::Value::String("val".to_string()),
    );
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("{{ params.name }}", &params, &machines).unwrap();
    assert_eq!(result, "val");
}

#[test]
fn test_fj003_consecutive_templates() {
    let mut params = HashMap::new();
    params.insert(
        "a".to_string(),
        serde_yaml_ng::Value::String("X".to_string()),
    );
    params.insert(
        "b".to_string(),
        serde_yaml_ng::Value::String("Y".to_string()),
    );
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("{{params.a}}{{params.b}}", &params, &machines).unwrap();
    assert_eq!(result, "XY");
}

#[test]
fn test_fj131_resolve_machine_hostname_field() {
    let params = HashMap::new();
    let mut machines = indexmap::IndexMap::new();
    machines.insert(
        "db".to_string(),
        Machine {
            hostname: "db-primary".to_string(),
            addr: "10.0.0.5".to_string(),
            user: "postgres".to_string(),
            arch: "aarch64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        },
    );
    let result = resolve_template("host={{machine.db.hostname}}", &params, &machines).unwrap();
    assert_eq!(result, "host=db-primary");
}

#[test]
fn test_fj131_resolve_machine_user_field() {
    let params = HashMap::new();
    let mut machines = indexmap::IndexMap::new();
    machines.insert(
        "db".to_string(),
        Machine {
            hostname: "db".to_string(),
            addr: "10.0.0.5".to_string(),
            user: "postgres".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        },
    );
    let result = resolve_template("user={{machine.db.user}}", &params, &machines).unwrap();
    assert_eq!(result, "user=postgres");
}

#[test]
fn test_fj131_resolve_machine_arch_field() {
    let params = HashMap::new();
    let mut machines = indexmap::IndexMap::new();
    machines.insert(
        "arm".to_string(),
        Machine {
            hostname: "arm".to_string(),
            addr: "10.0.0.6".to_string(),
            user: "root".to_string(),
            arch: "aarch64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        },
    );
    let result = resolve_template("arch={{machine.arm.arch}}", &params, &machines).unwrap();
    assert_eq!(result, "arch=aarch64");
}

#[test]
fn test_fj131_resolve_machine_invalid_field() {
    let params = HashMap::new();
    let mut machines = indexmap::IndexMap::new();
    machines.insert(
        "m".to_string(),
        Machine {
            hostname: "m".to_string(),
            addr: "1.1.1.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        },
    );
    let result = resolve_template("{{machine.m.cost}}", &params, &machines);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown machine field"));
}

#[test]
fn test_fj131_resolve_machine_ref_too_few_parts() {
    let params = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("{{machine.only}}", &params, &machines);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid machine ref"));
}

#[test]
fn test_fj131_resolve_unknown_template_type() {
    let params = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("{{foobar.baz}}", &params, &machines);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown template variable"));
}

#[test]
fn test_fj132_resolve_secret_missing() {
    // Use a key that won't exist in any CI/local env
    let result = resolve_secret("zzz-nonexistent-key-12345");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("FORJAR_SECRET_ZZZ_NONEXISTENT_KEY_12345"));
    assert!(err.contains("not found"));
}

#[test]
fn test_fj132_resolve_secret_env_key_format() {
    // Verify the env key derivation: hyphens → underscores, uppercase
    let result = resolve_secret("my-db-pass");
    // Will fail because env var doesn't exist, but error message shows the derived key
    let err = result.unwrap_err();
    assert!(
        err.contains("FORJAR_SECRET_MY_DB_PASS"),
        "should derive FORJAR_SECRET_MY_DB_PASS from 'my-db-pass'"
    );
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
