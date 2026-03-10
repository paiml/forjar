#![allow(dead_code)]
#![allow(clippy::field_reassign_with_default)]
//! FJ-1385/1379: Proof obligation labels/safety and why-explain
//! (split from falsification_planner).
//!
//! Usage: cargo test --test falsification_planner_b

use forjar::core::planner::hash_desired_state;
use forjar::core::planner::proof_obligation::{self, ProofObligation};
use forjar::core::planner::why::explain_why;
use forjar::core::types::*;
use indexmap::IndexMap;
use std::collections::HashMap;

// ============================================================================
// Helpers
// ============================================================================

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
