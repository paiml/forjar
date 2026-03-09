//! Coverage tests for validate_deep.rs — individual silent check functions.

use crate::core::types::ForjarConfig;
fn parse_config(yaml: &str) -> ForjarConfig {
    serde_yaml_ng::from_str(yaml).unwrap()
}

#[test]
fn templates_no_params_no_templates() {
    let cfg = parse_config("version: '1.0'\nname: t\nmachines: {}\nresources: {}\n");
    assert!(super::validate_deep::check_templates_silent(&cfg).is_ok());
}

#[test]
fn templates_resolved_params() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
params:
  port: "8080"
resources:
  cfg:
    type: file
    path: /tmp/x
    content: "port={{params.port}}"
"#,
    );
    assert!(super::validate_deep::check_templates_silent(&cfg).is_ok());
}

#[test]
fn templates_unresolved_params() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  cfg:
    type: file
    path: /tmp/x
    content: "host={{params.missing}}"
"#,
    );
    let err = super::validate_deep::check_templates_silent(&cfg).unwrap_err();
    assert!(err.contains("unresolved"));
}

// ── check_overlaps_silent ────────────────────────────────────────

#[test]
fn overlaps_no_overlap() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  a:
    type: file
    path: /tmp/a
    content: a
  b:
    type: file
    path: /tmp/b
    content: b
"#,
    );
    assert!(super::validate_deep::check_overlaps_silent(&cfg).is_ok());
}

#[test]
fn overlaps_detected() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  a:
    type: file
    path: /tmp/shared
    content: a
  b:
    type: file
    path: /tmp/shared
    content: b
"#,
    );
    let err = super::validate_deep::check_overlaps_silent(&cfg).unwrap_err();
    assert!(err.contains("overlap"));
}

#[test]
fn overlaps_no_paths() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  pkg:
    type: package
    packages:
      - nginx
"#,
    );
    assert!(super::validate_deep::check_overlaps_silent(&cfg).is_ok());
}

// ── check_secrets_silent ─────────────────────────────────────────

#[test]
fn secrets_clean_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("clean.yaml");
    std::fs::write(&f, "version: '1.0'\nname: test\n").unwrap();
    assert!(super::validate_deep::check_secrets_silent(&f).is_ok());
}

#[test]
fn secrets_password_detected() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("secrets.yaml");
    std::fs::write(&f, "db:\n  password: s3cret123\n").unwrap();
    let err = super::validate_deep::check_secrets_silent(&f).unwrap_err();
    assert!(err.contains("secret"));
}

#[test]
fn secrets_aws_key_detected() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("aws.yaml");
    std::fs::write(&f, "key: AKIAIOSFODNN7EXAMPLE\n").unwrap();
    let err = super::validate_deep::check_secrets_silent(&f).unwrap_err();
    assert!(err.contains("secret"));
}

#[test]
fn secrets_github_token() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("gh.yaml");
    std::fs::write(&f, "auth: ghp_xxxxxxxxxxxx\n").unwrap();
    let err = super::validate_deep::check_secrets_silent(&f).unwrap_err();
    assert!(err.contains("secret"));
}

#[test]
fn secrets_commented_line_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("commented.yaml");
    std::fs::write(&f, "# password: example\nname: test\n").unwrap();
    assert!(super::validate_deep::check_secrets_silent(&f).is_ok());
}

#[test]
fn secrets_nonexistent_file() {
    let path = std::path::Path::new("/tmp/forjar-nonexistent-xyz.yaml");
    assert!(super::validate_deep::check_secrets_silent(path).is_ok());
}

// ── check_naming_silent ──────────────────────────────────────────

#[test]
fn naming_valid_names() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  nginx:
    type: package
    packages: [nginx]
  my-app-2:
    type: package
    packages: [curl]
"#,
    );
    assert!(super::validate_deep::check_naming_silent(&cfg).is_ok());
}

#[test]
fn naming_uppercase_violation() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  BadName:
    type: package
    packages: [nginx]
"#,
    );
    let err = super::validate_deep::check_naming_silent(&cfg).unwrap_err();
    assert!(err.contains("naming violation"));
}

#[test]
fn naming_double_dash() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  my--app:
    type: package
    packages: [nginx]
"#,
    );
    let err = super::validate_deep::check_naming_silent(&cfg).unwrap_err();
    assert!(err.contains("naming violation"));
}

#[test]
fn naming_trailing_dash() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  app-:
    type: package
    packages: [nginx]
