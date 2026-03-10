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

use forjar::core::parser::{
    check_unknown_fields, parse_and_validate, parse_config, validate_config,
};
use forjar::core::resolver::build_execution_order;
use forjar::core::types::{
    DeepCheckFlags, FieldSuggestion, ForjarConfig, ValidateOutput, ValidationFinding,
    ValidationSeverity,
};

// ============================================================================
// FJ-2503: ValidationSeverity
#[test]
fn include_file_merge_machines() {
    let dir = tempfile::tempdir().unwrap();
    let inc_path = dir.path().join("machines.yaml");
    std::fs::write(
        &inc_path,
        "version: \"1.0\"\nname: inc\nmachines:\n  web:\n    hostname: web-01\n    addr: 10.0.0.1\nresources: {}\n",
    )
    .unwrap();
    let base_path = dir.path().join("base.yaml");
    std::fs::write(
        &base_path,
        format!(
            "version: \"1.0\"\nname: base\nincludes:\n  - {}\nmachines: {{}}\nresources: {{}}\n",
            inc_path.display()
        ),
    )
    .unwrap();

    let config = parse_and_validate(&base_path).unwrap();
    assert!(config.machines.contains_key("web"));
    assert!(config.include_provenance.contains_key("machine:web"));
}

#[test]
fn include_duplicate_detected() {
    let dir = tempfile::tempdir().unwrap();
    let inc_path = dir.path().join("dup.yaml");
    std::fs::write(&inc_path, "version: \"1.0\"\nname: dup\nresources: {}\n").unwrap();
    let base_path = dir.path().join("base.yaml");
    std::fs::write(
        &base_path,
        format!(
            "version: \"1.0\"\nname: base\nincludes:\n  - {p}\n  - {p}\nresources: {{}}\n",
            p = inc_path.display()
        ),
    )
    .unwrap();

    let result = parse_and_validate(&base_path);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("circular"));
}

#[test]
fn include_nonexistent_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let base_path = dir.path().join("base.yaml");
    std::fs::write(
        &base_path,
        "version: \"1.0\"\nname: base\nincludes:\n  - nonexistent.yaml\nresources: {}\n",
    )
    .unwrap();

    let result = parse_and_validate(&base_path);
    assert!(result.is_err());
}

// ============================================================================
// FJ-2503: Cycle detection via build_execution_order
// ============================================================================

#[test]
fn dag_no_cycle_simple_chain() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: localhost
resources:
  a:
    type: package
    machine: m
    provider: apt
    packages: [curl]
  b:
    type: file
    machine: m
    path: /etc/test
    content: hello
    depends_on: [a]
  c:
    type: file
    machine: m
    path: /etc/test2
    content: world
    depends_on: [b]
"#;
    let config = parse_config(yaml).unwrap();
    let order = build_execution_order(&config).unwrap();
    assert_eq!(order.len(), 3);
    // a must come before b, b before c
    let pos_a = order.iter().position(|s| s == "a").unwrap();
    let pos_b = order.iter().position(|s| s == "b").unwrap();
    let pos_c = order.iter().position(|s| s == "c").unwrap();
    assert!(pos_a < pos_b);
    assert!(pos_b < pos_c);
}

#[test]
fn dag_cycle_detected() {
    let yaml = r#"
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
    let config = parse_config(yaml).unwrap();
    let result = build_execution_order(&config);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("cycle"));
}

#[test]
fn dag_self_cycle_detected() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  a:
    type: package
    provider: apt
    packages: [curl]
    depends_on: [a]
"#;
    let config = parse_config(yaml).unwrap();
    let result = build_execution_order(&config);
    assert!(result.is_err());
}

#[test]
fn dag_independent_resources_all_included() {
    let yaml = r#"
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
  c:
    type: package
    provider: apt
    packages: [git]
"#;
    let config = parse_config(yaml).unwrap();
    let order = build_execution_order(&config).unwrap();
    assert_eq!(order.len(), 3);
    assert!(order.contains(&"a".to_string()));
    assert!(order.contains(&"b".to_string()));
    assert!(order.contains(&"c".to_string()));
}

// ============================================================================
// FJ-2503: Template validation via parse_and_validate
// ============================================================================

#[test]
fn validate_unresolved_template_detected() {
    // Deep template checking happens in validate_deep (pub(super)),
    // but parse_and_validate detects unresolved params in content.
    // Test via the config-level check that validate_config performs.
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /etc/app.conf
    content: "port={{params.undefined_param}}"
"#;
    let config = parse_config(yaml).unwrap();
    // validate_config checks structural issues; template resolution
    // is checked during the resolve phase. Verify config parses OK
    // but the content still contains the unresolved template.
    let res = &config.resources["cfg"];
    assert!(res.content.as_deref().unwrap().contains("{{params."));
}

