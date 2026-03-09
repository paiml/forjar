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

fn main() {
    println!("Forjar Include Hardening & Deep Validation Falsification");
    println!("{}", "=".repeat(58));

    // ── FJ-2503: Severity Model ──
    println!("\n[FJ-2503] Severity Model:");

    let sev_ok = ValidationSeverity::Error > ValidationSeverity::Warning
        && ValidationSeverity::Warning > ValidationSeverity::Hint;
    println!(
        "  Error > Warning > Hint: {} {}",
        if sev_ok { "yes" } else { "no" },
        if sev_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(sev_ok);

    // ── FJ-2503: Finding Builder ──
    println!("\n[FJ-2503] ValidationFinding Builder:");

    let f = ValidationFinding::error("bad mode")
        .for_resource("cfg")
        .for_field("mode")
        .with_suggestion("use 0644");
    let builder_ok = f.is_error() && f.resource.as_deref() == Some("cfg") && f.suggestion.is_some();
    println!(
        "  Builder chain: {} {}",
        if builder_ok { "correct" } else { "wrong" },
        if builder_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(builder_ok);

    // ── FJ-2503: ValidateOutput ──
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
    println!(
        "  Error makes invalid, counts correct: {} {}",
        if out_ok { "yes" } else { "no" },
        if out_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(out_ok);

    let valid_output = ValidateOutput::from_findings(vec![ValidationFinding::warning("w")], 1, 1);
    let valid_ok = valid_output.valid;
    println!(
        "  Warnings-only is valid: {} {}",
        if valid_ok { "yes" } else { "no" },
        if valid_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(valid_ok);

    // ── FJ-2500: FieldSuggestion ──
    println!("\n[FJ-2500] FieldSuggestion:");

    let close = FieldSuggestion::new("packges", "packages", 1);
    let far = FieldSuggestion::new("xyz", "packages", 6);
    let sug_ok = close.should_suggest() && !far.should_suggest();
    println!(
        "  Distance ≤2 suggests, >2 does not: {} {}",
        if sug_ok { "yes" } else { "no" },
        if sug_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(sug_ok);

    // ── FJ-2503: DeepCheckFlags ──
    println!("\n[FJ-2503] DeepCheckFlags:");

    let default_flags = DeepCheckFlags::default();
    let exhaust = DeepCheckFlags::exhaustive();
    let flags_ok = !default_flags.any_enabled() && exhaust.any_enabled() && exhaust.templates;
    println!(
        "  Default=none, exhaustive=all: {} {}",
        if flags_ok { "yes" } else { "no" },
        if flags_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(flags_ok);

    // ── FJ-2500: Unknown Fields ──
    println!("\n[FJ-2500] Unknown Field Detection:");

    let clean_yaml = "version: \"1.0\"\nname: test\nresources:\n  pkg:\n    type: package\n    provider: apt\n    packages: [curl]\n";
    let clean_ok = check_unknown_fields(clean_yaml).is_empty();
    println!(
        "  Valid config has no unknown fields: {} {}",
        if clean_ok { "yes" } else { "no" },
        if clean_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(clean_ok);

    let typo_yaml = "version: \"1.0\"\nname: test\nresources:\n  pkg:\n    type: package\n    provider: apt\n    packges: [curl]\n";
    let typo_ok = !check_unknown_fields(typo_yaml).is_empty();
    println!(
        "  Typo 'packges' detected: {} {}",
        if typo_ok { "yes" } else { "no" },
        if typo_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(typo_ok);

    // ── DAG Cycle Detection ──
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
    let linear_config = parse_config(linear_yaml).unwrap();
    let linear_ok = build_execution_order(&linear_config).is_ok();
    println!(
        "  Linear chain resolves: {} {}",
        if linear_ok { "yes" } else { "no" },
        if linear_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(linear_ok);

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
    let cycle_config = parse_config(cycle_yaml).unwrap();
    let cycle_ok = build_execution_order(&cycle_config).is_err();
    println!(
        "  Cycle a→b→a detected: {} {}",
        if cycle_ok { "yes" } else { "no" },
        if cycle_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(cycle_ok);

    println!("\n{}", "=".repeat(58));
    println!("All include hardening & deep validation criteria survived.");
}
