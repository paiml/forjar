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
                message: format!("resource '{id}' {ref_type} unknown resource '{ref_id}'"),
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
    for machine_name in resource.machine.iter() {
        if !config.machines.contains_key(machine_name) && machine_name != "localhost" {
            errors.push(ValidationError {
                message: format!("resource '{id}' references unknown machine '{machine_name}'"),
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

    validate_expansion_fields(id, resource, errors);
}

/// Validate the `count` / `for_each` expansion fields of a single resource.
fn validate_expansion_fields(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
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

/// forjar#335 / #360: `ignore_drift` is a field list, and forjar honours the
/// entries it can actually mask out of the observation before hashing it.
///
/// Everything else is still refused. An unrecognised entry cannot be masked, so
/// accepting it would suppress nothing while reading as though it suppressed
/// something — and a resource type with no field-shaped observation at all
/// keeps the original #335 refusal verbatim, because for that type the narrowed
/// form genuinely is unimplemented.
pub(super) fn validate_lifecycle(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    let Some(lifecycle) = resource.lifecycle.as_ref() else {
        return;
    };
    let vocabulary = crate::core::observation_mask::vocabulary(&resource.resource_type);
    let unhonoured = lifecycle.unhonoured_ignore_drift(vocabulary.unwrap_or(&[]));
    if unhonoured.is_empty() {
        return;
    }
    let remedy = match vocabulary {
        Some(fields) => format!(
            "forjar can suppress these fields for a {} resource: {} (forjar#360).",
            resource.resource_type,
            fields.join(", ")
        ),
        None => format!(
            "per-field drift suppression is not implemented for a {} resource (forjar#335) — \
             its state query reports no named fields to select over.",
            resource.resource_type
        ),
    };
    errors.push(ValidationError {
        message: format!(
            "resource '{id}' has lifecycle.ignore_drift {unhonoured:?}, which forjar cannot \
             honour — it would suppress NOTHING while reading as though it suppressed those \
             fields. {remedy} Write ignore_drift: [\"*\"] to suppress the whole resource, or \
             remove the key to keep drift detection on."
        ),
    });
}
