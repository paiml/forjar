//! FJ-220 + FJ-3200: Policy-as-Code evaluation engine.
//!
//! Evaluates policy rules against resources. FJ-3200 extends with
//! `assert`, `limit` types, severity levels, compliance mappings, and
//! aggregate `PolicyCheckResult`.

use super::*;
use crate::core::types::{PolicyCheckResult, PolicyRuleType, PolicyViolation};

/// Decides whether a resource has a field set.
type FieldPresence = fn(&Resource) -> bool;

/// Renders a resource field as the string a condition compares against.
type FieldValue = fn(&Resource) -> Option<String>;

/// Which fields `require` can name, and how presence is decided for each.
///
/// A table, not a `match`: the field names are distinct literals, so the arms
/// never overlapped and their order was never observable — the branch per field
/// bought nothing but complexity. An unlisted field is absent, as before.
const FIELD_PRESENCE: &[(&str, FieldPresence)] = &[
    ("owner", |r| r.owner.is_some()),
    ("group", |r| r.group.is_some()),
    ("mode", |r| r.mode.is_some()),
    ("tags", |r| !r.tags.is_empty()),
    ("path", |r| r.path.is_some()),
    ("content", |r| r.content.is_some()),
    ("source", |r| r.source.is_some()),
    ("name", |r| r.name.is_some()),
    ("provider", |r| r.provider.is_some()),
    ("packages", |r| !r.packages.is_empty()),
    ("depends_on", |r| !r.depends_on.is_empty()),
    ("shell", |r| r.shell.is_some()),
    ("home", |r| r.home.is_some()),
    ("schedule", |r| r.schedule.is_some()),
    ("command", |r| r.command.is_some()),
    ("image", |r| r.image.is_some()),
    ("state", |r| r.state.is_some()),
    ("when", |r| r.when.is_some()),
];

/// Check if a resource has a given field set (non-None, non-empty).
pub(crate) fn resource_has_field(resource: &Resource, field: &str) -> bool {
    FIELD_PRESENCE
        .iter()
        .find(|(name, _)| *name == field)
        .is_some_and(|(_, present)| present(resource))
}

/// Which fields a condition can compare against, and how each is stringified.
///
/// Same shape as [`FIELD_PRESENCE`], and deliberately a different set: `tags`,
/// `packages` and `depends_on` are lists with no single string value, while
/// `type` and `when` exist on only one of the two tables. Keeping them separate
/// preserves exactly which names each function answered to.
const FIELD_VALUES: &[(&str, FieldValue)] = &[
    ("owner", |r| r.owner.clone()),
    ("group", |r| r.group.clone()),
    ("mode", |r| r.mode.clone()),
    ("path", |r| r.path.clone()),
    ("content", |r| r.content.clone()),
    ("source", |r| r.source.clone()),
    ("name", |r| r.name.clone()),
    ("provider", |r| r.provider.clone()),
    ("state", |r| r.state.clone()),
    // `to_string()`, NOT the lowercased `Debug` spelling. `Debug` renders
    // `GithubRelease`; the ONLY spelling `type:` accepts is serde's
    // `github_release`, so a `deny` or `assert` rule comparing
    // `type == github_release` could never match (paiml/forjar#366).
    //
    // `remediate` reads this table too, but never for `type`: `type` is not in
    // `remediate::fixes::SETTABLE`, so `derive` rejects a `type`-keyed rule
    // before `apply_one` ever compares the value. The issue's claim that
    // remediation refused such a candidate with "the value is produced by a
    // recipe or a {{template}} expansion" describes an unreachable path.
    ("type", |r| Some(r.resource_type.to_string())),
    ("shell", |r| r.shell.clone()),
    ("home", |r| r.home.clone()),
    ("schedule", |r| r.schedule.clone()),
    ("command", |r| r.command.clone()),
    ("image", |r| r.image.clone()),
];

