//! FJ-004/1382/1385/1379: Planner, reversibility, proof obligations, and why-explain falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-004: Plan generation — create/update/destroy/noop actions
//!   - hash_desired_state determinism
//!   - plan() with empty config, tag filters, state=absent
//! - FJ-1382: Reversibility classification
//!   - classify for all resource types and actions
//!   - count_irreversible, warn_irreversible
//! - FJ-1385: Proof obligation taxonomy
//!   - Idempotent/Monotonic/Convergent/Destructive classification
//!   - label, is_safe predicates
//! - FJ-1379: Why-explain change reasons
//!   - explain_why for create/update/destroy/noop
//!
//! Usage: cargo test --test falsification_planner

use forjar::core::planner::proof_obligation::{self, ProofObligation};
use forjar::core::planner::reversibility::{self, Reversibility};
use forjar::core::planner::why::explain_why;
use forjar::core::planner::{hash_desired_state, plan};
use forjar::core::types::*;
use indexmap::IndexMap;
use std::collections::HashMap;

// ============================================================================
// Helpers
// ============================================================================

fn minimal_config() -> ForjarConfig {
    ForjarConfig {
        version: "1.0".into(),
        name: "test".into(),
        ..Default::default()
    }
}

fn file_resource(path: &str, content: &str) -> Resource {
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.path = Some(path.into());
    r.content = Some(content.into());
    r.machine = MachineTarget::Single("m1".into());
    r
}

fn resource_lock(hash: &str) -> ResourceLock {
    ResourceLock {
        resource_type: ResourceType::File,
        status: ResourceStatus::Converged,
        hash: hash.into(),
        applied_at: Some("now".into()),
        duration_seconds: None,
        details: HashMap::new(),
    }
}

