use super::data::resolve_data_sources;
use super::template::resolve_template;
use super::*;

#[test]
fn test_fj223_data_source_file() {
    let dir = tempfile::tempdir().unwrap();
    let data_file = dir.path().join("version.txt");
    std::fs::write(&data_file, "1.2.3\n").unwrap();

    let yaml = format!(
        r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/version
    content: "v={{{{data.app_version}}}}"
data:
  app_version:
    type: file
    value: "{}"
"#,
        data_file.display()
    );
    let mut config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    // Should have injected __data__app_version
    let val = config.params.get("__data__app_version").unwrap();
    assert_eq!(yaml_value_to_string(val), "1.2.3");
}

#[test]
fn test_fj223_data_source_command() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  hostname:
    type: command
    value: "echo test-host"
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    let val = config.params.get("__data__hostname").unwrap();
    assert_eq!(yaml_value_to_string(val), "test-host");
}

#[test]
fn test_fj223_data_source_file_with_default() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  missing:
    type: file
    value: /nonexistent/file
    default: "fallback"
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    let val = config.params.get("__data__missing").unwrap();
    assert_eq!(yaml_value_to_string(val), "fallback");
}

#[test]
fn test_fj223_data_source_file_no_default_fails() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  missing:
    type: file
    value: /nonexistent/file
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let result = resolve_data_sources(&mut config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("file error"));
}

#[test]
fn test_fj223_data_template_resolution() {
    let dir = tempfile::tempdir().unwrap();
    let data_file = dir.path().join("env.txt");
    std::fs::write(&data_file, "production").unwrap();

    let yaml = format!(
        r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/env.conf
    content: "env={{{{data.env}}}}"
data:
  env:
    type: file
    value: "{}"
"#,
        data_file.display()
    );
    let mut config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    // Now resolve the template
    let resolved = resolve_template("env={{data.env}}", &config.params, &config.machines).unwrap();
    assert_eq!(resolved, "env=production");
}

#[test]
fn test_fj223_data_source_command_with_default() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  fail:
    type: command
    value: "exit 1"
    default: "fallback"
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    let val = config.params.get("__data__fail").unwrap();
    assert_eq!(yaml_value_to_string(val), "fallback");
}

#[test]
fn test_fj223_data_source_dns() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  loopback:
    type: dns
    value: localhost
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    let val = config.params.get("__data__loopback").unwrap();
    let ip = yaml_value_to_string(val);
    assert!(ip == "127.0.0.1" || ip == "::1", "got: {ip}");
}

// ============================================================================
// FJ-1260: forjar-state data source tests
// ============================================================================

#[test]
fn test_fj1260_forjar_state_reads_outputs_from_lock() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    // Write a mock global lock with outputs
    let lock_yaml = r#"
schema: "1"
name: infra-config
outputs:
  db_host: "db.example.com"
  db_port: "5432"
"#;
    std::fs::write(state_dir.join("forjar.lock.yaml"), lock_yaml).unwrap();

    let yaml = format!(
        r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {{}}
data:
  db_connection:
    type: forjar-state
    state_dir: "{}"
    outputs: [db_host]
"#,
        state_dir.display()
    );
    let mut config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    let val = config.params.get("__data__db_connection").unwrap();
    assert_eq!(yaml_value_to_string(val), "db.example.com");
}

#[test]
fn test_fj1260_forjar_state_falls_back_to_default() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  upstream:
    type: forjar-state
    state_dir: /nonexistent/state
    default: "localhost:8080"
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    let val = config.params.get("__data__upstream").unwrap();
    assert_eq!(yaml_value_to_string(val), "localhost:8080");
}

#[test]
fn test_fj1260_forjar_state_fails_without_default() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  upstream:
    type: forjar-state
    state_dir: /nonexistent/state
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let result = resolve_data_sources(&mut config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("state lock not found"));
}

#[test]
fn test_fj1260_forjar_state_returns_all_outputs_as_json() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let lock_yaml = r#"
schema: "1"
name: infra
outputs:
  host: "web.example.com"
  port: "443"
