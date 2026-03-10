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

#[test]
fn cis_pack_name() {
    let pack = cis_ubuntu_2204_pack();
    assert_eq!(pack.name, "cis-ubuntu-22.04");
}

#[test]
fn cis_pack_version() {
    let pack = cis_ubuntu_2204_pack();
    assert_eq!(pack.version, "1.0.0");
}

#[test]
fn cis_pack_framework() {
    let pack = cis_ubuntu_2204_pack();
    assert_eq!(pack.framework, "CIS");
}

#[test]
fn cis_pack_has_description() {
    let pack = cis_ubuntu_2204_pack();
    assert!(pack.description.is_some());
    assert!(pack.description.as_ref().unwrap().contains("CIS"));
}

// ============================================================================
// FJ-3206: Rule count and IDs
// ============================================================================

#[test]
fn cis_pack_has_24_rules() {
    let pack = cis_ubuntu_2204_pack();
    assert_eq!(pack.rules.len(), 24, "expected 24 CIS rules");
}

#[test]
fn cis_rule_ids_all_prefixed() {
    let pack = cis_ubuntu_2204_pack();
    for rule in &pack.rules {
        assert!(
            rule.id.starts_with("CIS-"),
            "rule {} should start with CIS-",
            rule.id
        );
    }
}

#[test]
fn cis_rule_ids_unique() {
    let pack = cis_ubuntu_2204_pack();
    let mut ids: Vec<&str> = pack.rules.iter().map(|r| r.id.as_str()).collect();
    let original_len = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), original_len, "duplicate rule IDs found");
}

#[test]
fn cis_all_rules_have_controls() {
    let pack = cis_ubuntu_2204_pack();
    for rule in &pack.rules {
        assert!(
            !rule.controls.is_empty(),
            "rule {} has no controls",
            rule.id
        );
    }
}

#[test]
fn cis_all_rules_have_titles() {
    let pack = cis_ubuntu_2204_pack();
    for rule in &pack.rules {
        assert!(!rule.title.is_empty(), "rule {} has empty title", rule.id);
    }
}

// ============================================================================
// FJ-3206: Severity distribution
// ============================================================================

#[test]
fn cis_severity_errors_count() {
    let pack = cis_ubuntu_2204_pack();
    let (errors, _, _) = severity_summary(&pack);
    assert!(errors >= 12, "expected >=12 error rules, got {errors}");
}

#[test]
fn cis_severity_warnings_count() {
    let pack = cis_ubuntu_2204_pack();
    let (_, warnings, _) = severity_summary(&pack);
    assert!(warnings >= 8, "expected >=8 warning rules, got {warnings}");
}

#[test]
fn cis_severity_info_count() {
    let pack = cis_ubuntu_2204_pack();
    let (_, _, info) = severity_summary(&pack);
    assert!(info >= 1, "expected >=1 info rule, got {info}");
}

#[test]
fn cis_severity_sum_equals_total() {
    let pack = cis_ubuntu_2204_pack();
    let (errors, warnings, info) = severity_summary(&pack);
    assert_eq!(errors + warnings + info, 24);
}

// ============================================================================
// FJ-3206: Check types present
// ============================================================================

#[test]
fn cis_has_assert_rules() {
    let pack = cis_ubuntu_2204_pack();
    let assert_count = pack
        .rules
        .iter()
        .filter(|r| matches!(r.check, ComplianceCheck::Assert { .. }))
        .count();
    assert!(
        assert_count >= 5,
        "expected >=5 Assert rules, got {assert_count}"
    );
}

#[test]
fn cis_has_deny_rules() {
    let pack = cis_ubuntu_2204_pack();
    let deny_count = pack
        .rules
        .iter()
        .filter(|r| matches!(r.check, ComplianceCheck::Deny { .. }))
        .count();
    assert!(deny_count >= 5, "expected >=5 Deny rules, got {deny_count}");
}

#[test]
fn cis_has_require_rules() {
    let pack = cis_ubuntu_2204_pack();
    let require_count = pack
        .rules
        .iter()
        .filter(|r| matches!(r.check, ComplianceCheck::Require { .. }))
        .count();
    assert!(
        require_count >= 5,
        "expected >=5 Require rules, got {require_count}"
    );
}

#[test]
fn cis_has_require_tag_rules() {
    let pack = cis_ubuntu_2204_pack();
    let tag_count = pack
        .rules
        .iter()
        .filter(|r| matches!(r.check, ComplianceCheck::RequireTag { .. }))
        .count();
    assert!(
        tag_count >= 2,
        "expected >=2 RequireTag rules, got {tag_count}"
    );
}

