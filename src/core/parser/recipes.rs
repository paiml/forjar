//! Recipe expansion: replace recipe-type resources with their expanded resources.

use super::*;
use std::collections::HashMap;
use std::path::Path;

/// Maximum sub-recipe nesting depth before we bail out.
const MAX_RECIPE_DEPTH: usize = 16;

/// Expand recipe resources into their constituent resources.
///
/// Runs expansion passes in a loop until no `ResourceType::Recipe` entries
/// remain. Each pass expands one level of nesting. Cycle detection walks
/// each resource ID's ancestor chain — if the same recipe name appears in
/// a resource's ancestry, that's a cycle.
pub fn expand_recipes(config: &mut ForjarConfig, config_dir: Option<&Path>) -> Result<(), String> {
    let base_dir = config_dir.unwrap_or_else(|| Path::new("."));
    let mut expansion_map: HashMap<String, String> = HashMap::new();
    let mut recipe_versions: HashMap<String, String> = HashMap::new();

    for depth in 0..MAX_RECIPE_DEPTH {
        let has_recipes = config
            .resources
            .values()
            .any(|r| r.resource_type == ResourceType::Recipe);
        if !has_recipes {
            return Ok(());
        }

        let terminal_map = build_terminal_map(config, base_dir)?;
        let expanded = expand_one_level(
            config,
            base_dir,
            &terminal_map,
            &mut expansion_map,
            &mut recipe_versions,
        )?;
        config.resources = expanded;

        if depth == MAX_RECIPE_DEPTH - 1 {
            check_expansion_complete(config)?;
        }
    }

    Ok(())
}

/// Build map of recipe resource ID → terminal (last) expanded resource ID.
fn build_terminal_map(
    config: &ForjarConfig,
    base_dir: &Path,
) -> Result<HashMap<String, String>, String> {
    let mut terminal_map = HashMap::new();
    for (id, resource) in &config.resources {
        if resource.resource_type != ResourceType::Recipe {
            continue;
        }
        let recipe_name = resource
            .recipe
            .as_deref()
            .ok_or_else(|| format!("recipe resource '{id}' has no recipe name"))?;
        let recipe_path = base_dir.join("recipes").join(format!("{recipe_name}.yaml"));
        if recipe_path.exists() {
            if let Ok(recipe_file) = recipe::load_recipe(&recipe_path) {
                if let Some(terminal) = recipe::recipe_terminal_id(id, &recipe_file) {
                    terminal_map.insert(id.clone(), terminal);
                }
            }
        }
    }
    Ok(terminal_map)
}

/// Expand one level of recipe nesting, producing a new resource map.
fn expand_one_level(
    config: &ForjarConfig,
    base_dir: &Path,
    terminal_map: &HashMap<String, String>,
    expansion_map: &mut HashMap<String, String>,
    recipe_versions: &mut HashMap<String, String>,
) -> Result<indexmap::IndexMap<String, Resource>, String> {
    let mut expanded = indexmap::IndexMap::new();

    for (id, resource) in &config.resources {
        if resource.resource_type != ResourceType::Recipe {
            expanded.insert(id.clone(), resource.clone());
            continue;
        }

        let recipe_name = resource
            .recipe
            .as_deref()
            .ok_or_else(|| format!("recipe resource '{id}' has no recipe name"))?;

        detect_cycle(id, recipe_name, expansion_map)?;
        expansion_map.insert(id.clone(), recipe_name.to_string());

        let resolved_deps = resolve_recipe_deps(&resource.depends_on, terminal_map);

        let recipe_path = base_dir.join("recipes").join(format!("{recipe_name}.yaml"));
        if !recipe_path.exists() {
            return Err(format!(
                "recipe '{}' not found at {}",
                recipe_name,
                recipe_path.display()
            ));
        }

        let recipe_file = recipe::load_recipe(&recipe_path)?;
        check_version_conflict(recipe_name, &recipe_file, recipe_versions, id)?;

        let expanded_resources = recipe::expand_recipe(
            id,
            &recipe_file,
            &resource.machine,
            &resource.inputs,
            &resolved_deps,
        )?;

        for (res_id, res) in expanded_resources {
            expanded.insert(res_id, res);
        }
    }

    Ok(expanded)
}

/// Cycle detection: walk ancestor chain looking for same recipe name.
fn detect_cycle(
    id: &str,
    recipe_name: &str,
    expansion_map: &HashMap<String, String>,
) -> Result<(), String> {
    let mut ancestor = id;
    while let Some(slash_pos) = ancestor.rfind('/') {
        ancestor = &ancestor[..slash_pos];
        if expansion_map.get(ancestor).map(|s| s.as_str()) == Some(recipe_name) {
            return Err(format!(
                "recipe cycle detected: '{recipe_name}' at resource '{id}'"
            ));
        }
    }
    Ok(())
}

/// Rewrite depends_on: replace recipe IDs with their terminal resource.
fn resolve_recipe_deps(deps: &[String], terminal_map: &HashMap<String, String>) -> Vec<String> {
    deps.iter()
        .map(|dep| {
            terminal_map
                .get(dep.as_str())
                .cloned()
                .unwrap_or_else(|| dep.clone())
        })
        .collect()
}

/// FJ-1392: Version conflict detection — same recipe at different versions.
fn check_version_conflict(
    recipe_name: &str,
    recipe_file: &recipe::RecipeFile,
    recipe_versions: &mut HashMap<String, String>,
    resource_id: &str,
) -> Result<(), String> {
    if let Some(ref ver) = recipe_file.recipe.version {
        if let Some(existing_ver) = recipe_versions.get(recipe_name) {
            if existing_ver != ver {
                return Err(format!(
                    "recipe version conflict: '{recipe_name}' required at v{existing_ver} and v{ver} (resource '{resource_id}')"
                ));
            }
        } else {
            recipe_versions.insert(recipe_name.to_string(), ver.clone());
        }
    }
    Ok(())
}

/// Check that no recipe resources remain after expansion.
fn check_expansion_complete(config: &ForjarConfig) -> Result<(), String> {
    let still_has = config
        .resources
        .values()
        .any(|r| r.resource_type == ResourceType::Recipe);
    if still_has {
        return Err(format!(
            "recipe expansion exceeded max depth of {MAX_RECIPE_DEPTH}"
        ));
    }
    Ok(())
}