/// Get a string representation of a resource field for condition checks.
pub fn resource_field_value(resource: &Resource, field: &str) -> Option<String> {
    FIELD_VALUES
        .iter()
        .find(|(name, _)| *name == field)
        .and_then(|(_, value)| value(resource))
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
///
/// One rule type per arm, each arm's test in its own function: a rule that
/// names no field (or no condition) is never a violation, and that guard used
/// to be repeated four times inside this `match`.
fn evaluate_rule(rule: &PolicyRule, resource: &Resource) -> bool {
    match rule.rule_type {
        PolicyRuleType::Require => violates_require(rule, resource),
        PolicyRuleType::Deny | PolicyRuleType::Warn => violates_deny_or_warn(rule, resource),
        PolicyRuleType::Assert => violates_assert(rule, resource),
        PolicyRuleType::Limit => violates_limit(rule, resource),
    }
}

/// `require`: the named field must be set.
fn violates_require(rule: &PolicyRule, resource: &Resource) -> bool {
    if let Some(ref field) = rule.field {
        !resource_has_field(resource, field)
    } else {
        false
    }
}

/// `deny` / `warn`: matching the condition IS the violation.
fn violates_deny_or_warn(rule: &PolicyRule, resource: &Resource) -> bool {
    if let (Some(ref field), Some(ref value)) = (&rule.condition_field, &rule.condition_value) {
        resource_field_value(resource, field).as_deref() == Some(value.as_str())
    } else {
        false
    }
}

/// `assert`: the condition must be true. Violation if field != expected value.
fn violates_assert(rule: &PolicyRule, resource: &Resource) -> bool {
    if let (Some(ref field), Some(ref expected)) = (&rule.condition_field, &rule.condition_value) {
        resource_field_value(resource, field).as_deref() != Some(expected.as_str())
    } else {
        false
    }
}

/// `limit`: the named list field must stay within min/max.
fn violates_limit(rule: &PolicyRule, resource: &Resource) -> bool {
    if let Some(ref field) = rule.field {
        let count = resource_field_count(resource, field);
        let over_max = rule.max_count.is_some_and(|max| count > max);
        let under_min = rule.min_count.is_some_and(|min| count < min);
        over_max || under_min
    } else {
        false
    }
}

/// Check if a resource matches the rule's scope filters.
///
/// THE scope matcher. `core::policy_coverage` reports which resources a rule
/// covers and must answer with the same predicate this evaluator decides with;
/// it used to carry its own substring variant, and so reported enforcement
/// that never happened (paiml/forjar#356).
///
/// The type is compared through `Display`, which is serde's spelling — the one
/// a `type:` key accepts. It used to be compared through the lowercased `Debug`
/// spelling, which agrees for the fifteen single-word variants and diverges for
/// the six multi-word ones: `resource_type: github_release`, the only form a
/// document can declare, matched NOTHING, while `githubrelease` — which serde
/// rejects with `unknown variant` — was the one string that worked. A rule
/// scoped to any multi-word type was silently inert, and the apply gate
/// (`cli::apply_preflight::check_policy_violations`) failed OPEN
/// (paiml/forjar#366).
pub(crate) fn matches_scope(rule: &PolicyRule, resource: &Resource) -> bool {
    if let Some(ref rt) = rule.resource_type {
        if resource.resource_type.to_string() != *rt {
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

/// Every `(rule index, resource id)` pair that violates, in report order.
///
/// paiml/forjar#356: `PolicyViolation` carries the rule's *message*, its type
/// and its OPTIONAL id — but not the rule. A caller that has to read
/// `condition_field` / `condition_value` (a remediation deriving the value it
/// must write from the policy that demanded it) cannot recover the rule from a
/// violation, and matching on `policy_id` is ambiguous the moment two rules
/// leave `id` unset.
///
/// So the pairing is published, and [`evaluate_policies_full`] is built ON it
/// rather than beside it: there is one iteration order and one violation
/// predicate, and the two cannot drift.
pub fn violating_pairs(config: &ForjarConfig) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (index, rule) in config.policies.iter().enumerate() {
        for (id, resource) in &config.resources {
            if matches_scope(rule, resource) && evaluate_rule(rule, resource) {
                out.push((index, id.clone()));
            }
        }
    }
    out
}

/// FJ-3200: Full policy evaluation with aggregate result.
pub fn evaluate_policies_full(config: &ForjarConfig) -> PolicyCheckResult {
    let violations = violating_pairs(config)
        .into_iter()
        .map(|(index, resource_id)| {
            let rule = &config.policies[index];
            PolicyViolation {
                rule_message: rule.message.clone(),
                resource_id,
                rule_type: rule.rule_type.clone(),
                severity: rule.effective_severity(),
                policy_id: rule.id.clone(),
                remediation: rule.remediation.clone(),
                compliance: rule.compliance.clone(),
            }
        })
        .collect();

    PolicyCheckResult {
        violations,
        rules_evaluated: config.policies.len(),
        resources_checked: config.resources.len(),
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

/// FJ-3207: Serialize policy check result to SARIF 2.1.0 format for CI integration.
///
/// Produces a valid SARIF log object compatible with GitHub Code Scanning,
/// Azure DevOps, and other SARIF-consuming tools.
///
/// The emitter itself lives in `core::quality_gate::sarif`, and this function
/// is a projection onto it. There used to be exactly one SARIF emitter in the
/// repo and it was here, reachable only from policy evaluation; the quality
/// gate needs the same output for findings that are not policy violations, and
/// a second copy of a schema emitter is a copy that drifts.
pub fn policy_check_to_sarif(result: &PolicyCheckResult) -> String {
    use crate::core::quality_gate::{checks::violation_to_finding, sarif::findings_to_sarif};
    let findings: Vec<_> = result.violations.iter().map(violation_to_finding).collect();
    // `forjar.yaml` is the historic literal. Policy evaluation is handed a
    // parsed config with no path attached, so there is nothing better to say
    // here; the quality gate, which does know the path, passes the real one.
    let sarif = findings_to_sarif(&findings, "forjar.yaml");
    serde_json::to_string_pretty(&sarif).unwrap_or_else(|_| "{}".to_string())
}
