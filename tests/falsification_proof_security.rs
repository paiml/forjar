//! FJ-1385/1382: Proof obligations and reversibility classification.
//!
//! Popperian rejection criteria for:
//! - FJ-1385: classify (all resource types × all actions), label, is_safe
//! - FJ-1382: reversibility classify (create/update/destroy), count_irreversible,
//!   warn_irreversible, file destroy ± source
//!
//! Usage: cargo test --test falsification_proof_security

use forjar::core::planner::proof_obligation::{self, ProofObligation};
use forjar::core::planner::reversibility::{self, Reversibility};
use forjar::core::types::*;
use indexmap::IndexMap;

// ============================================================================
// FJ-1385: ProofObligation classify
// ============================================================================

#[test]
fn proof_noop_always_idempotent() {
    for rtype in all_resource_types() {
        assert_eq!(
            proof_obligation::classify(&rtype, &PlanAction::NoOp),
            ProofObligation::Idempotent,
            "NoOp on {rtype:?} must be idempotent"
        );
    }
}

#[test]
fn proof_create_file_idempotent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::File, &PlanAction::Create),
        ProofObligation::Idempotent
    );
}

#[test]
fn proof_create_package_idempotent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::Package, &PlanAction::Create),
        ProofObligation::Idempotent
    );
}

#[test]
fn proof_create_service_convergent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::Service, &PlanAction::Create),
        ProofObligation::Convergent
    );
}

#[test]
fn proof_create_model_monotonic() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::Model, &PlanAction::Create),
        ProofObligation::Monotonic
    );
}

#[test]
fn proof_update_file_idempotent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::File, &PlanAction::Update),
        ProofObligation::Idempotent
    );
}

#[test]
fn proof_update_package_convergent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::Package, &PlanAction::Update),
        ProofObligation::Convergent
    );
}

#[test]
fn proof_destroy_file_destructive() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::File, &PlanAction::Destroy),
        ProofObligation::Destructive
    );
}

#[test]
fn proof_destroy_service_convergent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::Service, &PlanAction::Destroy),
        ProofObligation::Convergent
    );
}

#[test]
fn proof_destroy_user_destructive() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::User, &PlanAction::Destroy),
        ProofObligation::Destructive
    );
}

#[test]
fn proof_destroy_cron_idempotent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::Cron, &PlanAction::Destroy),
        ProofObligation::Idempotent
    );
}

#[test]
fn proof_destroy_task_destructive() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::Task, &PlanAction::Destroy),
        ProofObligation::Destructive
    );
}

#[test]
fn proof_destroy_wasm_destructive() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::WasmBundle, &PlanAction::Destroy),
        ProofObligation::Destructive
    );
}

#[test]
fn proof_label() {
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
}

#[test]
fn proof_is_safe() {
    assert!(proof_obligation::is_safe(&ProofObligation::Idempotent));
    assert!(proof_obligation::is_safe(&ProofObligation::Monotonic));
    assert!(proof_obligation::is_safe(&ProofObligation::Convergent));
    assert!(!proof_obligation::is_safe(&ProofObligation::Destructive));
}

// ============================================================================
// FJ-1382: Reversibility classify
// ============================================================================

fn make_resource(rtype: ResourceType) -> Resource {
    Resource {
        resource_type: rtype,
        ..Default::default()
    }
}

#[test]
fn reversibility_create_always_reversible() {
    for rtype in all_resource_types() {
        let r = make_resource(rtype.clone());
        assert_eq!(
            reversibility::classify(&r, &PlanAction::Create),
            Reversibility::Reversible,
            "Create on {rtype:?} must be reversible"
        );
    }
}

#[test]
fn reversibility_update_always_reversible() {
    for rtype in all_resource_types() {
        let r = make_resource(rtype.clone());
        assert_eq!(
            reversibility::classify(&r, &PlanAction::Update),
            Reversibility::Reversible,
        );
    }
}

#[test]
fn reversibility_noop_always_reversible() {
    let r = make_resource(ResourceType::File);
    assert_eq!(
        reversibility::classify(&r, &PlanAction::NoOp),
        Reversibility::Reversible
    );
}

#[test]
fn reversibility_destroy_service_reversible() {
    let r = make_resource(ResourceType::Service);
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Reversible
    );
}

