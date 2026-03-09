//! FJ-2500: Unknown field detection with Levenshtein suggestions.
//!
//! Popperian rejection criteria for:
//! - FJ-2500: Config-level unknown field detection
//! - FJ-2500: Resource/machine/policy unknown field detection
//! - FJ-2500: Recipe YAML unknown field detection
//! - FJ-2500: Levenshtein "did you mean?" suggestions
//!
//! Usage: cargo test --test falsification_parser_unknown_fields

use forjar::core::parser::{check_unknown_fields, check_unknown_recipe_fields};

// ============================================================================
// FJ-2500: unknown field detection — config level
// ============================================================================

#[test]
fn unknown_field_at_root() {
    let yaml = r#"
version: "1.0"
name: test
typo_field: oops
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(!warnings.is_empty());
    assert!(warnings[0].message.contains("typo_field"));
}

#[test]
fn unknown_field_in_resource() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    packges: [nginx]
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(!warnings.is_empty());
    assert!(warnings[0].message.contains("packges"));
    // Levenshtein suggestion
    assert!(warnings[0].message.contains("packages"));
}

#[test]
fn unknown_field_in_machine() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostnme: web-01
    addr: 10.0.0.1
    user: deploy
    arch: x86_64
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(!warnings.is_empty());
    assert!(warnings[0].message.contains("hostnme"));
    assert!(warnings[0].message.contains("hostname"));
}

#[test]
fn no_unknown_fields_clean_config() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    packages: [curl]
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(warnings.is_empty());
}

#[test]
fn unknown_field_in_policy() {
    let yaml = r#"
version: "1.0"
name: test
policy:
  failre: continue
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(!warnings.is_empty());
    assert!(warnings[0].message.contains("failre"));
    assert!(warnings[0].message.contains("failure"));
}

#[test]
fn unknown_field_multiple_in_resource() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    packges: [nginx]
    versin: "1.0"
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(warnings.len() >= 2);
}

#[test]
fn unknown_field_no_suggestion_distant() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    zzzzzzz: wrong
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(!warnings.is_empty());
    assert!(warnings[0].message.contains("zzzzzzz"));
}

// ============================================================================
// FJ-2500: unknown field detection — recipe
// ============================================================================

#[test]
fn unknown_recipe_field_at_root() {
    let yaml = r#"
recipe:
  name: test-recipe
rsources:
  pkg:
    type: package
"#;
    let warnings = check_unknown_recipe_fields(yaml);
    assert!(!warnings.is_empty());
    assert!(warnings[0].message.contains("rsources"));
    assert!(warnings[0].message.contains("resources"));
}

#[test]
fn unknown_recipe_meta_field() {
    let yaml = r#"
recipe:
  name: test-recipe
  vrsion: "1.0"
resources:
  pkg:
    type: package
"#;
    let warnings = check_unknown_recipe_fields(yaml);
    assert!(!warnings.is_empty());
    assert!(warnings[0].message.contains("vrsion"));
    assert!(warnings[0].message.contains("version"));
}

#[test]
fn clean_recipe_no_warnings() {
    let yaml = r#"
recipe:
  name: test-recipe
  version: "1.0"
resources:
  pkg:
    type: package
    packages: [curl]
"#;
    let warnings = check_unknown_recipe_fields(yaml);
    assert!(warnings.is_empty());
}
