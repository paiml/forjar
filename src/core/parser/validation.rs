//! Validation of resource references, machine config, and expansion fields.

use super::*;

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
                    "resource '{}' references unknown machine '{}'",
                    id, machine_name
                ),
            });
        }
    }

    // FJ-064: Validate arch filter values
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
        if !config.resources.contains_key(dep) {
            // FJ-203/FJ-204: Dep might reference a resource that will be expanded.
            // After expansion, the original ID disappears -- this is valid.
            let dep_resource = config.resources.get(dep);
            let will_expand = dep_resource
                .map(|r| r.count.is_some() || r.for_each.is_some())
                .unwrap_or(false);
            if !will_expand {
                errors.push(ValidationError {
                    message: format!("resource '{}' depends on unknown resource '{}'", id, dep),
                });
            }
        }
        if dep == id {
            errors.push(ValidationError {
                message: format!("resource '{}' depends on itself", id),
            });
        }
    }

    // FJ-224: Validate triggers -- must reference existing resources
    for trigger in &resource.triggers {
        if !config.resources.contains_key(trigger) {
            let trigger_resource = config.resources.get(trigger);
            let will_expand = trigger_resource
                .map(|r| r.count.is_some() || r.for_each.is_some())
                .unwrap_or(false);
            if !will_expand {
                errors.push(ValidationError {
                    message: format!(
                        "resource '{}' triggers on unknown resource '{}'",
                        id, trigger
                    ),
                });
            }
        }
        if trigger == id {
            errors.push(ValidationError {
                message: format!("resource '{}' triggers on itself", id),
            });
        }
    }

    // FJ-203/FJ-204: Validate count and for_each
    if resource.count.is_some() && resource.for_each.is_some() {
        errors.push(ValidationError {
            message: format!("resource '{}' cannot have both 'count' and 'for_each'", id),
        });
    }
    if let Some(count) = resource.count {
        if count == 0 {
            errors.push(ValidationError {
                message: format!("resource '{}' has count: 0 (must be >= 1)", id),
            });
        }
    }
    if let Some(ref items) = resource.for_each {
        if items.is_empty() {
            errors.push(ValidationError {
                message: format!("resource '{}' has empty for_each list", id),
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
                "machine '{}' uses container transport but has no 'container' block",
                key
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
                message: format!("machine '{}' is ephemeral but has no container image", key),
            });
        }
    }
}
