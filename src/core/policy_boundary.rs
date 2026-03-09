//! FJ-3209: Policy boundary config generation and mutation testing.
//!
//! Generates boundary configurations that exercise policy rules at their
//! decision boundaries. Every deny rule must reject at least one generated
//! config; every assert rule must pass on golden configs. This validates
//! that policy rules are not vacuous.

use crate::core::compliance_pack::{
    evaluate_pack, ComplianceCheck, CompliancePack, ComplianceRule, PackEvalResult,
};
use std::collections::HashMap;

/// A boundary config: a resource map designed to test a specific rule.
#[derive(Debug, Clone)]
pub struct BoundaryConfig {
    /// Which rule this boundary targets.
    pub target_rule_id: String,
    /// Whether this config should pass (golden) or fail (boundary).
    pub expected_pass: bool,
    /// The generated resource map.
    pub resources: HashMap<String, HashMap<String, String>>,
    /// Description of what boundary is being tested.
    pub description: String,
}

/// Result of boundary testing a pack.
#[derive(Debug, Clone)]
pub struct BoundaryTestResult {
    /// Pack name.
    pub pack_name: String,
    /// Total rules tested.
    pub rules_tested: usize,
    /// Rules that had at least one boundary config generated.
    pub rules_with_boundary: usize,
    /// Individual test outcomes.
    pub outcomes: Vec<BoundaryOutcome>,
}

impl BoundaryTestResult {
    /// Whether all boundary tests passed.
    pub fn all_passed(&self) -> bool {
        self.outcomes.iter().all(|o| o.passed)
    }

    /// Count failures.
    pub fn failure_count(&self) -> usize {
        self.outcomes.iter().filter(|o| !o.passed).count()
    }
}

/// Outcome of testing a single boundary config.
#[derive(Debug, Clone)]
pub struct BoundaryOutcome {
    /// Rule ID being tested.
    pub rule_id: String,
    /// Whether the outcome matched expectation.
    pub passed: bool,
    /// What was expected (pass or fail).
    pub expected: String,
    /// What actually happened.
    pub actual: String,
    /// Description.
    pub description: String,
}

/// Generate boundary configs for all rules in a compliance pack.
pub fn generate_boundary_configs(pack: &CompliancePack) -> Vec<BoundaryConfig> {
    let mut configs = Vec::new();
    for rule in &pack.rules {
        configs.extend(generate_for_rule(rule));
    }
    configs
}

/// Generate boundary configs for a single rule.
fn generate_for_rule(rule: &ComplianceRule) -> Vec<BoundaryConfig> {
    match &rule.check {
        ComplianceCheck::Assert {
            resource_type,
            field,
            expected,
        } => generate_assert_boundary(&rule.id, resource_type, field, expected),
        ComplianceCheck::Deny {
            resource_type,
            field,
            pattern,
        } => generate_deny_boundary(&rule.id, resource_type, field, pattern),
        ComplianceCheck::Require {
            resource_type,
            field,
        } => generate_require_boundary(&rule.id, resource_type, field),
        ComplianceCheck::RequireTag { tag } => generate_require_tag_boundary(&rule.id, tag),
        ComplianceCheck::Script { .. } => {
            // Script checks cannot have auto-generated boundaries
            Vec::new()
        }
    }
}

/// Assert: golden config has correct value, boundary has wrong value.
fn generate_assert_boundary(
    rule_id: &str,
    resource_type: &str,
    field: &str,
    expected: &str,
) -> Vec<BoundaryConfig> {
    let resource_name = format!("boundary-{resource_type}");

    // Golden: correct value → should pass
    let mut golden = HashMap::new();
    let mut fields = HashMap::new();
    fields.insert("type".into(), resource_type.into());
    fields.insert(field.into(), expected.into());
    golden.insert(resource_name.clone(), fields);

    // Boundary: wrong value → should fail
    let mut boundary = HashMap::new();
    let mut bad_fields = HashMap::new();
    bad_fields.insert("type".into(), resource_type.into());
    bad_fields.insert(field.into(), format!("NOT_{expected}"));
    boundary.insert(resource_name, bad_fields);

    vec![
        BoundaryConfig {
            target_rule_id: rule_id.into(),
            expected_pass: true,
            resources: golden,
            description: format!("golden: {field}={expected}"),
        },
        BoundaryConfig {
            target_rule_id: rule_id.into(),
            expected_pass: false,
            resources: boundary,
            description: format!("boundary: {field}=NOT_{expected}"),
        },
    ]
}