fn machine(name: &str) -> Machine {
    Machine {
        hostname: name.into(),
        addr: "127.0.0.1".into(),
        user: "root".into(),
        arch: "x86_64".into(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

// ============================================================================
// FJ-004: hash_desired_state — determinism
// ============================================================================

#[test]
fn hash_deterministic() {
    let r = file_resource("/etc/test.conf", "hello");
    let h1 = hash_desired_state(&r);
    let h2 = hash_desired_state(&r);
    assert_eq!(h1, h2, "same resource should produce same hash");
}

#[test]
fn hash_differs_for_different_content() {
    let r1 = file_resource("/etc/test.conf", "hello");
    let r2 = file_resource("/etc/test.conf", "world");
    assert_ne!(
        hash_desired_state(&r1),
        hash_desired_state(&r2),
        "different content should produce different hash"
    );
}

#[test]
fn hash_differs_for_different_path() {
    let r1 = file_resource("/etc/a.conf", "same");
    let r2 = file_resource("/etc/b.conf", "same");
    assert_ne!(
        hash_desired_state(&r1),
        hash_desired_state(&r2),
        "different paths should produce different hash"
    );
}

#[test]
fn hash_differs_for_different_type() {
    let mut r1 = Resource::default();
    r1.resource_type = ResourceType::File;
    r1.name = Some("test".into());

    let mut r2 = Resource::default();
    r2.resource_type = ResourceType::Package;
    r2.name = Some("test".into());

    assert_ne!(hash_desired_state(&r1), hash_desired_state(&r2));
}

// ============================================================================
// FJ-004: plan() — create action (no lock)
// ============================================================================

#[test]
fn plan_creates_when_no_lock() {
    let mut config = minimal_config();
    config.machines.insert("m1".into(), machine("m1"));
    config
        .resources
        .insert("cfg".into(), file_resource("/etc/cfg", "data"));

    let order = vec!["cfg".to_string()];
    let locks = HashMap::new();

    let plan = plan(&config, &order, &locks, None);
    assert_eq!(plan.to_create, 1);
    assert_eq!(plan.changes.len(), 1);
    assert_eq!(plan.changes[0].action, PlanAction::Create);
}

// ============================================================================
// FJ-004: plan() — noop when hash matches
// ============================================================================

#[test]
fn plan_noop_when_converged_same_hash() {
    let mut config = minimal_config();
    config.machines.insert("m1".into(), machine("m1"));
    let r = file_resource("/etc/cfg", "data");
    let desired_hash = hash_desired_state(&r);
    config.resources.insert("cfg".into(), r);

    let order = vec!["cfg".to_string()];
    let mut locks = HashMap::new();
    let mut lock = StateLock {
        schema: "1.0".into(),
        machine: "m1".into(),
        hostname: "m1".into(),
        generated_at: "now".into(),
        generator: "test".into(),
        blake3_version: "1.8".into(),
        resources: IndexMap::new(),
    };
    lock.resources
        .insert("cfg".into(), resource_lock(&desired_hash));
    locks.insert("m1".into(), lock);

    let plan = plan(&config, &order, &locks, None);
    assert_eq!(plan.unchanged, 1);
    assert_eq!(plan.to_create, 0);
    assert_eq!(plan.to_update, 0);
}

// ============================================================================
// FJ-004: plan() — update when hash differs
// ============================================================================

#[test]
fn plan_updates_when_hash_differs() {
    let mut config = minimal_config();
    config.machines.insert("m1".into(), machine("m1"));
    config
        .resources
        .insert("cfg".into(), file_resource("/etc/cfg", "new-data"));

    let order = vec!["cfg".to_string()];
    let mut locks = HashMap::new();
    let mut lock = StateLock {
        schema: "1.0".into(),
        machine: "m1".into(),
        hostname: "m1".into(),
        generated_at: "now".into(),
        generator: "test".into(),
        blake3_version: "1.8".into(),
        resources: IndexMap::new(),
    };
    lock.resources
        .insert("cfg".into(), resource_lock("old-hash"));
    locks.insert("m1".into(), lock);

    let plan = plan(&config, &order, &locks, None);
    assert_eq!(plan.to_update, 1);
}

// ============================================================================
// FJ-004: plan() — destroy when state=absent
// ============================================================================

#[test]
fn plan_destroys_absent_resource() {
    let mut config = minimal_config();
    config.machines.insert("m1".into(), machine("m1"));
    let mut r = file_resource("/etc/old", "data");
    r.state = Some("absent".into());
    config.resources.insert("old".into(), r);

    let order = vec!["old".to_string()];
    let mut locks = HashMap::new();
    let mut lock = StateLock {
        schema: "1.0".into(),
        machine: "m1".into(),
        hostname: "m1".into(),
        generated_at: "now".into(),
        generator: "test".into(),
        blake3_version: "1.8".into(),
        resources: IndexMap::new(),
    };
    lock.resources
        .insert("old".into(), resource_lock("some-hash"));
    locks.insert("m1".into(), lock);

    let plan = plan(&config, &order, &locks, None);
    assert_eq!(plan.to_destroy, 1);
}

// ============================================================================
// FJ-004: plan() — tag filter
// ============================================================================

#[test]
fn plan_tag_filter_includes_matching() {
    let mut config = minimal_config();
    config.machines.insert("m1".into(), machine("m1"));
    let mut r = file_resource("/etc/web.conf", "data");
    r.tags = vec!["web".into()];
    config.resources.insert("web-cfg".into(), r);

    let order = vec!["web-cfg".to_string()];
    let plan = plan(&config, &order, &HashMap::new(), Some("web"));
    assert_eq!(plan.changes.len(), 1);
}

#[test]
fn plan_tag_filter_excludes_non_matching() {
    let mut config = minimal_config();
    config.machines.insert("m1".into(), machine("m1"));
    let mut r = file_resource("/etc/db.conf", "data");
    r.tags = vec!["db".into()];
    config.resources.insert("db-cfg".into(), r);

    let order = vec!["db-cfg".to_string()];
    let plan = plan(&config, &order, &HashMap::new(), Some("web"));
    assert_eq!(plan.changes.len(), 0);
}

// ============================================================================
// FJ-1382: Reversibility — classify
// ============================================================================

#[test]
fn reversibility_noop_reversible() {
    let r = Resource::default();
    assert_eq!(
        reversibility::classify(&r, &PlanAction::NoOp),
        Reversibility::Reversible
    );
}

#[test]
fn reversibility_create_reversible() {
    let r = Resource::default();
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Create),
        Reversibility::Reversible
    );
}

#[test]
fn reversibility_file_destroy_with_content() {
    let r = file_resource("/etc/test", "data");
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Reversible,
        "file with content is re-creatable"
    );
}

#[test]
fn reversibility_file_destroy_without_content() {
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Irreversible,
        "file without content/source cannot be re-created"
    );
}

#[test]
fn reversibility_user_destroy_irreversible() {
    let mut r = Resource::default();
    r.resource_type = ResourceType::User;
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Irreversible
    );
}

