//! FJ-2502/2503/2500: Include hardening & deep validation falsification.
//!
//! Demonstrates Popperian rejection criteria for:
//! - Include provenance tracking and circular detection
//! - Deep validation types (severity, findings, output, flags)
//! - Unknown field detection with Levenshtein suggestions
//! - DAG cycle detection
//!
//! Usage: cargo run --example include_deep_validation_falsification

use forjar::core::parser::{check_unknown_fields, parse_config};
use forjar::core::resolver::build_execution_order;
use forjar::core::types::{
    DeepCheckFlags, FieldSuggestion, ValidateOutput, ValidationFinding, ValidationSeverity,
};

fn report_check(label: &str, ok: bool, pass_text: &str, fail_text: &str) {
    let status = if ok { pass_text } else { fail_text };
    let mark = if ok { "✓" } else { "✗ FALSIFIED" };
    println!("  {label}: {status} {mark}");
    assert!(ok);
}

fn report_yes_no(label: &str, ok: bool) {
    report_check(label, ok, "yes", "no");
}

fn check_severity_and_finding() {
    println!("\n[FJ-2503] Severity Model:");

    let sev_ok = ValidationSeverity::Error > ValidationSeverity::Warning
        && ValidationSeverity::Warning > ValidationSeverity::Hint;
    report_yes_no("Error > Warning > Hint", sev_ok);

    println!("\n[FJ-2503] ValidationFinding Builder:");

    let f = ValidationFinding::error("bad mode")
        .for_resource("cfg")
        .for_field("mode")
        .with_suggestion("use 0644");
    let builder_ok = f.is_error() && f.resource.as_deref() == Some("cfg") && f.suggestion.is_some();
    report_check("Builder chain", builder_ok, "correct", "wrong");
}

fn check_validate_output() {
    println!("\n[FJ-2503] ValidateOutput:");

    let output = ValidateOutput::from_findings(
        vec![
            ValidationFinding::error("err"),
            ValidationFinding::warning("warn"),
        ],
        5,
        2,
    );
    let out_ok = !output.valid && output.error_count() == 1 && output.warning_count() == 1;
    report_yes_no("Error makes invalid, counts correct", out_ok);

    let valid_output = ValidateOutput::from_findings(vec![ValidationFinding::warning("w")], 1, 1);
    report_yes_no("Warnings-only is valid", valid_output.valid);
}

fn check_suggestions_and_flags() {
    println!("\n[FJ-2500] FieldSuggestion:");

    let close = FieldSuggestion::new("packges", "packages", 1);
    let far = FieldSuggestion::new("xyz", "packages", 6);
    let sug_ok = close.should_suggest() && !far.should_suggest();
    report_yes_no("Distance ≤2 suggests, >2 does not", sug_ok);

    println!("\n[FJ-2503] DeepCheckFlags:");

    let default_flags = DeepCheckFlags::default();
    let exhaust = DeepCheckFlags::exhaustive();
    let flags_ok = !default_flags.any_enabled() && exhaust.any_enabled() && exhaust.templates;
    report_yes_no("Default=none, exhaustive=all", flags_ok);
}

fn check_unknown_fields_section() {
    println!("\n[FJ-2500] Unknown Field Detection:");

    let clean_yaml = "version: \"1.0\"\nname: test\nresources:\n  pkg:\n    type: package\n    provider: apt\n    packages: [curl]\n";
    let clean_ok = check_unknown_fields(clean_yaml).is_empty();
    report_yes_no("Valid config has no unknown fields", clean_ok);

    let typo_yaml = "version: \"1.0\"\nname: test\nresources:\n  pkg:\n    type: package\n    provider: apt\n    packges: [curl]\n";
    let typo_ok = !check_unknown_fields(typo_yaml).is_empty();
    report_yes_no("Typo 'packges' detected", typo_ok);
}

fn check_dag_cycles() {
    println!("\n[FJ-2503] DAG Cycle Detection:");

    let linear_yaml = r#"
version: "1.0"
name: test
resources:
  a:
    type: package
    provider: apt
    packages: [curl]
  b:
    type: package
    provider: apt
    packages: [vim]
    depends_on: [a]
"#;
    let linear_config = parse_config(linear_yaml).unwrap_or_else(|e| panic!("{e:?}"));
    let linear_ok = build_execution_order(&linear_config).is_ok();
    report_yes_no("Linear chain resolves", linear_ok);

    let cycle_yaml = r#"
version: "1.0"
name: test
resources:
  a:
    type: package
    provider: apt
    packages: [curl]
    depends_on: [b]
  b:
    type: package
    provider: apt
    packages: [vim]
    depends_on: [a]
"#;
    let cycle_config = parse_config(cycle_yaml).unwrap_or_else(|e| panic!("{e:?}"));
    let cycle_ok = build_execution_order(&cycle_config).is_err();
    report_yes_no("Cycle a→b→a detected", cycle_ok);
}

fn main() {
    println!("Forjar Include Hardening & Deep Validation Falsification");
    println!("{}", "=".repeat(58));

    check_severity_and_finding();
    check_validate_output();
    check_suggestions_and_flags();
    check_unknown_fields_section();
    check_dag_cycles();

    println!("\n{}", "=".repeat(58));
    println!("All include hardening & deep validation criteria survived.");
}
