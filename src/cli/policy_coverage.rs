//! FJ-3208: `forjar policy-coverage` — policy rule coverage report.
//!
//! Analyzes policy rules against resources to produce a coverage matrix:
//! which rules match which resource types, compliance framework gaps,
//! and overall rule distribution.

use crate::core::parser;
use crate::core::types::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Policy coverage report.
#[derive(Debug)]
struct CoverageReport {
    /// Total policy rules defined.
    total_rules: usize,
    /// Total resources in config.
    total_resources: usize,
    /// Rules that matched at least one resource (triggered violation).
    rules_triggered: usize,
    /// Rules by type.
    by_type: BTreeMap<String, usize>,
    /// Rules by severity.
    by_severity: BTreeMap<String, usize>,
    /// Rules by resource_type scope.
    by_resource_scope: BTreeMap<String, usize>,
    /// Compliance frameworks referenced.
    frameworks: BTreeMap<String, usize>,
    /// Resources with zero violations.
    clean_resources: usize,
    /// Rule IDs that were never triggered.
    untriggered_rules: Vec<String>,
}

/// Run `forjar policy-coverage` — analyze policy rule coverage.
pub(crate) fn cmd_policy_coverage(file: &Path, json: bool) -> Result<(), String> {
    let config = super::helpers::parse_and_validate(file)?;
    let result = parser::evaluate_policies_full(&config);
    let report = build_report(&config, &result);

    if json {
        print_json(&report);
    } else {
        print_table(&report);
    }
    Ok(())
}

fn build_report(config: &ForjarConfig, result: &PolicyCheckResult) -> CoverageReport {
    let mut by_type: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_severity: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_resource_scope: BTreeMap<String, usize> = BTreeMap::new();
    let mut frameworks: BTreeMap<String, usize> = BTreeMap::new();

    for rule in &config.policies {
        let type_key = format!("{:?}", rule.rule_type).to_lowercase();
        *by_type.entry(type_key).or_default() += 1;

        let sev_key = format!("{:?}", rule.effective_severity()).to_lowercase();
        *by_severity.entry(sev_key).or_default() += 1;

        let scope = rule.resource_type.as_deref().unwrap_or("*").to_string();
        *by_resource_scope.entry(scope).or_default() += 1;

        for cm in &rule.compliance {
            *frameworks.entry(cm.framework.clone()).or_default() += 1;
        }
    }

    // Find triggered rule IDs
    let triggered_ids: BTreeSet<String> = result
        .violations
        .iter()
        .filter_map(|v| v.policy_id.clone())
        .collect();

    let all_ids: Vec<String> = config.policies.iter().map(|r| r.display_id()).collect();

    let untriggered_rules: Vec<String> = all_ids
        .into_iter()
        .filter(|id| !triggered_ids.contains(id))
        .collect();

    // Resources with zero violations
    let violated_resources: BTreeSet<&str> = result
        .violations
        .iter()
        .map(|v| v.resource_id.as_str())
        .collect();

    let total_resources = result.resources_checked;
    let clean_resources = total_resources.saturating_sub(violated_resources.len());

    CoverageReport {
        total_rules: config.policies.len(),
        total_resources,
        rules_triggered: triggered_ids.len(),
        by_type,
        by_severity,
        by_resource_scope,
        frameworks,
        clean_resources,
        untriggered_rules,
    }
}

fn print_table(r: &CoverageReport) {
    println!("Policy Coverage Report");
    println!("======================");
    println!();
    println!(
        "Rules: {} total, {} triggered, {} untriggered",
        r.total_rules,
        r.rules_triggered,
        r.untriggered_rules.len()
    );
    println!(
        "Resources: {} total, {} clean (no violations)",
        r.total_resources, r.clean_resources
    );
    println!();

    if !r.by_type.is_empty() {
        println!("By rule type:");
        for (t, n) in &r.by_type {
            println!("  {t:<12} {n}");
        }
        println!();
    }

    if !r.by_severity.is_empty() {
        println!("By severity:");
        for (s, n) in &r.by_severity {
            println!("  {s:<12} {n}");
        }
        println!();
    }

    if !r.by_resource_scope.is_empty() {
        println!("By resource scope:");
        for (s, n) in &r.by_resource_scope {
            println!("  {s:<12} {n}");
        }
        println!();
    }

    if !r.frameworks.is_empty() {
        println!("Compliance frameworks:");
        for (f, n) in &r.frameworks {
            println!("  {f:<12} {n} rule(s)");
        }
        println!();
    }

    if !r.untriggered_rules.is_empty() {
        println!("Untriggered rules (no violations found):");
        for id in &r.untriggered_rules {
            println!("  {id}");
        }
    }
}

