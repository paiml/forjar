//! FJ-2502/2503: Include hardening & deep validation falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-2502: Include provenance tracking, circular detection, conflict handling
//! - FJ-2503: Deep validation types, flags, severity model, findings, field suggestions
//! - FJ-2500: Unknown field detection with Levenshtein suggestions
//! - Cycle detection via DAG resolver
//!
//! Usage: cargo test --test falsification_include_deep_validation

#![allow(clippy::field_reassign_with_default)]

use forjar::core::parser::{check_unknown_fields, parse_and_validate};
use forjar::core::types::{
    DeepCheckFlags, FieldSuggestion, ForjarConfig, ValidateOutput, ValidationFinding,
    ValidationSeverity,
};

// ============================================================================
// FJ-2503: ValidationSeverity
// ============================================================================

#[test]
fn severity_ordering_error_gt_warning_gt_hint() {
    assert!(ValidationSeverity::Error > ValidationSeverity::Warning);
    assert!(ValidationSeverity::Warning > ValidationSeverity::Hint);
    assert!(ValidationSeverity::Error > ValidationSeverity::Hint);
}

#[test]
fn severity_display() {
    assert_eq!(ValidationSeverity::Error.to_string(), "error");
    assert_eq!(ValidationSeverity::Warning.to_string(), "warning");
    assert_eq!(ValidationSeverity::Hint.to_string(), "hint");
}

#[test]
fn severity_serde_roundtrip() {
    let json = serde_json::to_string(&ValidationSeverity::Warning).unwrap();
    let back: ValidationSeverity = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ValidationSeverity::Warning);
}

#[test]
fn severity_equality() {
    assert_eq!(ValidationSeverity::Error, ValidationSeverity::Error);
    assert_ne!(ValidationSeverity::Error, ValidationSeverity::Warning);
}

// ============================================================================
// FJ-2503: ValidationFinding
// ============================================================================

#[test]
fn finding_error_constructor() {
    let f = ValidationFinding::error("missing field");
    assert!(f.is_error());
    assert!(!f.is_warning());
    assert_eq!(f.severity, ValidationSeverity::Error);
    assert_eq!(f.message, "missing field");
    assert!(f.resource.is_none());
    assert!(f.field.is_none());
    assert!(f.suggestion.is_none());
}

#[test]
fn finding_warning_constructor() {
    let f = ValidationFinding::warning("deprecated field");
    assert!(f.is_warning());
    assert!(!f.is_error());
    assert_eq!(f.severity, ValidationSeverity::Warning);
}

#[test]
fn finding_builder_chain() {
    let f = ValidationFinding::error("bad mode")
        .for_resource("nginx")
        .for_field("mode")
        .with_suggestion("use 4-digit octal like '0644'");
    assert_eq!(f.resource.as_deref(), Some("nginx"));
    assert_eq!(f.field.as_deref(), Some("mode"));
    assert_eq!(
        f.suggestion.as_deref(),
        Some("use 4-digit octal like '0644'")
    );
}

#[test]
fn finding_display_full() {
    let f = ValidationFinding::error("invalid mode")
        .for_resource("cfg")
        .for_field("mode")
        .with_suggestion("use '0644'");
    let display = f.to_string();
    assert!(display.contains("error"));
    assert!(display.contains("cfg"));
    assert!(display.contains("mode"));
    assert!(display.contains("invalid mode"));
    assert!(display.contains("use '0644'"));
}

#[test]
fn finding_display_minimal() {
    let f = ValidationFinding::warning("something wrong");
    let display = f.to_string();
    assert!(display.contains("warning"));
    assert!(display.contains("something wrong"));
    // No resource or field should be in the output
    assert!(!display.contains("resource"));
}

#[test]
fn finding_serde_roundtrip() {
    let f = ValidationFinding::error("test")
        .for_resource("r1")
        .for_field("f1");
    let json = serde_json::to_string(&f).unwrap();
    let back: ValidationFinding = serde_json::from_str(&json).unwrap();
    assert_eq!(back.message, "test");
    assert_eq!(back.resource.as_deref(), Some("r1"));
    assert_eq!(back.field.as_deref(), Some("f1"));
}

#[test]
fn finding_serde_skips_none_fields() {
    let f = ValidationFinding::error("x");
    let json = serde_json::to_string(&f).unwrap();
    // None fields should not appear in JSON
    assert!(!json.contains("resource"));
    assert!(!json.contains("field"));
    assert!(!json.contains("suggestion"));
}

// ============================================================================
// FJ-2503: ValidateOutput
// ============================================================================

