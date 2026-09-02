//! What `lifecycle.ignore_drift` is allowed to say.
//!
//! The STRUCT stays in `resource.rs` beside the field that carries it; the
//! impl lives here so there is exactly ONE place that decides what an
//! `ignore_drift` entry means, and the parser and the tripwire cannot drift
//! apart on it (forjar#335 was precisely that — the schema said "field
//! list", the engine read "any entry means everything").

use super::resource::LifecycleRules;

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