/// Deny: boundary config contains denied pattern, golden does not.
fn generate_deny_boundary(
    rule_id: &str,
    resource_type: &str,
    field: &str,
    pattern: &str,
) -> Vec<BoundaryConfig> {
    let resource_name = format!("boundary-{resource_type}");

    // Golden: safe value → should pass (deny not triggered)
    let mut golden = HashMap::new();
    let mut fields = HashMap::new();
    fields.insert("type".into(), resource_type.into());
    fields.insert(field.into(), "safe_value".into());
    golden.insert(resource_name.clone(), fields);

    // Boundary: denied pattern → should fail
    let mut boundary = HashMap::new();
    let mut bad_fields = HashMap::new();
    bad_fields.insert("type".into(), resource_type.into());
    bad_fields.insert(field.into(), pattern.into());
    boundary.insert(resource_name, bad_fields);

    vec![
        BoundaryConfig {
            target_rule_id: rule_id.into(),
            expected_pass: true,
            resources: golden,
            description: format!("golden: {field}=safe (not {pattern})"),
        },
        BoundaryConfig {
            target_rule_id: rule_id.into(),
            expected_pass: false,
            resources: boundary,
            description: format!("boundary: {field}={pattern} (denied)"),
        },
    ]
}

/// Require: boundary has missing field, golden has field present.
fn generate_require_boundary(
    rule_id: &str,
    resource_type: &str,
    field: &str,
) -> Vec<BoundaryConfig> {
    let resource_name = format!("boundary-{resource_type}");

    // Golden: field present → should pass
    let mut golden = HashMap::new();
    let mut fields = HashMap::new();
    fields.insert("type".into(), resource_type.into());
    fields.insert(field.into(), "present".into());
    golden.insert(resource_name.clone(), fields);

    // Boundary: field missing → should fail
    let mut boundary = HashMap::new();
    let mut missing_fields = HashMap::new();
    missing_fields.insert("type".into(), resource_type.into());
    // Deliberately omit the required field
    boundary.insert(resource_name, missing_fields);

    vec![
        BoundaryConfig {
            target_rule_id: rule_id.into(),
            expected_pass: true,
            resources: golden,
            description: format!("golden: {field} present"),
        },
        BoundaryConfig {
            target_rule_id: rule_id.into(),
            expected_pass: false,
            resources: boundary,
            description: format!("boundary: {field} missing"),
        },
    ]
}

/// RequireTag: boundary has no tags, golden has required tag.
fn generate_require_tag_boundary(rule_id: &str, tag: &str) -> Vec<BoundaryConfig> {
    // Golden: tag present → should pass
    let mut golden = HashMap::new();
    let mut fields = HashMap::new();
    fields.insert("tags".into(), tag.into());
    golden.insert("boundary-resource".into(), fields);

    // Boundary: no tags → should fail
    let mut boundary = HashMap::new();
    let no_tags = HashMap::new();
    boundary.insert("boundary-resource".into(), no_tags);

    vec![
        BoundaryConfig {
            target_rule_id: rule_id.into(),
            expected_pass: true,
            resources: golden,
            description: format!("golden: tag '{tag}' present"),
        },
        BoundaryConfig {
            target_rule_id: rule_id.into(),
            expected_pass: false,
            resources: boundary,
            description: format!("boundary: no tags (missing '{tag}')"),
        },
    ]
}

/// Run boundary testing: evaluate each boundary config against the pack.
pub fn test_boundaries(pack: &CompliancePack) -> BoundaryTestResult {
    let configs = generate_boundary_configs(pack);
    let mut outcomes = Vec::new();
    let mut rules_tested = std::collections::HashSet::new();

    for config in &configs {
        rules_tested.insert(&config.target_rule_id);
        let eval = evaluate_pack(pack, &config.resources);
        let rule_result = find_rule_result(&eval, &config.target_rule_id);

        let actual_passed = rule_result.is_none_or(|r| r.passed);
        let outcome_passed = actual_passed == config.expected_pass;

        outcomes.push(BoundaryOutcome {
            rule_id: config.target_rule_id.clone(),
            passed: outcome_passed,
            expected: if config.expected_pass { "pass" } else { "fail" }.into(),
            actual: if actual_passed { "pass" } else { "fail" }.into(),
            description: config.description.clone(),
        });
    }

    BoundaryTestResult {
        pack_name: pack.name.clone(),
        rules_tested: pack.rules.len(),
        rules_with_boundary: rules_tested.len(),
        outcomes,
    }
}

/// Find a specific rule's result in the evaluation.
fn find_rule_result<'a>(
    eval: &'a PackEvalResult,
    rule_id: &str,
) -> Option<&'a crate::core::compliance_pack::RuleEvalResult> {
    eval.results.iter().find(|r| r.rule_id == rule_id)
}

/// Format boundary test results as human-readable text.
pub fn format_boundary_results(result: &BoundaryTestResult) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "Boundary Testing: {} (rules: {}, boundaries: {})",
        result.pack_name,
        result.rules_tested,
        result.outcomes.len()
    ));

    for outcome in &result.outcomes {
        let status = if outcome.passed { "PASS" } else { "FAIL" };
        lines.push(format!(
            "  [{status}] {}: {} (expected={}, actual={})",
            outcome.rule_id, outcome.description, outcome.expected, outcome.actual
        ));
    }

    let pass_count = result.outcomes.iter().filter(|o| o.passed).count();
    lines.push(format!(
        "Result: {}/{} boundary tests passed",
        pass_count,
        result.outcomes.len()
    ));

    lines.join("\n")
}
