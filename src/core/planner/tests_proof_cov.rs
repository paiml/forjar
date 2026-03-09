//! Exhaustive branch coverage for proof_obligation.rs — every ResourceType × PlanAction.

use super::proof_obligation::*;
use crate::core::types::*;

// ── classify_create: every ResourceType ────────────────────────────

#[test]
fn create_package_idempotent() {
    assert_eq!(
        classify(&ResourceType::Package, &PlanAction::Create),
        ProofObligation::Idempotent
    );
}

#[test]
fn create_mount_idempotent() {
    assert_eq!(
        classify(&ResourceType::Mount, &PlanAction::Create),
        ProofObligation::Idempotent
    );
}

#[test]
fn create_user_idempotent() {
    assert_eq!(
        classify(&ResourceType::User, &PlanAction::Create),
        ProofObligation::Idempotent
    );
}

#[test]
fn create_cron_idempotent() {
    assert_eq!(
        classify(&ResourceType::Cron, &PlanAction::Create),
        ProofObligation::Idempotent
    );
}

#[test]
fn create_wasm_bundle_idempotent() {
    assert_eq!(
        classify(&ResourceType::WasmBundle, &PlanAction::Create),
        ProofObligation::Idempotent
    );
}

#[test]
fn create_image_idempotent() {
    assert_eq!(
        classify(&ResourceType::Image, &PlanAction::Create),
        ProofObligation::Idempotent
    );
}

#[test]
fn create_network_convergent() {
    assert_eq!(
        classify(&ResourceType::Network, &PlanAction::Create),
        ProofObligation::Convergent
    );
}

#[test]
fn create_docker_convergent() {
    assert_eq!(
        classify(&ResourceType::Docker, &PlanAction::Create),
        ProofObligation::Convergent
    );
}

#[test]
fn create_pepita_convergent() {
    assert_eq!(
        classify(&ResourceType::Pepita, &PlanAction::Create),
        ProofObligation::Convergent
    );
}

#[test]
fn create_gpu_convergent() {
    assert_eq!(
        classify(&ResourceType::Gpu, &PlanAction::Create),
        ProofObligation::Convergent
    );
}

#[test]
fn create_recipe_convergent() {
    assert_eq!(
        classify(&ResourceType::Recipe, &PlanAction::Create),
        ProofObligation::Convergent
    );
}

#[test]
fn create_task_convergent() {
    assert_eq!(
        classify(&ResourceType::Task, &PlanAction::Create),
        ProofObligation::Convergent
    );
}

#[test]
fn create_build_convergent() {
    assert_eq!(
        classify(&ResourceType::Build, &PlanAction::Create),
        ProofObligation::Convergent
    );
}

// ── classify_update: branch coverage ──────────────────────────────

#[test]
fn update_service_convergent() {
    assert_eq!(
        classify(&ResourceType::Service, &PlanAction::Update),
        ProofObligation::Convergent
    );
}

#[test]
fn update_default_convergent() {
    // Covers the _ => Convergent fallthrough
    for rtype in &[
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
    ] {
        assert_eq!(
            classify(rtype, &PlanAction::Update),
            ProofObligation::Convergent,
            "update {rtype:?} should be convergent"
        );
    }
}

// ── classify_destroy: every ResourceType ──────────────────────────

#[test]
fn destroy_cron_idempotent() {
    assert_eq!(
        classify(&ResourceType::Cron, &PlanAction::Destroy),
        ProofObligation::Idempotent
    );
}

#[test]
fn destroy_package_convergent() {
    assert_eq!(
        classify(&ResourceType::Package, &PlanAction::Destroy),
        ProofObligation::Convergent
    );
}

#[test]
fn destroy_mount_convergent() {
    assert_eq!(
        classify(&ResourceType::Mount, &PlanAction::Destroy),
        ProofObligation::Convergent
    );
}

#[test]
fn destroy_docker_convergent() {
    assert_eq!(
        classify(&ResourceType::Docker, &PlanAction::Destroy),
        ProofObligation::Convergent
    );
}

#[test]
fn destroy_pepita_convergent() {
    assert_eq!(
        classify(&ResourceType::Pepita, &PlanAction::Destroy),
        ProofObligation::Convergent
    );
}

#[test]
fn destroy_network_convergent() {
    assert_eq!(
        classify(&ResourceType::Network, &PlanAction::Destroy),
        ProofObligation::Convergent
    );
}

#[test]
fn destroy_gpu_convergent() {
    assert_eq!(
        classify(&ResourceType::Gpu, &PlanAction::Destroy),
        ProofObligation::Convergent
    );
}

#[test]
fn destroy_recipe_convergent() {
    assert_eq!(
        classify(&ResourceType::Recipe, &PlanAction::Destroy),
        ProofObligation::Convergent
    );
}

#[test]
fn destroy_build_convergent() {
    assert_eq!(
        classify(&ResourceType::Build, &PlanAction::Destroy),
        ProofObligation::Convergent
    );
}

#[test]
fn destroy_task_destructive() {
    assert_eq!(
        classify(&ResourceType::Task, &PlanAction::Destroy),
        ProofObligation::Destructive
    );
}

#[test]
fn destroy_wasm_bundle_destructive() {
    assert_eq!(
        classify(&ResourceType::WasmBundle, &PlanAction::Destroy),
        ProofObligation::Destructive
    );
}

#[test]
fn destroy_image_destructive() {
    assert_eq!(
        classify(&ResourceType::Image, &PlanAction::Destroy),
        ProofObligation::Destructive
    );
}

// ── noop: remaining types ─────────────────────────────────────────

#[test]
fn noop_all_remaining_types() {
    for rtype in &[
        ResourceType::Mount,
        ResourceType::Cron,
        ResourceType::Network,
        ResourceType::Docker,
        ResourceType::Pepita,
        ResourceType::Gpu,
        ResourceType::Recipe,
        ResourceType::Task,
        ResourceType::WasmBundle,
        ResourceType::Image,
        ResourceType::Build,
    ] {
        assert_eq!(
            classify(rtype, &PlanAction::NoOp),
            ProofObligation::Idempotent,
            "noop {rtype:?} should be idempotent"
        );
    }
}
