//! FJ-019: Recipe loading, input validation, and expansion falsification.
//!
//! Popperian rejection criteria for:
//! - Recipe YAML parsing (RecipeFile, RecipeMetadata, RecipeInput)
//! - Input validation: string, int (bounds), bool, path, enum (choices)
//! - Template resolution: {{inputs.X}} in resource fields
//! - Recipe expansion: namespaced IDs, dependency namespacing, external deps
//! - Missing input rejection, unknown input type rejection
//! - recipe_terminal_id for dependency chains
//!
//! Usage: cargo test --test falsification_recipe_expansion

use forjar::core::recipe::{
    expand_recipe, parse_recipe, recipe_terminal_id, validate_inputs, RecipeInput, RecipeMetadata,
};
use forjar::core::types::MachineTarget;
use indexmap::IndexMap;
use std::collections::HashMap;

// ============================================================================
// Helpers
// ============================================================================

fn minimal_recipe_yaml() -> &'static str {
    r#"
recipe:
  name: test-recipe
  version: "1.0"
  description: "A test recipe"
resources:
  web-server:
    type: package
    name: "nginx"
"#
}

fn recipe_with_inputs_yaml() -> &'static str {
    r#"
recipe:
  name: web-stack
  version: "2.0"
  inputs:
    port:
      type: int
      description: "HTTP port"
      default: 80
      min: 1
      max: 65535
    domain:
      type: string
      description: "Domain name"
    env:
      type: enum
      choices: ["dev", "staging", "production"]
      default: "dev"
resources:
  pkg:
    type: package
    name: "nginx"
  config:
    type: file
    path: "/etc/nginx/sites/{{inputs.domain}}.conf"
    content: "server { listen {{inputs.port}}; server_name {{inputs.domain}}; }"
    depends_on:
      - pkg
"#
}

fn machine() -> MachineTarget {
    MachineTarget::default()
}

// ============================================================================
// FJ-019: Parse — Minimal Recipe
// ============================================================================

#[test]
fn parse_minimal_recipe() {
    let rf = parse_recipe(minimal_recipe_yaml()).unwrap();
    assert_eq!(rf.recipe.name, "test-recipe");
    assert_eq!(rf.recipe.version.as_deref(), Some("1.0"));
    assert_eq!(rf.resources.len(), 1);
}

#[test]
fn parse_recipe_resource_name() {
    let rf = parse_recipe(minimal_recipe_yaml()).unwrap();
    assert!(rf.resources.contains_key("web-server"));
}

#[test]
fn parse_recipe_with_inputs() {
    let rf = parse_recipe(recipe_with_inputs_yaml()).unwrap();
    assert_eq!(rf.recipe.inputs.len(), 3);
    assert!(rf.recipe.inputs.contains_key("port"));
    assert!(rf.recipe.inputs.contains_key("domain"));
    assert!(rf.recipe.inputs.contains_key("env"));
}

#[test]
fn parse_recipe_input_types() {
    let rf = parse_recipe(recipe_with_inputs_yaml()).unwrap();
    assert_eq!(rf.recipe.inputs["port"].input_type, "int");
    assert_eq!(rf.recipe.inputs["domain"].input_type, "string");
    assert_eq!(rf.recipe.inputs["env"].input_type, "enum");
}

#[test]
fn parse_recipe_input_bounds() {
    let rf = parse_recipe(recipe_with_inputs_yaml()).unwrap();
    let port = &rf.recipe.inputs["port"];
    assert_eq!(port.min, Some(1));
    assert_eq!(port.max, Some(65535));
}

#[test]
fn parse_recipe_input_choices() {
    let rf = parse_recipe(recipe_with_inputs_yaml()).unwrap();
    let env = &rf.recipe.inputs["env"];
    assert_eq!(env.choices, vec!["dev", "staging", "production"]);
}

#[test]
fn parse_recipe_invalid_yaml() {
    let result = parse_recipe("{{{{not valid yaml");
    assert!(result.is_err());
}

// ============================================================================
// FJ-019: Input Validation — String
// ============================================================================

#[test]
fn validate_string_input() {
    let meta = make_meta(vec![("name", "string", None, None, None, &[])]);
    let mut provided = HashMap::new();
    provided.insert(
        "name".to_string(),
        serde_yaml_ng::Value::String("hello".into()),
    );
    let resolved = validate_inputs(&meta, &provided).unwrap();
    assert_eq!(resolved["name"], "hello");
}

// ============================================================================
// FJ-019: Input Validation — Int with Bounds
// ============================================================================

