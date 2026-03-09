//! FJ-220 + FJ-3200: Policy-as-Code types.
//!
//! Defines policy rules for plan-time enforcement, with FJ-3200 extensions
//! for compliance IDs, remediation hints, assert/limit types, and SARIF output.

use serde::{Deserialize, Serialize};

// ============================================================================
// Policy Rule
// ============================================================================

/// A policy rule for plan-time enforcement.
///
/// # FJ-220 Base
/// Supports `require` (field must exist), `deny` (block on match), and `warn`.
///
/// # FJ-3200 Extensions
/// Adds `assert` (condition must be true) and `limit` (bound checking),
/// plus `id`, `severity`, `remediation`, and `compliance` mapping fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule type: `require`, `deny`, `warn`, `assert`, or `limit`
    #[serde(rename = "type")]
    pub rule_type: PolicyRuleType,

    /// Human-readable description of what this rule checks
    pub message: String,

    /// FJ-3200: Stable policy identifier (e.g., "SEC-001", "PERF-003")
    #[serde(default)]
    pub id: Option<String>,

    /// Resource type filter (e.g., "file", "package"). None = all types.
    #[serde(default)]
    pub resource_type: Option<String>,

    /// Tag filter — only check resources with this tag
    #[serde(default)]
    pub tag: Option<String>,

    /// For `require`: field that must be set (e.g., "owner", "tags", "mode")
    #[serde(default)]
    pub field: Option<String>,

    /// For `deny`/`warn`/`assert`: field to check
    #[serde(default)]
    pub condition_field: Option<String>,

    /// For `deny`/`warn`: value that triggers the rule (equality check)
    /// For `assert`: value that must match (inverted — violation if NOT equal)
    #[serde(default)]
    pub condition_value: Option<String>,

    /// FJ-3200: For `limit` type — maximum count of items in a list field
    #[serde(default)]
    pub max_count: Option<usize>,

    /// FJ-3200: For `limit` type — minimum count of items in a list field
    #[serde(default)]
    pub min_count: Option<usize>,

    /// FJ-3200: Severity level (independent of rule type)
    /// Defaults: deny/assert → error, require → error, warn → warning, limit → warning
    #[serde(default)]
    pub severity: Option<PolicySeverity>,

    /// FJ-3200: How to fix the violation
    #[serde(default)]
    pub remediation: Option<String>,

    /// FJ-3200: Compliance framework mappings
    #[serde(default)]
    pub compliance: Vec<ComplianceMapping>,
}

// ============================================================================
// Policy Rule Type
// ============================================================================

/// Policy rule type — determines evaluation semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyRuleType {
    /// Resource must have a field set
    Require,
    /// Block apply if condition matches
    Deny,
    /// Advisory warning (does not block)
    Warn,
    /// FJ-3200: Condition must be true (violation if field != expected value)
    Assert,
    /// FJ-3200: Field count/value must be within bounds
    Limit,
}

// ============================================================================
// Policy Severity
// ============================================================================

/// FJ-3200: Policy violation severity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicySeverity {
    /// Blocks apply — must be fixed before convergence
    Error,
    /// Logged as warning — apply proceeds
    Warning,
    /// Informational — advisory only
    Info,
}

impl PolicyRule {
    /// Effective severity: explicit if set, else derived from rule type.
    pub fn effective_severity(&self) -> PolicySeverity {
        if let Some(ref s) = self.severity {
            return s.clone();
        }
        match self.rule_type {
            PolicyRuleType::Deny | PolicyRuleType::Assert | PolicyRuleType::Require => {
                PolicySeverity::Error
            }
            PolicyRuleType::Warn | PolicyRuleType::Limit => PolicySeverity::Warning,
        }
    }

    /// Stable display ID: the explicit id or a generated one from the message.
    pub fn display_id(&self) -> String {
        if let Some(ref id) = self.id {
            id.clone()
        } else {
            // Generate from first 20 chars of message, slugified
            let slug: String = self
                .message
                .chars()
                .take(20)
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect();
            format!("RULE-{slug}")
        }
    }
}

// ============================================================================
// Compliance Mapping
// ============================================================================

/// FJ-3200: Mapping from a policy rule to an external compliance framework.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceMapping {
    /// Framework name (e.g., "cis", "stig", "soc2", "pci-dss")
    pub framework: String,
    /// Control identifier within the framework (e.g., "6.1.2", "V-238196")
    pub control: String,
}

// ============================================================================
// Policy Violation
// ============================================================================

/// Result of evaluating a policy rule against a resource.
#[derive(Debug, Clone)]
pub struct PolicyViolation {
    /// Rule that was violated
    pub rule_message: String,
    /// Resource that violated the rule
    pub resource_id: String,
    /// Rule type that was violated
    pub rule_type: PolicyRuleType,
    /// Effective severity
    pub severity: PolicySeverity,
    /// FJ-3200: Policy ID (if set)
    pub policy_id: Option<String>,
    /// FJ-3200: Remediation hint
    pub remediation: Option<String>,
    /// FJ-3200: Compliance mappings
    pub compliance: Vec<ComplianceMapping>,
}

impl PolicyViolation {
    /// True if this violation should block apply.
    pub fn is_blocking(&self) -> bool {
        self.severity == PolicySeverity::Error
    }
}

// ============================================================================
// Policy Check Result
// ============================================================================

/// FJ-3200: Aggregate result of evaluating all policy rules.
#[derive(Debug, Clone)]
pub struct PolicyCheckResult {
    /// All violations found
    pub violations: Vec<PolicyViolation>,
    /// Total rules evaluated
    pub rules_evaluated: usize,
    /// Total resources checked
    pub resources_checked: usize,
}