#[test]
fn reversibility_service_destroy_reversible() {
    let mut r = Resource::default();
    r.resource_type = ResourceType::Service;
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Reversible
    );
}

#[test]
fn reversibility_package_destroy_reversible() {
    let mut r = Resource::default();
    r.resource_type = ResourceType::Package;
    assert_eq!(
        reversibility::classify(&r, &PlanAction::Destroy),
        Reversibility::Reversible
    );
}

// ============================================================================
// FJ-1382: count_irreversible / warn_irreversible
// ============================================================================

#[test]
fn count_irreversible_plan() {
    let mut config = minimal_config();
    let mut user = Resource::default();
    user.resource_type = ResourceType::User;
    user.state = Some("absent".into());
    config.resources.insert("user1".into(), user);

    let plan_result = ExecutionPlan {
        name: "test".into(),
        changes: vec![PlannedChange {
            resource_id: "user1".into(),
            machine: "m1".into(),
            resource_type: ResourceType::User,
            action: PlanAction::Destroy,
            description: "destroy user".into(),
        }],
        execution_order: vec!["user1".into()],
        to_create: 0,
        to_update: 0,
        to_destroy: 1,
        unchanged: 0,
    };

    assert_eq!(reversibility::count_irreversible(&config, &plan_result), 1);
    let warnings = reversibility::warn_irreversible(&config, &plan_result);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("irreversible"));
}

// ============================================================================
// FJ-1385: Proof obligations — classify
// ============================================================================

#[test]
fn proof_noop_idempotent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::File, &PlanAction::NoOp),
        ProofObligation::Idempotent
    );
}

#[test]
fn proof_create_file_idempotent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::File, &PlanAction::Create),
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
fn proof_update_file_idempotent() {
    assert_eq!(
        proof_obligation::classify(&ResourceType::File, &PlanAction::Update),
        ProofObligation::Idempotent
    );
}

// ============================================================================
// FJ-1385: label and is_safe
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
fn proof_is_safe() {
    assert!(proof_obligation::is_safe(&ProofObligation::Idempotent));
    assert!(proof_obligation::is_safe(&ProofObligation::Monotonic));
    assert!(proof_obligation::is_safe(&ProofObligation::Convergent));
    assert!(!proof_obligation::is_safe(&ProofObligation::Destructive));
}

// ============================================================================
// FJ-1379: explain_why — create (no lock)
// ============================================================================

#[test]
fn why_create_no_lock() {
    let r = file_resource("/etc/cfg", "data");
    let reason = explain_why("cfg", &r, "m1", &HashMap::new());
    assert_eq!(reason.action, PlanAction::Create);
    assert!(!reason.reasons.is_empty());
}

// ============================================================================
// FJ-1379: explain_why — noop (converged, matching hash)
// ============================================================================

#[test]
fn why_noop_matching_hash() {
    let r = file_resource("/etc/cfg", "data");
    let desired_hash = hash_desired_state(&r);

    let mut locks = HashMap::new();
    let mut lock = StateLock {
        schema: "1.0".into(),
        machine: "m1".into(),
        hostname: "m1".into(),
        generated_at: "now".into(),
        generator: "test".into(),
        blake3_version: "1.8".into(),
        resources: IndexMap::new(),
    };
    lock.resources
        .insert("cfg".into(), resource_lock(&desired_hash));
    locks.insert("m1".into(), lock);

    let reason = explain_why("cfg", &r, "m1", &locks);
    assert_eq!(reason.action, PlanAction::NoOp);
}

// ============================================================================
// FJ-1379: explain_why — destroy (state=absent)
// ============================================================================

#[test]
fn why_destroy_absent() {
    let mut r = file_resource("/etc/old", "data");
    r.state = Some("absent".into());

    let mut locks = HashMap::new();
    let mut lock = StateLock {
        schema: "1.0".into(),
        machine: "m1".into(),
        hostname: "m1".into(),
        generated_at: "now".into(),
        generator: "test".into(),
        blake3_version: "1.8".into(),
        resources: IndexMap::new(),
    };
    lock.resources.insert("old".into(), resource_lock("h"));
    locks.insert("m1".into(), lock);

    let reason = explain_why("old", &r, "m1", &locks);
    assert_eq!(reason.action, PlanAction::Destroy);
    assert!(reason.reasons.iter().any(|r| r.contains("absent")));
}
