//! FJ-3208: Policy coverage report.
//!
//! Analyzes which resources have policies applied and which are
//! uncovered. Reports coverage percentage, uncovered resources,
//! and policy distribution by type.

use crate::core::types::{ForjarConfig, PolicyRule, PolicyRuleType};
use std::collections::{HashMap, HashSet};

/// Result of computing policy coverage.
#[derive(Debug, Clone)]
pub struct PolicyCoverage {
    /// Total resources in the config.
    pub total_resources: usize,
    /// Resources with at least one policy.
    pub covered_resources: usize,
    /// Resource IDs with no policies.
    pub uncovered: Vec<String>,
    /// Per-resource policy count.
    pub per_resource: HashMap<String, usize>,
    /// Policy count by type.
    pub by_type: HashMap<String, usize>,
    /// Compliance framework coverage.
    pub frameworks: HashSet<String>,
}

impl PolicyCoverage {
    /// Coverage percentage (0.0 - 100.0).
    pub fn coverage_percent(&self) -> f64 {
        if self.total_resources == 0 {
            return 100.0;
        }
        (self.covered_resources as f64 / self.total_resources as f64) * 100.0
    }

    /// Whether all resources have at least one policy.
    pub fn fully_covered(&self) -> bool {
        self.uncovered.is_empty()
    }
}

/// Compute policy coverage for a config.
pub fn compute_coverage(config: &ForjarConfig) -> PolicyCoverage {
    let resource_ids: Vec<String> = config.resources.keys().cloned().collect();
    let total_resources = resource_ids.len();

    let mut per_resource: HashMap<String, usize> = HashMap::new();
    let mut by_type: HashMap<String, usize> = HashMap::new();
    let mut frameworks: HashSet<String> = HashSet::new();

    for policy in &config.policies {
        let type_name = policy_type_name(&policy.rule_type);
        *by_type.entry(type_name).or_insert(0) += 1;

        // Extract compliance framework names
        for mapping in &policy.compliance {
            frameworks.insert(mapping.framework.clone());
        }

        // Determine which resources this policy covers
        let covered = matching_resources(config, policy);
        for rid in covered {
            *per_resource.entry(rid).or_insert(0) += 1;
        }
    }

    let covered_resources = per_resource.len();
    let mut uncovered: Vec<String> = resource_ids
        .into_iter()
        .filter(|id| !per_resource.contains_key(id))
        .collect();
    uncovered.sort();

    PolicyCoverage {
        total_resources,
        covered_resources,
        uncovered,
        per_resource,
        by_type,
        frameworks,
    }
}

/// Determine which resource IDs a policy applies to.
fn matching_resources(config: &ForjarConfig, policy: &PolicyRule) -> Vec<String> {
    let mut matched = Vec::new();

    for (id, resource) in &config.resources {
        // Check resource_type scope
        if let Some(ref scope_type) = policy.resource_type {
            let rtype = format!("{:?}", resource.resource_type).to_lowercase();
            if !rtype.contains(&scope_type.to_lowercase()) {
                continue;
            }
        }

        // Check tag scope
        if let Some(ref scope_tag) = policy.tag {
            if !resource.tags.iter().any(|t| t == scope_tag) {
                continue;
            }
        }

        matched.push(id.clone());
    }

    matched
}

fn policy_type_name(rt: &PolicyRuleType) -> String {
    match rt {
        PolicyRuleType::Require => "require".into(),
        PolicyRuleType::Deny => "deny".into(),
        PolicyRuleType::Warn => "warn".into(),
        PolicyRuleType::Assert => "assert".into(),
        PolicyRuleType::Limit => "limit".into(),
    }
}

/// Format coverage report as human-readable text.
pub fn format_coverage(cov: &PolicyCoverage) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "Policy Coverage: {:.1}% ({}/{})",
        cov.coverage_percent(),
        cov.covered_resources,
        cov.total_resources
    ));

    if !cov.by_type.is_empty() {
        lines.push("  Policies by type:".into());
        let mut types: Vec<_> = cov.by_type.iter().collect();
        types.sort_by_key(|(k, _)| (*k).clone());
        for (t, count) in types {
            lines.push(format!("    {t}: {count}"));
        }
    }

    if !cov.frameworks.is_empty() {
        let mut fws: Vec<_> = cov.frameworks.iter().cloned().collect();
        fws.sort();
        lines.push(format!("  Frameworks: {}", fws.join(", ")));
    }

    if !cov.uncovered.is_empty() {
        lines.push(format!("  Uncovered ({}):", cov.uncovered.len()));
        for id in &cov.uncovered {
            lines.push(format!("    - {id}"));
        }
    }

    lines.join("\n")
}

