//! FJ-1385/1382/1379/004: Planner proof obligations, reversibility, change
//! explanation, and desired-state hashing falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-1385: Proof obligation taxonomy
//!   - classify: NoOp→Idempotent, Create/Update/Destroy per resource type
//!   - label: human-readable string for each category
//!   - is_safe: only Destructive returns false
//! - FJ-1382: Reversibility classification
//!   - classify: Create/Update→Reversible, Destroy→type-dependent
//!   - count_irreversible / warn_irreversible: plan-level analysis
//! - FJ-1379: Change explanation (--why)
//!   - explain_why: absent/present/no-lock/new-resource/failed/drifted/hash-change
//!   - format_why: human-readable output
//! - FJ-004: hash_desired_state
//!   - determinism: same resource → same hash
//!   - sensitivity: different content → different hash
//!
//! Usage: cargo test --test falsification_planner_proof_reversibility

use forjar::core::planner::hash_desired_state;
use forjar::core::planner::proof_obligation::{self, ProofObligation};
use forjar::core::planner::reversibility::{self, Reversibility};
use forjar::core::planner::why::{explain_why, format_why};
use forjar::core::types::*;
use indexmap::IndexMap;
use std::collections::HashMap;

// ============================================================================
// FJ-1385: proof_obligation::classify
// ============================================================================

#[test]
fn proof_noop_always_idempotent() {
    for rtype in [
        ResourceType::File,
        ResourceType::Package,
        ResourceType::Service,
        ResourceType::Model,
        ResourceType::Docker,
    ] {
        assert_eq!(
            proof_obligation::classify(&rtype, &PlanAction::NoOp),
            ProofObligation::Idempotent,
            "NoOp must always be idempotent for {rtype:?}"
        );
    }
}

#[test]
fn proof_create_file_idempotent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::File, &PlanAction::Create),
        ProofObligation::Idempotent,
    );
}

#[test]
fn proof_create_service_convergent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::Service, &PlanAction::Create),
        ProofObligation::Convergent,
    );
}

#[test]
fn proof_create_model_monotonic() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::Model, &PlanAction::Create),
        ProofObligation::Monotonic,
    );
}

#[test]
fn proof_update_file_idempotent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::File, &PlanAction::Update),
        ProofObligation::Idempotent,
    );
}

#[test]
fn proof_update_package_convergent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::Package, &PlanAction::Update),
        ProofObligation::Convergent,
    );
}

#[test]
fn proof_destroy_file_destructive() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::File, &PlanAction::Destroy),
        ProofObligation::Destructive,
    );
}

#[test]
fn proof_destroy_service_convergent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::Service, &PlanAction::Destroy),
        ProofObligation::Convergent,
    );
}

#[test]
fn proof_destroy_user_destructive() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::User, &PlanAction::Destroy),
        ProofObligation::Destructive,
    );
}

#[test]
fn proof_destroy_cron_idempotent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::Cron, &PlanAction::Destroy),
        ProofObligation::Idempotent,
    );
}

// ============================================================================
// FJ-1385: label / is_safe
// ============================================================================

