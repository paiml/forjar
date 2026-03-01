//! Include/recipe tests (FJ-254).

#![allow(unused_imports)]
use super::*;
use std::path::Path;

#[test]
fn test_fj254_includes_empty_by_default() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: m1
    path: /tmp/app
    content: "hello"
"#;
    let config = parse_config(yaml).unwrap();
    assert!(config.includes.is_empty());
}

#[test]
fn test_fj254_includes_parsed() {
    let yaml = r#"
version: "1.0"
name: test
includes:
  - base.yaml
  - overrides.yaml
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: m1
    path: /tmp/app
    content: "hello"
"#;
    let config = parse_config(yaml).unwrap();
    assert_eq!(config.includes.len(), 2);
    assert_eq!(config.includes[0], "base.yaml");
    assert_eq!(config.includes[1], "overrides.yaml");
}

#[test]
fn test_fj254_merge_params() {
    let dir = tempfile::tempdir().unwrap();

    let base_yaml = r#"
version: "1.0"
name: base
params:
  env: staging
  region: us-east
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: m1
    path: /tmp/app
    content: "hello"
"#;
    let override_yaml = r#"
version: "1.0"
name: override
params:
  env: production
resources: {}
"#;
    std::fs::write(dir.path().join("override.yaml"), override_yaml).unwrap();

    let base = parse_config(base_yaml).unwrap();
    let mut base_with_includes = base;
    base_with_includes.includes = vec!["override.yaml".to_string()];

    let merged = includes::merge_includes(base_with_includes, dir.path()).unwrap();
    assert_eq!(
        merged.params["env"],
        serde_yaml_ng::Value::String("production".to_string())
    );
    assert_eq!(
        merged.params["region"],
        serde_yaml_ng::Value::String("us-east".to_string())
    );
}

#[test]
fn test_fj254_merge_machines() {
    let dir = tempfile::tempdir().unwrap();

    let base_yaml = r#"
version: "1.0"
name: base
machines:
  web:
    hostname: web
    addr: 10.0.0.1
resources:
  app:
    type: file
    machine: web
    path: /tmp/app
    content: "hello"
"#;
    let extra_yaml = r#"
version: "1.0"
name: extra
machines:
  db:
    hostname: db
    addr: 10.0.0.2
resources: {}
"#;
    std::fs::write(dir.path().join("extra.yaml"), extra_yaml).unwrap();

    let mut base = parse_config(base_yaml).unwrap();
    base.includes = vec!["extra.yaml".to_string()];
    let merged = includes::merge_includes(base, dir.path()).unwrap();
    assert!(merged.machines.contains_key("web"));
    assert!(merged.machines.contains_key("db"));
    assert_eq!(merged.machines.len(), 2);
}

#[test]
fn test_fj254_merge_resources() {
    let dir = tempfile::tempdir().unwrap();

    let base_yaml = r#"
version: "1.0"
name: base
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  config:
    type: file
    machine: m1
    path: /etc/base.conf
    content: "base"
"#;
    let extra_yaml = r#"
version: "1.0"
name: extra
resources:
  packages:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#;
    std::fs::write(dir.path().join("extra.yaml"), extra_yaml).unwrap();

    let mut base = parse_config(base_yaml).unwrap();
    base.includes = vec!["extra.yaml".to_string()];
    let merged = includes::merge_includes(base, dir.path()).unwrap();
    assert!(merged.resources.contains_key("config"));
    assert!(merged.resources.contains_key("packages"));
}

#[test]
fn test_fj254_merge_resource_override() {
    let dir = tempfile::tempdir().unwrap();

    let base_yaml = r#"
version: "1.0"
name: base
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  config:
    type: file
    machine: m1
    path: /etc/base.conf
    content: "original"
"#;
    let override_yaml = r#"
version: "1.0"
name: override
resources:
  config:
    type: file
    machine: m1
    path: /etc/base.conf
    content: "overridden"
"#;
    std::fs::write(dir.path().join("override.yaml"), override_yaml).unwrap();

    let mut base = parse_config(base_yaml).unwrap();
    base.includes = vec!["override.yaml".to_string()];
    let merged = includes::merge_includes(base, dir.path()).unwrap();
    assert_eq!(
        merged.resources["config"].content.as_deref(),
        Some("overridden")
    );
}