#[test]
fn validate_int_in_range() {
    let meta = make_meta(vec![("port", "int", None, Some(1), Some(65535), &[])]);
    let mut provided = HashMap::new();
    provided.insert(
        "port".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(8080)),
    );
    let resolved = validate_inputs(&meta, &provided).unwrap();
    assert_eq!(resolved["port"], "8080");
}

#[test]
fn validate_int_below_min() {
    let meta = make_meta(vec![("port", "int", None, Some(1), Some(65535), &[])]);
    let mut provided = HashMap::new();
    provided.insert(
        "port".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(0)),
    );
    let result = validate_inputs(&meta, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains(">= 1"));
}

#[test]
fn validate_int_above_max() {
    let meta = make_meta(vec![("port", "int", None, Some(1), Some(65535), &[])]);
    let mut provided = HashMap::new();
    provided.insert(
        "port".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(70000)),
    );
    let result = validate_inputs(&meta, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("<= 65535"));
}

#[test]
fn validate_int_at_boundary_min() {
    let meta = make_meta(vec![("x", "int", None, Some(0), Some(100), &[])]);
    let mut provided = HashMap::new();
    provided.insert(
        "x".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(0)),
    );
    assert!(validate_inputs(&meta, &provided).is_ok());
}

#[test]
fn validate_int_at_boundary_max() {
    let meta = make_meta(vec![("x", "int", None, Some(0), Some(100), &[])]);
    let mut provided = HashMap::new();
    provided.insert(
        "x".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(100)),
    );
    assert!(validate_inputs(&meta, &provided).is_ok());
}

#[test]
fn validate_int_wrong_type() {
    let meta = make_meta(vec![("port", "int", None, None, None, &[])]);
    let mut provided = HashMap::new();
    provided.insert(
        "port".to_string(),
        serde_yaml_ng::Value::String("not_a_number".into()),
    );
    assert!(validate_inputs(&meta, &provided).is_err());
}

// ============================================================================
// FJ-019: Input Validation — Bool
// ============================================================================

#[test]
fn validate_bool_true() {
    let meta = make_meta(vec![("enabled", "bool", None, None, None, &[])]);
    let mut provided = HashMap::new();
    provided.insert("enabled".to_string(), serde_yaml_ng::Value::Bool(true));
    let resolved = validate_inputs(&meta, &provided).unwrap();
    assert_eq!(resolved["enabled"], "true");
}

#[test]
fn validate_bool_wrong_type() {
    let meta = make_meta(vec![("enabled", "bool", None, None, None, &[])]);
    let mut provided = HashMap::new();
    provided.insert(
        "enabled".to_string(),
        serde_yaml_ng::Value::String("yes".into()),
    );
    assert!(validate_inputs(&meta, &provided).is_err());
}

// ============================================================================
// FJ-019: Input Validation — Path
// ============================================================================

#[test]
fn validate_path_absolute() {
    let meta = make_meta(vec![("dir", "path", None, None, None, &[])]);
    let mut provided = HashMap::new();
    provided.insert(
        "dir".to_string(),
        serde_yaml_ng::Value::String("/etc/nginx".into()),
    );
    let resolved = validate_inputs(&meta, &provided).unwrap();
    assert_eq!(resolved["dir"], "/etc/nginx");
}

#[test]
fn validate_path_relative_rejected() {
    let meta = make_meta(vec![("dir", "path", None, None, None, &[])]);
    let mut provided = HashMap::new();
    provided.insert(
        "dir".to_string(),
        serde_yaml_ng::Value::String("relative/path".into()),
    );
    assert!(validate_inputs(&meta, &provided).is_err());
}

// ============================================================================
// FJ-019: Input Validation — Enum
// ============================================================================

#[test]
fn validate_enum_valid_choice() {
    let meta = make_meta(vec![(
        "env",
        "enum",
        None,
        None,
        None,
        &["dev", "staging", "prod"],
    )]);
    let mut provided = HashMap::new();
    provided.insert(
        "env".to_string(),
        serde_yaml_ng::Value::String("staging".into()),
    );
    let resolved = validate_inputs(&meta, &provided).unwrap();
    assert_eq!(resolved["env"], "staging");
}