fn print_json(r: &CoverageReport) {
    let json = serde_json::json!({
        "total_rules": r.total_rules,
        "total_resources": r.total_resources,
        "rules_triggered": r.rules_triggered,
        "clean_resources": r.clean_resources,
        "by_type": r.by_type,
        "by_severity": r.by_severity,
        "by_resource_scope": r.by_resource_scope,
        "compliance_frameworks": r.frameworks,
        "untriggered_rules": r.untriggered_rules,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&json).unwrap_or_default()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_report_basics() {
        let config = ForjarConfig {
            policies: vec![PolicyRule {
                rule_type: PolicyRuleType::Require,
                message: "need owner".into(),
                id: Some("P-001".into()),
                resource_type: Some("file".into()),
                tag: None,
                field: Some("owner".into()),
                condition_field: None,
                condition_value: None,
                max_count: None,
                min_count: None,
                severity: None,
                remediation: None,
                compliance: vec![ComplianceMapping {
                    framework: "cis".into(),
                    control: "5.1".into(),
                }],
            }],
            ..Default::default()
        };

        let result = PolicyCheckResult {
            violations: vec![],
            rules_evaluated: 1,
            resources_checked: 3,
        };

        let report = build_report(&config, &result);
        assert_eq!(report.total_rules, 1);
        assert_eq!(report.total_resources, 3);
        assert_eq!(report.clean_resources, 3);
        assert_eq!(report.untriggered_rules.len(), 1);
        assert_eq!(report.frameworks["cis"], 1);
        assert_eq!(report.by_type["require"], 1);
        assert_eq!(report.by_resource_scope["file"], 1);
    }

    #[test]
    fn build_report_with_violations() {
        let config = ForjarConfig {
            policies: vec![
                PolicyRule {
                    rule_type: PolicyRuleType::Deny,
                    message: "no root".into(),
                    id: Some("SEC-001".into()),
                    resource_type: None,
                    tag: None,
                    field: None,
                    condition_field: Some("user".into()),
                    condition_value: Some("root".into()),
                    max_count: None,
                    min_count: None,
                    severity: None,
                    remediation: None,
                    compliance: vec![],
                },
                PolicyRule {
                    rule_type: PolicyRuleType::Warn,
                    message: "should have tags".into(),
                    id: Some("QA-001".into()),
                    resource_type: None,
                    tag: None,
                    field: None,
                    condition_field: None,
                    condition_value: None,
                    max_count: None,
                    min_count: None,
                    severity: None,
                    remediation: None,
                    compliance: vec![],
                },
            ],
            ..Default::default()
        };

        let result = PolicyCheckResult {
            violations: vec![PolicyViolation {
                rule_message: "no root".into(),
                resource_id: "r1".into(),
                rule_type: PolicyRuleType::Deny,
                severity: PolicySeverity::Error,
                policy_id: Some("SEC-001".into()),
                remediation: None,
                compliance: vec![],
            }],
            rules_evaluated: 2,
            resources_checked: 5,
        };

        let report = build_report(&config, &result);
        assert_eq!(report.rules_triggered, 1);
        assert_eq!(report.untriggered_rules, vec!["QA-001"]);
        assert_eq!(report.clean_resources, 4);
    }

    #[test]
    fn print_table_no_panic() {
        let report = CoverageReport {
            total_rules: 0,
            total_resources: 0,
            rules_triggered: 0,
            by_type: BTreeMap::new(),
            by_severity: BTreeMap::new(),
            by_resource_scope: BTreeMap::new(),
            frameworks: BTreeMap::new(),
            clean_resources: 0,
            untriggered_rules: vec![],
        };
        print_table(&report); // should not panic
    }

    #[test]
    fn print_json_no_panic() {
        let report = CoverageReport {
            total_rules: 2,
            total_resources: 5,
            rules_triggered: 1,
            by_type: BTreeMap::from([("deny".into(), 1), ("warn".into(), 1)]),
            by_severity: BTreeMap::from([("error".into(), 1), ("warning".into(), 1)]),
            by_resource_scope: BTreeMap::from([("*".into(), 2)]),
            frameworks: BTreeMap::from([("cis".into(), 1)]),
            clean_resources: 4,
            untriggered_rules: vec!["QA-001".into()],
        };
        print_json(&report); // should not panic
    }

    #[test]
    fn build_report_multiple_frameworks() {
        let config = ForjarConfig {
            policies: vec![
                PolicyRule {
                    rule_type: PolicyRuleType::Deny,
                    message: "r1".into(),
                    id: Some("S1".into()),
                    resource_type: None,
                    tag: None,
                    field: None,
                    condition_field: None,
                    condition_value: None,
                    max_count: None,
                    min_count: None,
                    severity: None,
                    remediation: None,
                    compliance: vec![
                        ComplianceMapping {
                            framework: "cis".into(),
                            control: "1.1".into(),
                        },
                        ComplianceMapping {
                            framework: "stig".into(),
                            control: "V-1".into(),
                        },
                    ],
                },
                PolicyRule {
                    rule_type: PolicyRuleType::Assert,
                    message: "r2".into(),
                    id: Some("S2".into()),
                    resource_type: Some("package".into()),
                    tag: None,
                    field: None,
                    condition_field: Some("state".into()),
                    condition_value: Some("installed".into()),
                    max_count: None,
                    min_count: None,
                    severity: Some(PolicySeverity::Info),
                    remediation: None,
                    compliance: vec![ComplianceMapping {
                        framework: "soc2".into(),
                        control: "CC6.1".into(),
                    }],
                },
            ],
            ..Default::default()
        };

        let result = PolicyCheckResult {
            violations: vec![],
            rules_evaluated: 2,
            resources_checked: 3,
        };

        let report = build_report(&config, &result);
        assert_eq!(report.frameworks.len(), 3);
        assert_eq!(report.frameworks["cis"], 1);
        assert_eq!(report.frameworks["stig"], 1);
        assert_eq!(report.frameworks["soc2"], 1);
        assert_eq!(report.by_severity["error"], 1);
        assert_eq!(report.by_severity["info"], 1);
        assert_eq!(report.by_resource_scope["*"], 1);
        assert_eq!(report.by_resource_scope["package"], 1);
    }

    #[test]
    fn cmd_coverage_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = "version: \"1.0\"\nname: test\nmachines:\n  m1:\n    addr: localhost\n    hostname: m1\nresources:\n  cfg:\n    type: file\n    path: /etc/app.conf\n    content: \"key=val\"\npolicies:\n  - type: require\n    message: needs owner\n    field: owner\n";
        std::fs::write(dir.path().join("forjar.yaml"), config).unwrap();
        let result = cmd_policy_coverage(&dir.path().join("forjar.yaml"), true);
        assert!(result.is_ok(), "failed: {:?}", result.err());
    }
}
