//! Plan types: what the planner returns and what every plan surface renders.
//!
//! Split out of `state_types.rs` for forjar#497, which adds the unprobed
//! census to `ExecutionPlan`; that file sits at the 500-line ceiling.

use serde::{Deserialize, Serialize};

use super::{PlanAction, ResourceType};

/// A single planned change.
#[derive(Debug, Clone, Serialize)]
pub struct PlannedChange {
    /// Resource ID
    pub resource_id: String,

    /// Target machine
    pub machine: String,

    /// Resource type
    pub resource_type: ResourceType,

    /// Action to take
    pub action: PlanAction,

    /// Human-readable description
    pub description: String,
}

/// Full execution plan.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionPlan {
    /// Config name
    pub name: String,

    /// Planned changes grouped by machine
    pub changes: Vec<PlannedChange>,

    /// Topological execution order (resource IDs)
    pub execution_order: Vec<String>,

    /// Number of resources to create.
    pub to_create: u32,
    /// Number of resources to update.
    pub to_update: u32,
    /// Number of resources to destroy.
    pub to_destroy: u32,
    /// Number of unchanged resources.
    pub unchanged: u32,

    /// forjar#497: the converged resources whose declared build inputs or
    /// artifacts this plan did not measure, per (resource, machine). By the
    /// action alone the planner cannot tell "probed, nothing stale" from
    /// "never probed" — both fall through to the config-hash comparison and
    /// plan `NoOp` — so it says which it was. A TOTAL list on every surface,
    /// in the forjar#342 / #372 shape; the prose disclosure is derived from it.
    ///
    /// Not serialised when empty: the plan seal digests this serialisation, so
    /// a plan with nothing unprobed stays byte-identical and every plan file
    /// sealed before the field existed still verifies.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unprobed: Vec<UnprobedResource>,
}

/// forjar#497: one converged resource this plan could not measure.
///
/// `reason` says why no probe was taken — normally that this host does not
/// answer for the machine, so the resource's declared build I/O lives
/// somewhere the planner never reads. It is a statement about MEASUREMENT,
/// never about change: whether the inputs moved is exactly what is unknown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnprobedResource {
    /// Resource ID.
    pub resource_id: String,
    /// The machine on which it was not probed.
    pub machine: String,
    /// Why no probe was taken.
    pub reason: String,
}