impl PolicyCheckResult {
    /// True if any violation is blocking (error severity).
    pub fn has_blocking_violations(&self) -> bool {
        self.violations.iter().any(|v| v.is_blocking())
    }

    /// Count of error-severity violations.
    pub fn error_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == PolicySeverity::Error)
            .count()
    }

    /// Count of warning-severity violations.
    pub fn warning_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == PolicySeverity::Warning)
            .count()
    }

    /// Count of info-severity violations.
    pub fn info_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == PolicySeverity::Info)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_severity_defaults() {
        let deny = PolicyRule {
            rule_type: PolicyRuleType::Deny,
            message: "test".into(),
            id: None,
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
        };
        assert_eq!(deny.effective_severity(), PolicySeverity::Error);

        let warn = PolicyRule {
            rule_type: PolicyRuleType::Warn,
            severity: None,
            ..deny.clone()
        };
        assert_eq!(warn.effective_severity(), PolicySeverity::Warning);

        let assert_r = PolicyRule {
            rule_type: PolicyRuleType::Assert,
            severity: None,
            ..deny.clone()
        };
        assert_eq!(assert_r.effective_severity(), PolicySeverity::Error);

        let limit = PolicyRule {
            rule_type: PolicyRuleType::Limit,
            severity: None,
            ..deny.clone()
        };
        assert_eq!(limit.effective_severity(), PolicySeverity::Warning);

        let require = PolicyRule {
            rule_type: PolicyRuleType::Require,
            severity: None,
            ..deny.clone()
        };
        assert_eq!(require.effective_severity(), PolicySeverity::Error);
    }

    #[test]
    fn test_effective_severity_override() {
        let rule = PolicyRule {
            rule_type: PolicyRuleType::Deny,
            message: "test".into(),
            id: None,
            resource_type: None,
            tag: None,
            field: None,
            condition_field: None,
            condition_value: None,
            max_count: None,
            min_count: None,
            severity: Some(PolicySeverity::Info),
            remediation: None,
            compliance: vec![],
        };
        assert_eq!(rule.effective_severity(), PolicySeverity::Info);
    }

    #[test]
    fn test_display_id_explicit() {
        let rule = PolicyRule {
            rule_type: PolicyRuleType::Deny,
            message: "no root".into(),
            id: Some("SEC-001".into()),
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
        };
        assert_eq!(rule.display_id(), "SEC-001");
    }

    #[test]
    fn test_display_id_generated() {
        let rule = PolicyRule {
            rule_type: PolicyRuleType::Warn,
            message: "files should have owner".into(),
            id: None,
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
        };
        assert_eq!(rule.display_id(), "RULE-files-should-have-ow");
    }

    #[test]
    fn test_violation_is_blocking() {
        let v = PolicyViolation {
            rule_message: "test".into(),
            resource_id: "r1".into(),
            rule_type: PolicyRuleType::Deny,
            severity: PolicySeverity::Error,
            policy_id: None,
            remediation: None,
            compliance: vec![],
        };
        assert!(v.is_blocking());

        let v2 = PolicyViolation {
            severity: PolicySeverity::Warning,
            ..v.clone()
        };
        assert!(!v2.is_blocking());
    }

    #[test]
    fn test_policy_check_result_counts() {
        let result = PolicyCheckResult {
            violations: vec![
                PolicyViolation {
                    rule_message: "e1".into(),
                    resource_id: "r1".into(),
                    rule_type: PolicyRuleType::Deny,
                    severity: PolicySeverity::Error,
                    policy_id: None,
                    remediation: None,
                    compliance: vec![],
                },
                PolicyViolation {
                    rule_message: "w1".into(),
                    resource_id: "r2".into(),
                    rule_type: PolicyRuleType::Warn,
                    severity: PolicySeverity::Warning,
                    policy_id: None,
                    remediation: None,
                    compliance: vec![],
                },
                PolicyViolation {
                    rule_message: "i1".into(),
                    resource_id: "r3".into(),
                    rule_type: PolicyRuleType::Warn,
                    severity: PolicySeverity::Info,
                    policy_id: None,
                    remediation: None,
                    compliance: vec![],
                },
            ],
            rules_evaluated: 5,
            resources_checked: 10,
        };
        assert!(result.has_blocking_violations());
        assert_eq!(result.error_count(), 1);
        assert_eq!(result.warning_count(), 1);
        assert_eq!(result.info_count(), 1);
    }

    #[test]
    fn test_policy_check_result_no_blocking() {
        let result = PolicyCheckResult {
            violations: vec![PolicyViolation {
                rule_message: "w1".into(),
                resource_id: "r1".into(),
                rule_type: PolicyRuleType::Warn,
                severity: PolicySeverity::Warning,
                policy_id: None,
                remediation: None,
                compliance: vec![],
            }],
            rules_evaluated: 1,
            resources_checked: 1,
        };
        assert!(!result.has_blocking_violations());
    }

    #[test]
    fn test_compliance_mapping_serde() {
        let m = ComplianceMapping {
            framework: "cis".into(),
            control: "6.1.2".into(),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("cis"));
        assert!(json.contains("6.1.2"));
    }

    #[test]
    fn test_policy_severity_serde() {
        let s = PolicySeverity::Error;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"error\"");
        let w: PolicySeverity = serde_json::from_str("\"warning\"").unwrap();
        assert_eq!(w, PolicySeverity::Warning);
    }

    #[test]
    fn test_policy_rule_type_serde_new_variants() {
        let a: PolicyRuleType = serde_json::from_str("\"assert\"").unwrap();
        assert_eq!(a, PolicyRuleType::Assert);
        let l: PolicyRuleType = serde_json::from_str("\"limit\"").unwrap();
        assert_eq!(l, PolicyRuleType::Limit);
    }
}
