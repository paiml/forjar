//! Tests for recipe expansion, template resolution, and loading.

#![allow(unused_imports)]
use std::collections::HashMap;
use std::path::Path;

use crate::core::types::MachineTarget;

use super::expansion::{
    expand_recipe, load_recipe, parse_recipe, recipe_terminal_id, resolve_input_template,
};
use super::validation::validate_inputs;

const RECIPE_YAML: &str = r#"
recipe:
  name: nfs-server
  version: "1.0"
  description: "NFS server recipe"
  inputs:
    export_path:
      type: path
      description: "Path to export"
    network:
      type: string
      default: "192.168.50.0/24"
    port:
      type: int
      default: 2049
      min: 1024
      max: 65535

resources:
  packages:
    type: package
    provider: apt
    packages: [nfs-kernel-server]

  exports:
    type: file
    path: /etc/exports
    content: "{{inputs.export_path}} {{inputs.network}}(rw,sync)"
    depends_on: [packages]

  service:
    type: service
    name: nfs-kernel-server
    state: running
    enabled: true
    restart_on: [exports]
    depends_on: [packages, exports]
"#;

#[test]
fn test_fj019_parse_recipe() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    assert_eq!(recipe.recipe.name, "nfs-server");
    assert_eq!(recipe.recipe.inputs.len(), 3);
    assert_eq!(recipe.resources.len(), 3);
}

#[test]
fn test_fj019_expand_recipe() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let machine = MachineTarget::Single("lambda".to_string());
    let mut inputs = HashMap::new();
    inputs.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("/mnt/raid".to_string()),
    );

    let expanded = expand_recipe("nfs", &recipe, &machine, &inputs, &[]).unwrap();

    assert_eq!(expanded.len(), 3);
    assert!(expanded.contains_key("nfs/packages"));
    assert!(expanded.contains_key("nfs/exports"));
    assert!(expanded.contains_key("nfs/service"));

    let exports = &expanded["nfs/exports"];
    assert!(exports.content.as_ref().unwrap().contains("/mnt/raid"));
    assert!(exports
        .content
        .as_ref()
        .unwrap()
        .contains("192.168.50.0/24"));
    assert!(exports.depends_on.contains(&"nfs/packages".to_string()));
    assert_eq!(exports.machine.to_vec(), vec!["lambda"]);
}

#[test]
fn test_fj019_expand_with_external_deps() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let machine = MachineTarget::Single("m1".to_string());
    let mut inputs = HashMap::new();
    inputs.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("/mnt/data".to_string()),
    );

    let expanded =
        expand_recipe("nfs", &recipe, &machine, &inputs, &["base-pkg".to_string()]).unwrap();

    let first = &expanded["nfs/packages"];
    assert!(first.depends_on.contains(&"base-pkg".to_string()));
}

#[test]
fn test_fj019_recipe_terminal_id() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let terminal = recipe_terminal_id("nfs", &recipe);
    assert_eq!(terminal, Some("nfs/service".to_string()));
}

#[test]
fn test_fj019_namespaced_restart_on() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let machine = MachineTarget::Single("m1".to_string());
    let mut inputs = HashMap::new();
    inputs.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("/mnt/data".to_string()),
    );

    let expanded = expand_recipe("nfs", &recipe, &machine, &inputs, &[]).unwrap();
    let service = &expanded["nfs/service"];
    assert!(service.restart_on.contains(&"nfs/exports".to_string()));
}

#[test]
fn test_fj019_load_recipe_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test-recipe.yaml");
    std::fs::write(&path, RECIPE_YAML).unwrap();

    let recipe = load_recipe(&path).unwrap();
    assert_eq!(recipe.recipe.name, "nfs-server");
}

#[test]
fn test_fj019_resolve_input_template() {
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), "world".to_string());
    let result = resolve_input_template("hello {{inputs.name}}!", &inputs).unwrap();
    assert_eq!(result, "hello world!");
}

#[test]
fn test_fj019_resolve_multiple_inputs() {
    let mut inputs = HashMap::new();
    inputs.insert("a".to_string(), "X".to_string());
    inputs.insert("b".to_string(), "Y".to_string());
    let result = resolve_input_template("{{inputs.a}}-{{inputs.b}}", &inputs).unwrap();
    assert_eq!(result, "X-Y");
}