#[test]
fn reversibility_destroy_user_irreversible() {
    let r = make_resource(ResourceType::User);
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Irreversible
    );
}

#[test]
fn reversibility_destroy_file_no_source_irreversible() {
    let r = make_resource(ResourceType::File);
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Irreversible
    );
}

#[test]
fn reversibility_destroy_file_with_content_reversible() {
    let mut r = make_resource(ResourceType::File);
    r.content = Some("data".into());
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Reversible
    );
}

#[test]
fn reversibility_destroy_file_with_source_reversible() {
    let mut r = make_resource(ResourceType::File);
    r.source = Some("/src/file".into());
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Reversible
    );
}

#[test]
fn reversibility_destroy_network_irreversible() {
    let r = make_resource(ResourceType::Network);
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Irreversible
    );
}

#[test]
fn reversibility_destroy_model_irreversible() {
    let r = make_resource(ResourceType::Model);
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Irreversible
    );
}

#[test]
fn reversibility_destroy_docker_reversible() {
    let r = make_resource(ResourceType::Docker);
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Reversible
    );
}

#[test]
fn reversibility_destroy_build_reversible() {
    let r = make_resource(ResourceType::Build);
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Reversible
    );
}

// ── count_irreversible / warn_irreversible ──

fn make_config(resources: Vec<(&str, Resource)>) -> ForjarConfig {
    let mut res = IndexMap::new();
    for (id, r) in resources {
        res.insert(id.to_string(), r);
    }
    ForjarConfig {
        version: "1.0".into(),
        name: "test".into(),
        resources: res,
        description: None,
        params: Default::default(),
        machines: Default::default(),
        policy: Default::default(),
        outputs: Default::default(),
        policies: Default::default(),
        data: Default::default(),
        includes: Default::default(),
        include_provenance: Default::default(),
        checks: Default::default(),
        moved: Default::default(),
        secrets: Default::default(),
        environments: Default::default(),
    }
}

fn make_plan(changes: Vec<PlannedChange>) -> ExecutionPlan {
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

fn planned_destroy(id: &str, rtype: ResourceType) -> PlannedChange {
    PlannedChange {
        resource_id: id.into(),
        machine: "m".into(),
        resource_type: rtype,
        action: PlanAction::Destroy,
        description: String::new(),
    }
}

#[test]
fn count_irreversible_mixed() {
    let config = make_config(vec![
        ("svc", make_resource(ResourceType::Service)),
        ("user", make_resource(ResourceType::User)),
        ("net", make_resource(ResourceType::Network)),
    ]);
    let plan = make_plan(vec![
        planned_destroy("svc", ResourceType::Service),
        planned_destroy("user", ResourceType::User),
        planned_destroy("net", ResourceType::Network),
    ]);
    assert_eq!(reversibility::count_irreversible(&config, &plan), 2);
}

#[test]
fn count_irreversible_none() {
    let config = make_config(vec![("svc", make_resource(ResourceType::Service))]);
    let plan = make_plan(vec![planned_destroy("svc", ResourceType::Service)]);
    assert_eq!(reversibility::count_irreversible(&config, &plan), 0);
}

#[test]
fn warn_irreversible_messages() {
    let config = make_config(vec![("db-user", make_resource(ResourceType::User))]);
    let plan = make_plan(vec![planned_destroy("db-user", ResourceType::User)]);
    let warnings = reversibility::warn_irreversible(&config, &plan);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("db-user"));
    assert!(warnings[0].contains("irreversible"));
}

#[test]
fn warn_irreversible_empty() {
    let config = make_config(vec![("svc", make_resource(ResourceType::Service))]);
    let plan = make_plan(vec![planned_destroy("svc", ResourceType::Service)]);
    assert!(reversibility::warn_irreversible(&config, &plan).is_empty());
}

// ── helpers ──

fn all_resource_types() -> Vec<ResourceType> {
    vec![
        ResourceType::File,
        ResourceType::Package,
        ResourceType::Service,
        ResourceType::Mount,
        ResourceType::User,
        ResourceType::Cron,
        ResourceType::Network,
        ResourceType::Docker,
        ResourceType::Pepita,
        ResourceType::Gpu,
        ResourceType::Model,
        ResourceType::Recipe,
        ResourceType::Task,
        ResourceType::WasmBundle,
        ResourceType::Image,
        ResourceType::Build,
    ]
}
