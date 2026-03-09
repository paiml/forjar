//! FJ-1385/1382: Proof obligation taxonomy and reversibility classification.
//!
//! Popperian rejection criteria for:
//! - FJ-1385: Proof obligation (classify, label, is_safe)
//! - FJ-1382: Reversibility (classify, count_irreversible, warn_irreversible)
//!
//! Usage: cargo test --test falsification_planner_proof_rev

use forjar::core::planner::proof_obligation::{self, ProofObligation};
use forjar::core::planner::reversibility::{self, Reversibility};
use forjar::core::types::*;

fn resource(rtype: ResourceType) -> Resource {
    Resource {
        resource_type: rtype,
        ..Default::default()
    }
}

fn plan_change(id: &str, action: PlanAction, rtype: ResourceType) -> PlannedChange {
    PlannedChange {
        resource_id: id.into(),
        machine: "m1".into(),
        resource_type: rtype,
        action,
        description: "test".into(),
    }
}

fn exec_plan(changes: Vec<PlannedChange>) -> ExecutionPlan {
    ExecutionPlan {
        name: "test".into(),
        changes,
        execution_order: vec![],
        to_create: 0,
        to_update: 0,
        to_destroy: 0,
        unchanged: 0,
    }
}

// ============================================================================
// FJ-1385: ProofObligation — NoOp is always idempotent
// ============================================================================

#[test]
fn po_noop_always_idempotent() {
    for rt in [
        ResourceType::File,
        ResourceType::Package,
        ResourceType::Service,
        ResourceType::Docker,
        ResourceType::Model,
        ResourceType::User,
    ] {
        assert_eq!(
            proof_obligation::classify(&rt, &PlanAction::NoOp),
            ProofObligation::Idempotent
        );
    }
}

// ============================================================================
// FJ-1385: ProofObligation — Create classification
// ============================================================================

#[test]
fn po_create_idempotent_types() {
    for rt in [
        ResourceType::File,
        ResourceType::Package,
        ResourceType::Mount,
        ResourceType::User,
        ResourceType::Cron,
        ResourceType::WasmBundle,
        ResourceType::Image,
    ] {
        assert_eq!(
            proof_obligation::classify(&rt, &PlanAction::Create),
            ProofObligation::Idempotent,
            "Create {rt:?} should be Idempotent"
        );
    }
}

#[test]
fn po_create_convergent_types() {
    for rt in [
        ResourceType::Service,
        ResourceType::Docker,
        ResourceType::Pepita,
        ResourceType::Network,
        ResourceType::Gpu,
        ResourceType::Recipe,
        ResourceType::Task,
        ResourceType::Build,
    ] {
        assert_eq!(
            proof_obligation::classify(&rt, &PlanAction::Create),
            ProofObligation::Convergent,
            "Create {rt:?} should be Convergent"
        );
    }
}

#[test]
fn po_create_model_monotonic() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::Model, &PlanAction::Create),
        ProofObligation::Monotonic
    );
}

// ============================================================================
// FJ-1385: ProofObligation — Update classification
// ============================================================================

#[test]
fn po_update_file_idempotent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::File, &PlanAction::Update),
        ProofObligation::Idempotent
    );
}

#[test]
fn po_update_others_convergent() {
    for rt in [
        ResourceType::Package,
        ResourceType::Service,
        ResourceType::Docker,
        ResourceType::Model,
        ResourceType::Network,
    ] {
        assert_eq!(
            proof_obligation::classify(&rt, &PlanAction::Update),
            ProofObligation::Convergent,
            "Update {rt:?} should be Convergent"
        );
    }
}

// ============================================================================
// FJ-1385: ProofObligation — Destroy classification
// ============================================================================

#[test]
fn po_destroy_destructive_types() {
    for rt in [
        ResourceType::File,
        ResourceType::User,
        ResourceType::Model,
        ResourceType::Task,
        ResourceType::WasmBundle,
        ResourceType::Image,
    ] {
        assert_eq!(
            proof_obligation::classify(&rt, &PlanAction::Destroy),
            ProofObligation::Destructive,
            "Destroy {rt:?} should be Destructive"
        );
    }
}

#[test]
fn po_destroy_convergent_types() {
    for rt in [
        ResourceType::Service,
        ResourceType::Package,
        ResourceType::Mount,
        ResourceType::Docker,
        ResourceType::Pepita,
        ResourceType::Network,
        ResourceType::Gpu,
        ResourceType::Recipe,
        ResourceType::Build,
    ] {
        assert_eq!(
            proof_obligation::classify(&rt, &PlanAction::Destroy),
            ProofObligation::Convergent,
            "Destroy {rt:?} should be Convergent"
        );
    }
}

#[test]
fn po_destroy_cron_idempotent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::Cron, &PlanAction::Destroy),
        ProofObligation::Idempotent
    );
}

// ============================================================================
// FJ-1385: label + is_safe
// ============================================================================