#[test]
fn validate_enum_invalid_choice() {
    let meta = make_meta(vec![(
        "env",
        "enum",
        None,
        None,
        None,
        &["dev", "staging", "prod"],
    )]);
    let mut provided = HashMap::new();
    provided.insert(
        "env".to_string(),
        serde_yaml_ng::Value::String("testing".into()),
    );
    let result = validate_inputs(&meta, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must be one of"));
}

// ============================================================================
// FJ-019: Input Validation — Defaults & Missing
// ============================================================================

#[test]
fn validate_uses_default() {
    let meta = make_meta(vec![(
        "port",
        "int",
        Some(serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(
            80,
        ))),
        None,
        None,
        &[],
    )]);
    let provided = HashMap::new();
    let resolved = validate_inputs(&meta, &provided).unwrap();
    assert_eq!(resolved["port"], "80");
}

#[test]
fn validate_missing_required_input() {
    let meta = make_meta(vec![("domain", "string", None, None, None, &[])]);
    let provided = HashMap::new();
    let result = validate_inputs(&meta, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires input"));
}

#[test]
fn validate_unknown_type() {
    let meta = make_meta(vec![("x", "float", None, None, None, &[])]);
    let mut provided = HashMap::new();
    provided.insert("x".to_string(), serde_yaml_ng::Value::String("1.5".into()));
    let result = validate_inputs(&meta, &provided);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown input type"));
}

// ============================================================================
// FJ-019: Recipe Expansion
// ============================================================================

#[test]
fn expand_recipe_namespaces_ids() {
    let rf = parse_recipe(minimal_recipe_yaml()).unwrap();
    let expanded = expand_recipe("my-web", &rf, &machine(), &HashMap::new(), &[]).unwrap();
    assert!(expanded.contains_key("my-web/web-server"));
}

#[test]
fn expand_recipe_with_template_substitution() {
    let rf = parse_recipe(recipe_with_inputs_yaml()).unwrap();
    let mut inputs = HashMap::new();
    inputs.insert(
        "port".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(8080)),
    );
    inputs.insert(
        "domain".to_string(),
        serde_yaml_ng::Value::String("example.com".into()),
    );
    let expanded = expand_recipe("web", &rf, &machine(), &inputs, &[]).unwrap();

    let config = &expanded["web/config"];
    assert!(config.path.as_ref().unwrap().contains("example.com.conf"));
    assert!(config.content.as_ref().unwrap().contains("8080"));
    assert!(config.content.as_ref().unwrap().contains("example.com"));
}

#[test]
fn expand_recipe_namespaces_depends_on() {
    let rf = parse_recipe(recipe_with_inputs_yaml()).unwrap();
    let mut inputs = HashMap::new();
    inputs.insert(
        "domain".to_string(),
        serde_yaml_ng::Value::String("x.com".into()),
    );
    let expanded = expand_recipe("web", &rf, &machine(), &inputs, &[]).unwrap();

    let config = &expanded["web/config"];
    assert!(config.depends_on.contains(&"web/pkg".to_string()));
}

#[test]
fn expand_recipe_external_deps() {
    let rf = parse_recipe(minimal_recipe_yaml()).unwrap();
    let expanded = expand_recipe(
        "r",
        &rf,
        &machine(),
        &HashMap::new(),
        &["other/resource".into()],
    )
    .unwrap();

    let first = expanded.values().next().unwrap();
    assert!(first.depends_on.contains(&"other/resource".to_string()));
}

// ============================================================================
// FJ-019: recipe_terminal_id
// ============================================================================

#[test]
fn terminal_id_single_resource() {
    let rf = parse_recipe(minimal_recipe_yaml()).unwrap();
    let id = recipe_terminal_id("my-recipe", &rf).unwrap();
    assert_eq!(id, "my-recipe/web-server");
}

#[test]
fn terminal_id_multi_resource() {
    let rf = parse_recipe(recipe_with_inputs_yaml()).unwrap();
    let id = recipe_terminal_id("web", &rf).unwrap();
    assert_eq!(id, "web/config"); // last resource in YAML
}

// ============================================================================
// Helper: Build RecipeMetadata programmatically
// ============================================================================

fn make_meta(
    inputs: Vec<(
        &str,
        &str,
        Option<serde_yaml_ng::Value>,
        Option<i64>,
        Option<i64>,
        &[&str],
    )>,
) -> RecipeMetadata {
    let mut input_map = IndexMap::new();
    for (name, itype, default, min, max, choices) in inputs {
        input_map.insert(
            name.to_string(),
            RecipeInput {
                input_type: itype.to_string(),
                description: None,
                default,
                min,
                max,
                choices: choices.iter().map(|s| s.to_string()).collect(),
            },
        );
    }
    RecipeMetadata {
        name: "test-recipe".into(),
        version: None,
        description: None,
        inputs: input_map,
        requires: vec![],
    }
}
