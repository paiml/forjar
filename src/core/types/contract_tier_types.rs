//! FJ-2203: Verification tier types — 6-level contract coverage model.
//!
//! Tracks verification maturity from unlabeled (L0) to structurally
//! enforced (L5) per the provable design-by-contract spec.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2203: Verification tier for a contract-annotated function.
///
/// # Examples
///
/// ```
/// use forjar::core::types::VerificationTier;
///
/// let tier = VerificationTier::Bounded;
/// assert_eq!(tier.level(), 3);
/// assert_eq!(tier.label(), "bounded");
/// assert!(tier > VerificationTier::Runtime);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerificationTier {
    /// Level 0: No contract annotation.
    #[default]
    Unlabeled,
    /// Level 1: `#[contract]` macro present, no verification.
    Labeled,
    /// Level 2: `#[ensures]` / `debug_assert!` active at runtime.
    Runtime,
    /// Level 3: Kani bounded model checking harness covers this function.
    Bounded,
    /// Level 4: Verus spec with proof block (machine-checked).
    Proved,
    /// Level 5: Trait + executor enforcement (structurally unbreakable).
    Structural,
}

impl VerificationTier {
    /// Numeric level (0–5).
    pub fn level(self) -> u8 {
        match self {
            Self::Unlabeled => 0,
            Self::Labeled => 1,
            Self::Runtime => 2,
            Self::Bounded => 3,
            Self::Proved => 4,
            Self::Structural => 5,
        }
    }

    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Unlabeled => "unlabeled",
            Self::Labeled => "labeled",
            Self::Runtime => "runtime",
            Self::Bounded => "bounded",
            Self::Proved => "proved",
            Self::Structural => "structural",
        }
    }
}

impl fmt::Display for VerificationTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L{} ({})", self.level(), self.label())
    }
}

/// FJ-2203: Contract entry for a single function on the critical path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEntry {
    /// Function name (e.g., `hash_desired_state`).
    pub function: String,
    /// Module path (e.g., `core::planner`).
    pub module: String,
    /// Contract ID (e.g., `blake3-state-v1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_id: Option<String>,
    /// Current verification tier.
    pub tier: VerificationTier,
    /// What verifies this contract (e.g., `kani::proof_hash_determinism`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verified_by: Vec<String>,
}

impl fmt::Display for ContractEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}: {}", self.module, self.function, self.tier)
    }
}

/// FJ-2203: Handler invariant status for a resource type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerInvariantStatus {
    /// Resource type name (e.g., "file", "package").
    pub resource_type: String,
    /// Verification tier for handler invariant.
    pub tier: VerificationTier,
    /// Whether this handler is exempt from the invariant.
    #[serde(default)]
    pub exempt: bool,
    /// Exemption reason (if exempt).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exemption_reason: Option<String>,
}

impl fmt::Display for HandlerInvariantStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.exempt {
            write!(f, "{}: EXEMPT", self.resource_type)?;
            if let Some(ref reason) = self.exemption_reason {
                write!(f, " ({reason})")?;
            }
            Ok(())
        } else {
            write!(f, "{}: {}", self.resource_type, self.tier)
        }
    }
}

/// FJ-2203: Contract coverage report across the critical path.
///
/// # Examples
///
/// ```
/// use forjar::core::types::{ContractCoverageReport, ContractEntry, VerificationTier};
///
/// let report = ContractCoverageReport {
///     total_functions: 24,
///     entries: vec![
///         ContractEntry {
///             function: "hash_desired_state".into(),
///             module: "core::planner".into(),
///             contract_id: Some("blake3-state-v1".into()),
///             tier: VerificationTier::Bounded,
///             verified_by: vec!["kani::proof_hash_determinism".into()],
///         },
///     ],
///     handler_invariants: vec![],
/// };
/// let hist = report.histogram();
/// assert_eq!(hist[3], 1); // one Bounded entry
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractCoverageReport {
    /// Total functions on the critical path.
    pub total_functions: usize,
    /// Per-function contract entries.
    pub entries: Vec<ContractEntry>,
    /// Per-resource-type handler invariant status.
    pub handler_invariants: Vec<HandlerInvariantStatus>,
}

impl ContractCoverageReport {
    /// Count of entries at each tier (histogram indexed by tier level).
    pub fn histogram(&self) -> [usize; 6] {
        let mut hist = [0usize; 6];
        for entry in &self.entries {
            hist[entry.tier.level() as usize] += 1;
        }
        hist
    }

    /// Count entries at or above a given tier.
    pub fn at_or_above(&self, tier: VerificationTier) -> usize {
        self.entries.iter().filter(|e| e.tier >= tier).count()
    }