#[test]
fn po_labels_and_safety() {
    assert_eq!(
        proof_obligation::label(&ProofObligation::Idempotent),
        "idempotent"
    );
    assert_eq!(
        proof_obligation::label(&ProofObligation::Monotonic),
        "monotonic"
    );
    assert_eq!(
        proof_obligation::label(&ProofObligation::Convergent),
        "convergent"
    );
    assert_eq!(
        proof_obligation::label(&ProofObligation::Destructive),
        "destructive"
    );
    assert!(proof_obligation::is_safe(&ProofObligation::Idempotent));
    assert!(proof_obligation::is_safe(&ProofObligation::Monotonic));
    assert!(proof_obligation::is_safe(&ProofObligation::Convergent));
    assert!(!proof_obligation::is_safe(&ProofObligation::Destructive));
}

// ============================================================================
// FJ-1382: Reversibility — NoOp/Create/Update always reversible
// ============================================================================

#[test]
fn rev_noop_create_update_reversible() {
    let r = resource(ResourceType::File);
    for action in [PlanAction::NoOp, PlanAction::Create, PlanAction::Update] {
        assert_eq!(
            reversibility::classify(&r, &action),
            Reversibility::Reversible,
            "{action:?} should be Reversible"
        );
    }
}

// ============================================================================
// FJ-1382: Reversibility — Destroy classification
// ============================================================================

#[test]
fn rev_destroy_file_with_content_reversible() {
    let mut r = resource(ResourceType::File);
    r.content = Some("data".into());
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Reversible
    );
}

#[test]
fn rev_destroy_file_with_source_reversible() {
    let mut r = resource(ResourceType::File);
    r.source = Some("https://example.com/f".into());
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Reversible
    );
}

#[test]
fn rev_destroy_file_bare_irreversible() {
    assert_eq!(
        reversibility::classify(&resource(ResourceType::File), &PlanAction::Destroy),
        Reversibility::Irreversible
    );
}

#[test]
fn rev_destroy_reversible_types() {
    for rt in [
        ResourceType::Service,
        ResourceType::Cron,
        ResourceType::Package,
        ResourceType::Mount,
        ResourceType::Docker,
        ResourceType::Pepita,
        ResourceType::Gpu,
        ResourceType::WasmBundle,
        ResourceType::Image,
        ResourceType::Build,
    ] {
        assert_eq!(
            reversibility::classify(&resource(rt.clone()), &PlanAction::Destroy),
            Reversibility::Reversible,
            "Destroy {rt:?} should be Reversible"
        );
    }
}

#[test]
fn rev_destroy_irreversible_types() {
    for rt in [
        ResourceType::User,
        ResourceType::Network,
        ResourceType::Model,
        ResourceType::Task,
        ResourceType::Recipe,
    ] {
        assert_eq!(
            reversibility::classify(&resource(rt.clone()), &PlanAction::Destroy),
            Reversibility::Irreversible,
            "Destroy {rt:?} should be Irreversible"
        );
    }
}

// ============================================================================
// FJ-1382: count_irreversible + warn_irreversible
// ============================================================================

#[test]
fn rev_count_irreversible() {
    let mut cfg = ForjarConfig::default();
    cfg.resources
        .insert("f1".into(), resource(ResourceType::File));
    cfg.resources
        .insert("s1".into(), resource(ResourceType::Service));
    cfg.resources
        .insert("u1".into(), resource(ResourceType::User));
    let plan = exec_plan(vec![
        plan_change("f1", PlanAction::Destroy, ResourceType::File),
        plan_change("s1", PlanAction::Destroy, ResourceType::Service),
        plan_change("u1", PlanAction::Destroy, ResourceType::User),
    ]);
    assert_eq!(reversibility::count_irreversible(&cfg, &plan), 2);
}

#[test]
fn rev_warn_irreversible_messages() {
    let mut cfg = ForjarConfig::default();
    cfg.resources
        .insert("u1".into(), resource(ResourceType::User));
    let plan = exec_plan(vec![plan_change(
        "u1",
        PlanAction::Destroy,
        ResourceType::User,
    )]);
    let warnings = reversibility::warn_irreversible(&cfg, &plan);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("irreversible") && warnings[0].contains("u1"));
}

#[test]
fn rev_count_non_destroy_zero() {
    let mut cfg = ForjarConfig::default();
    cfg.resources
        .insert("f1".into(), resource(ResourceType::File));
    let plan = exec_plan(vec![plan_change(
        "f1",
        PlanAction::Create,
        ResourceType::File,
    )]);
    assert_eq!(reversibility::count_irreversible(&cfg, &plan), 0);
}

#[test]
fn rev_missing_resource_counts_as_irreversible() {
    let plan = exec_plan(vec![plan_change(
        "ghost",
        PlanAction::Destroy,
        ResourceType::File,
    )]);
    assert_eq!(
        reversibility::count_irreversible(&ForjarConfig::default(), &plan),
        1
    );
}
