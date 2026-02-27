//! FJ-220: Policy evaluation against resources.

use super::*;
use crate::core::types::{PolicyRuleType, PolicyViolation};

/// Check if a resource has a given field set (non-None, non-empty).
fn resource_has_field(resource: &Resource, field: &str) -> bool {
    match field {
        "owner" => resource.owner.is_some(),
        "group" => resource.group.is_some(),
        "mode" => resource.mode.is_some(),
        "tags" => !resource.tags.is_empty(),
        "path" => resource.path.is_some(),
        "content" => resource.content.is_some(),
        "source" => resource.source.is_some(),
        "name" => resource.name.is_some(),
        "provider" => resource.provider.is_some(),
        "packages" => !resource.packages.is_empty(),
        "depends_on" => !resource.depends_on.is_empty(),
        "shell" => resource.shell.is_some(),
        "home" => resource.home.is_some(),
        "schedule" => resource.schedule.is_some(),
        "command" => resource.command.is_some(),
        "image" => resource.image.is_some(),
        "state" => resource.state.is_some(),
        "when" => resource.when.is_some(),
        _ => false,
    }
}

/// Get a string representation of a resource field for condition checks.
fn resource_field_value(resource: &Resource, field: &str) -> Option<String> {
    match field {
        "owner" => resource.owner.clone(),
        "group" => resource.group.clone(),
        "mode" => resource.mode.clone(),
        "path" => resource.path.clone(),
        "content" => resource.content.clone(),
        "source" => resource.source.clone(),
        "name" => resource.name.clone(),
        "provider" => resource.provider.clone(),
        "state" => resource.state.clone(),
        "type" => Some(format!("{:?}", resource.resource_type).to_lowercase()),
        "shell" => resource.shell.clone(),
        "home" => resource.home.clone(),
        "schedule" => resource.schedule.clone(),
        "command" => resource.command.clone(),
        "image" => resource.image.clone(),
        _ => None,
    }
}

/// Evaluate all policy rules against all resources. Returns violations.
pub fn evaluate_policies(config: &ForjarConfig) -> Vec<PolicyViolation> {
    let mut violations = Vec::new();

    for rule in &config.policies {
        for (id, resource) in &config.resources {
            // Filter by resource_type if specified
            if let Some(ref rt) = rule.resource_type {
                let actual = format!("{:?}", resource.resource_type).to_lowercase();
                if actual != *rt {
                    continue;
                }
            }

            // Filter by tag if specified
            if let Some(ref tag) = rule.tag {
                if !resource.tags.contains(tag) {
                    continue;
                }
            }

            let violated = match rule.rule_type {
                PolicyRuleType::Require => {
                    // Resource must have the field set
                    if let Some(ref field) = rule.field {
                        !resource_has_field(resource, field)
                    } else {
                        false
                    }
                }
                PolicyRuleType::Deny | PolicyRuleType::Warn => {
                    // Check if condition field matches condition value
                    if let (Some(ref field), Some(ref value)) =
                        (&rule.condition_field, &rule.condition_value)
                    {
                        resource_field_value(resource, field).as_deref() == Some(value.as_str())
                    } else {
                        false
                    }
                }
            };

            if violated {
                violations.push(PolicyViolation {
                    rule_message: rule.message.clone(),
                    resource_id: id.clone(),
                    severity: rule.rule_type.clone(),
                });
            }
        }
    }

    violations
}
