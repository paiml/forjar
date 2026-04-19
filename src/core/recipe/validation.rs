//! Recipe input validation -- type checking, bounds, and enum constraints.

use provable_contracts_macros::contract;
use std::collections::HashMap;

use super::types::RecipeMetadata;
pub(crate) use super::validation_types::validate_input_type;
#[cfg(test)]
pub(crate) use super::validation_types::validate_int;

/// Validate recipe inputs against their declarations.
///
/// Recipes without any declared inputs short-circuit to an empty map — the
/// STRONG `recipe-determinism-v1` `validate_inputs` precondition
/// (`inputs.len() > 0`) only applies when there is something to validate.
#[contract("recipe-determinism-v1", equation = "validate_inputs")]
pub fn validate_inputs(
    recipe: &RecipeMetadata,
    provided: &HashMap<String, serde_yaml_ng::Value>,
) -> Result<HashMap<String, String>, String> {
    // FJ-019: A recipe with no declared inputs is a legitimate case (e.g. a
    // recipe that just installs packages). Short-circuit before the contract
    // precondition fires so we uphold `inputs.len() > 0` (the contract applies
    // only when there is actual validation work to do).
    if recipe.inputs.is_empty() {
        let _ = provided; // explicitly acknowledge unused when empty
        return Ok(HashMap::new());
    }

    // Contract: recipe-determinism-v1.yaml precondition (pv codegen)
    contract_pre_validate_inputs!(recipe.inputs);
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
