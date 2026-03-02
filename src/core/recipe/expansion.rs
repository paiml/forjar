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

/// Resolve `{{inputs.X}}` in an optional string field, returning None if the field is None.
fn resolve_opt(
    field: &Option<String>,
    inputs: &HashMap<String, String>,
) -> Result<Option<String>, String> {
    match field {
        Some(ref v) => Ok(Some(resolve_input_template(v, inputs)?)),
        None => Ok(None),
    }
}

/// Resolve `{{inputs.X}}` in each element of a Vec<String>.
fn resolve_vec(fields: &[String], inputs: &HashMap<String, String>) -> Result<Vec<String>, String> {
    fields
        .iter()
        .map(|v| resolve_input_template(v, inputs))
        .collect()
}

/// Resolve input templates in all string fields of a resource.
pub(crate) fn resolve_resource_inputs(
    resource: &Resource,
    inputs: &HashMap<String, String>,
) -> Result<Resource, String> {
    let mut r = resource.clone();

    // File/path fields
    r.path = resolve_opt(&r.path, inputs)?;
    r.content = resolve_opt(&r.content, inputs)?;
    r.source = resolve_opt(&r.source, inputs)?;
    r.target = resolve_opt(&r.target, inputs)?;
    r.owner = resolve_opt(&r.owner, inputs)?;
    r.group = resolve_opt(&r.group, inputs)?;
    r.mode = resolve_opt(&r.mode, inputs)?;
    r.options = resolve_opt(&r.options, inputs)?;

    // Service/naming fields
    r.name = resolve_opt(&r.name, inputs)?;
    r.image = resolve_opt(&r.image, inputs)?;
    r.restart = resolve_opt(&r.restart, inputs)?;
    r.command = resolve_opt(&r.command, inputs)?;
    r.schedule = resolve_opt(&r.schedule, inputs)?;

    // Network fields
    r.protocol = resolve_opt(&r.protocol, inputs)?;
    r.port = resolve_opt(&r.port, inputs)?;
    r.action = resolve_opt(&r.action, inputs)?;
    r.from_addr = resolve_opt(&r.from_addr, inputs)?;

    // GPU fields
    r.gpu_backend = resolve_opt(&r.gpu_backend, inputs)?;
    r.driver_version = resolve_opt(&r.driver_version, inputs)?;
    r.cuda_version = resolve_opt(&r.cuda_version, inputs)?;
    r.rocm_version = resolve_opt(&r.rocm_version, inputs)?;
    r.compute_mode = resolve_opt(&r.compute_mode, inputs)?;

    // Model fields
    r.format = resolve_opt(&r.format, inputs)?;
    r.quantization = resolve_opt(&r.quantization, inputs)?;
    r.checksum = resolve_opt(&r.checksum, inputs)?;
    r.cache_dir = resolve_opt(&r.cache_dir, inputs)?;

    // Package fields
    r.provider = resolve_opt(&r.provider, inputs)?;
    r.version = resolve_opt(&r.version, inputs)?;

    // Conditional execution
    r.when = resolve_opt(&r.when, inputs)?;

    // Lifecycle hooks
    r.pre_apply = resolve_opt(&r.pre_apply, inputs)?;
    r.post_apply = resolve_opt(&r.post_apply, inputs)?;

    // Store / derivation fields
    r.script = resolve_opt(&r.script, inputs)?;

    // Docker fields (Vec<String>)
    r.ports = resolve_vec(&r.ports, inputs)?;
    r.environment = resolve_vec(&r.environment, inputs)?;
    r.volumes = resolve_vec(&r.volumes, inputs)?;

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