/// BH-MUT-0002: Kills mutation of `first && !external_depends_on.is_empty()`.
#[test]
fn test_fj019_expand_empty_external_deps_not_injected() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let machine = MachineTarget::Single("m1".to_string());
    let mut inputs = HashMap::new();
    inputs.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("/mnt/data".to_string()),
    );

    let expanded = expand_recipe("nfs", &recipe, &machine, &inputs, &[]).unwrap();

    let first = &expanded["nfs/packages"];
    assert!(
        first.depends_on.is_empty(),
        "first resource should have no deps when external_depends_on is empty"
    );
}

/// BH-MUT-0002: Only the first resource should get external dependencies.
#[test]
fn test_fj019_expand_external_deps_only_on_first_resource() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let machine = MachineTarget::Single("m1".to_string());
    let mut inputs = HashMap::new();
    inputs.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("/mnt/data".to_string()),
    );

    let expanded =
        expand_recipe("nfs", &recipe, &machine, &inputs, &["base-pkg".to_string()]).unwrap();

    let first = &expanded["nfs/packages"];
    assert!(first.depends_on.contains(&"base-pkg".to_string()));

    let second = &expanded["nfs/exports"];
    assert!(
        !second.depends_on.contains(&"base-pkg".to_string()),
        "non-first resource should not get external dependencies"
    );

    let third = &expanded["nfs/service"];
    assert!(
        !third.depends_on.contains(&"base-pkg".to_string()),
        "non-first resource should not get external dependencies"
    );
}

#[test]
fn test_fj019_unclosed_input_template() {
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), "world".to_string());
    let result = resolve_input_template("{{inputs.name", &inputs);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unclosed template"));
}

#[test]
fn test_fj019_unknown_input_reference() {
    let inputs = HashMap::new();
    let result = resolve_input_template("{{inputs.ghost}}", &inputs);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown input"));
}

#[test]
fn test_fj019_no_template_passthrough() {
    let inputs = HashMap::new();
    let result = resolve_input_template("plain string", &inputs).unwrap();
    assert_eq!(result, "plain string");
}

#[test]
fn test_fj019_empty_template_passthrough() {
    let inputs = HashMap::new();
    let result = resolve_input_template("", &inputs).unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_fj019_terminal_id_empty_resources() {
    let yaml = r#"
recipe:
  name: empty
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let terminal = recipe_terminal_id("x", &recipe);
    assert!(terminal.is_none());
}

#[test]
fn test_fj019_load_recipe_nonexistent_file() {
    let result = load_recipe(Path::new("/nonexistent/recipe.yaml"));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cannot read recipe"));
}

#[test]
fn test_fj019_parse_recipe_invalid_yaml() {
    let result = parse_recipe(":::not valid yaml[[[");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("recipe parse error"));
}

#[test]
fn test_fj019_expand_multiple_external_deps() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let machine = MachineTarget::Single("m1".to_string());
    let mut inputs = HashMap::new();
    inputs.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("/mnt/data".to_string()),
    );

    let expanded = expand_recipe(
        "nfs",
        &recipe,
        &machine,
        &inputs,
        &["dep-a".to_string(), "dep-b".to_string()],
    )
    .unwrap();

    let first = &expanded["nfs/packages"];
    assert!(first.depends_on.contains(&"dep-a".to_string()));
    assert!(first.depends_on.contains(&"dep-b".to_string()));
}

#[test]
fn test_fj019_expand_all_defaults() {
    let yaml = r#"
recipe:
  name: defaults-only
  inputs:
    port:
      type: int
      default: 8080
    name:
      type: string
      default: "my-app"
resources:
  cfg:
    type: file
    path: "/etc/{{inputs.name}}/config"
    content: "port={{inputs.port}}"
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let machine = MachineTarget::Single("m1".to_string());
    let expanded = expand_recipe("app", &recipe, &machine, &HashMap::new(), &[]).unwrap();
    let cfg = &expanded["app/cfg"];
    assert_eq!(cfg.path.as_deref(), Some("/etc/my-app/config"));
    assert_eq!(cfg.content.as_deref(), Some("port=8080"));
}