#[test]
fn proof_label_strings() {
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
fn proof_is_safe_only_destructive_unsafe() {
    assert!(proof_obligation::is_safe(&ProofObligation::Idempotent));
    assert!(proof_obligation::is_safe(&ProofObligation::Monotonic));
    assert!(proof_obligation::is_safe(&ProofObligation::Convergent));
    assert!(!proof_obligation::is_safe(&ProofObligation::Destructive));
}

// ============================================================================
// FJ-1382: reversibility::classify
// ============================================================================

fn make_resource(rtype: ResourceType) -> Resource {
    Resource {
        resource_type: rtype,
        ..Default::default()
    }
}

#[test]
fn rev_non_destroy_always_reversible() {
    for action in [PlanAction::Create, PlanAction::Update, PlanAction::NoOp] {
        for rtype in [
            ResourceType::File,
            ResourceType::Package,
            ResourceType::Service,
        ] {
            assert_eq!(
                reversibility::classify(&make_resource(rtype.clone()), &action),
                Reversibility::Reversible,
                "{action:?} must be reversible for {rtype:?}"
            );
        }
    }
}

#[test]
fn rev_destroy_file_content_dependent() {
    // File with content → reversible (can be re-created)
    let r_with = Resource {
        resource_type: ResourceType::File,
        content: Some("hello".into()),
        ..Default::default()
    };
    assert_eq!(
        reversibility::classify(&r_with, &PlanAction::Destroy),
        Reversibility::Reversible
    );
    // File without content → irreversible
    let r_bare = make_resource(ResourceType::File);
    assert_eq!(
        reversibility::classify(&r_bare, &PlanAction::Destroy),
        Reversibility::Irreversible
    );
}

#[test]
fn rev_destroy_type_specific() {
    // Reversible destroys
    for rtype in [
        ResourceType::Service,
        ResourceType::Package,
        ResourceType::Docker,
    ] {
        assert_eq!(
            reversibility::classify(&make_resource(rtype.clone()), &PlanAction::Destroy),
            Reversibility::Reversible,
            "{rtype:?} destroy should be reversible"
        );
    }
    // Irreversible destroys
    for rtype in [
        ResourceType::User,
        ResourceType::Model,
        ResourceType::Network,
    ] {
        assert_eq!(
            reversibility::classify(&make_resource(rtype.clone()), &PlanAction::Destroy),
            Reversibility::Irreversible,
            "{rtype:?} destroy should be irreversible"
        );
    }
}

// ============================================================================
// FJ-1382: count_irreversible / warn_irreversible
// ============================================================================

fn make_plan_with_destroys() -> (ForjarConfig, ExecutionPlan) {
    let mut resources = IndexMap::new();
    resources.insert("my-user".into(), make_resource(ResourceType::User));
    resources.insert("my-svc".into(), make_resource(ResourceType::Service));
    resources.insert(
        "my-file".into(),
        Resource {
            resource_type: ResourceType::File,
            content: Some("data".into()),
            ..Default::default()
        },
    );
    let config = ForjarConfig {
        name: "test".into(),
        resources,
        ..Default::default()
    };
    let plan = ExecutionPlan {
        name: "test".into(),
        changes: vec![
            PlannedChange {
                resource_id: "my-user".into(),
                machine: "localhost".into(),
                resource_type: ResourceType::User,
                action: PlanAction::Destroy,
                description: "destroy user".into(),
            },
            PlannedChange {
                resource_id: "my-svc".into(),
                machine: "localhost".into(),
                resource_type: ResourceType::Service,
                action: PlanAction::Destroy,
                description: "stop service".into(),
            },
            PlannedChange {
                resource_id: "my-file".into(),
                machine: "localhost".into(),
                resource_type: ResourceType::File,
                action: PlanAction::Destroy,
                description: "delete file".into(),
            },
        ],
        execution_order: vec![],
        to_create: 0,
        to_update: 0,
        to_destroy: 3,
        unchanged: 0,
    };
    (config, plan)
}

#[test]
fn rev_count_irreversible_finds_user_only() {
    let (config, plan) = make_plan_with_destroys();
    // User destroy is irreversible; service and file-with-content are reversible
    assert_eq!(reversibility::count_irreversible(&config, &plan), 1);
}

#[test]
fn rev_warn_irreversible_lists_user() {
    let (config, plan) = make_plan_with_destroys();
    let warnings = reversibility::warn_irreversible(&config, &plan);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("my-user"));
    assert!(warnings[0].contains("irreversible"));
}

// ============================================================================
// FJ-1379: explain_why
// ============================================================================

fn make_lock(resource_id: &str, hash: &str, status: ResourceStatus) -> StateLock {
    let mut resources = IndexMap::new();
    resources.insert(
        resource_id.to_string(),
        ResourceLock {
            resource_type: ResourceType::Package,
            status,
            hash: hash.into(),
            applied_at: None,
            duration_seconds: None,
            details: HashMap::new(),
        },
    );
    StateLock {
        schema: "1.0".into(),
        machine: "web-01".into(),
        hostname: "web-01".into(),
        generated_at: "now".into(),
        generator: "test".into(),
        blake3_version: "1.0".into(),
        resources,
    }
}

#[test]
fn why_absent_with_lock_entry_destroys() {
    let r = Resource {
        resource_type: ResourceType::Package,
        state: Some("absent".into()),
        ..Default::default()
    };
    let mut locks = HashMap::new();
    locks.insert(
        "web-01".into(),
        make_lock("nginx-pkg", "hash123", ResourceStatus::Converged),
    );
    let reason = explain_why("nginx-pkg", &r, "web-01", &locks);
    assert_eq!(reason.action, PlanAction::Destroy);
    assert!(!reason.reasons.is_empty());
}