/// Format coverage report as JSON.
pub fn coverage_to_json(cov: &PolicyCoverage) -> serde_json::Value {
    serde_json::json!({
        "total_resources": cov.total_resources,
        "covered_resources": cov.covered_resources,
        "coverage_percent": cov.coverage_percent(),
        "fully_covered": cov.fully_covered(),
        "uncovered": cov.uncovered,
        "by_type": cov.by_type,
        "frameworks": cov.frameworks.iter().cloned().collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{
        ComplianceMapping, ForjarConfig, PolicyRule, PolicyRuleType, Resource,
    };

    fn make_config(resources: &[(&str, &str)], policies: Vec<PolicyRule>) -> ForjarConfig {
        let mut config = ForjarConfig::default();
        for (name, rtype) in resources {
            let resource = Resource {
                resource_type: match *rtype {
                    "file" => crate::core::types::ResourceType::File,
                    "package" => crate::core::types::ResourceType::Package,
                    "service" => crate::core::types::ResourceType::Service,
                    _ => crate::core::types::ResourceType::File,
                },
                ..Default::default()
            };
            config.resources.insert(name.to_string(), resource);
        }
        config.policies = policies;
        config
    }

    fn require_policy(rtype: &str) -> PolicyRule {
        PolicyRule {
            id: Some(format!("P-{rtype}")),
            rule_type: PolicyRuleType::Require,
            message: "test".into(),
            resource_type: Some(rtype.into()),
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

    #[test]
    fn full_coverage() {
        let config = make_config(
            &[("f1", "file"), ("f2", "file")],
            vec![require_policy("file")],
        );
        let cov = compute_coverage(&config);
        assert_eq!(cov.total_resources, 2);
        assert_eq!(cov.covered_resources, 2);
        assert!(cov.fully_covered());
        assert!((cov.coverage_percent() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn partial_coverage() {
        let config = make_config(
            &[("f1", "file"), ("p1", "package")],
            vec![require_policy("file")],
        );
        let cov = compute_coverage(&config);
        assert_eq!(cov.covered_resources, 1);
        assert_eq!(cov.uncovered, vec!["p1"]);
        assert!(!cov.fully_covered());
        assert!((cov.coverage_percent() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn no_policies() {
        let config = make_config(&[("f1", "file"), ("p1", "package")], vec![]);
        let cov = compute_coverage(&config);
        assert_eq!(cov.covered_resources, 0);
        assert_eq!(cov.uncovered.len(), 2);
        assert!((cov.coverage_percent() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn no_resources() {
        let config = make_config(&[], vec![require_policy("file")]);
        let cov = compute_coverage(&config);
        assert_eq!(cov.total_resources, 0);
        assert!(cov.fully_covered());
        assert!((cov.coverage_percent() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn by_type_counts() {
        let config = make_config(
            &[("f1", "file")],
            vec![require_policy("file"), require_policy("package")],
        );
        let cov = compute_coverage(&config);
        assert_eq!(cov.by_type.get("require"), Some(&2));
    }

    #[test]
    fn framework_tracking() {
        let mut policy = require_policy("file");
        policy.compliance = vec![ComplianceMapping {
            framework: "CIS".into(),
            control: "1.1".into(),
        }];
        let config = make_config(&[("f1", "file")], vec![policy]);
        let cov = compute_coverage(&config);
        assert!(cov.frameworks.contains("CIS"));
    }

    #[test]
    fn format_report() {
        let config = make_config(
            &[("f1", "file"), ("p1", "package")],
            vec![require_policy("file")],
        );
        let cov = compute_coverage(&config);
        let report = format_coverage(&cov);
        assert!(report.contains("50.0%"));
        assert!(report.contains("p1"));
    }

    #[test]
    fn json_output() {
        let config = make_config(&[("f1", "file")], vec![require_policy("file")]);
        let cov = compute_coverage(&config);
        let json = coverage_to_json(&cov);
        assert_eq!(json["total_resources"], 1);
        assert_eq!(json["fully_covered"], true);
    }

    #[test]
    fn per_resource_multiple_policies() {
        let config = make_config(
            &[("f1", "file")],
            vec![require_policy("file"), require_policy("file")],
        );
        let cov = compute_coverage(&config);
        assert_eq!(cov.per_resource.get("f1"), Some(&2));
    }
}