"#;
    std::fs::write(state_dir.join("forjar.lock.yaml"), lock_yaml).unwrap();

    let yaml = format!(
        r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {{}}
data:
  all_outputs:
    type: forjar-state
    state_dir: "{}"
"#,
        state_dir.display()
    );
    let mut config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    let val = config.params.get("__data__all_outputs").unwrap();
    let json_str = yaml_value_to_string(val);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["host"], "web.example.com");
    assert_eq!(parsed["port"], "443");
}

#[test]
fn test_fj1260_forjar_state_missing_requested_output() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let lock_yaml = r#"
schema: "1"
name: infra
outputs:
  host: "web.example.com"
"#;
    std::fs::write(state_dir.join("forjar.lock.yaml"), lock_yaml).unwrap();

    let yaml = format!(
        r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {{}}
data:
  db:
    type: forjar-state
    state_dir: "{}"
    outputs: [nonexistent_key]
"#,
        state_dir.display()
    );
    let mut config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let result = resolve_data_sources(&mut config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("none of requested outputs"));
}

#[test]
fn test_fj223_no_data_sources() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();
    // No __data__ keys added
    assert!(!config.params.keys().any(|k| k.starts_with("__data__")));
}

// Fault isolation: one failing data source must not stop unrelated resources.
const ISOLATION_YAML: &str = r#"
version: "1.0"
name: iso
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  dependent:
    type: file
    machine: m1
    path: /etc/dep
    content: "v={{ data.broken }}"
  downstream:
    type: file
    machine: m1
    path: /etc/down
    content: static
    depends_on: [dependent]
  unrelated:
    type: file
    machine: m1
    path: /etc/unrel
    content: "static"
data:
  broken:
    type: command
    value: "exit 127"
"#;

#[test]
fn test_isolated_records_failure_without_erroring() {
    let mut config: ForjarConfig = serde_yaml_ng::from_str(ISOLATION_YAML).unwrap();
    let failures = super::data::resolve_data_sources_isolated(&mut config);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].key, "broken");
    assert!(
        failures[0].message.contains("exit 127"),
        "{}",
        failures[0].message
    );
    assert!(!config.params.contains_key("__data__broken"));
}

#[test]
fn test_isolated_still_resolves_healthy_sources() {
    let yaml = ISOLATION_YAML.replace(
        "data:\n",
        "data:\n  good:\n    type: command\n    value: \"echo ok\"\n",
    );
    let mut config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let failures = super::data::resolve_data_sources_isolated(&mut config);
    assert_eq!(failures.len(), 1);
    assert!(config.params.contains_key("__data__good"));
}

#[test]
fn test_dependents_of_failed_sources_are_transitive_and_exact() {
    let mut config: ForjarConfig = serde_yaml_ng::from_str(ISOLATION_YAML).unwrap();
    let failures = super::data::resolve_data_sources_isolated(&mut config);
    let dep = super::data::dependents_of_failures(&config, &failures);
    assert!(dep["dependent"].contains("data source 'broken'"));
    assert!(dep["dependent"].contains("exit 127"));
    assert!(dep.contains_key("downstream"), "depends_on cascade");
    assert!(!dep.contains_key("unrelated"));
}

#[test]
fn test_strict_resolver_still_errors() {
    let mut config: ForjarConfig = serde_yaml_ng::from_str(ISOLATION_YAML).unwrap();
    let err = resolve_data_sources(&mut config).unwrap_err();
    assert!(err.contains("command failed (exit 127)"));
}

#[test]
fn test_reference_match_respects_hyphenated_keys() {
    let yaml = "version: \"1.0\"\nname: h\nmachines: { m: { hostname: m, addr: 127.0.0.1 } }\ndata:\n  foo: { type: command, value: \"exit 1\" }\nresources:\n  uses_other: { type: file, machine: m, path: /tmp/h, content: \"{{data.foo-bar}}\" }\n  uses_foo: { type: file, machine: m, path: /tmp/h2, content: \"{{data.foo}}\" }\n";
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let failures = super::data::resolve_data_sources_isolated(&mut config);
    let dep = super::data::dependents_of_failures(&config, &failures);
    assert!(dep.contains_key("uses_foo"));
    assert!(
        !dep.contains_key("uses_other"),
        "data.foo-bar is not data.foo"
    );
}