#[test]
fn why_absent_no_lock_entry_noop() {
    let r = Resource {
        resource_type: ResourceType::Package,
        state: Some("absent".into()),
        ..Default::default()
    };
    let locks = HashMap::new();
    let reason = explain_why("nginx-pkg", &r, "web-01", &locks);
    assert_eq!(reason.action, PlanAction::NoOp);
}

#[test]
fn why_no_lock_file_first_apply() {
    let r = Resource {
        resource_type: ResourceType::Package,
        packages: vec!["nginx".into()],
        ..Default::default()
    };
    let locks = HashMap::new();
    let reason = explain_why("nginx-pkg", &r, "web-01", &locks);
    assert_eq!(reason.action, PlanAction::Create);
    assert!(reason.reasons.iter().any(|r| r.contains("first apply")));
}

#[test]
fn why_new_resource_creates() {
    let r = Resource {
        resource_type: ResourceType::Package,
        packages: vec!["nginx".into()],
        ..Default::default()
    };
    let mut locks = HashMap::new();
    // Lock exists but doesn't contain this resource
    locks.insert(
        "web-01".into(),
        make_lock("other-pkg", "hash", ResourceStatus::Converged),
    );
    let reason = explain_why("nginx-pkg", &r, "web-01", &locks);
    assert_eq!(reason.action, PlanAction::Create);
    assert!(reason.reasons.iter().any(|r| r.contains("new resource")));
}

#[test]
fn why_failed_retries() {
    let r = Resource {
        resource_type: ResourceType::Package,
        packages: vec!["nginx".into()],
        ..Default::default()
    };
    let mut locks = HashMap::new();
    locks.insert(
        "web-01".into(),
        make_lock("nginx-pkg", "hash", ResourceStatus::Failed),
    );
    let reason = explain_why("nginx-pkg", &r, "web-01", &locks);
    assert_eq!(reason.action, PlanAction::Update);
    assert!(reason.reasons.iter().any(|r| r.contains("retry")));
}

#[test]
fn why_drifted_updates() {
    let r = Resource {
        resource_type: ResourceType::Package,
        packages: vec!["nginx".into()],
        ..Default::default()
    };
    let mut locks = HashMap::new();
    locks.insert(
        "web-01".into(),
        make_lock("nginx-pkg", "hash", ResourceStatus::Drifted),
    );
    let reason = explain_why("nginx-pkg", &r, "web-01", &locks);
    assert_eq!(reason.action, PlanAction::Update);
    assert!(reason.reasons.iter().any(|r| r.contains("drifted")));
}

// ============================================================================
// FJ-1379: format_why
// ============================================================================

#[test]
fn why_format_includes_resource_machine_action() {
    let r = Resource {
        resource_type: ResourceType::Package,
        state: Some("absent".into()),
        ..Default::default()
    };
    let locks = HashMap::new();
    let reason = explain_why("nginx-pkg", &r, "web-01", &locks);
    let output = format_why(&reason);
    assert!(output.contains("nginx-pkg"));
    assert!(output.contains("web-01"));
}

// ============================================================================
// FJ-004: hash_desired_state
// ============================================================================

#[test]
fn hash_desired_state_deterministic() {
    let r = Resource {
        resource_type: ResourceType::File,
        path: Some("/etc/app.conf".into()),
        content: Some("key=value".into()),
        mode: Some("0644".into()),
        ..Default::default()
    };
    let h1 = hash_desired_state(&r);
    let h2 = hash_desired_state(&r);
    assert_eq!(h1, h2, "same resource must produce same hash");
    assert!(h1.starts_with("blake3:"), "hash must have blake3 prefix");
}

#[test]
fn hash_desired_state_sensitive_to_content() {
    let r1 = Resource {
        resource_type: ResourceType::File,
        content: Some("version-a".into()),
        ..Default::default()
    };
    let r2 = Resource {
        resource_type: ResourceType::File,
        content: Some("version-b".into()),
        ..Default::default()
    };
    assert_ne!(
        hash_desired_state(&r1),
        hash_desired_state(&r2),
        "different content must produce different hash"
    );
}

#[test]
fn hash_desired_state_sensitive_to_type() {
    let r1 = Resource {
        resource_type: ResourceType::File,
        ..Default::default()
    };
    let r2 = Resource {
        resource_type: ResourceType::Package,
        ..Default::default()
    };
    assert_ne!(
        hash_desired_state(&r1),
        hash_desired_state(&r2),
        "different resource types must produce different hash"
    );
}
