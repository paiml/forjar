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
    // Maps resource_id → recipe_name for each expansion performed
    let mut expansion_map: HashMap<String, String> = HashMap::new();

    for depth in 0..MAX_RECIPE_DEPTH {
        // Check if any recipe resources remain
        let has_recipes = config
            .resources
            .values()
            .any(|r| r.resource_type == ResourceType::Recipe);
        if !has_recipes {
            return Ok(());
        }

        let mut expanded = indexmap::IndexMap::new();

        for (id, resource) in &config.resources {
            if resource.resource_type != ResourceType::Recipe {
                expanded.insert(id.clone(), resource.clone());
                continue;
            }

            let recipe_name = resource
                .recipe
                .as_deref()
                .ok_or_else(|| format!("recipe resource '{}' has no recipe name", id))?;

            // Cycle detection: walk ancestor chain looking for same recipe name
            let mut ancestor = id.as_str();
            while let Some(slash_pos) = ancestor.rfind('/') {
                ancestor = &ancestor[..slash_pos];
                if expansion_map.get(ancestor).map(|s| s.as_str()) == Some(recipe_name) {
                    return Err(format!(
                        "recipe cycle detected: '{}' at resource '{}'",
                        recipe_name, id
                    ));
                }
            }

            expansion_map.insert(id.clone(), recipe_name.to_string());

            // Look for recipe file relative to config directory
            let recipe_path = base_dir
                .join("recipes")
                .join(format!("{}.yaml", recipe_name));
            if !recipe_path.exists() {
                return Err(format!(
                    "recipe '{}' not found at {}",
                    recipe_name,
                    recipe_path.display()
                ));
            }

            let recipe_file = recipe::load_recipe(&recipe_path)?;
            let expanded_resources = recipe::expand_recipe(
                id,
                &recipe_file,
                &resource.machine,
                &resource.inputs,
                &resource.depends_on,
            )?;

            for (res_id, res) in expanded_resources {
                expanded.insert(res_id, res);
            }
        }

        config.resources = expanded;

        // Safety: if we've hit the depth cap, check once more
        if depth == MAX_RECIPE_DEPTH - 1 {
            let still_has = config
                .resources
                .values()
                .any(|r| r.resource_type == ResourceType::Recipe);
            if still_has {
                return Err(format!(
                    "recipe expansion exceeded max depth of {}",
                    MAX_RECIPE_DEPTH
                ));
            }
        }
    }

    Ok(())
}
