//! FJ-019: Recipe loading, input validation, and expansion into resources.
//!
//! Recipes are reusable, parameterized infrastructure patterns. A recipe
//! declares typed inputs and a set of resources. When instantiated, the
//! recipe's resources are expanded into the main config with namespaced IDs
//! (e.g., `my-recipe/resource-name`).

use super::types::{MachineTarget, Resource};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A recipe source — where to load recipes from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RecipeSource {
    Local {
        path: String,
    },
    Git {
        git: String,
        #[serde(default)]
        r#ref: Option<String>,
        #[serde(default)]
        path: Option<String>,
    },
}

/// A recipe file — declares inputs and resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeFile {
    pub recipe: RecipeMetadata,
    pub resources: IndexMap<String, Resource>,
}

/// Recipe metadata and input declarations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeMetadata {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub inputs: IndexMap<String, RecipeInput>,
    #[serde(default)]
    pub requires: Vec<RecipeRequirement>,
}

/// A recipe input declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeInput {
    #[serde(rename = "type")]
    pub input_type: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default: Option<serde_yaml_ng::Value>,
    #[serde(default)]
    pub min: Option<i64>,
    #[serde(default)]
    pub max: Option<i64>,
    #[serde(default)]
    pub choices: Vec<String>,
}

/// A recipe dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeRequirement {
    pub recipe: String,
}

/// Load a recipe from a YAML file.
pub fn load_recipe(path: &Path) -> Result<RecipeFile, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read recipe {}: {}", path.display(), e))?;
    parse_recipe(&content)
}

/// Parse a recipe from a YAML string.
pub fn parse_recipe(yaml: &str) -> Result<RecipeFile, String> {
    serde_yaml_ng::from_str(yaml).map_err(|e| format!("recipe parse error: {}", e))
}

/// Validate recipe inputs against their declarations.
pub fn validate_inputs(
    recipe: &RecipeMetadata,
    provided: &HashMap<String, serde_yaml_ng::Value>,
) -> Result<HashMap<String, String>, String> {
    let mut resolved = HashMap::new();

    for (name, decl) in &recipe.inputs {
        let value = if let Some(v) = provided.get(name) {
            v.clone()
        } else if let Some(ref default) = decl.default {
            default.clone()
        } else {
            return Err(format!(
                "recipe '{}' requires input '{}' (type: {})",
                recipe.name, name, decl.input_type
            ));
        };

        let string_val = validate_input_type(name, &decl.input_type, &value, decl)?;
        resolved.insert(name.clone(), string_val);
    }

    Ok(resolved)
}

/// Validate a single input value against its declared type.
fn validate_input_type(
    name: &str,
    type_name: &str,
    value: &serde_yaml_ng::Value,
    decl: &RecipeInput,
) -> Result<String, String> {
    match type_name {
        "string" => match value {
            serde_yaml_ng::Value::String(s) => Ok(s.clone()),
            other => Ok(format!("{:?}", other)),
        },
        "int" => {
            let n = match value {
                serde_yaml_ng::Value::Number(n) => n
                    .as_i64()
                    .ok_or_else(|| format!("input '{}' must be an integer", name))?,
                _ => return Err(format!("input '{}' must be an integer", name)),
            };
            if let Some(min) = decl.min {
                if n < min {
                    return Err(format!("input '{}' must be >= {}", name, min));
                }
            }
            if let Some(max) = decl.max {
                if n > max {
                    return Err(format!("input '{}' must be <= {}", name, max));
                }
            }
            Ok(n.to_string())
        }
        "bool" => match value {
            serde_yaml_ng::Value::Bool(b) => Ok(b.to_string()),
            _ => Err(format!("input '{}' must be a boolean", name)),
        },
        "path" => match value {
            serde_yaml_ng::Value::String(s) => {
                if !s.starts_with('/') {
                    return Err(format!("input '{}' must be an absolute path", name));
                }
                Ok(s.clone())
            }
            _ => Err(format!("input '{}' must be a path string", name)),
        },
        "enum" => match value {
            serde_yaml_ng::Value::String(s) => {
                if !decl.choices.is_empty() && !decl.choices.contains(s) {
                    return Err(format!(
                        "input '{}' must be one of: {}",
                        name,
                        decl.choices.join(", ")
                    ));
                }
                Ok(s.clone())
            }
            _ => Err(format!("input '{}' must be a string", name)),
        },
        _ => Err(format!("unknown input type '{}' for '{}'", type_name, name)),
    }
}

/// Resolve `{{inputs.X}}` templates in a string.
fn resolve_input_template(
    template: &str,
    inputs: &HashMap<String, String>,
) -> Result<String, String> {
    let mut result = template.to_string();
    let mut start = 0;

    while let Some(open) = result[start..].find("{{inputs.") {
        let open = start + open;
        let close = result[open..]
            .find("}}")
            .ok_or_else(|| format!("unclosed template at position {}", open))?;
        let close = open + close + 2;
        let key = result[open + 9..close - 2].trim();

        let value = inputs
            .get(key)
            .ok_or_else(|| format!("unknown input: {}", key))?;

        result.replace_range(open..close, value);
        start = open + value.len();
    }

    Ok(result)
}

