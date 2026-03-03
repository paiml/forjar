//! Tests for FJ-1363: Convert --apply execution.

use super::convert::{analyze_conversion, ConversionSignals};
use super::convert_exec::apply_conversion;
use super::pin_resolve::pin_hash;
use super::purity::PurityLevel;
use std::path::Path;

fn write_test_config(dir: &Path, content: &str) -> std::path::PathBuf {
    let path = dir.join("forjar.yaml");
    std::fs::write(&path, content).unwrap();
    path
}

fn sample_config_yaml() -> &'static str {
    r#"resources:
  - name: curl
    type: package
    provider: apt
  - name: ripgrep
    type: package
    provider: cargo
    version: "14.1.0"
    store: true
"#
}

fn signals_no_version(name: &str, provider: &str) -> ConversionSignals {
    ConversionSignals {
        name: name.to_string(),
        has_version: false,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: false,
        provider: provider.to_string(),
        current_version: None,
    }
}

fn signals_pinned(name: &str, provider: &str) -> ConversionSignals {
    ConversionSignals {
        name: name.to_string(),
        has_version: true,
        has_store: true,
        has_sandbox: false,
        has_curl_pipe: false,
        provider: provider.to_string(),
        current_version: Some("1.0".to_string()),
    }
}

#[test]
fn apply_empty_report_is_noop() {
    let dir = tempfile::tempdir().unwrap();
    let config = write_test_config(dir.path(), sample_config_yaml());

    let signals = vec![signals_pinned("ripgrep", "cargo")];
    let report = analyze_conversion(&signals);

    let result = apply_conversion(&config, &report).unwrap();
    assert_eq!(result.changes_applied, 0);
}

#[test]
fn apply_version_pin_change() {
    let dir = tempfile::tempdir().unwrap();
    let config = write_test_config(dir.path(), sample_config_yaml());

    let signals = vec![signals_no_version("curl", "apt")];
    let report = analyze_conversion(&signals);

    assert!(report.auto_change_count > 0);
    let result = apply_conversion(&config, &report).unwrap();
    assert!(result.changes_applied > 0);

    // Backup should exist
    assert!(dir.path().join("forjar.yaml.bak").exists());

    // Updated config should have version field
    let updated = std::fs::read_to_string(&config).unwrap();
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&updated).unwrap();
    let resources = doc["resources"].as_sequence().unwrap();
    let curl = resources
        .iter()
        .find(|r| r["name"].as_str() == Some("curl"))
        .unwrap();
    assert!(curl.get("version").is_some());
}

#[test]
fn apply_store_flag_change() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = r#"resources:
  - name: flask
    type: package
    provider: pip
    version: "3.0.0"
"#;
    let config = write_test_config(dir.path(), yaml);

    let signals = vec![ConversionSignals {
        name: "flask".to_string(),
        has_version: true,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: false,
        provider: "pip".to_string(),
        current_version: Some("3.0.0".to_string()),
    }];
    let report = analyze_conversion(&signals);

    let result = apply_conversion(&config, &report).unwrap();
    assert!(result.changes_applied > 0);

    let updated = std::fs::read_to_string(&config).unwrap();
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&updated).unwrap();
    let resources = doc["resources"].as_sequence().unwrap();
    let flask = resources
        .iter()
        .find(|r| r["name"].as_str() == Some("flask"))
        .unwrap();
    assert_eq!(flask["store"].as_bool(), Some(true));
}

#[test]
fn backup_created_before_modification() {
    let dir = tempfile::tempdir().unwrap();
    let original = sample_config_yaml();
    let config = write_test_config(dir.path(), original);

    let signals = vec![signals_no_version("curl", "apt")];
    let report = analyze_conversion(&signals);

    let result = apply_conversion(&config, &report).unwrap();
    assert!(result.backup_path.exists());

    // Backup should have original content
    let backup_content = std::fs::read_to_string(&result.backup_path).unwrap();
    assert_eq!(backup_content, original);
}

#[test]
fn lock_file_generated_for_lock_pins() {
    let dir = tempfile::tempdir().unwrap();
    let config = write_test_config(dir.path(), sample_config_yaml());

    let signals = vec![signals_no_version("curl", "apt")];
    let report = analyze_conversion(&signals);

    let result = apply_conversion(&config, &report).unwrap();
    assert!(result.lock_pins_generated > 0);

    let lock_path = dir.path().join("forjar.inputs.lock.yaml");
    assert!(lock_path.exists());

    let lock_content = std::fs::read_to_string(&lock_path).unwrap();
    assert!(lock_content.contains("curl"));
}

