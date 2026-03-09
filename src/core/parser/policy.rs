//! FJ-220 + FJ-3200: Policy-as-Code evaluation engine.
//!
//! Evaluates policy rules against resources. FJ-3200 extends with
//! `assert`, `limit` types, severity levels, compliance mappings, and
//! aggregate `PolicyCheckResult`.

use super::*;
use crate::core::types::{PolicyCheckResult, PolicyRuleType, PolicyViolation};

/// Check if a resource has a given field set (non-None, non-empty).
pub(crate) fn resource_has_field(resource: &Resource, field: &str) -> bool {
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
pub(crate) fn resource_field_value(resource: &Resource, field: &str) -> Option<String> {
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

/// Get the count of items in a list-type field.
pub(crate) fn resource_field_count(resource: &Resource, field: &str) -> usize {
    match field {
        "tags" => resource.tags.len(),
        "packages" => resource.packages.len(),
        "depends_on" => resource.depends_on.len(),
        _ => 0,
    }
}

/// Evaluate a single rule against a single resource. Returns true if violated.
fn evaluate_rule(rule: &PolicyRule, resource: &Resource) -> bool {
    match rule.rule_type {
        PolicyRuleType::Require => {
            if let Some(ref field) = rule.field {
                !resource_has_field(resource, field)
            } else {
                false
            }
        }
        PolicyRuleType::Deny | PolicyRuleType::Warn => {
            if let (Some(ref field), Some(ref value)) =
                (&rule.condition_field, &rule.condition_value)
            {
                resource_field_value(resource, field).as_deref() == Some(value.as_str())
            } else {
                false
            }
        }
        PolicyRuleType::Assert => {
            // Assert: condition must be true. Violation if field != expected value.
            if let (Some(ref field), Some(ref expected)) =
                (&rule.condition_field, &rule.condition_value)
            {
                resource_field_value(resource, field).as_deref() != Some(expected.as_str())
            } else {
                false
            }
        }
        PolicyRuleType::Limit => {
            if let Some(ref field) = rule.field {
                let count = resource_field_count(resource, field);
                let over_max = rule.max_count.is_some_and(|max| count > max);
                let under_min = rule.min_count.is_some_and(|min| count < min);
                over_max || under_min
            } else {
                false
            }
        }
    }
}

/// Check if a resource matches the rule's scope filters.
fn matches_scope(rule: &PolicyRule, resource: &Resource) -> bool {
    if let Some(ref rt) = rule.resource_type {
        let actual = format!("{:?}", resource.resource_type).to_lowercase();
        if actual != *rt {
            return false;
        }
    }
    if let Some(ref tag) = rule.tag {
        if !resource.tags.contains(tag) {
            return false;
        }
    }
    true
}

/// FJ-220: Evaluate all policy rules against all resources. Returns violations.
///
/// Backward-compatible wrapper — returns Vec<PolicyViolation>.
pub fn evaluate_policies(config: &ForjarConfig) -> Vec<PolicyViolation> {
    evaluate_policies_full(config).violations
}

/// FJ-3200: Full policy evaluation with aggregate result.
pub fn evaluate_policies_full(config: &ForjarConfig) -> PolicyCheckResult {
    let mut violations = Vec::new();
    let rules_evaluated = config.policies.len();
    let resources_checked = config.resources.len();

    for rule in &config.policies {
        for (id, resource) in &config.resources {
            if !matches_scope(rule, resource) {
                continue;
            }

            if evaluate_rule(rule, resource) {
                violations.push(PolicyViolation {
                    rule_message: rule.message.clone(),
                    resource_id: id.clone(),
                    rule_type: rule.rule_type.clone(),
                    severity: rule.effective_severity(),
                    policy_id: rule.id.clone(),
                    remediation: rule.remediation.clone(),
                    compliance: rule.compliance.clone(),
                });
            }
        }
    }

    PolicyCheckResult {
        violations,
        rules_evaluated,
        resources_checked,
    }
}

/// FJ-3200: Serialize policy check result to JSON.
pub fn policy_check_to_json(result: &PolicyCheckResult) -> String {
    let violations_json: Vec<serde_json::Value> = result
        .violations
        .iter()
        .map(|v| {
            let compliance: Vec<serde_json::Value> = v
                .compliance
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "framework": c.framework,
                        "control": c.control,
                    })
                })
                .collect();
            serde_json::json!({
                "policy_id": v.policy_id,
                "resource_id": v.resource_id,
                "message": v.rule_message,
                "severity": format!("{:?}", v.severity).to_lowercase(),
                "rule_type": format!("{:?}", v.rule_type).to_lowercase(),
                "remediation": v.remediation,
                "compliance": compliance,
            })
        })
        .collect();

    let report = serde_json::json!({
        "passed": !result.has_blocking_violations(),
        "rules_evaluated": result.rules_evaluated,
        "resources_checked": result.resources_checked,
        "error_count": result.error_count(),
        "warning_count": result.warning_count(),
        "info_count": result.info_count(),
        "violations": violations_json,
    });

    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
}