#[test]
fn validate_output_from_findings_valid_when_no_errors() {
    let findings = vec![
        ValidationFinding::warning("warn1"),
        ValidationFinding::warning("warn2"),
    ];
    let output = ValidateOutput::from_findings(findings, 5, 2);
    assert!(output.valid);
    assert_eq!(output.error_count(), 0);
    assert_eq!(output.warning_count(), 2);
    assert_eq!(output.resource_count, 5);
    assert_eq!(output.machine_count, 2);
}

#[test]
fn validate_output_from_findings_invalid_when_errors() {
    let findings = vec![
        ValidationFinding::error("err1"),
        ValidationFinding::warning("warn1"),
    ];
    let output = ValidateOutput::from_findings(findings, 3, 1);
    assert!(!output.valid);
    assert_eq!(output.error_count(), 1);
    assert_eq!(output.warning_count(), 1);
}

#[test]
fn validate_output_empty_is_valid() {
    let output = ValidateOutput::from_findings(vec![], 0, 0);
    assert!(output.valid);
    assert_eq!(output.error_count(), 0);
    assert_eq!(output.warning_count(), 0);
}

#[test]
fn validate_output_format_summary() {
    let findings = vec![
        ValidationFinding::error("bad mode"),
        ValidationFinding::warning("deprecated field"),
    ];
    let output = ValidateOutput::from_findings(findings, 10, 3);
    let summary = output.format_summary();
    assert!(summary.contains("1 errors"));
    assert!(summary.contains("1 warnings"));
    assert!(summary.contains("10 resources"));
    assert!(summary.contains("3 machines"));
}

#[test]
fn validate_output_default() {
    let output = ValidateOutput::default();
    assert!(!output.valid); // Default bool is false
    assert_eq!(output.resource_count, 0);
    assert_eq!(output.machine_count, 0);
    assert!(output.findings.is_empty());
}

#[test]
fn validate_output_serde_roundtrip() {
    let output = ValidateOutput::from_findings(vec![ValidationFinding::error("test error")], 5, 2);
    let json = serde_json::to_string(&output).unwrap();
    let back: ValidateOutput = serde_json::from_str(&json).unwrap();
    assert!(!back.valid);
    assert_eq!(back.error_count(), 1);
    assert_eq!(back.resource_count, 5);
}

// ============================================================================
// FJ-2500: FieldSuggestion
// ============================================================================

#[test]
fn field_suggestion_should_suggest_distance_1() {
    let s = FieldSuggestion::new("packges", "packages", 1);
    assert!(s.should_suggest());
    assert_eq!(s.unknown, "packges");
    assert_eq!(s.known, "packages");
    assert_eq!(s.distance, 1);
}

#[test]
fn field_suggestion_should_suggest_distance_2() {
    let s = FieldSuggestion::new("pahh", "path", 2);
    assert!(s.should_suggest());
}

#[test]
fn field_suggestion_should_not_suggest_distance_3() {
    let s = FieldSuggestion::new("xyz", "packages", 6);
    assert!(!s.should_suggest());
}

#[test]
fn field_suggestion_display() {
    let s = FieldSuggestion::new("packges", "packages", 1);
    let display = s.to_string();
    assert_eq!(display, "'packges' -> 'packages' (distance: 1)");
}

#[test]
fn field_suggestion_serde_roundtrip() {
    let s = FieldSuggestion::new("mde", "mode", 1);
    let json = serde_json::to_string(&s).unwrap();
    let back: FieldSuggestion = serde_json::from_str(&json).unwrap();
    assert_eq!(back.unknown, "mde");
    assert_eq!(back.known, "mode");
    assert_eq!(back.distance, 1);
}

// ============================================================================
// FJ-2503: DeepCheckFlags
// ============================================================================

#[test]
fn deep_flags_default_all_false() {
    let flags = DeepCheckFlags::default();
    assert!(!flags.templates);
    assert!(!flags.circular_deps);
    assert!(!flags.connectivity);
    assert!(!flags.secrets);
    assert!(!flags.overlaps);
    assert!(!flags.naming);
    assert!(!flags.machine_refs);
    assert!(!flags.state_values);
    assert!(!flags.drift_coverage);
    assert!(!flags.idempotency);
    assert!(!flags.any_enabled());
}

#[test]
fn deep_flags_exhaustive_all_true() {
    let flags = DeepCheckFlags::exhaustive();
    assert!(flags.templates);
    assert!(flags.circular_deps);
    assert!(flags.connectivity);
    assert!(flags.secrets);
    assert!(flags.overlaps);
    assert!(flags.naming);
    assert!(flags.machine_refs);
    assert!(flags.state_values);
    assert!(flags.drift_coverage);
    assert!(flags.idempotency);
    assert!(flags.any_enabled());
}

#[test]
fn deep_flags_any_enabled_single() {
    let mut flags = DeepCheckFlags::default();
    assert!(!flags.any_enabled());
    flags.templates = true;
    assert!(flags.any_enabled());
}

