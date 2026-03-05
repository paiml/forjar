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
            message: format!("resource '{id}' (package) has no packages"),
        });
    }
    if resource.provider.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (package) has no provider"),
        });
    }
}

fn validate_file(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.path.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (file) has no path"),
        });
    }
    if resource.content.is_some() && resource.source.is_some() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (file) has both content and source (pick one)"),
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
            message: format!("resource '{id}' (file) state=symlink requires a target"),
        });
    }
}

fn validate_service(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (service) has no name"),
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
            message: format!("resource '{id}' (mount) has no source"),
        });
    }
    if resource.path.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (mount) has no path"),
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
            message: format!("resource '{id}' (user) has no name"),
        });
    }
}

fn validate_docker(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (docker) has no name"),
        });
    }
    if resource.image.is_none() && resource.state.as_deref() != Some("absent") {
        errors.push(ValidationError {
            message: format!("resource '{id}' (docker) has no image"),
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
            message: format!("resource '{id}' (cron) has no name"),
        });
    }
    if resource.schedule.is_none() && resource.state.as_deref() != Some("absent") {
        errors.push(ValidationError {
            message: format!("resource '{id}' (cron) has no schedule"),
        });
    }
    if let Some(ref sched) = resource.schedule {
        let fields: Vec<&str> = sched.split_whitespace().collect();
        if fields.len() != 5 {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (cron) schedule '{sched}' must have exactly 5 fields (min hour dom mon dow)"
                ),
            });
        }
    }
    if resource.command.is_none() && resource.state.as_deref() != Some("absent") {
        errors.push(ValidationError {
            message: format!("resource '{id}' (cron) has no command"),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["present", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (cron) has invalid state '{state}' (expected: present, absent)"
                ),
            });
        }
    }
}

fn validate_network(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.port.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (network) has no port"),
        });
    }
    if let Some(ref proto) = resource.protocol {
        let valid = ["tcp", "udp"];
        if !valid.contains(&proto.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (network) has invalid protocol '{proto}' (expected: tcp, udp)"
                ),
            });
        }
    }
    if let Some(ref action) = resource.action {
        let valid = ["allow", "deny", "reject"];
        if !valid.contains(&action.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (network) has invalid action '{action}' (expected: allow, deny, reject)"
                ),
            });
        }
    }
}

fn validate_pepita(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (pepita) has no name"),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["present", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (pepita) has invalid state '{state}' (expected: present, absent)"
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
                message: format!("resource '{id}' (pepita) has empty cpuset"),
            });
        }
    }
}

fn validate_model(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (model) has no name"),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["present", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (model) has invalid state '{state}' (expected: present, absent)"
                ),
            });
        }
    }
}

fn validate_gpu(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.driver_version.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (gpu) has no driver_version"),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["present", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (gpu) has invalid state '{state}' (expected: present, absent)"
                ),
            });
        }
    }
}

fn validate_recipe(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.recipe.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (recipe) has no recipe name"),
        });
    }
}

fn validate_task(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    // FJ-2700: Pipeline tasks use stages instead of command
    let is_pipeline = resource
        .task_mode
        .as_ref()
        .is_some_and(|m| *m == crate::core::types::TaskMode::Pipeline);
    if resource.command.is_none() && !is_pipeline {
        errors.push(ValidationError {
            message: format!("resource '{id}' (task) has no command"),
        });
    }
    if is_pipeline && resource.stages.is_empty() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (task pipeline) has no stages"),
        });
    }
    if let Some(ref timeout) = resource.timeout {
        if *timeout == 0 {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (task) has timeout of 0 (use no timeout or a positive value)"
                ),
            });
        }
    }
}
