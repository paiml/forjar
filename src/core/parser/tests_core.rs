//! Core parsing and validation tests (FJ-002).

use super::*;

#[test]
fn test_fj002_parse_valid() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#;
    let config = parse_config(yaml).unwrap();
    assert_eq!(config.name, "test");
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "unexpected errors: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_fj002_bad_version() {
    let yaml = r#"
version: "2.0"
name: test
machines: {}
resources: {}
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.message.contains("version")));
}

#[test]
fn test_fj002_unknown_machine() {
    let yaml = r#"
version: "1.0"
name: test
machines: {}
resources:
  pkg:
    type: package
    machine: nonexistent
    provider: apt
    packages: [curl]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.message.contains("unknown machine")));
}

#[test]
fn test_fj002_unknown_dependency() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
    depends_on: [ghost]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("unknown resource")));
}

#[test]
fn test_fj002_self_dependency() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
    depends_on: [pkg]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("depends on itself")));
}

#[test]
fn test_fj002_parse_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.yaml");
    std::fs::write(
        &path,
        r#"
version: "1.0"
name: file-test
machines: {}
resources: {}
"#,
    )
    .unwrap();
    let config = parse_config_file(&path).unwrap();
    assert_eq!(config.name, "file-test");
}

#[test]
fn test_fj002_parse_invalid_yaml() {
    let result = parse_config("not: [valid: yaml: {{");
    assert!(result.is_err());
}

#[test]
fn test_fj002_empty_name() {
    let yaml = r#"
version: "1.0"
name: ""
machines: {}
resources: {}
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("name must not be empty")));
}

#[test]
fn test_fj002_validation_error_display() {
    let err = ValidationError {
        message: "test error".to_string(),
    };
    assert_eq!(format!("{err}"), "test error");
}

#[test]
fn test_fj002_parse_config_file_missing() {
    let result = parse_config_file(std::path::Path::new("/nonexistent/forjar.yaml"));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("failed to read"));
}

#[test]
fn test_fj002_localhost_accepted_without_definition() {
    let yaml = r#"
version: "1.0"
name: test
machines: {}
resources:
  pkg:
    type: package
    machine: localhost
    provider: apt
    packages: [curl]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        !errors.iter().any(|e| e.message.contains("unknown machine")),
        "localhost should be accepted without explicit definition"
    );
}

#[test]
fn test_fj002_deep_dependency_cycle_5_nodes() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  a:
    type: file
    machine: m1
    path: /a
    depends_on: [b]
  b:
    type: file
    machine: m1
    path: /b
    depends_on: [c]
  c:
    type: file
    machine: m1
    path: /c
    depends_on: [d]
  d:
    type: file
    machine: m1
    path: /d
    depends_on: [e]
  e:
    type: file
    machine: m1
    path: /e
    depends_on: [a]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "cycle detection is planning-time, not parse-time: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_fj002_diamond_dependency_valid() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  a:
    type: file
    machine: m1
    path: /a
    depends_on: [b, c]
  b:
    type: file
    machine: m1
    path: /b
    depends_on: [d]
  c:
    type: file
    machine: m1
    path: /c
    depends_on: [d]
  d:
    type: file
    machine: m1
    path: /d
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.is_empty(), "diamond pattern is valid: {errors:?}");
}

#[test]
fn test_fj002_multiple_validation_errors_same_config() {
    let yaml = r#"
version: "2.0"
name: ""
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  bad-pkg:
    type: package
    machine: m1
  bad-file:
    type: file
    machine: m1
  bad-svc:
    type: service
    machine: m1
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let msgs: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
    assert!(msgs.iter().any(|m| m.contains("version must be")));
    assert!(msgs.iter().any(|m| m.contains("name must not be empty")));
    assert!(msgs.iter().any(|m| m.contains("no packages")));
    assert!(msgs.iter().any(|m| m.contains("no provider")));
    assert!(msgs.iter().any(|m| m.contains("no path")));
    assert!(msgs.iter().any(|m| m.contains("(service) has no name")));
    assert!(
        errors.len() >= 6,
        "expected >= 6 errors, got {}",
        errors.len()
    );
}

/// forjar#354: malformed YAML and a valid-YAML non-config must not report the
/// same thing.
///
/// Both used to be `"YAML parse error: {e}"` with exit 3, so a caller could not
/// use `forjar validate` as a YAML syntax checker without flagging every file
/// in the repo that was never meant to be a forjar config.
#[test]
fn fj354_malformed_yaml_reports_a_parse_error() {
    let err = super::parse_config("a: 1\n  b: [unclosed\n").expect_err("must not parse");
    assert!(
        err.starts_with("YAML parse error:"),
        "a genuine syntax failure must say so: {err}"
    );
}

#[test]
fn fj354_valid_yaml_that_is_not_a_config_says_so() {
    let err = super::parse_config("a: 1\nb: two\n").expect_err("must not parse as a config");
    assert!(
        err.starts_with("not a forjar config:"),
        "this YAML parses fine; the failure is ours, not the document's: {err}"
    );
    assert!(
        !err.contains("YAML parse error"),
        "must not claim a parse error for YAML that parsed: {err}"
    );
}

/// The discriminator cannot be `location()`: serde attaches one to a schema
/// error too. This pins that, so a future "simplification" back to `location()`
/// fails here rather than silently re-merging the two classes.
#[test]
fn fj354_a_schema_error_also_carries_a_location() {
    let e = serde_yaml_ng::from_str::<super::ForjarConfig>("a: 1\nb: two\n")
        .expect_err("missing required fields");
    assert!(
        e.location().is_some(),
        "if this ever becomes None, location() would work as a discriminator \
         and this test should be revisited"
    );
}

#[test]
fn fj354_a_partial_config_is_a_schema_failure_not_a_syntax_one() {
    let err = super::parse_config("version: \"1.0\"\n").expect_err("no name, no resources");
    assert!(err.starts_with("not a forjar config:"), "{err}");
}
