//! Tests for FJ-2500: unknown field detection.

use super::unknown_fields::*;

#[test]
fn valid_config_no_unknowns() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    packages: [curl]
    provider: apt
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert!(unknowns.is_empty(), "expected no unknowns: {unknowns:?}");
}

#[test]
fn typo_in_top_level_field() {
    let yaml = r#"
version: "1.0"
name: test
resorces:
  pkg:
    type: package
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "resorces");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("resources"));
}

#[test]
fn typo_in_resource_field() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    packges: [curl]
    provider: apt
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "packges");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("packages"));
    assert!(unknowns[0].path.contains("resources.pkg"));
}

#[test]
fn typo_in_machine_field() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
    tranport: container
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "tranport");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("transport"));
}

#[test]
fn unknown_field_no_suggestion() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    zzz_garbage_field: true
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "zzz_garbage_field");
    assert!(unknowns[0].suggestion.is_none());
}

#[test]
fn multiple_unknowns() {
    let yaml = r#"
version: "1.0"
name: test
machins:
  web:
    hostname: web-01
    addr: 10.0.0.1
resorces:
  pkg:
    type: package
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 2);
    let keys: Vec<_> = unknowns.iter().map(|u| u.key.as_str()).collect();
    assert!(keys.contains(&"machins"));
    assert!(keys.contains(&"resorces"));
}

#[test]
fn nested_container_unknown_field() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  ci:
    hostname: ci-01
    addr: container
    container:
      runtime: docker
      imge: ubuntu:22.04
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "imge");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("image"));
}

#[test]
fn nested_lifecycle_unknown_field() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  critical:
    type: file
    path: /etc/config
    lifecycle:
      prevent_destory: true
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "prevent_destory");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("prevent_destroy"));
}

#[test]
fn policy_unknown_field() {
    let yaml = r#"
version: "1.0"
name: test
policy:
  parllel_machines: true
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "parllel_machines");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("parallel_machines"));
}

#[test]
fn notify_unknown_field() {
    let yaml = r#"
version: "1.0"
name: test
policy:
  notify:
    on_succes: "echo done"
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "on_succes");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("on_success"));
}

#[test]
fn data_source_unknown_field() {
    let yaml = r#"
version: "1.0"
name: test
data:
  my-data:
    type: file
    vale: /etc/hosts
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "vale");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("value"));
}

#[test]
fn policies_list_unknown_field() {
    let yaml = r#"
version: "1.0"
name: test
policies:
  - type: require
    messge: "must have owner"
    field: owner
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "messge");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("message"));
}

#[test]
fn moved_unknown_field() {
    let yaml = r#"
version: "1.0"
name: test
moved:
  - form: old-name
    to: new-name
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "form");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("from"));
}

#[test]
fn checks_unknown_field() {
    let yaml = r#"
version: "1.0"
name: test
checks:
  health:
    machine: web
    comand: "curl localhost"
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "comand");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("command"));
}

#[test]
fn output_unknown_field() {
    let yaml = r#"
version: "1.0"
name: test
outputs:
  ip:
    vale: "{{machines.web.addr}}"
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "vale");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("value"));
}

#[test]
fn pepita_unknown_field() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  ns:
    hostname: ns-01
    addr: pepita
    pepita:
      rootfs: /rootfs
      memory_m: 512
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "memory_m");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("memory_mb"));
}

#[test]
fn display_with_suggestion() {
    let u = UnknownField {
        path: "resources.pkg.packges".to_string(),
        key: "packges".to_string(),
        suggestion: Some("packages".to_string()),
    };
    let msg = u.to_string();
    assert!(msg.contains("did you mean 'packages'"));
    assert!(msg.contains("resources.pkg.packges"));
}

#[test]
fn display_without_suggestion() {
    let u = UnknownField {
        path: "resources.pkg.zzz".to_string(),
        key: "zzz".to_string(),
        suggestion: None,
    };
    let msg = u.to_string();
    assert!(msg.contains("unknown field 'zzz'"));
    assert!(!msg.contains("did you mean"));
}

#[test]
fn empty_yaml_no_unknowns() {
    let yaml = "version: \"1.0\"\nname: test\n";
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert!(unknowns.is_empty());
}

#[test]
fn non_mapping_yaml_no_unknowns() {
    let yaml = "- item1\n- item2\n";
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert!(unknowns.is_empty());
}

#[test]
fn errors_conversion() {
    let unknowns = vec![UnknownField {
        path: "test".to_string(),
        key: "bad".to_string(),
        suggestion: None,
    }];
    let errors = unknown_fields_to_errors(&unknowns);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("unknown field 'bad'"));
}

#[test]
fn full_valid_config_all_sections() {
    let yaml = r#"
version: "1.0"
name: full-test
description: "Test all sections"
params:
  env: production
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
resources:
  pkg:
    type: package
    packages: [curl]
policy:
  failure: stop_on_first
  tripwire: true
  notify:
    on_success: "echo ok"
outputs:
  web_ip:
    value: "{{machines.web.addr}}"
policies:
  - type: require
    message: must have owner
    field: owner
data:
  hosts:
    type: file
    value: /etc/hosts
checks:
  ping:
    machine: web
    command: "ping -c1 localhost"
moved:
  - from: old
    to: new
includes: []
"#;
    let unknowns = detect_unknown_fields(yaml).unwrap();
    assert!(unknowns.is_empty(), "expected no unknowns: {unknowns:?}");
}

// -- Recipe unknown field tests --

#[test]
fn recipe_valid_no_unknowns() {
    let yaml = r#"
recipe:
  name: my-recipe
  version: "1.0"
  description: Test recipe
  inputs:
    port:
      type: string
      default: "8080"
      description: Port number
  requires:
    - recipe: base-recipe
resources:
  config:
    type: file
    path: /etc/config
"#;
    let unknowns = detect_unknown_recipe_fields(yaml).unwrap();
    assert!(unknowns.is_empty(), "expected no unknowns: {unknowns:?}");
}

#[test]
fn recipe_top_level_typo() {
    let yaml = r#"
recpe:
  name: test
resources: {}
"#;
    let unknowns = detect_unknown_recipe_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "recpe");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("recipe"));
}

#[test]
fn recipe_meta_unknown_field() {
    let yaml = r#"
recipe:
  name: test
  vesion: "1.0"
"#;
    let unknowns = detect_unknown_recipe_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "vesion");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("version"));
}

#[test]
fn recipe_input_unknown_field() {
    let yaml = r#"
recipe:
  name: test
  inputs:
    port:
      type: string
      defalt: "8080"
"#;
    let unknowns = detect_unknown_recipe_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "defalt");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("default"));
}

#[test]
fn recipe_requirement_unknown_field() {
    let yaml = r#"
recipe:
  name: test
  requires:
    - recpe: base
"#;
    let unknowns = detect_unknown_recipe_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "recpe");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("recipe"));
}

#[test]
fn recipe_resource_typo() {
    let yaml = r#"
recipe:
  name: test
resources:
  svc:
    type: service
    enbled: true
"#;
    let unknowns = detect_unknown_recipe_fields(yaml).unwrap();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].key, "enbled");
    assert_eq!(unknowns[0].suggestion.as_deref(), Some("enabled"));
}

#[test]
fn recipe_input_all_valid_fields() {
    let yaml = r#"
recipe:
  name: test
  inputs:
    count:
      type: int
      description: Number of instances
      default: 3
      min: 1
      max: 10
      choices: ["1", "3", "5"]
"#;
    let unknowns = detect_unknown_recipe_fields(yaml).unwrap();
    assert!(unknowns.is_empty(), "expected no unknowns: {unknowns:?}");
}
