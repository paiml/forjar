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
    ///
    /// NOT an identity within a config — see [`PolicyRule::display_id_at`].
    /// Two rules that declare no `id:` and share a `message:` return the same
    /// string from here, which is why nothing keys a per-rule map on it.
    pub fn display_id(&self) -> String {
        display_id_of(self.id.as_deref(), &self.message)
    }

    /// THE rule's identity within a config: its explicit `id:` when it declares
    /// one, else `RULE-<index>-<slug>` built from its position in `policies:`.
    ///
    /// [`PolicyRule::display_id`] derives the generated half from the message
    /// alone, and a message is not unique: two un-id'd rules sharing one
    /// collapse to a single string. `policy-coverage` counted DISTINCT such
    /// strings, so a rule that never ran was neither counted nor listed
    /// (`total_rules: 2, rules_triggered: 1, untriggered_rules: []`), and
    /// `remediate` keyed its selector and its reason map on it, so
    /// `--policy-id RULE-<slug>` edited a rule the caller did not select and
    /// reported one rule's "why not" under the other's name (paiml/forjar#369).
    ///
    /// The index is a total, injective function of the declaration, so no two
    /// rules that declare NO `id:` can collide — and every call site already
    /// holds it: `parser::violating_pairs` yields it, `remediate::Candidate`
    /// stores it, and `policies.iter().enumerate()` produces it.
    ///
    /// This is not injective over a config that declares the same `id:` twice,
    /// because an explicit id is returned verbatim. Nothing in `validate` or
    /// `lint` diagnoses that, and the #369 shape survives it on the `remediate`
    /// surface: measured on this branch, two `assert` rules both declaring
    /// `id: SEC-1` make `--policy-id SEC-1` rewrite BOTH `mode` and `owner`,
    /// and an unfixable rule still reports its twin's reason. `policy-coverage`
    /// is unaffected — its arithmetic is derived from the index, not from this
    /// string. Closing the rest means keying `remediate`'s `ReasonMap` on the
    /// index and diagnosing duplicate ids at parse time; both are outside
    /// paiml/forjar#369, which is about rules that declare no id at all.
    pub fn display_id_at(&self, index: usize) -> String {
        match self.id.as_deref() {
            Some(id) => id.to_string(),
            None => format!("RULE-{index}-{}", message_slug(&self.message)),
        }
    }
}

/// THE display-id derivation, shared by [`PolicyRule::display_id`] and
/// [`PolicyViolation::display_id`].
///
/// It has to be shared. `policy-coverage` decides which rules fired by
/// intersecting rule ids with violation ids, and until paiml/forjar#356 the
/// rule side used `display_id()` while the violation side used the raw
/// `Option<String>`. For a rule with no explicit `id:` those never intersect,
/// so EVERY un-id'd rule was reported as untriggered — including in a report
/// that, two lines above, counted the resource it had just failed.
fn display_id_of(id: Option<&str>, message: &str) -> String {
    if let Some(id) = id {
        return id.to_string();
    }
    format!("RULE-{}", message_slug(message))
}

/// The first 40 characters of a message, slugified.
///
/// A slug of PROSE, and prose is not unique — which is exactly why
/// [`PolicyRule::display_id_at`] prefixes it with the rule's index rather than
/// using it alone.
fn message_slug(message: &str) -> String {
    message
        .chars()
        .take(40)
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect()
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

    /// The id of the rule this violation came from, derived exactly as
    /// [`PolicyRule::display_id`] derives it — so the two sets can be compared.
    ///
    /// NOT the same as reading `policy_id`: that is `None` for a rule declared
    /// without an explicit `id:`, and `None` matches no rule.
    pub fn display_id(&self) -> String {
        display_id_of(self.policy_id.as_deref(), &self.rule_message)
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
#[path = "tests_policy_rule_types.rs"]
mod tests;