// ============================================================================
// FJ-3206: Section coverage
// ============================================================================

#[test]
fn cis_covers_section_1_filesystem() {
    let pack = cis_ubuntu_2204_pack();
    assert!(pack.rules.iter().any(|r| r.id.starts_with("CIS-1.")));
}

#[test]
fn cis_covers_section_2_services() {
    let pack = cis_ubuntu_2204_pack();
    assert!(pack.rules.iter().any(|r| r.id.starts_with("CIS-2.")));
}

#[test]
fn cis_covers_section_3_network() {
    let pack = cis_ubuntu_2204_pack();
    assert!(pack.rules.iter().any(|r| r.id.starts_with("CIS-3.")));
}

#[test]
fn cis_covers_section_4_access() {
    let pack = cis_ubuntu_2204_pack();
    assert!(pack.rules.iter().any(|r| r.id.starts_with("CIS-4.")));
}

#[test]
fn cis_covers_section_5_auth() {
    let pack = cis_ubuntu_2204_pack();
    assert!(pack.rules.iter().any(|r| r.id.starts_with("CIS-5.")));
}

#[test]
fn cis_covers_section_6_maintenance() {
    let pack = cis_ubuntu_2204_pack();
    assert!(pack.rules.iter().any(|r| r.id.starts_with("CIS-6.")));
}

// ============================================================================
// FJ-3206: STIG cross-mapping
// ============================================================================

#[test]
fn cis_cross_maps_to_stig() {
    let pack = cis_ubuntu_2204_pack();
    let stig_rules: Vec<_> = pack
        .rules
        .iter()
        .filter(|r| r.controls.iter().any(|c| c.starts_with("STIG")))
        .collect();
    assert!(
        !stig_rules.is_empty(),
        "at least one rule should cross-map to STIG"
    );
}

// ============================================================================
// FJ-3206: YAML serialization
// ============================================================================

#[test]
fn cis_yaml_serialization() {
    let yaml = cis_ubuntu_yaml().unwrap();
    assert!(yaml.contains("cis-ubuntu-22.04"));
    assert!(yaml.contains("CIS-1.1.1"));
    assert!(yaml.contains("CIS-6.2.1"));
}

#[test]
fn cis_yaml_roundtrip() {
    let yaml = cis_ubuntu_yaml().unwrap();
    let parsed: forjar::core::compliance_pack::CompliancePack =
        serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(parsed.name, "cis-ubuntu-22.04");
    assert_eq!(parsed.rules.len(), 24);
}

#[test]
fn cis_yaml_contains_all_sections() {
    let yaml = cis_ubuntu_yaml().unwrap();
    for section in ["1.1", "2.1", "3.1", "4.1", "5.1", "6.1"] {
        assert!(
            yaml.contains(&format!("CIS-{section}")),
            "YAML should contain section {section}"
        );
    }
}

// ============================================================================
// FJ-3206: evaluate_pack — passing config
// ============================================================================

#[test]
fn cis_evaluate_passing_file() {
    let pack = cis_ubuntu_2204_pack();
    let mut resources = HashMap::new();
    let mut fields = HashMap::new();
    fields.insert("type".into(), "file".into());
    fields.insert("owner".into(), "root".into());
    fields.insert("group".into(), "root".into());
    fields.insert("mode".into(), "0644".into());
    fields.insert("tags".into(), "system,environment".into());
    resources.insert("config-file".into(), fields);

    let result = evaluate_pack(&pack, &resources);
    assert!(result.passed_count() > 0);
}

// ============================================================================
// FJ-3206: evaluate_pack — world-writable fails CIS-5.3.1
// ============================================================================

#[test]
fn cis_evaluate_world_writable_fails() {
    let pack = cis_ubuntu_2204_pack();
    let mut resources = HashMap::new();
    let mut fields = HashMap::new();
    fields.insert("type".into(), "file".into());
    fields.insert("mode".into(), "777".into());
    resources.insert("bad-file".into(), fields);

    let result = evaluate_pack(&pack, &resources);
    let failed_ids: Vec<_> = result
        .results
        .iter()
        .filter(|r| !r.passed)
        .map(|r| r.rule_id.as_str())
        .collect();
    assert!(
        failed_ids.contains(&"CIS-5.3.1"),
        "CIS-5.3.1 should fail for mode 777, failures: {failed_ids:?}"
    );
}

// ============================================================================
// FJ-3206: evaluate_pack — SSH root login fails CIS-5.2.1
// ============================================================================