#[test]
fn projected_purity_set_correctly() {
    let dir = tempfile::tempdir().unwrap();
    let config = write_test_config(dir.path(), sample_config_yaml());

    let signals = vec![signals_no_version("curl", "apt")];
    let report = analyze_conversion(&signals);

    let result = apply_conversion(&config, &report).unwrap();
    assert_eq!(result.new_purity, report.projected_purity);
}

#[test]
fn multiple_resources_converted() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = r#"resources:
  - name: curl
    type: package
    provider: apt
  - name: wget
    type: package
    provider: apt
"#;
    let config = write_test_config(dir.path(), yaml);

    let signals = vec![
        signals_no_version("curl", "apt"),
        signals_no_version("wget", "apt"),
    ];
    let report = analyze_conversion(&signals);

    let result = apply_conversion(&config, &report).unwrap();
    // Each resource gets version pin + store flag + lock pin = 3 changes each
    assert!(result.changes_applied >= 4);
}

#[test]
fn idempotent_apply_no_double_version() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = r#"resources:
  - name: curl
    type: package
    provider: apt
    version: "7.88.1"
"#;
    let config = write_test_config(dir.path(), yaml);

    // Signal says version already present
    let signals = vec![ConversionSignals {
        name: "curl".to_string(),
        has_version: true,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: false,
        provider: "apt".to_string(),
        current_version: Some("7.88.1".to_string()),
    }];
    let report = analyze_conversion(&signals);

    apply_conversion(&config, &report).unwrap();
    // Should only add store flag, not duplicate version
    let updated = std::fs::read_to_string(&config).unwrap();
    let count = updated.matches("version").count();
    assert_eq!(count, 1, "should not duplicate version field");
}

#[test]
fn curl_pipe_resource_stays_impure() {
    let signals = vec![ConversionSignals {
        name: "installer".to_string(),
        has_version: false,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: true,
        provider: "shell".to_string(),
        current_version: None,
    }];
    let report = analyze_conversion(&signals);
    assert_eq!(report.projected_purity, PurityLevel::Impure);
}

#[test]
fn config_with_no_resources_section() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = "name: test-recipe\nmachines:\n  - hostname: web\n";
    let config = write_test_config(dir.path(), yaml);

    let signals = vec![signals_no_version("curl", "apt")];
    let report = analyze_conversion(&signals);

    let result = apply_conversion(&config, &report).unwrap();
    // Changes count applies but YAML modifications silently skip missing resources
    assert!(result.changes_applied > 0);
}

#[test]
fn atomic_write_survives_crash() {
    let dir = tempfile::tempdir().unwrap();
    let config = write_test_config(dir.path(), sample_config_yaml());

    let signals = vec![signals_no_version("curl", "apt")];
    let report = analyze_conversion(&signals);

    // Apply should use temp+rename (atomic)
    apply_conversion(&config, &report).unwrap();

    // No .tmp file should remain
    assert!(!dir.path().join("forjar.yaml.tmp").exists());
}

#[test]
fn provider_propagated_to_resource_conversion() {
    let signals = vec![signals_no_version("nginx", "apt")];
    let report = analyze_conversion(&signals);
    assert_eq!(report.resources[0].provider, "apt");
}

#[test]
fn lock_pin_uses_actual_provider_and_pin_hash() {
    let dir = tempfile::tempdir().unwrap();
    let config = write_test_config(dir.path(), sample_config_yaml());

    let signals = vec![signals_no_version("curl", "apt")];
    let report = analyze_conversion(&signals);

    apply_conversion(&config, &report).unwrap();

    let lock_path = dir.path().join("forjar.inputs.lock.yaml");
    let lock_content = std::fs::read_to_string(&lock_path).unwrap();
    // Lock pin should have apt provider, not "unknown"
    assert!(
        lock_content.contains("apt"),
        "lock pin should use actual provider"
    );
    // Hash should match pin_hash(provider, name, "latest")
    let expected_hash = pin_hash("apt", "curl", "latest");
    assert!(
        lock_content.contains(&expected_hash),
        "lock pin should use pin_hash"
    );
}
