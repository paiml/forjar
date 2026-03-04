//! Validation of resource references, machine config, and expansion fields.

use super::*;

/// Check a reference (depends_on or triggers) against config, allowing expandable resources.
fn validate_ref(
    config: &ForjarConfig,
    id: &str,
    ref_id: &str,
    ref_type: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Skip deps containing {{item}} or {{index}} — they resolve after for_each/count expansion.
    if ref_id.contains("{{item}}") || ref_id.contains("{{index}}") {
        return;
    }
    if !config.resources.contains_key(ref_id) {
        let will_expand = config
            .resources
            .get(ref_id)
            .map(|r| r.count.is_some() || r.for_each.is_some())
            .unwrap_or(false);
        if !will_expand {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' {ref_type} unknown resource '{ref_id}'"
                ),
            });
        }
    }
    if ref_id == id {
        errors.push(ValidationError {
            message: format!("resource '{id}' {ref_type} itself"),
        });
    }
}

/// Validate machine and dependency references for a single resource.
pub(super) fn validate_resource_refs(
    config: &ForjarConfig,
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    for machine_name in resource.machine.to_vec() {
        if !config.machines.contains_key(&machine_name) && machine_name != "localhost" {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' references unknown machine '{machine_name}'"
                ),
            });
        }
    }

    for arch in &resource.arch {
        if !KNOWN_ARCHITECTURES.contains(&arch.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' has unknown arch '{}' (expected one of: {})",
                    id,
                    arch,
                    KNOWN_ARCHITECTURES.join(", ")
                ),
            });
        }
    }

    for dep in &resource.depends_on {
        validate_ref(config, id, dep, "depends on", errors);
    }

    for trigger in &resource.triggers {
        validate_ref(config, id, trigger, "triggers on", errors);
    }

    if resource.count.is_some() && resource.for_each.is_some() {
        errors.push(ValidationError {
            message: format!("resource '{id}' cannot have both 'count' and 'for_each'"),
        });
    }
    if let Some(count) = resource.count {
        if count == 0 {
            errors.push(ValidationError {
                message: format!("resource '{id}' has count: 0 (must be >= 1)"),
            });
        }
    }
    if let Some(ref items) = resource.for_each {
        if items.is_empty() {
            errors.push(ValidationError {
                message: format!("resource '{id}' has empty for_each list"),
            });
        }
    }
}

/// Validate machine configuration (container transport rules, arch).
pub(super) fn validate_machine(key: &str, machine: &Machine, errors: &mut Vec<ValidationError>) {
    // FJ-064: Validate machine arch
    if !KNOWN_ARCHITECTURES.contains(&machine.arch.as_str()) {
        errors.push(ValidationError {
            message: format!(
                "machine '{}' has unknown arch '{}' (expected one of: {})",
                key,
                machine.arch,
                KNOWN_ARCHITECTURES.join(", ")
            ),
        });
    }

    if machine.is_container_transport() && machine.container.is_none() {
        errors.push(ValidationError {
            message: format!(
                "machine '{key}' uses container transport but has no 'container' block"
            ),
        });
    }

    if let Some(ref container) = machine.container {
        if container.runtime != "docker" && container.runtime != "podman" {
            errors.push(ValidationError {
                message: format!(
                    "machine '{}' container runtime must be 'docker' or 'podman', got '{}'",
                    key, container.runtime
                ),
            });
        }
        if container.ephemeral && container.image.is_none() {
            errors.push(ValidationError {
                message: format!("machine '{key}' is ephemeral but has no container image"),
            });
        }
    }
}
