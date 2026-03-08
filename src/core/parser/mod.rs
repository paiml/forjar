//! FJ-002: YAML parsing and validation.
//!
//! Parses forjar.yaml and validates structural constraints:
//! - Version must be "1.0"
//! - Machine references in resources must exist
//! - depends_on references must exist
//! - Required fields per resource type

mod expansion;
mod format_validation;
mod includes;
mod policy;
mod recipes;
mod resource_types;
pub(crate) mod unknown_fields;
mod validation;

#[cfg(test)]
mod tests_arch;
#[cfg(test)]
mod tests_core;
#[cfg(test)]
mod tests_expansion;
#[cfg(test)]
mod tests_format_validation;
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
#[cfg(test)]
mod tests_policy;
#[cfg(test)]
mod tests_sudo_inference;
#[cfg(test)]
mod tests_triggers;
#[cfg(test)]
mod tests_unknown_fields;
#[cfg(test)]
mod tests_validation;

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
    /// Human-readable error description.
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
///
/// # Examples
///
/// ```
/// use forjar::core::parser::parse_config;
///
/// let yaml = r#"
/// version: "1.0"
/// name: my-stack
/// resources:
///   pkg-curl:
///     type: package
///     packages: [curl]
/// "#;
/// let config = parse_config(yaml).expect("valid");
/// assert_eq!(config.name, "my-stack");
/// assert!(config.resources.contains_key("pkg-curl"));
/// ```
pub fn parse_config(yaml: &str) -> Result<ForjarConfig, String> {
    serde_yaml_ng::from_str(yaml).map_err(|e| format!("YAML parse error: {e}"))
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
        check_sudo_inference(id, resource, config, &mut errors);
    }

    for (key, machine) in &config.machines {
        validation::validate_machine(key, machine, &mut errors);
    }

    // FJ-2501: Format validation (mode, port, path, owner/group, addr)
    errors.extend(format_validation::validate_formats(config));

    errors
}

/// Paths that require root/sudo for writes.
const PRIVILEGED_PREFIXES: &[&str] = &[
    "/etc/",
    "/usr/lib/systemd/",
    "/boot/",
    "/var/lib/",
    "/opt/",
    "/usr/local/bin/",
    "/usr/local/sbin/",
];

/// Warn when a file resource writes to a privileged path or has owner:root without sudo.
fn check_sudo_inference(
    id: &str,
    resource: &Resource,
    config: &ForjarConfig,
    errors: &mut Vec<ValidationError>,
) {
    if resource.sudo {
        return; // Already has sudo
    }
    if resource.resource_type != ResourceType::File {
        return; // Only applies to file resources
    }
    // Check if the machine is local with user=root — sudo not needed
    for machine_name in resource.machine.to_vec() {
        if let Some(machine) = config.machines.get(&machine_name) {
            if machine.user == "root" {
                return; // Running as root, sudo not needed
            }
        }
    }
    let needs_sudo = resource.owner.as_deref() == Some("root")
        || resource
            .path
            .as_deref()
            .is_some_and(|p| PRIVILEGED_PREFIXES.iter().any(|pfx| p.starts_with(pfx)));
    if needs_sudo {
        let reason = if resource.owner.as_deref() == Some("root") {
            "owner: root"
        } else {
            "privileged path"
        };
        errors.push(ValidationError {
            message: format!(
                "resource '{id}' has {reason} but no sudo: true — add sudo: true or the write will fail with permission denied"
            ),
        });
    }
}

/// Validate YAML for unknown fields and return warnings.
/// This performs the second pass of two-pass parsing (FJ-2500).
pub fn check_unknown_fields(yaml: &str) -> Vec<ValidationError> {
    match unknown_fields::detect_unknown_fields(yaml) {
        Ok(unknowns) => unknown_fields::unknown_fields_to_errors(&unknowns),
        Err(_) => Vec::new(), // Parse errors handled by first pass
    }
}

/// Validate recipe YAML for unknown fields and return warnings (FJ-2500).
pub fn check_unknown_recipe_fields(yaml: &str) -> Vec<ValidationError> {
    match unknown_fields::detect_unknown_recipe_fields(yaml) {
        Ok(unknowns) => unknown_fields::unknown_fields_to_errors(&unknowns),
        Err(_) => Vec::new(),
    }
}

/// Parse, validate, and expand recipes in a config file.
/// This is the main entry point for loading a config for plan/apply.
pub fn parse_and_validate(path: &Path) -> Result<ForjarConfig, String> {
    parse_and_validate_opts(path, false)
}

/// Parse, validate, expand — with strict mode for unknown fields (FJ-2500).
/// When `deny_unknown` is true, unknown YAML fields are hard errors.
/// When false, unknown fields are printed as warnings to stderr.
pub fn parse_and_validate_opts(path: &Path, deny_unknown: bool) -> Result<ForjarConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let mut config = parse_config(&content)?;

    // FJ-2500: Detect unknown fields (two-pass parsing)
    let unknown_warnings = check_unknown_fields(&content);
    if !unknown_warnings.is_empty() {
        if deny_unknown {
            return Err(format!(
                "unknown field errors:\n{}",
                unknown_warnings
                    .iter()
                    .map(|e| format!("  - {e}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        for w in &unknown_warnings {
            eprintln!("warning: {w}");
        }
    }

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
                .map(|e| format!("  - {e}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    expand_recipes(&mut config, path.parent())?;
    expand_resources(&mut config);
    Ok(config)
}