/// Resolve input templates in all string fields of a resource.
fn resolve_resource_inputs(
    resource: &Resource,
    inputs: &HashMap<String, String>,
) -> Result<Resource, String> {
    let mut r = resource.clone();

    if let Some(ref path) = r.path {
        r.path = Some(resolve_input_template(path, inputs)?);
    }
    if let Some(ref content) = r.content {
        r.content = Some(resolve_input_template(content, inputs)?);
    }
    if let Some(ref source) = r.source {
        r.source = Some(resolve_input_template(source, inputs)?);
    }
    if let Some(ref target) = r.target {
        r.target = Some(resolve_input_template(target, inputs)?);
    }
    if let Some(ref options) = r.options {
        r.options = Some(resolve_input_template(options, inputs)?);
    }

    Ok(r)
}

/// Expand a recipe instance into namespaced resources.
///
/// Given a recipe resource in the config (type: recipe), load and expand it
/// into individual resources with IDs like `recipe-id/resource-name`.
pub fn expand_recipe(
    recipe_id: &str,
    recipe_file: &RecipeFile,
    machine: &MachineTarget,
    provided_inputs: &HashMap<String, serde_yaml_ng::Value>,
    external_depends_on: &[String],
) -> Result<IndexMap<String, Resource>, String> {
    // Validate inputs
    let resolved_inputs = validate_inputs(&recipe_file.recipe, provided_inputs)?;

    let mut expanded = IndexMap::new();
    let mut first = true;

    for (res_name, resource) in &recipe_file.resources {
        let namespaced_id = format!("{}/{}", recipe_id, res_name);

        // Resolve input templates
        let mut resolved = resolve_resource_inputs(resource, &resolved_inputs)?;

        // Propagate machine target
        resolved.machine = machine.clone();

        // Namespace internal depends_on references
        let mut new_deps: Vec<String> = resolved
            .depends_on
            .iter()
            .map(|dep| {
                if recipe_file.resources.contains_key(dep) {
                    format!("{}/{}", recipe_id, dep)
                } else {
                    dep.clone()
                }
            })
            .collect();

        // First resource in recipe gets external dependencies
        if first && !external_depends_on.is_empty() {
            new_deps.extend(external_depends_on.iter().cloned());
            first = false;
        }

        resolved.depends_on = new_deps;

        // Namespace restart_on references
        resolved.restart_on = resolved
            .restart_on
            .iter()
            .map(|dep| {
                if recipe_file.resources.contains_key(dep) {
                    format!("{}/{}", recipe_id, dep)
                } else {
                    dep.clone()
                }
            })
            .collect();

        expanded.insert(namespaced_id, resolved);
    }

    Ok(expanded)
}

/// Get the last resource ID in a recipe expansion (for external depends_on).
pub fn recipe_terminal_id(recipe_id: &str, recipe_file: &RecipeFile) -> Option<String> {
    recipe_file
        .resources
        .keys()
        .last()
        .map(|name| format!("{}/{}", recipe_id, name))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_fj019_validate_inputs_ok() {
        let recipe = parse_recipe(RECIPE_YAML).unwrap();
        let mut provided = HashMap::new();
        provided.insert(
            "export_path".to_string(),
            serde_yaml_ng::Value::String("/mnt/data".to_string()),
        );
        let resolved = validate_inputs(&recipe.recipe, &provided).unwrap();
        assert_eq!(resolved["export_path"], "/mnt/data");
        assert_eq!(resolved["network"], "192.168.50.0/24"); // default
        assert_eq!(resolved["port"], "2049"); // default
    }

    #[test]
    fn test_fj019_validate_inputs_missing_required() {
        let recipe = parse_recipe(RECIPE_YAML).unwrap();
        let provided = HashMap::new(); // no export_path
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

        // Check input resolution
        let exports = &expanded["nfs/exports"];
        assert!(exports.content.as_ref().unwrap().contains("/mnt/raid"));
        assert!(exports
            .content
            .as_ref()
            .unwrap()
            .contains("192.168.50.0/24"));

        // Check namespaced depends_on
        assert!(exports.depends_on.contains(&"nfs/packages".to_string()));

        // Check machine propagation
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

        // First resource should have external dependency
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
    /// When external_depends_on is empty, no resource should get external deps.
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

        // With empty external deps, first resource should only have its own deps
        let first = &expanded["nfs/packages"];
        assert!(
            first.depends_on.is_empty(),
            "first resource should have no deps when external_depends_on is empty"
        );
    }

    /// BH-MUT-0002: Kills mutation that would inject external deps into non-first resources.
    /// Only the first resource in a recipe should get external dependencies.
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

        // First resource gets external dep
        let first = &expanded["nfs/packages"];
        assert!(first.depends_on.contains(&"base-pkg".to_string()));

        // Second resource should NOT have external dep (only its namespaced internal dep)
        let second = &expanded["nfs/exports"];
        assert!(
            !second.depends_on.contains(&"base-pkg".to_string()),
            "non-first resource should not get external dependencies"
        );

        // Third resource should NOT have external dep either
        let third = &expanded["nfs/service"];
        assert!(
            !third.depends_on.contains(&"base-pkg".to_string()),
            "non-first resource should not get external dependencies"
        );
    }
}
