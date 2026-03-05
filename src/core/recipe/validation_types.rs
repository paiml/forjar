//! Per-type validators for recipe inputs (string, int, bool, path, enum).

use super::types::RecipeInput;

/// Validate a string input -- accepts any value, coerces non-strings via Debug.
fn validate_string(value: &serde_yaml_ng::Value) -> Result<String, String> {
    match value {
        serde_yaml_ng::Value::String(s) => Ok(s.clone()),
        other => Ok(format!("{other:?}")),
    }
}

/// Validate an integer input value against optional min/max bounds.
pub(crate) fn validate_int(
    name: &str,
    value: &serde_yaml_ng::Value,
    decl: &RecipeInput,
) -> Result<String, String> {
    let n = match value {
        serde_yaml_ng::Value::Number(n) => n
            .as_i64()
            .ok_or_else(|| format!("input '{name}' must be an integer"))?,
        _ => return Err(format!("input '{name}' must be an integer")),
    };
    if let Some(min) = decl.min {
        if n < min {
            return Err(format!("input '{name}' must be >= {min}"));
        }
    }
    if let Some(max) = decl.max {
        if n > max {
            return Err(format!("input '{name}' must be <= {max}"));
        }
    }
    Ok(n.to_string())
}

/// Validate a boolean input value.
fn validate_bool(name: &str, value: &serde_yaml_ng::Value) -> Result<String, String> {
    match value {
        serde_yaml_ng::Value::Bool(b) => Ok(b.to_string()),
        _ => Err(format!("input '{name}' must be a boolean")),
    }
}

/// Validate a path input value (must be an absolute path starting with `/`).
fn validate_path(name: &str, value: &serde_yaml_ng::Value) -> Result<String, String> {
    match value {
        serde_yaml_ng::Value::String(s) if s.starts_with('/') => Ok(s.clone()),
        serde_yaml_ng::Value::String(_) => Err(format!("input '{name}' must be an absolute path")),
        _ => Err(format!("input '{name}' must be a path string")),
    }
}

/// Validate an enum input value against allowed choices.
fn validate_enum(
    name: &str,
    value: &serde_yaml_ng::Value,
    decl: &RecipeInput,
) -> Result<String, String> {
    match value {
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
        _ => Err(format!("input '{name}' must be a string")),
    }
}

/// Dispatch to the appropriate type validator.
pub(crate) fn validate_input_type(
    name: &str,
    type_name: &str,
    value: &serde_yaml_ng::Value,
    decl: &RecipeInput,
) -> Result<String, String> {
    match type_name {
        "string" => validate_string(value),
        "int" => validate_int(name, value, decl),
        "bool" => validate_bool(name, value),
        "path" => validate_path(name, value),
        "enum" => validate_enum(name, value, decl),
        _ => Err(format!("unknown input type '{type_name}' for '{name}'")),
    }
}
