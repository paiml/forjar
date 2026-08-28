//! FJ-1220 / forjar#335: what `lifecycle.ignore_drift` actually turns off.
//!
//! Split out of `mod.rs` so the one place that decides whether a tripwire stops
//! looking is a file you can read in full, and so `mod.rs` stays inside the
//! line budget.

use crate::core::types::{LifecycleRules, Resource};

/// Is this resource's drift suppressed wholesale?
///
/// `ignore_drift` is a FIELD LIST in the schema, and this used to return
/// `!is_empty()`. So `["mode"]` — tolerate a mode change, keep watching the
/// bytes — silently disabled content, owner, group, existence and image drift
/// as well: narrowing the written exemption widened the real one, and a typo
/// (`["modes"]`) was the broadest exemption forjar could express.
///
/// Only the wildcard `["*"]` is honoured. A narrowed list is rejected at
/// config validation (`core::parser::validation::validate_lifecycle`); one
/// arriving from a recipe — which expands AFTER `validate_config` — means
/// "keep looking", the safe direction for a tripwire.
pub(super) fn should_ignore_drift(
    resource_id: &str,
    resources: &indexmap::IndexMap<String, Resource>,
) -> bool {
    resources
        .get(resource_id)
        .and_then(|r| r.lifecycle.as_ref())
        .is_some_and(LifecycleRules::suppresses_all_drift)
}
