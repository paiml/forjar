//! The row types `PlanOutput` is built out of.
//!
//! Split from `types.rs` for forjar#497, which adds the unprobed census to the
//! plan surface: that file sits against the repo's 500-line cap, so the two
//! row structs live here and are re-exported from `types.rs`. There remains
//! exactly ONE path a consumer imports, following the `types_ops.rs`
//! precedent (#356).

use schemars::JsonSchema;
use serde::Serialize;

/// A single planned resource change.
#[derive(Debug, Serialize, JsonSchema)]
pub struct PlannedChangeOutput {
    /// Resource identifier.
    pub resource_id: String,
    /// Target machine name.
    pub machine: String,
    /// Planned action (create, update, destroy).
    pub action: String,
    /// Human-readable change description.
    pub description: String,
}

/// forjar#497: one converged resource this plan did not MEASURE.
///
/// The planner probes a resource's declared build I/O only on machines this
/// host answers for, and a missing probe falls through to the config-hash
/// comparison — so "probed, nothing stale" and "never probed" both arrive here
/// as `unchanged`. Each row says which resource, on which machine, and why no
/// probe stands behind it. It is a statement about measurement, never about
/// change: whether those inputs moved is exactly what is unknown.
#[derive(Debug, Serialize, JsonSchema)]
pub struct UnprobedOutput {
    /// Resource identifier.
    pub resource_id: String,
    /// The machine on which it was not probed.
    pub machine: String,
    /// Why no probe was taken.
    pub reason: String,
}

impl From<&crate::core::types::UnprobedResource> for UnprobedOutput {
    fn from(u: &crate::core::types::UnprobedResource) -> Self {
        Self {
            resource_id: u.resource_id.clone(),
            machine: u.machine.clone(),
            reason: u.reason.clone(),
        }
    }
}
