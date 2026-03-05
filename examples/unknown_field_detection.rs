//! FJ-2500: Unknown field detection with "did you mean?" suggestions.
//!
//! Demonstrates two-pass YAML parsing that catches typos in config files.
//! Run: cargo run --example unknown_field_detection

use forjar::core::parser::{check_unknown_fields, parse_config, ValidationError};

fn print_warnings(label: &str, warnings: &[ValidationError]) {
    println!("\n{label}:");
    for w in warnings {
        println!("  {w}");
    }
}

fn demo_valid() {
    let yaml = r#"
version: "1.0"
name: my-stack
resources:
  pkg:
    type: package
    packages: [curl, git]
    provider: apt
    machine: web
"#;
    let warnings = check_unknown_fields(yaml);
    println!("Valid config: {} warnings", warnings.len());
    assert!(warnings.is_empty());
}

fn demo_top_level_typo() {
    let yaml = r#"
version: "1.0"
name: my-stack
resorces:
  pkg:
    type: package
"#;
    let warnings = check_unknown_fields(yaml);
    print_warnings("Top-level typo 'resorces'", &warnings);
    assert_eq!(warnings.len(), 1);
}

fn demo_resource_typo() {
    let yaml = r#"
version: "1.0"
name: my-stack
resources:
  pkg:
    type: package
    packges: [curl]
    provider: apt
"#;
    let warnings = check_unknown_fields(yaml);
    print_warnings("Resource typo 'packges'", &warnings);
    assert_eq!(warnings.len(), 1);
}

fn demo_no_match() {
    let yaml = r#"
version: "1.0"
name: my-stack
resources:
  pkg:
    type: package
    zzz_garbage: true
"#;
    let warnings = check_unknown_fields(yaml);
    print_warnings("No close match 'zzz_garbage'", &warnings);
    assert_eq!(warnings.len(), 1);
}

fn demo_silent_data_loss() {
    let yaml = r#"
version: "1.0"
name: silent-loss
resources:
  web:
    type: package
    packges: [nginx, curl, git]
    provider: apt
"#;
    println!("\n--- Silent Data Loss Detection ---");
    let config = parse_config(yaml).unwrap();
    let resource = &config.resources["web"];
    println!("Typed parse: packages = {:?}", resource.packages);
    println!("  (empty! 'packges' was silently ignored by serde)");

    let warnings = check_unknown_fields(yaml);
    print_warnings("Two-pass detection catches it", &warnings);
    assert!(resource.packages.is_empty());
    assert_eq!(warnings.len(), 1);
}

fn main() {
    println!("=== FJ-2500: Unknown Field Detection ===\n");
    demo_valid();
    demo_top_level_typo();
    demo_resource_typo();
    demo_no_match();
    demo_silent_data_loss();
    println!("\n=== All checks passed ===");
}
