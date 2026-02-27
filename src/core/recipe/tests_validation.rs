//! Tests for recipe input validation -- type checking, bounds, enum constraints, proptests.

use std::collections::HashMap;

use super::expansion::parse_recipe;
use super::types::RecipeInput;
use super::validation::{validate_inputs, validate_input_type, validate_int};

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
fn test_fj019_validate_inputs_ok() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let mut provided = HashMap::new();
    provided.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("/mnt/data".to_string()),
    );
    let resolved = validate_inputs(&recipe.recipe, &provided).unwrap();
    assert_eq!(resolved["export_path"], "/mnt/data");
    assert_eq!(resolved["network"], "192.168.50.0/24");
    assert_eq!(resolved["port"], "2049");
}

#[test]
fn test_fj019_validate_inputs_missing_required() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let provided = HashMap::new();
    let result = validate_inputs(&recipe.recipe, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("export_path"));
}

#[test]
fn test_fj019_validate_path_type() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let mut provided = HashMap::new();
    provided.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("relative/path".to_string()),
    );
    let result = validate_inputs(&recipe.recipe, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("absolute path"));
}

#[test]
fn test_fj019_validate_int_range() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let mut provided = HashMap::new();
    provided.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("/mnt/data".to_string()),
    );
    provided.insert(
        "port".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(80)),
    );
    let result = validate_inputs(&recipe.recipe, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains(">= 1024"));
}

#[test]
fn test_fj019_validate_enum() {
    let yaml = r#"
recipe:
  name: test
  inputs:
    protocol:
      type: enum
      choices: [tcp, udp]
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let mut provided = HashMap::new();
    provided.insert(
        "protocol".to_string(),
        serde_yaml_ng::Value::String("icmp".to_string()),
    );
    let result = validate_inputs(&recipe.recipe, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("one of"));
}

#[test]
fn test_fj019_validate_bool() {
    let yaml = r#"
recipe:
  name: test
  inputs:
    enabled:
      type: bool
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let mut provided = HashMap::new();
    provided.insert("enabled".to_string(), serde_yaml_ng::Value::Bool(true));
    let resolved = validate_inputs(&recipe.recipe, &provided).unwrap();
    assert_eq!(resolved["enabled"], "true");
}

#[test]
fn test_fj019_validate_int_non_number() {
    let yaml = r#"
recipe:
  name: test
  inputs:
    count:
      type: int
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let mut provided = HashMap::new();
    provided.insert(
        "count".to_string(),
        serde_yaml_ng::Value::String("not-a-number".to_string()),
    );
    let result = validate_inputs(&recipe.recipe, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("integer"));
}

#[test]
fn test_fj019_validate_int_max() {
    let yaml = r#"
recipe:
  name: test
  inputs:
    count:
      type: int
      max: 10
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let mut provided = HashMap::new();
    provided.insert(
        "count".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(100)),
    );
    let result = validate_inputs(&recipe.recipe, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("<= 10"));
}

#[test]
fn test_fj019_validate_bool_non_bool() {
    let yaml = r#"
recipe:
  name: test
  inputs:
    flag:
      type: bool
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let mut provided = HashMap::new();
    provided.insert(
        "flag".to_string(),
        serde_yaml_ng::Value::String("yes".to_string()),
    );
    let result = validate_inputs(&recipe.recipe, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("boolean"));
}

#[test]
fn test_fj019_validate_path_non_string() {
    let yaml = r#"
recipe:
  name: test
  inputs:
    dir:
      type: path
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let mut provided = HashMap::new();
    provided.insert(
        "dir".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(42)),
    );
    let result = validate_inputs(&recipe.recipe, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("path string"));
}

