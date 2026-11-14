//! Cron and network resource required-field validation, split out of
//! `resource_types.rs` to keep that file under the 500-line limit.

use super::*;

pub(super) fn validate_cron(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
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
        // Skip templates and cron keywords (@daily, @weekly, etc.)
        let is_keyword = sched.starts_with('@');
        let is_template = sched.contains("{{");
        if !is_keyword && !is_template {
            let fields: Vec<&str> = sched.split_whitespace().collect();
            if fields.len() != 5 {
                errors.push(ValidationError {
                    message: format!(
                        "resource '{id}' (cron) schedule '{sched}' must have exactly 5 fields (min hour dom mon dow)"
                    ),
                });
            }
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

pub(super) fn validate_network(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
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
