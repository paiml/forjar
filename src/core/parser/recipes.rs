//! Recipe expansion: replace recipe-type resources with their expanded resources.

use super::*;
use std::path::Path;

/// Expand recipe resources into their constituent resources.
/// Recipe resources (type: recipe) are replaced with the expanded resources
/// from the referenced recipe file.
pub fn expand_recipes(config: &mut ForjarConfig, config_dir: Option<&Path>) -> Result<(), String> {
    let base_dir = config_dir.unwrap_or_else(|| Path::new("."));
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
    Ok(())
}