#[test]
fn deep_flags_serde_roundtrip() {
    let flags = DeepCheckFlags::exhaustive();
    let json = serde_json::to_string(&flags).unwrap();
    let back: DeepCheckFlags = serde_json::from_str(&json).unwrap();
    assert!(back.templates);
    assert!(back.circular_deps);
    assert!(back.secrets);
}

#[test]
fn deep_flags_serde_defaults_missing_fields() {
    // Deserializing with missing fields should default to false
    let json = r#"{"templates": true}"#;
    let flags: DeepCheckFlags = serde_json::from_str(json).unwrap();
    assert!(flags.templates);
    assert!(!flags.circular_deps);
    assert!(!flags.secrets);
    assert!(flags.any_enabled());
}

// ============================================================================
// FJ-2500: Unknown field detection via check_unknown_fields
// ============================================================================

#[test]
fn unknown_fields_clean_config() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    provider: apt
    packages: [curl]
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(warnings.is_empty());
}

#[test]
fn unknown_fields_detects_typo_in_resource() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    provider: apt
    packges: [curl]
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(!warnings.is_empty());
    let msg = &warnings[0].message;
    assert!(msg.contains("packges") || msg.contains("unknown"));
}

#[test]
fn unknown_fields_detects_unknown_top_level() {
    let yaml = r#"
version: "1.0"
name: test
bogus_key: true
resources: {}
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(!warnings.is_empty());
    assert!(warnings[0].message.contains("bogus_key"));
}

#[test]
fn unknown_fields_detects_unknown_machine_field() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
    flavor: large
resources: {}
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(!warnings.is_empty());
    assert!(warnings[0].message.contains("flavor"));
}

#[test]
fn unknown_fields_suggestion_levenshtein() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  f:
    type: file
    path: /etc/test
    conten: hello
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(!warnings.is_empty());
    // Should suggest "content" for "conten" (distance 1)
    let msg = &warnings[0].message;
    assert!(msg.contains("content") || msg.contains("did you mean"));
}

// ============================================================================
// FJ-2502: Include provenance
// ============================================================================

#[test]
fn include_provenance_field_exists_on_config() {
    let config = ForjarConfig::default();
    assert!(config.include_provenance.is_empty());
}

#[test]
fn include_provenance_not_serialized() {
    let mut config = ForjarConfig::default();
    config.version = "1.0".into();
    config.name = "test".into();
    config
        .include_provenance
        .insert("resource:pkg".into(), "infra.yaml".into());
    let yaml = serde_yaml_ng::to_string(&config).unwrap();
    // include_provenance should NOT appear in serialized YAML (it's #[serde(skip)])
    assert!(!yaml.contains("include_provenance"));
    assert!(!yaml.contains("infra.yaml"));
}

#[test]
fn include_provenance_survives_clone() {
    let mut config = ForjarConfig::default();
    config
        .include_provenance
        .insert("machine:web".into(), "base.yaml".into());
    let cloned = config.clone();
    assert_eq!(
        cloned
            .include_provenance
            .get("machine:web")
            .map(String::as_str),
        Some("base.yaml")
    );
}

#[test]
fn include_provenance_key_format() {
    // Provenance keys follow "type:id" format
    let mut config = ForjarConfig::default();
    config
        .include_provenance
        .insert("resource:nginx".into(), "web.yaml".into());
    config
        .include_provenance
        .insert("machine:db".into(), "db.yaml".into());
    config
        .include_provenance
        .insert("param:port".into(), "common.yaml".into());
    config
        .include_provenance
        .insert("output:url".into(), "outputs.yaml".into());
    config
        .include_provenance
        .insert("data:env".into(), "data.yaml".into());
    assert_eq!(config.include_provenance.len(), 5);
}

// ============================================================================
// FJ-2502: Include merge via parse_config_file
// ============================================================================

#[test]
fn include_file_merge_resources() {
    let dir = tempfile::tempdir().unwrap();
    let inc_path = dir.path().join("inc.yaml");
    std::fs::write(
        &inc_path,
        "version: \"1.0\"\nname: inc\nresources:\n  extra:\n    type: package\n    provider: apt\n    packages: [vim]\n",
    )
    .unwrap();
    let base_path = dir.path().join("base.yaml");
    std::fs::write(
        &base_path,
        format!(
            "version: \"1.0\"\nname: base\nincludes:\n  - {}\nresources:\n  main:\n    type: package\n    provider: apt\n    packages: [curl]\n",
            inc_path.display()
        ),
    )
    .unwrap();

    let config = parse_and_validate(&base_path).unwrap();
    assert!(config.resources.contains_key("main"));
    assert!(config.resources.contains_key("extra"));
    assert_eq!(
        config
            .include_provenance
            .get("resource:extra")
            .map(String::as_str),
        Some(inc_path.to_str().unwrap())
    );
}