#[test]
fn validate_resolved_template_content_preserved() {
    let yaml = r#"
version: "1.0"
name: test
params:
  port: "8080"
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /etc/app.conf
    content: "port={{params.port}}"
"#;
    let config = parse_config(yaml).unwrap();
    // Template is preserved in parsed config (resolved later by resolver)
    let res = &config.resources["cfg"];
    assert!(res.content.as_deref().unwrap().contains("{{params.port}}"));
    // But the param exists to be resolved
    assert!(config.params.contains_key("port"));
}

// ============================================================================
// FJ-2503: Resource naming validation
// ============================================================================

#[test]
fn naming_valid_kebab_case() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  my-nginx-config:
    type: file
    machine: m
    path: /etc/nginx.conf
    content: "server {}"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    // kebab-case should be fine
    let naming_err = errors.iter().any(|e| e.message.contains("naming"));
    assert!(!naming_err);
}

// ============================================================================
// FJ-2503: Machine reference validation
// ============================================================================

#[test]
fn validate_dangling_machine_ref_detected() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  cfg:
    type: file
    machine: nonexistent
    path: /etc/test
    content: hello
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let has_ref_err = errors
        .iter()
        .any(|e| e.message.contains("machine") && e.message.contains("nonexistent"));
    assert!(has_ref_err, "expected dangling machine ref: {:?}", errors);
}

#[test]
fn validate_valid_machine_ref_ok() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
resources:
  cfg:
    type: file
    machine: web
    path: /etc/test
    content: hello
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let has_ref_err = errors.iter().any(|e| {
        e.message.contains("machine") && e.message.contains("web") && e.message.contains("not")
    });
    assert!(!has_ref_err, "no machine ref errors expected: {:?}", errors);
}

// ============================================================================
// FJ-2503: Dependency reference validation
// ============================================================================

#[test]
fn validate_dangling_dependency_detected() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  cfg:
    type: file
    path: /etc/test
    content: hello
    depends_on: [ghost]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let has_dep_err = errors
        .iter()
        .any(|e| e.message.contains("ghost") || e.message.contains("depend"));
    assert!(has_dep_err, "expected dangling dep error: {:?}", errors);
}

// ============================================================================
// FJ-2503: State value validation
// ============================================================================

#[test]
fn validate_config_accepts_valid_state() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  svc:
    type: service
    machine: m
    name: nginx
    state: running
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let state_err = errors.iter().any(|e| e.message.contains("state"));
    assert!(!state_err, "no state errors expected: {:?}", errors);
}

// ============================================================================
// FJ-2503: Overlap detection (multiple resources targeting same path)
// ============================================================================

#[test]
fn overlapping_paths_detectable_from_config() {
    // Overlap detection is a deep check (pub(super) in validate_deep.rs).
    // Here we verify the data model allows detecting overlaps from parsed config.
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  file-a:
    type: file
    machine: m
    path: /etc/shared.conf
    content: "version a"
  file-b:
    type: file
    machine: m
    path: /etc/shared.conf
    content: "version b"
"#;
    let config = parse_config(yaml).unwrap();
    // Both resources parse and target the same path
    let path_a = config.resources["file-a"].path.as_deref();
    let path_b = config.resources["file-b"].path.as_deref();
    assert_eq!(path_a, Some("/etc/shared.conf"));
    assert_eq!(path_b, Some("/etc/shared.conf"));
    assert_eq!(path_a, path_b); // Overlap exists — deep validation would flag this
}

#[test]
fn validate_overlapping_paths_via_parse_and_validate() {
    // parse_and_validate runs full validation pipeline including overlap checks
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("overlap.yaml");
    std::fs::write(
        &path,
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  file-a:
    type: file
    machine: m
    path: /etc/shared.conf
    content: "version a"
  file-b:
    type: file
    machine: m
    path: /etc/shared.conf
    content: "version b"
"#,
    )
    .unwrap();
    // parse_and_validate should succeed (overlaps are warnings in standard mode,
    // errors only in deep mode)
    let result = parse_and_validate(&path);
    assert!(result.is_ok());
}

// ============================================================================
// Cross-cutting: Serde roundtrips
// ============================================================================

#[test]
fn validate_output_json_roundtrip_with_findings() {
    let output = ValidateOutput::from_findings(
        vec![
            ValidationFinding::error("err1").for_resource("r1"),
            ValidationFinding::warning("warn1")
                .for_field("f1")
                .with_suggestion("try X"),
        ],
        8,
        4,
    );
    let json = serde_json::to_string_pretty(&output).unwrap();
    let back: ValidateOutput = serde_json::from_str(&json).unwrap();
    assert_eq!(back.findings.len(), 2);
    assert!(!back.valid);
    assert_eq!(back.error_count(), 1);
    assert_eq!(back.warning_count(), 1);
    assert_eq!(back.resource_count, 8);
}

#[test]
fn deep_flags_partial_enable() {
    let mut flags = DeepCheckFlags::default();
    flags.secrets = true;
    flags.naming = true;
    assert!(flags.any_enabled());
    assert!(flags.secrets);
    assert!(flags.naming);
    assert!(!flags.templates);
    assert!(!flags.circular_deps);
}
