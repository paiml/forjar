//! Recipe loading, template resolution, and expansion into resources.

use super::super::types::{MachineTarget, Resource};
use super::types::RecipeFile;
use super::validation::validate_inputs;
use indexmap::IndexMap;
use provable_contracts_macros::contract;
use std::collections::HashMap;
use std::path::Path;

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

/// Resolve `{{inputs.X}}` templates in a string.
pub(crate) fn resolve_input_template(
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
pub(crate) fn resolve_resource_inputs(
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
#[contract("recipe-determinism-v1", equation = "expand_recipe")]
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
