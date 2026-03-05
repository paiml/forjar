//! FJ-2203: Handler contract types — ResourceHandler trait, hash invariants.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2203: Handler hash invariant assertion.
///
/// Records whether a handler's stored hash matches `hash_desired_state()`.
/// Used by `debug_assert_eq` in the executor and by `forjar contracts --coverage`.
///
/// # Examples
///
/// ```
/// use forjar::core::types::HashInvariantCheck;
///
/// let check = HashInvariantCheck::pass("nginx-pkg", "package", "blake3:abc123");
/// assert!(check.passed);
///
/// let check = HashInvariantCheck::fail(
///     "cron-job", "cron",
///     "blake3:aaa", "blake3:bbb",
///     "cron handler uses schedule hash, not full resource hash",
/// );
/// assert!(!check.passed);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashInvariantCheck {
    /// Resource identifier.
    pub resource_id: String,
    /// Resource type (file, package, service, etc.).
    pub resource_type: String,
    /// Whether the invariant holds.
    pub passed: bool,
    /// Hash from `hash_desired_state()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_hash: Option<String>,
    /// Hash actually stored by the handler.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_hash: Option<String>,
    /// Deviation reason (if invariant fails but is documented/acceptable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deviation_reason: Option<String>,
}

impl HashInvariantCheck {
    /// Create a passing check.
    pub fn pass(resource_id: &str, resource_type: &str, hash: &str) -> Self {
        Self {
            resource_id: resource_id.to_string(),
            resource_type: resource_type.to_string(),
            passed: true,
            expected_hash: Some(hash.to_string()),
            actual_hash: Some(hash.to_string()),
            deviation_reason: None,
        }
    }

    /// Create a failing check with deviation reason.
    pub fn fail(
        resource_id: &str,
        resource_type: &str,
        expected: &str,
        actual: &str,
        reason: &str,
    ) -> Self {
        Self {
            resource_id: resource_id.to_string(),
            resource_type: resource_type.to_string(),
            passed: false,
            expected_hash: Some(expected.to_string()),
            actual_hash: Some(actual.to_string()),
            deviation_reason: Some(reason.to_string()),
        }
    }
}

impl fmt::Display for HashInvariantCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.passed { "PASS" } else { "FAIL" };
        write!(f, "[{status}] {} ({})", self.resource_id, self.resource_type)?;
        if let Some(ref reason) = self.deviation_reason {
            write!(f, " — {reason}")?;
        }
        Ok(())
    }
}

/// FJ-2203: Handler audit report — result of auditing all resource handlers.
///
/// # Examples
///
/// ```
/// use forjar::core::types::{HandlerAuditReport, HashInvariantCheck};
///
/// let report = HandlerAuditReport {
///     checks: vec![
///         HashInvariantCheck::pass("pkg", "package", "blake3:abc"),
///         HashInvariantCheck::pass("file", "file", "blake3:def"),
///     ],
///     exemptions: vec![],
/// };
/// assert!(report.all_passed());
/// assert_eq!(report.pass_count(), 2);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerAuditReport {
    /// Per-handler invariant checks.
    pub checks: Vec<HashInvariantCheck>,
    /// Documented exemptions (handlers that intentionally deviate).
    pub exemptions: Vec<HandlerExemption>,
}

impl HandlerAuditReport {
    /// Number of passing checks.
    pub fn pass_count(&self) -> usize {
        self.checks.iter().filter(|c| c.passed).count()
    }

    /// Number of failing checks.
    pub fn fail_count(&self) -> usize {
        self.checks.iter().filter(|c| !c.passed).count()
    }

    /// Whether all non-exempt checks pass.
    pub fn all_passed(&self) -> bool {
        self.fail_count() == 0
    }

    /// Format as a human-readable report.
    pub fn format_report(&self) -> String {
        let mut out = String::from("Handler Hash Invariant Audit\n");
        out.push_str(&format!(
            "Checks: {} total, {} passed, {} failed\n",
            self.checks.len(),
            self.pass_count(),
            self.fail_count(),
        ));
        if !self.exemptions.is_empty() {
            out.push_str(&format!(
                "Exemptions: {} documented\n",
                self.exemptions.len(),
            ));
        }
        out.push_str("---\n");
        for check in &self.checks {
            out.push_str(&format!("  {check}\n"));
        }
        for exempt in &self.exemptions {
            out.push_str(&format!("  [EXEMPT] {} — {}\n", exempt.handler, exempt.reason));
        }
        out
    }
}

/// FJ-2203: Documented handler exemption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerExemption {
    /// Handler/resource type name.
    pub handler: String,
    /// Why this handler is exempt from the hash invariant.
    pub reason: String,
    /// Who approved the exemption.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
}

/// FJ-2200: Runtime contract assertion result.
///
/// Records the outcome of a `#[debug_ensures]` or `#[debug_requires]` check
/// at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAssertion {
    /// Function name.
    pub function: String,
    /// Module path.
    pub module: String,
    /// Contract kind (requires, ensures, invariant).
    pub kind: ContractKind,
    /// Whether the assertion held.
    pub held: bool,
    /// Contract expression (human-readable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
}

/// Kind of contract assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContractKind {
    /// Precondition (`#[requires]`).
    Requires,
    /// Postcondition (`#[ensures]`).
    Ensures,
    /// Loop or type invariant (`#[invariant]`).
    Invariant,
}