#[test]
fn cis_evaluate_ssh_root_login_fails() {
    let pack = cis_ubuntu_2204_pack();
    let mut resources = HashMap::new();
    let mut fields = HashMap::new();
    fields.insert("type".into(), "file".into());
    fields.insert("content".into(), "PermitRootLogin yes".into());
    resources.insert("sshd-config".into(), fields);

    let result = evaluate_pack(&pack, &resources);
    let failed_ids: Vec<_> = result
        .results
        .iter()
        .filter(|r| !r.passed)
        .map(|r| r.rule_id.as_str())
        .collect();
    assert!(
        failed_ids.contains(&"CIS-5.2.1"),
        "CIS-5.2.1 should fail for PermitRootLogin yes"
    );
}

// ============================================================================
// FJ-3206: evaluate_pack — denied package fails
// ============================================================================

#[test]
fn cis_evaluate_xinetd_denied() {
    let pack = cis_ubuntu_2204_pack();
    let mut resources = HashMap::new();
    let mut fields = HashMap::new();
    fields.insert("type".into(), "package".into());
    fields.insert("name".into(), "xinetd".into());
    resources.insert("bad-pkg".into(), fields);

    let result = evaluate_pack(&pack, &resources);
    let failed_ids: Vec<_> = result
        .results
        .iter()
        .filter(|r| !r.passed)
        .map(|r| r.rule_id.as_str())
        .collect();
    assert!(
        failed_ids.contains(&"CIS-2.1.1"),
        "CIS-2.1.1 should fail for xinetd"
    );
}

// ============================================================================
// FJ-3404: parse_plugin_type
// ============================================================================

#[test]
fn parse_plugin_type_valid_name() {
    assert_eq!(parse_plugin_type("plugin:nginx"), Some("nginx"));
}

#[test]
fn parse_plugin_type_with_dash() {
    assert_eq!(parse_plugin_type("plugin:my-custom"), Some("my-custom"));
}

#[test]
fn parse_plugin_type_with_underscore() {
    assert_eq!(parse_plugin_type("plugin:my_plugin"), Some("my_plugin"));
}

#[test]
fn parse_plugin_type_not_plugin() {
    assert_eq!(parse_plugin_type("package"), None);
    assert_eq!(parse_plugin_type("file"), None);
    assert_eq!(parse_plugin_type("service"), None);
}

#[test]
fn parse_plugin_type_bare_plugin() {
    assert_eq!(parse_plugin_type("plugin"), None);
}

#[test]
fn parse_plugin_type_empty_name() {
    assert_eq!(parse_plugin_type("plugin:"), Some(""));
}

// ============================================================================
// FJ-3404: is_plugin_type
// ============================================================================

#[test]
fn is_plugin_type_true() {
    assert!(is_plugin_type("plugin:foo"));
    assert!(is_plugin_type("plugin:bar-baz"));
}

#[test]
fn is_plugin_type_false() {
    assert!(!is_plugin_type("package"));
    assert!(!is_plugin_type("file"));
    assert!(!is_plugin_type("plugin"));
    assert!(!is_plugin_type(""));
}

// ============================================================================
// FJ-3404: available_plugin_types — empty dir
// ============================================================================

#[test]
fn available_plugins_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let types = available_plugin_types(dir.path());
    assert!(types.is_empty());
}

// ============================================================================
// FJ-3404: dispatch_check — missing plugin
// ============================================================================

#[test]
fn dispatch_check_missing() {
    let dir = tempfile::tempdir().unwrap();
    let config = serde_json::json!({"key": "value"});
    let result = dispatch_check(dir.path(), "nonexistent", &config);
    assert!(!result.success);
    assert_eq!(result.operation, "check");
    assert_eq!(result.plugin_name, "nonexistent");
}

// ============================================================================
// FJ-3404: dispatch_apply — missing plugin
// ============================================================================

#[test]
fn dispatch_apply_missing() {
    let dir = tempfile::tempdir().unwrap();
    let config = serde_json::json!({});
    let result = dispatch_apply(dir.path(), "nonexistent", &config);
    assert!(!result.success);
    assert_eq!(result.operation, "apply");
}

// ============================================================================
// FJ-3404: dispatch_destroy — missing plugin
// ============================================================================

#[test]
fn dispatch_destroy_missing() {
    let dir = tempfile::tempdir().unwrap();
    let config = serde_json::json!({});
    let result = dispatch_destroy(dir.path(), "nonexistent", &config);
    assert!(!result.success);
    assert_eq!(result.operation, "destroy");
}

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
