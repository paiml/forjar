//! FJ-1305: Purity classification and analysis.
//!
//! 4-level purity model for recipe resources (Section 3.1 of spec):
//!
//! | Level | Name        | Definition |
//! |-------|-------------|------------|
//! | 0     | Pure        | All inputs hashed, sandboxed, deterministic |
//! | 1     | Pinned      | Version-locked but not sandboxed |
//! | 2     | Constrained | Provider-scoped but floating version |
//! | 3     | Impure      | Unconstrained network/side-effect access |
//!
//! A recipe's purity level is the **maximum** (least pure) of all its
//! transitive dependencies (monotonicity invariant).

use serde::{Deserialize, Serialize};

/// Purity level for a resource or recipe (lower = purer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PurityLevel {
    /// Level 0: All inputs hashed, sandboxed, deterministic.
    Pure = 0,
    /// Level 1: Version-locked but not sandboxed.
    Pinned = 1,
    /// Level 2: Provider-scoped but floating version.
    Constrained = 2,
    /// Level 3: Unconstrained network/side-effect access.
    Impure = 3,
}

/// Result of purity classification for a single resource.
#[derive(Debug, Clone, PartialEq)]
pub struct PurityResult {
    /// Resource name.
    pub name: String,
    /// Classified purity level.
    pub level: PurityLevel,
    /// Reasons for this classification.
    pub reasons: Vec<String>,
}

/// Signals that influence purity classification.
#[derive(Debug, Clone, Default)]
pub struct PuritySignals {
    /// Whether the resource has a version pin.
    pub has_version: bool,
    /// Whether the resource uses the content store.
    pub has_store: bool,
    /// Whether the resource has sandbox isolation.
    pub has_sandbox: bool,
    /// Whether a curl|bash or wget|sh pattern was detected.
    pub has_curl_pipe: bool,
    /// Purity levels of transitive dependencies.
    pub dep_levels: Vec<PurityLevel>,
}

/// Classify a resource's purity level from its signals.
///
/// Classification rules (from least pure to most pure):
/// - `curl|bash` or `wget|sh` pattern → Impure (3)
/// - No version pin → Constrained (2)
/// - Version pin + store but no sandbox → Pinned (1)
/// - Version pin + store + sandbox → Pure (0)
///
/// Final level = max(own_level, max(dep_levels)) — monotonicity.
pub fn classify(name: &str, signals: &PuritySignals) -> PurityResult {
    let mut reasons = Vec::new();

    let own_level = if signals.has_curl_pipe {
        reasons.push("curl|bash or wget|sh pattern detected".to_string());
        PurityLevel::Impure
    } else if !signals.has_version {
        reasons.push("no version pin".to_string());
        PurityLevel::Constrained
    } else if !signals.has_store || !signals.has_sandbox {
        if !signals.has_store {
            reasons.push("version pinned but store not enabled".to_string());
        }
        if !signals.has_sandbox {
            reasons.push("version pinned but no sandbox".to_string());
        }
        PurityLevel::Pinned
    } else {
        reasons.push("version pinned + store + sandbox".to_string());
        PurityLevel::Pure
    };

    // Monotonicity: a resource is at least as impure as its deps
    let dep_max = signals.dep_levels.iter().max().copied();
    let final_level = match dep_max {
        Some(dep) if dep > own_level => {
            reasons.push(format!("dependency at level {dep:?} elevates purity"));
            dep
        }
        _ => own_level,
    };

    PurityResult {
        name: name.to_string(),
        level: final_level,
        reasons,
    }
}

/// Compute the aggregate purity level for a recipe (max of all resources).
pub fn recipe_purity(resource_levels: &[PurityLevel]) -> PurityLevel {
    resource_levels
        .iter()
        .max()
        .copied()
        .unwrap_or(PurityLevel::Pure)
}

/// Format a purity level as a human-readable label.
pub fn level_label(level: PurityLevel) -> &'static str {
    match level {
        PurityLevel::Pure => "Pure (0)",
        PurityLevel::Pinned => "Pinned (1)",
        PurityLevel::Constrained => "Constrained (2)",
        PurityLevel::Impure => "Impure (3)",
    }
}
