//! FJ-371: `forjar template` — expand a template to stdout without applying.
//!
//! # Why this file exists (Refs #211, #213)
//!
//! `template` used to read the file and print it back. The only expansion it
//! performed was a literal `{{inputs.KEY}}` → value substitution for keys that
//! `-V` supplied, so:
//!
//! * a recipe's declared `default:` was never applied — with no `-V`, output
//!   still contained `{{inputs.greeting}}`;
//! * `{{params.X}}` was left intact even though `forjar show`, on the same
//!   bytes, resolves it;
//! * an undeclared input was neither resolved nor reported;
//! * `-V novaluehere` (no `=`) was accepted and silently discarded.
//!
//! Printing the input verbatim and exiting 0 is the worst available answer: it
//! is indistinguishable from a successful expansion.
//!
//! The command now dispatches on what the file actually is, and both paths run
//! the engine that already exists rather than a private mini-substituter:
//!
//! * a RECIPE (`recipe:` + `resources:`) goes through
//!   `recipe::expand_recipe`, so declared defaults apply, types are checked
//!   and an unknown/missing input is an error;
//! * a CONFIG (a `forjar.yaml`) goes through `resolver::resolve_template`, the
//!   same resolver `show` uses, with `-V` merged over `params:`.
//!
//! Anything else is refused by name instead of echoed.

use crate::core::recipe::{expand_recipe, parse_recipe};
use crate::core::resolver::resolve_template;
use crate::core::types::{ForjarConfig, MachineTarget};
use std::collections::HashMap;
use std::path::Path;

/// FJ-371: Expand a recipe or config template to stdout without applying.
pub(crate) fn cmd_template(recipe: &Path, vars: &[String], json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(recipe)
        .map_err(|e| format!("cannot read recipe {}: {}", recipe.display(), e))?;
    let var_map = parse_vars(vars)?;

    let expanded = match classify(&content) {
        Kind::Recipe => expand_recipe_file(&content, &var_map)?,
        Kind::Config => expand_config_file(&content, &var_map)?,
        Kind::Unknown => {
            return Err(format!(
                "{} is neither a recipe (needs a top-level `recipe:` block) nor a \
                 forjar config (needs `resources:`) — nothing to expand",
                recipe.display()
            ))
        }
    };

    if json {
        println!(
            "{}",
            serde_json::json!({
                "recipe": recipe.display().to_string(),
                "vars": var_map,
                "expanded": expanded,
            })
        );
    } else {
        print!("{expanded}");
        if !expanded.ends_with('\n') {
            println!();
        }
    }
    Ok(())
}

/// What kind of document `template` was handed.
enum Kind {
    /// A recipe file: `recipe:` metadata plus `resources:`.
    Recipe,
    /// A forjar config.
    Config,
    /// Neither.
    Unknown,
}

fn classify(content: &str) -> Kind {
    let Ok(value) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(content) else {
        return Kind::Unknown;
    };
    let Some(map) = value.as_mapping() else {
        return Kind::Unknown;
    };
    let has = |k: &str| map.contains_key(serde_yaml_ng::Value::String(k.to_string()));
    if has("recipe") {
        Kind::Recipe
    } else if has("resources") {
        Kind::Config
    } else {
        Kind::Unknown
    }
}

/// `KEY=VALUE` pairs into a map. The `=` is already enforced by the clap value
/// parser; this re-checks so the library entry point cannot be fed junk either.
fn parse_vars(vars: &[String]) -> Result<HashMap<String, String>, String> {
    let mut map = HashMap::new();
    for v in vars {
        let (key, val) = v
            .split_once('=')
            .ok_or_else(|| format!("--var expects KEY=VALUE, got '{v}' (no '=' found)"))?;
        if key.trim().is_empty() {
            return Err(format!("--var expects KEY=VALUE, got '{v}' (empty key)"));
        }
        map.insert(key.to_string(), val.to_string());
    }
    Ok(map)
}

/// Expand a recipe: declared defaults apply, unknown inputs are rejected.
fn expand_recipe_file(content: &str, vars: &HashMap<String, String>) -> Result<String, String> {
    let recipe = parse_recipe(content)?;
    // Refs #213: `validate_inputs` only checks the inputs a recipe DECLARES —
    // an extra one is dropped. Silently discarding `-V typo=…` is the same
    // failure as accepting `-V novaluehere`, so name it here.
    for key in vars.keys() {
        if !recipe.recipe.inputs.contains_key(key) {
            let declared: Vec<&str> = recipe.recipe.inputs.keys().map(String::as_str).collect();
            return Err(format!(
                "recipe '{}' declares no input '{key}' (declared: {})",
                recipe.recipe.name,
                if declared.is_empty() {
                    "none".to_string()
                } else {
                    declared.join(", ")
                }
            ));
        }
    }
    let inputs: HashMap<String, serde_yaml_ng::Value> = vars
        .iter()
        .map(|(k, v)| (k.clone(), yaml_scalar(v)))
        .collect();
    let expanded = expand_recipe(
        &recipe.recipe.name,
        &recipe,
        &MachineTarget::default(),
        &inputs,
        &[],
    )?;
    serde_yaml_ng::to_string(&serde_json::json!({ "resources": expanded }))
        .map_err(|e| format!("serialize expanded recipe: {e}"))
}

/// Expand a config: `{{params.X}}` / `{{machine.X.field}}` / functions, with
/// `-V` merged over the config's own `params:`.
fn expand_config_file(content: &str, vars: &HashMap<String, String>) -> Result<String, String> {
    let config: ForjarConfig =
        serde_yaml_ng::from_str(content).map_err(|e| format!("YAML parse error: {e}"))?;
    let mut params = config.params.clone();
    for (k, v) in vars {
        params.insert(k.clone(), yaml_scalar(v));
    }
    resolve_template(content, &params, &config.machines)
}

/// Parse a `-V` value the way YAML would, so `-V port=8080` is an int and
/// `-V debug=true` is a bool — matching how the same key reads from `params:`.
fn yaml_scalar(raw: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::from_str::<serde_yaml_ng::Value>(raw)
        .unwrap_or_else(|_| serde_yaml_ng::Value::String(raw.to_string()))
}

#[cfg(test)]
#[path = "tests_template_cmd.rs"]
mod tests;
