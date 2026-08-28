//! Unit tests for the one policy-coverage calculation.

use super::*;
use crate::core::types::{ComplianceMapping, PolicyRule, PolicySeverity, Resource, ResourceType};

fn rtype(name: &str) -> ResourceType {
    match name {
        "file" => ResourceType::File,
        "package" => ResourceType::Package,
        "service" => ResourceType::Service,
        "network" => ResourceType::Network,
        _ => ResourceType::File,
    }
}

fn make_config(resources: &[(&str, &str)], policies: Vec<PolicyRule>) -> ForjarConfig {
    let mut config = ForjarConfig::default();
    for (name, t) in resources {
        config.resources.insert(
            (*name).to_string(),
            Resource {
                resource_type: rtype(t),
                ..Default::default()
            },
        );
    }
    config.policies = policies;
    config
}

fn require_policy(scope: &str) -> PolicyRule {
    PolicyRule {
        id: Some(format!("P-{scope}")),
        rule_type: PolicyRuleType::Require,
        message: "test".into(),
        resource_type: Some(scope.into()),
        tag: None,
        field: Some("owner".into()),
        condition_field: None,
        condition_value: None,
        max_count: None,
        min_count: None,
        severity: None,
        remediation: None,
        compliance: vec![],
    }
}

// ── the resource side ───────────────────────────────────────────────

#[test]
fn full_coverage() {
    let cov = compute_coverage(&make_config(
        &[("f1", "file"), ("f2", "file")],
        vec![require_policy("file")],
    ));
    assert_eq!(cov.total_resources, 2);
    assert_eq!(cov.covered_resources, 2);
    assert!(cov.fully_covered);
    assert!((cov.coverage_percent - 100.0).abs() < f64::EPSILON);
}

#[test]
fn partial_coverage() {
    let cov = compute_coverage(&make_config(
        &[("f1", "file"), ("p1", "package")],
        vec![require_policy("file")],
    ));
    assert_eq!(cov.covered_resources, 1);
    assert_eq!(cov.uncovered, vec!["p1"]);
    assert!(!cov.fully_covered);
    assert!((cov.coverage_percent - 50.0).abs() < f64::EPSILON);
}

#[test]
fn no_policies() {
    let cov = compute_coverage(&make_config(&[("f1", "file"), ("p1", "package")], vec![]));
    assert_eq!(cov.covered_resources, 0);
    assert_eq!(cov.uncovered.len(), 2);
    assert!((cov.coverage_percent - 0.0).abs() < f64::EPSILON);
}

#[test]
fn no_resources_is_fully_covered_not_zero_percent() {
    let cov = compute_coverage(&make_config(&[], vec![require_policy("file")]));
    assert_eq!(cov.total_resources, 0);
    assert!(cov.fully_covered);
    assert!((cov.coverage_percent - 100.0).abs() < f64::EPSILON);
}

#[test]
fn per_resource_multiple_policies() {
    let cov = compute_coverage(&make_config(
        &[("f1", "file")],
        vec![require_policy("file"), require_policy("file")],
    ));
    assert_eq!(cov.per_resource.get("f1"), Some(&2));
}

/// `uncovered` is sorted, not left in `resources` declaration order — the MCP
/// verb returns this document and an agent diffs it between calls.
#[test]
fn uncovered_is_sorted_not_declaration_ordered() {
    let cov = compute_coverage(&make_config(
        &[("zeta", "file"), ("alpha", "file"), ("mid", "file")],
        vec![],
    ));
    assert_eq!(cov.uncovered, vec!["alpha", "mid", "zeta"]);
}

/// REJECTION CRITERION for the scope-matcher divergence this module was
/// rewritten to end. `matches_scope` — the matcher the policy ENGINE uses —
/// compares the resource type for EQUALITY. The substring match this module
/// used before reported a `network` resource as covered by a rule scoped
/// `resource_type: work`, because "work" is a substring of "network". The
/// engine would then never evaluate that rule against it, so the coverage
/// report claimed enforcement that does not happen.
#[test]
fn a_substring_of_a_resource_type_is_not_a_scope_match() {
    let cov = compute_coverage(&make_config(
        &[("n1", "network")],
        vec![require_policy("work")],
    ));
    assert_eq!(
        cov.covered_resources, 0,
        "`work` matched `network` by substring: {:?}",
        cov.per_resource
    );
    assert_eq!(cov.uncovered, vec!["n1"]);
}

// ── the rule side ───────────────────────────────────────────────────

#[test]
fn by_type_counts() {
    let cov = compute_coverage(&make_config(
        &[("f1", "file")],
        vec![require_policy("file"), require_policy("package")],
    ));
    assert_eq!(cov.by_type.get("require"), Some(&2));
    assert_eq!(cov.total_rules, 2);
}

