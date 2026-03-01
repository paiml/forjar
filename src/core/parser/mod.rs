//! FJ-002: YAML parsing and validation.
//!
//! Parses forjar.yaml and validates structural constraints:
//! - Version must be "1.0"
//! - Machine references in resources must exist
//! - depends_on references must exist
//! - Required fields per resource type

mod expansion;
mod includes;
mod policy;
mod recipes;
mod resource_types;
mod validation;

#[cfg(test)]
mod tests_core;
#[cfg(test)]
mod tests_validation;
#[cfg(test)]
mod tests_arch;
#[cfg(test)]
mod tests_expansion;
#[cfg(test)]
mod tests_policy;
#[cfg(test)]
mod tests_triggers;
#[cfg(test)]
mod tests_includes;
#[cfg(test)]
mod tests_misc;
#[cfg(test)]
mod tests_misc_2;
#[cfg(test)]
mod tests_misc_2b;
#[cfg(test)]
mod tests_misc_3;
#[cfg(test)]
mod tests_misc_4;

use super::recipe;
use super::types::*;
use std::path::Path;

// Re-export public API
pub use expansion::expand_resources;
pub use policy::evaluate_policies;
pub use recipes::expand_recipes;

/// Recognized CPU architectures for the `arch` field.
const KNOWN_ARCHITECTURES: &[&str] =
    &["x86_64", "aarch64", "armv7l", "riscv64", "s390x", "ppc64le"];

/// Validation error.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Parse a forjar.yaml file from disk.
pub fn parse_config_file(path: &Path) -> Result<ForjarConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    parse_config(&content)
}

/// Parse a forjar.yaml from a string.
pub fn parse_config(yaml: &str) -> Result<ForjarConfig, String> {
    serde_yaml_ng::from_str(yaml).map_err(|e| format!("YAML parse error: {}", e))
}

/// Validate a parsed config. Returns a list of errors (empty = valid).
pub fn validate_config(config: &ForjarConfig) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if config.version != "1.0" {
        errors.push(ValidationError {
            message: format!("version must be \"1.0\", got \"{}\"", config.version),
        });
    }

    if config.name.is_empty() {
        errors.push(ValidationError {
            message: "name must not be empty".to_string(),
        });
    }

    for (id, resource) in &config.resources {
        validation::validate_resource_refs(config, id, resource, &mut errors);
        resource_types::validate_resource_type(id, resource, &mut errors);
    }

    for (key, machine) in &config.machines {
        validation::validate_machine(key, machine, &mut errors);
    }

    errors
}

/// Parse, validate, and expand recipes in a config file.
/// This is the main entry point for loading a config for plan/apply.
pub fn parse_and_validate(path: &Path) -> Result<ForjarConfig, String> {
    let mut config = parse_config_file(path)?;

    // FJ-254: Process includes before validation
    if !config.includes.is_empty() {
        let base_dir = path.parent().unwrap_or(Path::new("."));
        config = includes::merge_includes(config, base_dir)?;
    }

    let errors = validate_config(&config);
    if !errors.is_empty() {
        return Err(format!(
            "validation errors:\n{}",
            errors
                .iter()
                .map(|e| format!("  - {}", e))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    expand_recipes(&mut config, path.parent())?;
    expand_resources(&mut config);
    Ok(config)
}