    /// Format human-readable contract coverage report.
    pub fn format_summary(&self) -> String {
        let hist = self.histogram();
        let mut out = String::from("Contract Coverage Report\n========================\n");
        out.push_str(&format!("Total functions on critical path: {}\n", self.total_functions));
        for (i, &count) in hist.iter().enumerate().rev() {
            let tier = match i {
                5 => "structural",
                4 => "proved",
                3 => "bounded",
                2 => "runtime",
                1 => "labeled",
                _ => "unlabeled",
            };
            out.push_str(&format!("  Level {i} ({tier}): {count:>3}\n"));
        }

        if !self.handler_invariants.is_empty() {
            out.push_str("\nHandler Invariant Coverage:\n");
            for h in &self.handler_invariants {
                out.push_str(&format!("  {h}\n"));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_tier_ordering() {
        assert!(VerificationTier::Unlabeled < VerificationTier::Labeled);
        assert!(VerificationTier::Labeled < VerificationTier::Runtime);
        assert!(VerificationTier::Runtime < VerificationTier::Bounded);
        assert!(VerificationTier::Bounded < VerificationTier::Proved);
        assert!(VerificationTier::Proved < VerificationTier::Structural);
    }

    #[test]
    fn verification_tier_level() {
        assert_eq!(VerificationTier::Unlabeled.level(), 0);
        assert_eq!(VerificationTier::Structural.level(), 5);
    }

    #[test]
    fn verification_tier_display() {
        assert_eq!(VerificationTier::Bounded.to_string(), "L3 (bounded)");
        assert_eq!(VerificationTier::Proved.to_string(), "L4 (proved)");
    }

    #[test]
    fn verification_tier_default() {
        assert_eq!(VerificationTier::default(), VerificationTier::Unlabeled);
    }

    #[test]
    fn verification_tier_serde_roundtrip() {
        for tier in [
            VerificationTier::Unlabeled,
            VerificationTier::Labeled,
            VerificationTier::Runtime,
            VerificationTier::Bounded,
            VerificationTier::Proved,
            VerificationTier::Structural,
        ] {
            let json = serde_json::to_string(&tier).unwrap();
            let parsed: VerificationTier = serde_json::from_str(&json).unwrap();
            assert_eq!(tier, parsed);
        }
    }

    #[test]
    fn contract_entry_display() {
        let e = ContractEntry {
            function: "hash_desired_state".into(),
            module: "core::planner".into(),
            contract_id: Some("blake3-state-v1".into()),
            tier: VerificationTier::Bounded,
            verified_by: vec![],
        };
        assert_eq!(
            e.to_string(),
            "core::planner::hash_desired_state: L3 (bounded)"
        );
    }

    #[test]
    fn handler_invariant_display() {
        let h = HandlerInvariantStatus {
            resource_type: "file".into(),
            tier: VerificationTier::Bounded,
            exempt: false,
            exemption_reason: None,
        };
        assert_eq!(h.to_string(), "file: L3 (bounded)");
    }

    #[test]
    fn handler_invariant_exempt_display() {
        let h = HandlerInvariantStatus {
            resource_type: "task".into(),
            tier: VerificationTier::Unlabeled,
            exempt: true,
            exemption_reason: Some("imperative resource".into()),
        };
        let s = h.to_string();
        assert!(s.contains("EXEMPT"));
        assert!(s.contains("imperative resource"));
    }

    #[test]
    fn coverage_report_histogram() {
        let report = ContractCoverageReport {
            total_functions: 10,
            entries: vec![
                ContractEntry {
                    function: "f1".into(),
                    module: "m".into(),
                    contract_id: None,
                    tier: VerificationTier::Runtime,
                    verified_by: vec![],
                },
                ContractEntry {
                    function: "f2".into(),
                    module: "m".into(),
                    contract_id: None,
                    tier: VerificationTier::Runtime,
                    verified_by: vec![],
                },
                ContractEntry {
                    function: "f3".into(),
                    module: "m".into(),
                    contract_id: None,
                    tier: VerificationTier::Bounded,
                    verified_by: vec![],
                },
            ],
            handler_invariants: vec![],
        };
        let hist = report.histogram();
        assert_eq!(hist[2], 2); // Runtime
        assert_eq!(hist[3], 1); // Bounded
        assert_eq!(hist[0], 0); // Unlabeled
    }

    #[test]
    fn coverage_report_at_or_above() {
        let report = ContractCoverageReport {
            total_functions: 5,
            entries: vec![
                ContractEntry {
                    function: "a".into(),
                    module: "m".into(),
                    contract_id: None,
                    tier: VerificationTier::Unlabeled,
                    verified_by: vec![],
                },
                ContractEntry {
                    function: "b".into(),
                    module: "m".into(),
                    contract_id: None,
                    tier: VerificationTier::Runtime,
                    verified_by: vec![],
                },
                ContractEntry {
                    function: "c".into(),
                    module: "m".into(),
                    contract_id: None,
                    tier: VerificationTier::Proved,
                    verified_by: vec![],
                },
            ],
            handler_invariants: vec![],
        };
        assert_eq!(report.at_or_above(VerificationTier::Runtime), 2);
        assert_eq!(report.at_or_above(VerificationTier::Proved), 1);
        assert_eq!(report.at_or_above(VerificationTier::Structural), 0);
    }

    #[test]
    fn coverage_report_format_summary() {
        let report = ContractCoverageReport {
            total_functions: 3,
            entries: vec![ContractEntry {
                function: "f".into(),
                module: "m".into(),
                contract_id: None,
                tier: VerificationTier::Structural,
                verified_by: vec![],
            }],
            handler_invariants: vec![HandlerInvariantStatus {
                resource_type: "file".into(),
                tier: VerificationTier::Bounded,
                exempt: false,
                exemption_reason: None,
            }],
        };
        let text = report.format_summary();
        assert!(text.contains("Total functions on critical path: 3"));
        assert!(text.contains("Level 5 (structural):   1"));
        assert!(text.contains("file: L3 (bounded)"));
    }

    #[test]
    fn coverage_report_empty() {
        let report = ContractCoverageReport::default();
        assert_eq!(report.histogram(), [0; 6]);
        assert_eq!(report.at_or_above(VerificationTier::Unlabeled), 0);
    }
}