/// FJ-1006: Resolve {{inputs.*}} in Vec<String> fields (ports, environment, volumes).
/// Regression test for renacer-observability template bug.
#[test]
fn test_fj1006_resolve_inputs_in_docker_vec_fields() {
    let yaml = r#"
recipe:
  name: obs-test
  inputs:
    jaeger_port:
      type: int
      default: 16686
    grafana_port:
      type: int
      default: 3000
    app_env:
      type: string
      default: "production"
resources:
  jaeger:
    type: docker
    name: test-jaeger
    image: jaegertracing/all-in-one:1.54
    state: running
    ports:
      - "{{inputs.jaeger_port}}:16686"
      - "4317:4317"
    environment:
      - "APP_ENV={{inputs.app_env}}"
    volumes:
      - "/opt/data:/data"
  grafana:
    type: docker
    name: test-grafana
    image: grafana/grafana:10.3.1
    state: running
    ports:
      - "{{inputs.grafana_port}}:3000"
  firewall:
    type: network
    port: "{{inputs.jaeger_port}}"
    protocol: tcp
    action: allow
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let machine = MachineTarget::Single("m1".to_string());
    let expanded = expand_recipe("obs", &recipe, &machine, &HashMap::new(), &[]).unwrap();

    // Verify ports resolved
    let jaeger = &expanded["obs/jaeger"];
    assert_eq!(jaeger.ports, vec!["16686:16686", "4317:4317"]);

    // Verify environment resolved
    assert_eq!(jaeger.environment, vec!["APP_ENV=production"]);

    // Verify volumes passthrough (no templates)
    assert_eq!(jaeger.volumes, vec!["/opt/data:/data"]);

    // Verify grafana ports resolved
    let grafana = &expanded["obs/grafana"];
    assert_eq!(grafana.ports, vec!["3000:3000"]);

    // Verify network port (Option<String>) still works
    let firewall = &expanded["obs/firewall"];
    assert_eq!(firewall.port.as_deref(), Some("16686"));
}

// -- Proptest for expansion determinism --

use proptest::prelude::*;

proptest! {
    /// FALSIFY-RD-001: expand_recipe is deterministic.
    #[test]
    fn falsify_rd_001_expansion_determinism(path in "/[a-z]{1,8}") {
        let recipe = parse_recipe(RECIPE_YAML).unwrap();
        let machine = MachineTarget::Single("m1".to_string());
        let mut inputs = HashMap::new();
        inputs.insert(
            "export_path".to_string(),
            serde_yaml_ng::Value::String(path),
        );

        let e1 = expand_recipe("nfs", &recipe, &machine, &inputs, &[]).unwrap();
        let e2 = expand_recipe("nfs", &recipe, &machine, &inputs, &[]).unwrap();

        let keys1: Vec<_> = e1.keys().collect();
        let keys2: Vec<_> = e2.keys().collect();
        prop_assert_eq!(keys1, keys2, "expansion keys must be deterministic");

        for key in e1.keys() {
            prop_assert_eq!(
                e1[key].content.as_deref(),
                e2[key].content.as_deref(),
                "content must be deterministic for {}",
                key
            );
            prop_assert_eq!(
                &e1[key].depends_on,
                &e2[key].depends_on,
                "depends_on must be deterministic for {}",
                key
            );
        }
    }

    /// FALSIFY-RD-004: external deps only injected into first resource.
    #[test]
    fn falsify_rd_004_external_deps_placement(dep in "[a-z]{1,8}") {
        let recipe = parse_recipe(RECIPE_YAML).unwrap();
        let machine = MachineTarget::Single("m1".to_string());
        let mut inputs = HashMap::new();
        inputs.insert(
            "export_path".to_string(),
            serde_yaml_ng::Value::String("/mnt/data".to_string()),
        );

        let expanded = expand_recipe(
            "nfs", &recipe, &machine, &inputs, std::slice::from_ref(&dep),
        ).unwrap();

        let first_key = expanded.keys().next().unwrap();
        prop_assert!(
            expanded[first_key].depends_on.contains(&dep),
            "first resource must have external dep"
        );

        for (i, (key, resource)) in expanded.iter().enumerate() {
            if i > 0 {
                prop_assert!(
                    !resource.depends_on.contains(&dep),
                    "resource {} at position {} must not have external dep",
                    key, i
                );
            }
        }
    }
}
