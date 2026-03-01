//! Type-specific required-field validation for each resource type.

use super::*;

/// Validate type-specific required fields for a resource.
pub(super) fn validate_resource_type(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    match resource.resource_type {
        ResourceType::Package => validate_package(id, resource, errors),
        ResourceType::File => validate_file(id, resource, errors),
        ResourceType::Service => validate_service(id, resource, errors),
        ResourceType::Mount => validate_mount(id, resource, errors),
        ResourceType::User => validate_user(id, resource, errors),
        ResourceType::Docker => validate_docker(id, resource, errors),
        ResourceType::Cron => validate_cron(id, resource, errors),
        ResourceType::Network => validate_network(id, resource, errors),
        ResourceType::Pepita => validate_pepita(id, resource, errors),
        ResourceType::Model => validate_model(id, resource, errors),
        ResourceType::Gpu => validate_gpu(id, resource, errors),
        ResourceType::Recipe => validate_recipe(id, resource, errors),
        ResourceType::Task => validate_task(id, resource, errors),
    }
}

fn validate_package(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.packages.is_empty() {
        errors.push(ValidationError {
            message: format!("resource '{}' (package) has no packages", id),
        });
    }
    if resource.provider.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (package) has no provider", id),
        });
    }
}

fn validate_file(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.path.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (file) has no path", id),
        });
    }
    if resource.content.is_some() && resource.source.is_some() {
        errors.push(ValidationError {
            message: format!(
                "resource '{}' (file) has both content and source (pick one)",
                id
            ),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["file", "directory", "symlink", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (file) has invalid state '{}' (expected: {})",
                    id,
                    state,
                    valid.join(", ")
                ),
            });
        }
    }
    if resource.state.as_deref() == Some("symlink") && resource.target.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (file) state=symlink requires a target", id),
        });
    }
}

fn validate_service(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (service) has no name", id),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["running", "stopped", "enabled", "disabled"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (service) has invalid state '{}' (expected: {})",
                    id,
                    state,
                    valid.join(", ")
                ),
            });
        }
    }
}

fn validate_mount(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.source.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (mount) has no source", id),
        });
    }
    if resource.path.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (mount) has no path", id),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["mounted", "unmounted", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (mount) has invalid state '{}' (expected: {})",
                    id,
                    state,
                    valid.join(", ")
                ),
            });
        }
    }
}

fn validate_user(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (user) has no name", id),
        });
    }
}

fn validate_docker(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (docker) has no name", id),
        });
    }
    if resource.image.is_none() && resource.state.as_deref() != Some("absent") {
        errors.push(ValidationError {
            message: format!("resource '{}' (docker) has no image", id),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["running", "stopped", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (docker) has invalid state '{}' (expected: {})",
                    id,
                    state,
                    valid.join(", ")
                ),
            });
        }
    }
}

fn validate_cron(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (cron) has no name", id),
        });
    }
    if resource.schedule.is_none() && resource.state.as_deref() != Some("absent") {
        errors.push(ValidationError {
            message: format!("resource '{}' (cron) has no schedule", id),
        });
    }
    if let Some(ref sched) = resource.schedule {
        let fields: Vec<&str> = sched.split_whitespace().collect();
        if fields.len() != 5 {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (cron) schedule '{}' must have exactly 5 fields (min hour dom mon dow)",
                    id, sched
                ),
            });
        }
    }
    if resource.command.is_none() && resource.state.as_deref() != Some("absent") {
        errors.push(ValidationError {
            message: format!("resource '{}' (cron) has no command", id),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["present", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (cron) has invalid state '{}' (expected: present, absent)",
                    id, state
                ),
            });
        }
    }
}

fn validate_network(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.port.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (network) has no port", id),
        });
    }
    if let Some(ref proto) = resource.protocol {
        let valid = ["tcp", "udp"];
        if !valid.contains(&proto.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (network) has invalid protocol '{}' (expected: tcp, udp)",
                    id, proto
                ),
            });
        }
    }
    if let Some(ref action) = resource.action {
        let valid = ["allow", "deny", "reject"];
        if !valid.contains(&action.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (network) has invalid action '{}' (expected: allow, deny, reject)",
                    id, action
                ),
            });
        }
    }
}

fn validate_pepita(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (pepita) has no name", id),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["present", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (pepita) has invalid state '{}' (expected: present, absent)",
                    id, state
                ),
            });
        }
    }
    if resource.overlay_merged.is_some()
        && resource.overlay_lower.is_none()
        && resource.overlay_upper.is_none()
    {
        // overlay_merged without explicit dirs uses defaults -- valid but warn-worthy
    }
    if let Some(ref cpuset) = resource.cpuset {
        if cpuset.is_empty() {
            errors.push(ValidationError {
                message: format!("resource '{}' (pepita) has empty cpuset", id),
            });
        }
    }
}

fn validate_model(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (model) has no name", id),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["present", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (model) has invalid state '{}' (expected: present, absent)",
                    id, state
                ),
            });
        }
    }
}

fn validate_gpu(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.driver_version.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (gpu) has no driver_version", id),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["present", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (gpu) has invalid state '{}' (expected: present, absent)",
                    id, state
                ),
            });
        }
    }
}

fn validate_recipe(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.recipe.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (recipe) has no recipe name", id),
        });
    }
}

fn validate_task(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.command.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{}' (task) has no command", id),
        });
    }
    if let Some(ref timeout) = resource.timeout {
        if *timeout == 0 {
            errors.push(ValidationError {
                message: format!("resource '{}' (task) has timeout of 0 (use no timeout or a positive value)", id),
            });
        }
    }
}