#[test]
fn test_fj254_merge_policy_replaced() {
    let dir = tempfile::tempdir().unwrap();

    let base_yaml = r#"
version: "1.0"
name: base
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: m1
    path: /tmp/app
    content: "hello"
policy:
  tripwire: true
  lock_file: true
"#;
    let override_yaml = r#"
version: "1.0"
name: override
resources: {}
policy:
  tripwire: false
  lock_file: false
"#;
    std::fs::write(dir.path().join("override.yaml"), override_yaml).unwrap();

    let mut base = parse_config(base_yaml).unwrap();
    base.includes = vec!["override.yaml".to_string()];
    let merged = includes::merge_includes(base, dir.path()).unwrap();
    assert!(!merged.policy.tripwire);
    assert!(!merged.policy.lock_file);
}

#[test]
fn test_fj254_include_file_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let base_yaml = r#"
version: "1.0"
name: base
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: m1
    path: /tmp/app
    content: "hello"
"#;
    let mut base = parse_config(base_yaml).unwrap();
    base.includes = vec!["nonexistent.yaml".to_string()];
    let result = includes::merge_includes(base, dir.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("nonexistent.yaml"));
}

#[test]
fn test_fj254_includes_cleared_after_merge() {
    let dir = tempfile::tempdir().unwrap();
    let extra_yaml = r#"
version: "1.0"
name: extra
resources: {}
"#;
    std::fs::write(dir.path().join("extra.yaml"), extra_yaml).unwrap();

    let base_yaml = r#"
version: "1.0"
name: base
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: m1
    path: /tmp/app
    content: "hello"
"#;
    let mut base = parse_config(base_yaml).unwrap();
    base.includes = vec!["extra.yaml".to_string()];
    let merged = includes::merge_includes(base, dir.path()).unwrap();
    assert!(
        merged.includes.is_empty(),
        "includes should be cleared after merge"
    );
}

#[test]
fn test_fj254_multiple_includes_order() {
    let dir = tempfile::tempdir().unwrap();

    let first_yaml = r#"
version: "1.0"
name: first
params:
  env: staging
resources: {}
"#;
    let second_yaml = r#"
version: "1.0"
name: second
params:
  env: production
resources: {}
"#;
    std::fs::write(dir.path().join("first.yaml"), first_yaml).unwrap();
    std::fs::write(dir.path().join("second.yaml"), second_yaml).unwrap();

    let base_yaml = r#"
version: "1.0"
name: base
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: m1
    path: /tmp/app
    content: "hello"
"#;
    let mut base = parse_config(base_yaml).unwrap();
    base.includes = vec!["first.yaml".to_string(), "second.yaml".to_string()];
    let merged = includes::merge_includes(base, dir.path()).unwrap();
    assert_eq!(
        merged.params["env"],
        serde_yaml_ng::Value::String("production".to_string())
    );
}

#[test]
fn test_fj254_end_to_end_parse_and_validate() {
    let dir = tempfile::tempdir().unwrap();

    let base_yaml = r#"
version: "1.0"
name: base
includes:
  - machines.yaml
machines: {}
resources:
  app:
    type: file
    machine: web
    path: /tmp/app
    content: "hello"
"#;
    let machines_yaml = r#"
version: "1.0"
name: machines
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources: {}
"#;
    std::fs::write(dir.path().join("main.yaml"), base_yaml).unwrap();
    std::fs::write(dir.path().join("machines.yaml"), machines_yaml).unwrap();

    let result = parse_and_validate(&dir.path().join("main.yaml"));
    assert!(
        result.is_ok(),
        "should succeed with included machines: {:?}",
        result
    );
    let config = result.unwrap();
    assert!(config.machines.contains_key("web"));
    assert!(config.resources.contains_key("app"));
}

#[test]
fn test_fj254_include_without_resources_field() {
    let dir = tempfile::tempdir().unwrap();

    let params_yaml = r#"
version: "1.0"
name: shared-params
params:
  forjar_managed: "true"
  forjar_version: "1.0"
"#;
    std::fs::write(dir.path().join("params.yaml"), params_yaml).unwrap();

    let base_yaml = r#"
version: "1.0"
name: base
includes:
  - params.yaml
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: m1
    path: /tmp/app
    content: "hello"
"#;
    std::fs::write(dir.path().join("main.yaml"), base_yaml).unwrap();

    let result = parse_and_validate(&dir.path().join("main.yaml"));
    assert!(
        result.is_ok(),
        "include without resources field should work: {:?}",
        result
    );
    let config = result.unwrap();
    assert_eq!(
        config.params["forjar_managed"],
        serde_yaml_ng::Value::String("true".to_string())
    );
    assert!(config.resources.contains_key("app"));
}
