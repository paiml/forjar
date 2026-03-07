//! Tests for validation types — severity, findings, output, deep check flags.

use super::*;

#[test]
fn validation_severity_ordering() {
    assert!(ValidationSeverity::Hint < ValidationSeverity::Warning);
    assert!(ValidationSeverity::Warning < ValidationSeverity::Error);
}

#[test]
fn validation_severity_display() {
    assert_eq!(ValidationSeverity::Error.to_string(), "error");
    assert_eq!(ValidationSeverity::Warning.to_string(), "warning");
    assert_eq!(ValidationSeverity::Hint.to_string(), "hint");
}

#[test]
fn validation_severity_serde_roundtrip() {
    for sev in [
        ValidationSeverity::Hint,
        ValidationSeverity::Warning,
        ValidationSeverity::Error,
    ] {
        let json = serde_json::to_string(&sev).unwrap();
        let parsed: ValidationSeverity = serde_json::from_str(&json).unwrap();
        assert_eq!(sev, parsed);
    }
}

#[test]
fn validation_finding_builder() {
    let f = ValidationFinding::error("missing field")
        .for_resource("nginx")
        .for_field("packages")
        .with_suggestion("did you mean 'packages'?");
    assert!(f.is_error());
    assert!(!f.is_warning());
    assert_eq!(f.resource.as_deref(), Some("nginx"));
    assert_eq!(f.field.as_deref(), Some("packages"));
    assert!(f.suggestion.is_some());
}

#[test]
fn validation_finding_warning() {
    let f = ValidationFinding::warning("unknown field 'packges'").for_resource("db");
    assert!(f.is_warning());
    assert!(!f.is_error());
}

#[test]
fn validation_finding_display() {
    let f = ValidationFinding::error("bad field")
        .for_resource("r1")
        .for_field("mode")
        .with_suggestion("use octal format");
    let s = f.to_string();
    assert!(s.contains("error"));
    assert!(s.contains("r1"));
    assert!(s.contains("mode"));
    assert!(s.contains("bad field"));
    assert!(s.contains("use octal format"));
}

#[test]
fn validate_output_from_findings() {
    let findings = vec![
        ValidationFinding::error("missing packages"),
        ValidationFinding::warning("unknown field"),
    ];
    let output = ValidateOutput::from_findings(findings, 5, 2);
    assert!(!output.valid);
    assert_eq!(output.error_count(), 1);
    assert_eq!(output.warning_count(), 1);
    assert_eq!(output.resource_count, 5);
    assert_eq!(output.machine_count, 2);
}

#[test]
fn validate_output_valid_when_no_errors() {
    let findings = vec![ValidationFinding::warning("minor issue")];
    let output = ValidateOutput::from_findings(findings, 3, 1);
    assert!(output.valid);
}

#[test]
fn validate_output_format_summary() {
    let findings = vec![
        ValidationFinding::error("bad").for_resource("r1"),
        ValidationFinding::warning("meh"),
    ];
    let output = ValidateOutput::from_findings(findings, 10, 2);
    let text = output.format_summary();
    assert!(text.contains("1 errors"));
    assert!(text.contains("1 warnings"));
    assert!(text.contains("10 resources"));
}

#[test]
fn validate_output_empty() {
    let output = ValidateOutput::from_findings(vec![], 0, 0);
    assert!(output.valid);
    assert_eq!(output.error_count(), 0);
    assert_eq!(output.warning_count(), 0);
}

#[test]
fn field_suggestion_should_suggest() {
    assert!(FieldSuggestion::new("packges", "packages", 1).should_suggest());
    assert!(FieldSuggestion::new("maching", "machine", 2).should_suggest());
    assert!(!FieldSuggestion::new("foobar", "packages", 5).should_suggest());
}

#[test]
fn field_suggestion_display() {
    let s = FieldSuggestion::new("provder", "provider", 1);
    assert_eq!(s.to_string(), "'provder' -> 'provider' (distance: 1)");
}

#[test]
fn deep_check_flags_default_all_off() {
    let flags = DeepCheckFlags::default();
    assert!(!flags.any_enabled());
    assert!(!flags.templates);
    assert!(!flags.secrets);
    assert!(!flags.machine_refs);
    assert!(!flags.state_values);
    assert!(!flags.drift_coverage);
    assert!(!flags.idempotency);
}

#[test]
fn deep_check_flags_exhaustive() {
    let flags = DeepCheckFlags::exhaustive();
    assert!(flags.any_enabled());
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
}

#[test]
fn deep_check_flags_partial() {
    let flags = DeepCheckFlags {
        templates: true,
        ..Default::default()
    };
    assert!(flags.any_enabled());
    assert!(flags.templates);
    assert!(!flags.secrets);
}

#[test]
fn validation_finding_serde_roundtrip() {
    let f = ValidationFinding::error("test")
        .for_resource("r")
        .with_suggestion("fix it");
    let json = serde_json::to_string(&f).unwrap();
    let parsed: ValidationFinding = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.message, "test");
    assert_eq!(parsed.suggestion.as_deref(), Some("fix it"));
}