#[test]
fn by_severity_is_the_effective_severity_not_the_declared_one() {
    let mut declared = require_policy("file");
    declared.severity = Some(PolicySeverity::Info);
    // `require` with no explicit severity is Error by derivation.
    let cov = compute_coverage(&make_config(
        &[("f1", "file")],
        vec![declared, require_policy("package")],
    ));
    assert_eq!(cov.by_severity.get("info"), Some(&1));
    assert_eq!(cov.by_severity.get("error"), Some(&1));
}

#[test]
fn an_unscoped_rule_is_counted_under_star() {
    let mut anywhere = require_policy("file");
    anywhere.resource_type = None;
    let cov = compute_coverage(&make_config(&[("f1", "file")], vec![anywhere]));
    assert_eq!(cov.by_resource_scope.get("*"), Some(&1));
}

#[test]
fn framework_tracking_counts_rules_not_controls() {
    let mut policy = require_policy("file");
    policy.compliance = vec![
        ComplianceMapping {
            framework: "CIS".into(),
            control: "1.1".into(),
        },
        ComplianceMapping {
            framework: "CIS".into(),
            control: "1.2".into(),
        },
    ];
    let cov = compute_coverage(&make_config(&[("f1", "file")], vec![policy]));
    assert_eq!(
        cov.compliance_frameworks.get("CIS"),
        Some(&1),
        "one rule citing two CIS controls is one rule backing CIS"
    );
}

/// REJECTION CRITERION for the id-space mismatch (#356). A rule declared
/// without an explicit `id:` fires here — `f1` has no owner — and must be
/// counted as triggered. The version this replaced compared the rules'
/// `display_id()` against the violations' raw `policy_id: Option<String>`,
/// which is `None` for exactly these rules, so it reported EVERY un-id'd rule
/// as untriggered while `clean_resources` in the same document counted the
/// resource it had just failed.
#[test]
fn an_unnamed_rule_that_fires_is_not_reported_as_untriggered() {
    let mut anonymous = require_policy("file");
    anonymous.id = None;
    anonymous.message = "files need an owner".into();

    let cov = compute_coverage(&make_config(&[("f1", "file")], vec![anonymous]));

    assert_eq!(cov.clean_resources, 0, "the rule did fire: {cov:?}");
    assert_eq!(
        cov.rules_triggered, 1,
        "an un-id'd rule that fired was counted as never triggered: {cov:?}"
    );
    assert!(
        cov.untriggered_rules.is_empty(),
        "`{:?}` fired and is still listed as untriggered",
        cov.untriggered_rules
    );
}

#[test]
fn a_satisfied_rule_is_untriggered() {
    // `f1` has no owner, so the require-owner rule fires against it.
    let cov = compute_coverage(&make_config(
        &[("f1", "file")],
        vec![require_policy("file"), require_policy("package")],
    ));
    assert_eq!(cov.rules_triggered, 1);
    assert_eq!(cov.untriggered_rules, vec!["P-package"]);
}

/// The field that makes the two halves distinguishable. `pkg` is in no rule's
/// scope, so it is UNCOVERED and CLEAN at the same time — the exact case a
/// coverage report exists to surface, and the case that made the two former
/// calculations both print "1 of 2" while pointing at opposite resources.
#[test]
fn clean_is_not_covered() {
    let cov = compute_coverage(&make_config(
        &[("conf", "file"), ("pkg", "package")],
        vec![require_policy("file")],
    ));
    assert_eq!(cov.covered_resources, 1);
    assert_eq!(cov.uncovered, vec!["pkg"]);
    assert_eq!(cov.clean_resources, 1);
    assert_eq!(
        cov.per_resource.keys().collect::<Vec<_>>(),
        vec!["conf"],
        "the covered resource is `conf`; the clean one is `pkg`"
    );
}

// ── renderers ───────────────────────────────────────────────────────

#[test]
fn format_report() {
    let cov = compute_coverage(&make_config(
        &[("f1", "file"), ("p1", "package")],
        vec![require_policy("file")],
    ));
    let report = format_coverage(&cov);
    assert!(report.contains("50.0%"), "{report}");
    assert!(report.contains("p1"), "{report}");
}

#[test]
fn json_output_is_the_struct_itself() {
    let cov = compute_coverage(&make_config(
        &[("f1", "file")],
        vec![require_policy("file")],
    ));
    let json = coverage_to_json(&cov);
    assert_eq!(json["total_resources"], 1);
    assert_eq!(json["fully_covered"], true);
    assert_eq!(
        json,
        serde_json::to_value(&cov).unwrap(),
        "coverage_to_json must not be a second hand-written projection"
    );
}

/// A round trip proves the JSON carries the whole report — a `#[serde(skip)]`
/// slipped onto a field would make the verb answer with less than the CLI has.
#[test]
fn the_json_document_round_trips() {
    let cov = compute_coverage(&make_config(
        &[("conf", "file"), ("pkg", "package")],
        vec![require_policy("file")],
    ));
    let back: PolicyCoverage = serde_json::from_value(coverage_to_json(&cov)).unwrap();
    assert_eq!(back, cov);
}