#[test]
fn test_fj019_validate_enum_non_string() {
    let yaml = r#"
recipe:
  name: test
  inputs:
    proto:
      type: enum
      choices: [tcp, udp]
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let mut provided = HashMap::new();
    provided.insert("proto".to_string(), serde_yaml_ng::Value::Bool(true));
    let result = validate_inputs(&recipe.recipe, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must be a string"));
}

#[test]
fn test_fj019_validate_unknown_type() {
    let yaml = r#"
recipe:
  name: test
  inputs:
    x:
      type: float
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let mut provided = HashMap::new();
    provided.insert(
        "x".to_string(),
        serde_yaml_ng::Value::String("1.0".to_string()),
    );
    let result = validate_inputs(&recipe.recipe, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown input type"));
}

/// BH-MUT-0001: Valid enum choice should be accepted.
#[test]
fn test_fj019_validate_enum_valid_choice() {
    let yaml = r#"
recipe:
  name: test
  inputs:
    proto:
      type: enum
      choices: [tcp, udp]
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let mut provided = HashMap::new();
    provided.insert(
        "proto".to_string(),
        serde_yaml_ng::Value::String("tcp".to_string()),
    );
    let resolved = validate_inputs(&recipe.recipe, &provided).unwrap();
    assert_eq!(resolved["proto"], "tcp");
}

/// BH-MUT-0001: Enum with empty choices list should accept any string.
#[test]
fn test_fj019_validate_enum_empty_choices_accepts_any() {
    let yaml = r#"
recipe:
  name: test
  inputs:
    mode:
      type: enum
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let mut provided = HashMap::new();
    provided.insert(
        "mode".to_string(),
        serde_yaml_ng::Value::String("anything-goes".to_string()),
    );
    let resolved = validate_inputs(&recipe.recipe, &provided).unwrap();
    assert_eq!(resolved["mode"], "anything-goes");
}

#[test]
fn test_fj019_validate_string_non_string_coercion() {
    let yaml = r#"
recipe:
  name: test
  inputs:
    label:
      type: string
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let mut provided = HashMap::new();
    provided.insert(
        "label".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(42)),
    );
    let resolved = validate_inputs(&recipe.recipe, &provided).unwrap();
    assert!(!resolved["label"].is_empty());
}

#[test]
fn test_fj019_recipe_no_inputs() {
    let yaml = r#"
recipe:
  name: simple
  version: "1.0"
resources:
  pkg:
    type: package
    provider: apt
    packages: [curl]
"#;
    let recipe = parse_recipe(yaml).unwrap();
    assert!(recipe.recipe.inputs.is_empty());
    let resolved = validate_inputs(&recipe.recipe, &HashMap::new()).unwrap();
    assert!(resolved.is_empty());
}

#[test]
fn test_fj132_validate_input_valid() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let mut inputs = HashMap::new();
    inputs.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("/mnt".to_string()),
    );
    inputs.insert(
        "network".to_string(),
        serde_yaml_ng::Value::String("10.0.0.0/8".to_string()),
    );
    let result = validate_inputs(&recipe.recipe, &inputs);
    assert!(result.is_ok(), "valid inputs should pass validation");
}

#[test]
fn test_fj036_validate_inputs_rejects_wrong_type() {
    let yaml = r#"
recipe:
  name: test
  inputs:
    count:
      type: int
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let mut provided = HashMap::new();
    provided.insert(
        "count".to_string(),
        serde_yaml_ng::Value::String("not-an-integer".to_string()),
    );
    let result = validate_inputs(&recipe.recipe, &provided);
    assert!(result.is_err(), "string value for int input should be rejected");
    let err = result.unwrap_err();
    assert!(err.contains("integer"), "error should mention integer type: {}", err);
}

// -- Proptest for validation --

use proptest::prelude::*;

proptest! {
    /// FALSIFY-RD-002: validate_int rejects values outside [min, max].
    #[test]
    fn falsify_rd_002_int_bounds(n in -100i64..100, min in -50i64..0, max in 1i64..50) {
        let decl = RecipeInput {
            input_type: "int".to_string(),
            description: None,
            default: None,
            min: Some(min),
            max: Some(max),
            choices: vec![],
        };
        let value = serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(n));
        let result = validate_int("test", &value, &decl);

        if n < min {
            prop_assert!(result.is_err(), "n={} < min={} should be rejected", n, min);
        } else if n > max {
            prop_assert!(result.is_err(), "n={} > max={} should be rejected", n, max);
        } else {
            prop_assert!(result.is_ok(), "n={} in [{}, {}] should be accepted", n, min, max);
        }
    }

    /// FALSIFY-RD-003: path validation rejects non-absolute paths.
    #[test]
    fn falsify_rd_003_path_validation(s in "[a-z]{1,20}") {
        let decl = RecipeInput {
            input_type: "path".to_string(),
            description: None,
            default: None,
            min: None,
            max: None,
            choices: vec![],
        };
        let value = serde_yaml_ng::Value::String(s);
        let result = validate_input_type("test", "path", &value, &decl);
        prop_assert!(result.is_err(), "non-absolute path must be rejected");
    }
}
