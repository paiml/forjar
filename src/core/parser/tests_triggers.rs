//! Trigger tests (FJ-224).

use super::*;

#[test]
fn test_fj224_triggers_valid() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  config:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "hello"
  app:
    type: service
    machine: m1
    name: app
    depends_on: [config]
    triggers: [config]
"#;
    let config = parse_config(yaml).unwrap();
    assert!(config.resources["app"]
        .triggers
        .contains(&"config".to_string()));
}

#[test]
fn test_fj224_triggers_unknown_resource() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  app:
    type: service
    machine: m1
    name: app
    triggers: [ghost-resource]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("triggers on unknown resource")),
        "errors: {errors:?}"
    );
}

#[test]
fn test_fj224_triggers_self_reference() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: m1
    path: /tmp/app
    content: "x"
    triggers: [app]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("triggers on itself")),
        "errors: {errors:?}"
    );
}

#[test]
fn test_fj224_empty_triggers() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: m1
    path: /tmp/app
    content: "hello"
"#;
    let config = parse_config(yaml).unwrap();
    assert!(config.resources["app"].triggers.is_empty());
}
