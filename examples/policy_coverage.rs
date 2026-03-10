#![allow(clippy::field_reassign_with_default)]
//! Example: Policy coverage report (FJ-3208)
//!
//! Demonstrates computing policy coverage — which resources
//! have policies applied and which are uncovered.
//!
//! ```bash
//! cargo run --example policy_coverage
//! ```

use forjar::core::policy_coverage::{compute_coverage, coverage_to_json, format_coverage};
use forjar::core::types::{
    ComplianceMapping, ForjarConfig, PolicyRule, PolicyRuleType, PolicySeverity, Resource,
    ResourceType,
};

fn main() {
    println!("=== Policy Coverage Report (FJ-3208) ===\n");

    // Build a config with mixed resources
    let mut config = ForjarConfig::default();

    // Resources
    let mut nginx = Resource::default();
    nginx.resource_type = ResourceType::File;
    nginx.owner = Some("root".into());
    nginx.tags = vec!["web".into(), "config".into()];
    config.resources.insert("nginx-conf".into(), nginx);

    let mut sshd = Resource::default();
    sshd.resource_type = ResourceType::File;
    sshd.owner = Some("root".into());
    sshd.tags = vec!["security".into()];
    config.resources.insert("sshd-config".into(), sshd);

    let mut docker = Resource::default();
    docker.resource_type = ResourceType::Package;
    config.resources.insert("docker-ce".into(), docker);

    let mut redis = Resource::default();
    redis.resource_type = ResourceType::Service;
    config.resources.insert("redis-server".into(), redis);

    let mut backup = Resource::default();
    backup.resource_type = ResourceType::Cron;
    config.resources.insert("backup-job".into(), backup);

    // Policies — only covering file and package resources
    config.policies = vec![
        PolicyRule {
            id: Some("SEC-001".into()),
            rule_type: PolicyRuleType::Require,
            message: "All files must have an owner".into(),
            resource_type: Some("file".into()),
            tag: None,
            field: Some("owner".into()),
            condition_field: None,
            condition_value: None,
            max_count: None,
            min_count: None,
            severity: Some(PolicySeverity::Error),
            remediation: Some("Add owner field".into()),
            compliance: vec![ComplianceMapping {
                framework: "CIS".into(),
                control: "6.1.1".into(),
            }],
        },
        PolicyRule {
            id: Some("SEC-002".into()),
            rule_type: PolicyRuleType::Deny,
            message: "No wildcard packages".into(),
            resource_type: Some("package".into()),
            tag: None,
            field: Some("name".into()),
            condition_field: None,
            condition_value: None,
            max_count: None,
            min_count: None,
            severity: Some(PolicySeverity::Warning),
            remediation: None,
            compliance: vec![],
        },
        PolicyRule {
            id: Some("SEC-003".into()),
            rule_type: PolicyRuleType::Assert,
            message: "Web resources must be tagged".into(),
            resource_type: None,
            tag: Some("web".into()),
            field: Some("tags".into()),
            condition_field: None,
            condition_value: None,
            max_count: None,
            min_count: None,
            severity: Some(PolicySeverity::Error),
            remediation: None,
            compliance: vec![ComplianceMapping {
                framework: "SOC2".into(),
                control: "CC6.1".into(),
            }],
        },
    ];

    // 1. Compute coverage
    println!("1. Resources:");
    for (id, r) in &config.resources {
        println!(
            "   {id:<18} type={:<10} tags={:?}",
            format!("{:?}", r.resource_type).to_lowercase(),
            r.tags
        );
    }

    println!("\n2. Policies:");
    for p in &config.policies {
        println!(
            "   {:<10} [{:<7}] scope={} {}",
            p.id.as_deref().unwrap_or("-"),
            format!("{:?}", p.rule_type).to_lowercase(),
            p.resource_type.as_deref().unwrap_or("*"),
            p.message
        );
    }

    let coverage = compute_coverage(&config);

    // 2. Text report
    println!("\n3. Coverage Report:");
    println!("{}", format_coverage(&coverage));

    // 3. JSON output
    println!("\n4. JSON Output:");
    let json = coverage_to_json(&coverage);
    println!("{}", serde_json::to_string_pretty(&json).unwrap());

    println!("\nDone.");
}
