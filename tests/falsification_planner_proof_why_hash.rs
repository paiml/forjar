//! FJ-1379/004: Change explanation and desired-state hashing falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-1379: Change explanation (--why)
//!   - explain_why: absent/present/no-lock/new-resource/failed/drifted/hash-change
//!   - format_why: human-readable output
//! - FJ-004: hash_desired_state
//!   - determinism: same resource → same hash
//!   - sensitivity: different content → different hash
//!
//! Split from `tests/falsification_planner_proof_reversibility.rs`, which held
//! all four falsifications until it reached the 500-line file-health limit
//! exactly and PMAT-565's one added field pushed it over.
//!
//! Usage: cargo test --test falsification_planner_proof_why_hash

use forjar::core::planner::hash_desired_state;
use forjar::core::planner::why::{explain_why, format_why};
use forjar::core::types::*;
use indexmap::IndexMap;
use std::collections::HashMap;

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
            observed: None,
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
        created_by: None,
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
fn why_absent_no_lock_entry_destroys() {
    // GH-339: this asserted NoOp. A package declared absent that forjar never
    // installed is still very likely INSTALLED on the box — that is the whole
    // reason to declare it absent. "Not in the lock" is a fact about forjar's
    // bookkeeping, not about the machine.
    let r = Resource {
        resource_type: ResourceType::Package,
        state: Some("absent".into()),
        ..Default::default()
    };
    let locks = HashMap::new();
    let reason = explain_why("nginx-pkg", &r, "web-01", &locks);
    assert_eq!(reason.action, PlanAction::Destroy);
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
