//! FJ-1385: Proof obligation taxonomy — classify resource operations formally.
//!
//! Every resource operation is classified as one of:
//! - **Idempotent**: `apply(apply(s)) == apply(s)` — safe to re-run
//! - **Monotonic**: only adds state, never removes — safe to layer
//! - **Convergent**: `apply` always reaches the same fixed point from any starting state
//! - **Destructive**: removes state that cannot be reconstructed

use crate::core::types::{PlanAction, ResourceType};

/// Formal proof obligation category for a resource operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofObligation {
    /// `f(f(x)) = f(x)` — safe to re-run without side effects.
    Idempotent,
    /// Only adds state; never removes or modifies existing state.
    Monotonic,
    /// Reaches same fixed point from any reachable starting state.
    Convergent,
    /// Removes state that may not be reconstructable.
    Destructive,
}

/// Classify a resource type + action pair into its proof obligation category.
pub fn classify(rtype: &ResourceType, action: &PlanAction) -> ProofObligation {
    match action {
        PlanAction::NoOp => ProofObligation::Idempotent,
        PlanAction::Create => classify_create(rtype),
        PlanAction::Update => classify_update(rtype),
        PlanAction::Destroy => classify_destroy(rtype),
    }
}

/// Create operations: file/service/package are idempotent; model is monotonic.
fn classify_create(rtype: &ResourceType) -> ProofObligation {
    match rtype {
        ResourceType::File => ProofObligation::Idempotent,
        ResourceType::Package => ProofObligation::Idempotent,
        ResourceType::Service => ProofObligation::Convergent,
        ResourceType::Mount => ProofObligation::Idempotent,
        ResourceType::User => ProofObligation::Idempotent,
        ResourceType::Cron => ProofObligation::Idempotent,
        ResourceType::Network => ProofObligation::Convergent,
        ResourceType::Docker | ResourceType::Pepita => ProofObligation::Convergent,
        ResourceType::Gpu => ProofObligation::Convergent,
        ResourceType::Model => ProofObligation::Monotonic,
        ResourceType::Recipe => ProofObligation::Convergent,
        ResourceType::Task => ProofObligation::Convergent,
    }
}

/// Update operations: generally convergent (reach desired state from any current).
fn classify_update(rtype: &ResourceType) -> ProofObligation {
    match rtype {
        ResourceType::File => ProofObligation::Idempotent,
        ResourceType::Package => ProofObligation::Convergent,
        ResourceType::Service => ProofObligation::Convergent,
        _ => ProofObligation::Convergent,
    }
}

/// Destroy operations: service/package are convergent; file/user/model are destructive.
fn classify_destroy(rtype: &ResourceType) -> ProofObligation {
    match rtype {
        ResourceType::Service => ProofObligation::Convergent,
        ResourceType::Package => ProofObligation::Convergent,
        ResourceType::Cron => ProofObligation::Idempotent,
        ResourceType::Mount => ProofObligation::Convergent,
        ResourceType::File => ProofObligation::Destructive,
        ResourceType::User => ProofObligation::Destructive,
        ResourceType::Model => ProofObligation::Destructive,
        ResourceType::Docker | ResourceType::Pepita => ProofObligation::Convergent,
        ResourceType::Network => ProofObligation::Convergent,
        ResourceType::Gpu => ProofObligation::Convergent,
        ResourceType::Recipe => ProofObligation::Convergent,
        ResourceType::Task => ProofObligation::Destructive,
    }
}

/// Human-readable label for a proof obligation.
pub fn label(po: &ProofObligation) -> &'static str {
    match po {
        ProofObligation::Idempotent => "idempotent",
        ProofObligation::Monotonic => "monotonic",
        ProofObligation::Convergent => "convergent",
        ProofObligation::Destructive => "destructive",
    }
}

/// Returns true if the obligation is safe to re-run without confirmation.
pub fn is_safe(po: &ProofObligation) -> bool {
    matches!(
        po,
        ProofObligation::Idempotent | ProofObligation::Monotonic | ProofObligation::Convergent
    )
}