impl fmt::Display for ContractKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Requires => write!(f, "requires"),
            Self::Ensures => write!(f, "ensures"),
            Self::Invariant => write!(f, "invariant"),
        }
    }
}

/// FJ-2201: Kani proof harness metadata.
///
/// Tracks which Kani harnesses exist, what property they prove,
/// and their current verification status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaniHarness {
    /// Harness function name.
    pub name: String,
    /// Property being proved.
    pub property: String,
    /// Target function (the function being verified).
    pub target_function: String,
    /// Verification status.
    pub status: ProofStatus,
    /// Bound depth (number of symbolic elements).
    #[serde(default)]
    pub bound: Option<u32>,
}

/// Proof verification status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProofStatus {
    /// Proof passes (property verified).
    Verified,
    /// Proof fails (counterexample found).
    Failed,
    /// Proof is pending (not yet run).
    Pending,
    /// Proof timed out (inconclusive).
    Timeout,
    /// Proof is deprecated (kept for reference).
    Deprecated,
}

impl fmt::Display for ProofStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Verified => write!(f, "verified"),
            Self::Failed => write!(f, "failed"),
            Self::Pending => write!(f, "pending"),
            Self::Timeout => write!(f, "timeout"),
            Self::Deprecated => write!(f, "deprecated"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_invariant_pass() {
        let c = HashInvariantCheck::pass("pkg", "package", "blake3:abc");
        assert!(c.passed);
        assert_eq!(c.expected_hash, c.actual_hash);
        assert!(c.deviation_reason.is_none());
    }

    #[test]
    fn hash_invariant_fail() {
        let c = HashInvariantCheck::fail("cron", "cron", "blake3:a", "blake3:b", "schedule only");
        assert!(!c.passed);
        assert_ne!(c.expected_hash, c.actual_hash);
        assert_eq!(c.deviation_reason.as_deref(), Some("schedule only"));
    }

    #[test]
    fn hash_invariant_display() {
        let pass = HashInvariantCheck::pass("pkg", "package", "h");
        assert!(pass.to_string().contains("[PASS]"));

        let fail = HashInvariantCheck::fail("c", "cron", "a", "b", "reason");
        assert!(fail.to_string().contains("[FAIL]"));
        assert!(fail.to_string().contains("reason"));
    }

    #[test]
    fn handler_audit_report_all_pass() {
        let report = HandlerAuditReport {
            checks: vec![
                HashInvariantCheck::pass("a", "file", "h1"),
                HashInvariantCheck::pass("b", "package", "h2"),
            ],
            exemptions: vec![],
        };
        assert!(report.all_passed());
        assert_eq!(report.pass_count(), 2);
        assert_eq!(report.fail_count(), 0);
    }

    #[test]
    fn handler_audit_report_with_failure() {
        let report = HandlerAuditReport {
            checks: vec![
                HashInvariantCheck::pass("a", "file", "h1"),
                HashInvariantCheck::fail("b", "cron", "h2", "h3", "deviation"),
            ],
            exemptions: vec![],
        };
        assert!(!report.all_passed());
        assert_eq!(report.fail_count(), 1);
    }

    #[test]
    fn handler_audit_report_format() {
        let report = HandlerAuditReport {
            checks: vec![HashInvariantCheck::pass("pkg", "package", "h")],
            exemptions: vec![HandlerExemption {
                handler: "task".into(),
                reason: "imperative by nature".into(),
                approved_by: Some("spec review".into()),
            }],
        };
        let s = report.format_report();
        assert!(s.contains("Handler Hash Invariant Audit"));
        assert!(s.contains("1 passed"));
        assert!(s.contains("[EXEMPT] task"));
    }

    #[test]
    fn contract_kind_display() {
        assert_eq!(ContractKind::Requires.to_string(), "requires");
        assert_eq!(ContractKind::Ensures.to_string(), "ensures");
        assert_eq!(ContractKind::Invariant.to_string(), "invariant");
    }

    #[test]
    fn contract_assertion_serde() {
        let a = ContractAssertion {
            function: "determine_present_action".into(),
            module: "core::planner".into(),
            kind: ContractKind::Ensures,
            held: true,
            expression: Some("result.is_noop() || result.is_apply()".into()),
        };
        let json = serde_json::to_string(&a).unwrap();
        let parsed: ContractAssertion = serde_json::from_str(&json).unwrap();
        assert!(parsed.held);
        assert_eq!(parsed.kind, ContractKind::Ensures);
    }

    #[test]
    fn kani_harness_serde() {
        let h = KaniHarness {
            name: "proof_blake3_idempotency".into(),
            property: "hashing is deterministic".into(),
            target_function: "blake3::hash".into(),
            status: ProofStatus::Verified,
            bound: Some(16),
        };
        let json = serde_json::to_string(&h).unwrap();
        let parsed: KaniHarness = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, ProofStatus::Verified);
    }

    #[test]
    fn proof_status_display() {
        assert_eq!(ProofStatus::Verified.to_string(), "verified");
        assert_eq!(ProofStatus::Deprecated.to_string(), "deprecated");
    }

    #[test]
    fn hash_invariant_serde_roundtrip() {
        let c = HashInvariantCheck::fail("r", "t", "a", "b", "reason");
        let json = serde_json::to_string(&c).unwrap();
        let parsed: HashInvariantCheck = serde_json::from_str(&json).unwrap();
        assert!(!parsed.passed);
        assert_eq!(parsed.deviation_reason.as_deref(), Some("reason"));
    }
}
