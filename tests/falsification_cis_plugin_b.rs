//! FJ-3206/3404: CIS Ubuntu pack and plugin dispatch falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-3206: CIS Ubuntu 22.04 LTS compliance pack
//!   - Pack metadata (name, version, framework, description)
//!   - 24 rules with unique CIS-prefixed IDs
//!   - Severity distribution (error >= 12, warning >= 8, info >= 1)
//!   - YAML serialization roundtrip
//!   - Pack evaluation against passing/failing configs
//!   - Cross-mapping to STIG controls
//! - FJ-3404: Plugin type dispatch
//!   - parse_plugin_type for valid/invalid types
//!   - is_plugin_type predicate
//!   - available_plugin_types in empty directory
//!   - dispatch_check/apply/destroy for missing and real plugins
//!   - resolve_plugin with BLAKE3 verification
//!
//! Usage: cargo test --test falsification_cis_plugin
#![allow(dead_code)]

use forjar::core::cis_ubuntu_pack::{cis_ubuntu_2204_pack, cis_ubuntu_yaml, severity_summary};
use forjar::core::compliance_pack::{evaluate_pack, ComplianceCheck};
use forjar::core::plugin_dispatch::{
    available_plugin_types, dispatch_apply, dispatch_check, dispatch_destroy, is_plugin_type,
    parse_plugin_type, resolve_plugin,
};
use std::collections::HashMap;

// ============================================================================
// FJ-3206: Pack metadata
// ============================================================================
// FJ-3404: dispatch with real plugin (BLAKE3 verified)
// ============================================================================

fn create_test_plugin(dir: &std::path::Path, name: &str) {
    let plugin_dir = dir.join(name);
    std::fs::create_dir_all(&plugin_dir).unwrap();

    let wasm_bytes = b"fake wasm module content for testing";
    let hash = blake3::hash(wasm_bytes).to_hex().to_string();
    std::fs::write(plugin_dir.join("plugin.wasm"), wasm_bytes).unwrap();
    std::fs::write(
        plugin_dir.join("plugin.yaml"),
        format!(
            "name: {name}\nversion: \"0.1.0\"\nabi_version: 1\nwasm: plugin.wasm\nblake3: {hash}\npermissions:\n  fs: {{}}\n  net: {{}}\n  env: {{}}\n  exec: {{}}\n"
        ),
    )
    .unwrap();
}

#[test]
fn dispatch_check_real_plugin() {
    let dir = tempfile::tempdir().unwrap();
    create_test_plugin(dir.path(), "test-plugin");

    let config = serde_json::json!({"setting": true});
    let result = dispatch_check(dir.path(), "test-plugin", &config);
    assert!(result.success, "dispatch failed: {}", result.message);
    assert_eq!(result.operation, "check");
}

#[test]
fn dispatch_apply_real_plugin() {
    let dir = tempfile::tempdir().unwrap();
    create_test_plugin(dir.path(), "test-apply");

    let config = serde_json::json!({});
    let result = dispatch_apply(dir.path(), "test-apply", &config);
    assert!(result.success, "dispatch failed: {}", result.message);
    assert_eq!(result.operation, "apply");
    // Message format varies: with runtime includes version, without includes "stub"
    assert!(!result.message.is_empty());
}

#[test]
fn dispatch_destroy_real_plugin() {
    let dir = tempfile::tempdir().unwrap();
    create_test_plugin(dir.path(), "test-destroy");

    let config = serde_json::json!({});
    let result = dispatch_destroy(dir.path(), "test-destroy", &config);
    assert!(result.success, "dispatch failed: {}", result.message);
    assert_eq!(result.operation, "destroy");
}

// ============================================================================
// FJ-3404: resolve_plugin — verified
// ============================================================================

#[test]
fn resolve_plugin_verified() {
    let dir = tempfile::tempdir().unwrap();
    create_test_plugin(dir.path(), "verified-plugin");

    let resolved = resolve_plugin(dir.path(), "verified-plugin");
    assert!(resolved.is_ok(), "resolve failed: {:?}", resolved.err());
}

#[test]
fn resolve_plugin_missing() {
    let dir = tempfile::tempdir().unwrap();
    let resolved = resolve_plugin(dir.path(), "no-such-plugin");
    assert!(resolved.is_err());
}

// ============================================================================
// FJ-3404: dispatch result fields
// ============================================================================

#[test]
fn dispatch_result_error_status_on_failure() {
    let dir = tempfile::tempdir().unwrap();
    let config = serde_json::json!({});
    let result = dispatch_check(dir.path(), "missing", &config);
    assert_eq!(result.status, forjar::core::types::PluginStatus::Error);
}

#[test]
fn dispatch_result_converged_on_success() {
    let dir = tempfile::tempdir().unwrap();
    create_test_plugin(dir.path(), "ok-plugin");

    let config = serde_json::json!({});
    let result = dispatch_check(dir.path(), "ok-plugin", &config);
    assert_eq!(result.status, forjar::core::types::PluginStatus::Converged);
}