"#,
    );
    let err = super::validate_deep::check_naming_silent(&cfg).unwrap_err();
    assert!(err.contains("naming violation"));
}

#[test]
fn naming_empty_resources() {
    let cfg = parse_config("version: '1.0'\nname: t\nmachines: {}\nresources: {}\n");
    assert!(super::validate_deep::check_naming_silent(&cfg).is_ok());
}

// ── check_idempotency_silent ─────────────────────────────────────

#[test]
fn idempotency_known_types() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  pkg:
    type: package
    packages: [nginx]
  f:
    type: file
    path: /tmp/x
    content: hello
"#,
    );
    assert!(super::validate_deep::check_idempotency_silent(&cfg).is_ok());
}

// ── check_connectivity_silent ────────────────────────────────────

#[test]
fn connectivity_localhost_ok() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources: {}
"#,
    );
    assert!(super::validate_deep::check_connectivity_silent(&cfg).is_ok());
}

#[test]
fn connectivity_empty_addr() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines:
  m1:
    hostname: m1
    addr: ""
resources: {}
"#,
    );
    let err = super::validate_deep::check_connectivity_silent(&cfg).unwrap_err();
    assert!(err.contains("connectivity"));
}

#[test]
fn connectivity_container_ok() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines:
  ctr:
    hostname: ctr
    addr: container
resources: {}
"#,
    );
    assert!(super::validate_deep::check_connectivity_silent(&cfg).is_ok());
}

#[test]
fn connectivity_remote_no_hostname() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines:
  remote:
    hostname: ""
    addr: 10.0.0.5
resources: {}
"#,
    );
    let err = super::validate_deep::check_connectivity_silent(&cfg).unwrap_err();
    assert!(err.contains("connectivity"));
}

// ── check_machine_refs_silent ────────────────────────────────────

#[test]
fn machine_refs_valid() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: local
    packages: [nginx]
"#,
    );
    assert!(super::validate_deep::check_machine_refs_silent(&cfg).is_ok());
}

#[test]
fn machine_refs_dangling() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: nonexistent
    packages: [nginx]
"#,
    );
    let err = super::validate_deep::check_machine_refs_silent(&cfg).unwrap_err();
    assert!(err.contains("dangling"));
}

// ── check_state_values_silent ────────────────────────────────────

#[test]
fn state_values_valid_file() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  f:
    type: file
    path: /tmp/x
    content: hello
    state: file
"#,
    );
    assert!(super::validate_deep::check_state_values_silent(&cfg).is_ok());
}

#[test]
fn state_values_valid_service() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  svc:
    type: service
    name: nginx
    state: running
"#,
    );
    assert!(super::validate_deep::check_state_values_silent(&cfg).is_ok());
}

#[test]
fn state_values_invalid_service() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  svc:
    type: service
    name: nginx
    state: invalid_state
"#,
    );
    let err = super::validate_deep::check_state_values_silent(&cfg).unwrap_err();
    assert!(err.contains("invalid state"));
}

#[test]
fn state_values_valid_mount() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  mnt:
    type: mount
    path: /mnt/data
    device: /dev/sda1
    state: mounted
"#,
    );
    assert!(super::validate_deep::check_state_values_silent(&cfg).is_ok());
}

#[test]
fn state_values_invalid_file() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  f:
    type: file
    path: /tmp/x
    content: hello
    state: running
"#,
    );
    let err = super::validate_deep::check_state_values_silent(&cfg).unwrap_err();
    assert!(err.contains("invalid state"));
}

#[test]
fn state_values_no_state_field() {
    let cfg = parse_config(
        r#"
version: "1.0"
name: t
machines: {}
resources:
  pkg:
    type: package
    packages: [nginx]
"#,
    );
    assert!(super::validate_deep::check_state_values_silent(&cfg).is_ok());
}

// ── emit_deep_json ───────────────────────────────────────────────

#[test]
fn emit_deep_json_all_pass() {
    let results: Vec<(&str, Result<(), String>)> =
        vec![("templates", Ok(())), ("naming", Ok(()))];
    assert!(super::validate_deep::emit_deep_json(&results).is_ok());
}

#[test]
fn emit_deep_json_with_failures() {
    let results: Vec<(&str, Result<(), String>)> = vec![
        ("templates", Ok(())),
        ("naming", Err("2 violations".to_string())),
    ];
    let err = super::validate_deep::emit_deep_json(&results).unwrap_err();
    assert!(err.contains("1 deep validation"));
}
