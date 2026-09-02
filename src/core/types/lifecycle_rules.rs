//! What `lifecycle.ignore_drift` is allowed to say.
//!
//! Split out of `resource.rs` to keep that file under the 500-line budget, in
//! the same spirit as `lock_observed.rs`: there is exactly ONE place that
//! decides what an `ignore_drift` entry means, so the parser and the tripwire
//! cannot drift apart on it (forjar#335 was precisely that — the schema said
//! "field list", the engine read "any entry means everything").

use serde::{Deserialize, Serialize};

/// FJ-1220: Lifecycle protection rules for a resource.
///
/// Controls how a resource is handled during destroy, replacement, and drift
/// detection. The STRUCT lives here beside its only impl (and not in
/// `resource.rs`) for the reason this module was created: `resource.rs` is at
/// the 500-line budget, and Refs #406 needed one more field on `Resource`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifecycleRules {
    /// Prevent this resource from being destroyed (forjar destroy skips with warning)
    #[serde(default)]
    pub prevent_destroy: bool,

    /// Write new version before removing old (avoids config-absent window)
    #[serde(default)]
    pub create_before_destroy: bool,

    /// Fields whose drift is suppressed (reported as "suppressed" not "detected")
    #[serde(default)]
    pub ignore_drift: Vec<String>,
}

impl LifecycleRules {
    /// The ONLY `ignore_drift` entry forjar implements: suppress every
    /// dimension for this resource. Per-field suppression is forjar#335.
    pub const IGNORE_DRIFT_ALL: &'static str = "*";

    /// True when this resource's drift is suppressed wholesale.
    pub fn suppresses_all_drift(&self) -> bool {
        self.ignore_drift
            .iter()
            .any(|f| f == Self::IGNORE_DRIFT_ALL)
    }

    /// Entries that are NOT the wildcard — the narrowed form the engine cannot
    /// honour. Empty means the declaration is implementable as written.
    pub fn unhonoured_ignore_drift(&self) -> Vec<&str> {
        self.ignore_drift
            .iter()
            .map(String::as_str)
            .filter(|f| *f != Self::IGNORE_DRIFT_ALL)
            .collect()
    }
}
