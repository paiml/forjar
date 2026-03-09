//! Additional coverage tests for template function edge cases.

use super::functions::resolve_function;
use super::*;
use std::collections::HashMap;

fn machines_with_web() -> indexmap::IndexMap<String, Machine> {
    let mut machines = indexmap::IndexMap::new();
    machines.insert(
        "web".to_string(),
        Machine {
            hostname: "web-prod".to_string(),
            addr: "10.0.0.1".to_string(),
            user: "deploy".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        },
    );
    machines
}

#[test]
fn machine_ref_user_field() {
    let p = HashMap::new();
    let machines = machines_with_web();
    let result = resolve_function("upper(machine.web.user)", &p, &machines).unwrap();
    assert_eq!(result, "DEPLOY");
}

#[test]
fn machine_ref_arch_field() {
    let p = HashMap::new();
    let machines = machines_with_web();
    let result = resolve_function("lower(machine.web.arch)", &p, &machines).unwrap();
    assert_eq!(result, "x86_64");
}

#[test]
fn machine_ref_addr_field() {
    let p = HashMap::new();
    let machines = machines_with_web();
    let result = resolve_function("trim(machine.web.addr)", &p, &machines).unwrap();
    assert_eq!(result, "10.0.0.1");
}

#[test]
fn machine_ref_unknown_field() {
    let p = HashMap::new();
    let machines = machines_with_web();
    let result = resolve_function("upper(machine.web.bogus)", &p, &machines);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown machine field"));
}

#[test]
fn machine_ref_unknown_machine() {
    let p = HashMap::new();
    let machines = machines_with_web();
    let result = resolve_function("upper(machine.db.hostname)", &p, &machines);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown machine"));
}

#[test]
fn malformed_function_no_parens() {
    let p = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_function("upperhello", &p, &machines);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("malformed function"));
}

#[test]
fn malformed_function_unclosed_paren() {
    let p = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_function("upper(hello", &p, &machines);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unclosed parenthesis"));
}

#[test]
fn single_quote_arg() {
    let p = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_function("upper('hello')", &p, &machines).unwrap();
    assert_eq!(result, "HELLO");
}

#[test]
fn unknown_param_ref() {
    let p = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_function("upper(params.missing)", &p, &machines);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown param"));
}

#[test]
fn bare_string_literal() {
    let p = HashMap::new();
    let machines = indexmap::IndexMap::new();
    // bare string (no quotes, no prefix) treated as literal
    let result = resolve_function("upper(bare_text)", &p, &machines).unwrap();
    assert_eq!(result, "BARE_TEXT");
}

// ── Data source staleness + edge case coverage ─────────────────────

#[test]
fn dns_error_with_default() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  bad_dns:
    type: dns
    value: "nonexistent-domain-12345.invalid"
    default: "0.0.0.0"
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    super::data::resolve_data_sources(&mut config).unwrap();
    let val = config.params.get("__data__bad_dns").unwrap();
    assert_eq!(yaml_value_to_string(val), "0.0.0.0");
}

#[test]
fn dns_error_without_default() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  bad_dns:
    type: dns
    value: "nonexistent-domain-12345.invalid"
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let result = super::data::resolve_data_sources(&mut config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("DNS"));
}

#[test]
fn command_error_without_default() {
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
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let result = super::data::resolve_data_sources(&mut config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("command failed"));
}

#[test]
fn forjar_state_no_outputs_section_with_default() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    // Lock without outputs section
    let lock_yaml = r#"
schema: "1"
name: infra
last_apply: "2026-01-01T00:00:00Z"
machines: {}
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
  upstream:
    type: forjar-state
    state_dir: "{}"
    default: "fallback-value"
"#,
        state_dir.display()
    );
    let mut config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    super::data::resolve_data_sources(&mut config).unwrap();
    let val = config.params.get("__data__upstream").unwrap();
    assert_eq!(yaml_value_to_string(val), "fallback-value");
}

#[test]
fn forjar_state_staleness_check() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    // Lock with very old last_apply to trigger staleness
    let lock_yaml = r#"
schema: "1"
name: infra
last_apply: "2020-01-01T00:00:00Z"
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
  upstream:
    type: forjar-state
    state_dir: "{}"
    outputs: [host]
    max_staleness: "1h"
"#,
        state_dir.display()
    );
    let mut config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    // Should succeed (staleness is a warning, not an error)
    super::data::resolve_data_sources(&mut config).unwrap();
    let val = config.params.get("__data__upstream").unwrap();
    assert_eq!(yaml_value_to_string(val), "web.example.com");
}

#[test]
fn require_value_missing() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  broken:
    type: file
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let result = super::data::resolve_data_sources(&mut config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires 'value' field"));
}
