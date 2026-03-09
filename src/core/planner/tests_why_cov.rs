//! Extended coverage for why.rs — drifted status, absent no-destroy,
//! diff_resource_fields branches, truncate_hash, format_why edge cases.

use super::why::{explain_why, format_why};
use crate::core::types::*;
use indexmap::IndexMap;
use std::collections::HashMap;

fn minimal_resource(rtype: ResourceType) -> Resource {
    Resource {
        resource_type: rtype,
        machine: MachineTarget::Single("local".to_string()),
        ..Resource::default()
    }
}

fn make_lock(
    machine: &str,
    resources: IndexMap<String, ResourceLock>,
) -> HashMap<String, StateLock> {
    let mut locks = HashMap::new();
    locks.insert(
        machine.to_string(),
        StateLock {
            schema: "v1".to_string(),
            machine: machine.to_string(),
            hostname: "localhost".to_string(),
            generated_at: "2026-03-08T12:00:00Z".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.5.5".to_string(),
            resources,
        },
    );
    locks
}

// ── explain_absent: resource not in lock ─────────────────────────

#[test]
fn absent_not_in_lock_noop() {
    let mut resource = minimal_resource(ResourceType::File);
    resource.state = Some("absent".to_string());

    let locks = make_lock("local", IndexMap::new());
    let reason = explain_why("missing", &resource, "local", &locks);
    assert_eq!(reason.action, PlanAction::NoOp);
    assert!(reason.reasons.iter().any(|r| r.contains("not in lock")));
}

#[test]
fn absent_no_lock_at_all() {
    let mut resource = minimal_resource(ResourceType::Package);
    resource.state = Some("absent".to_string());

    let locks = HashMap::new();
    let reason = explain_why("pkg", &resource, "local", &locks);
    assert_eq!(reason.action, PlanAction::NoOp);
    assert!(reason.reasons.iter().any(|r| r.contains("not in lock")));
}

// ── explain_present: drifted status ─────────────────────────────

#[test]
fn present_drifted_update() {
    let resource = minimal_resource(ResourceType::Service);

    let mut rl_resources = IndexMap::new();
    rl_resources.insert(
        "svc".to_string(),
        ResourceLock {
            resource_type: ResourceType::Service,
            status: ResourceStatus::Drifted,
            applied_at: Some("2026-03-08T12:00:00Z".to_string()),
            duration_seconds: Some(0.1),
            hash: "blake3:xxx".to_string(),
            details: HashMap::new(),
        },
    );

    let locks = make_lock("local", rl_resources);
    let reason = explain_why("svc", &resource, "local", &locks);
    assert_eq!(reason.action, PlanAction::Update);
    assert!(reason.reasons.iter().any(|r| r.contains("drifted")));
}

// ── diff_resource_fields: path changed ──────────────────────────

#[test]
fn path_diff_detected() {
    let mut resource = minimal_resource(ResourceType::File);
    resource.path = Some("/new/path.txt".to_string());
    resource.content = Some("hello".to_string());

    let mut details = HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String("/old/path.txt".to_string()),
    );

    let mut rl_resources = IndexMap::new();
    rl_resources.insert(
        "f".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: Some("2026-03-08T12:00:00Z".to_string()),
            duration_seconds: Some(0.01),
            hash: "blake3:old".to_string(),
            details,
        },
    );

    let locks = make_lock("local", rl_resources);
    let reason = explain_why("f", &resource, "local", &locks);
    assert_eq!(reason.action, PlanAction::Update);
    assert!(reason.reasons.iter().any(|r| r.contains("path changed")));
}

// ── diff_resource_fields: version changed ───────────────────────

#[test]
fn version_diff_detected() {
    let mut resource = minimal_resource(ResourceType::Package);
    resource.version = Some("2.0.0".to_string());
    resource.packages = vec!["nginx".to_string()];
    resource.provider = Some("apt".to_string());

    let mut details = HashMap::new();
    details.insert(
        "version".to_string(),
        serde_yaml_ng::Value::String("1.0.0".to_string()),
    );

    let mut rl_resources = IndexMap::new();
    rl_resources.insert(
        "pkg".to_string(),
        ResourceLock {
            resource_type: ResourceType::Package,
            status: ResourceStatus::Converged,
            applied_at: Some("2026-03-08T12:00:00Z".to_string()),
            duration_seconds: Some(0.5),
            hash: "blake3:old".to_string(),
            details,
        },
    );

    let locks = make_lock("local", rl_resources);
    let reason = explain_why("pkg", &resource, "local", &locks);
    assert_eq!(reason.action, PlanAction::Update);
    assert!(reason.reasons.iter().any(|r| r.contains("version changed")));
}

// ── diff_resource_fields: packages changed ──────────────────────

#[test]
fn packages_diff_detected() {
    let mut resource = minimal_resource(ResourceType::Package);
    resource.packages = vec!["nginx".to_string(), "redis".to_string()];
    resource.provider = Some("apt".to_string());

    let mut details = HashMap::new();
    details.insert(
        "packages".to_string(),
        serde_yaml_ng::Value::String("nginx".to_string()),
    );

    let mut rl_resources = IndexMap::new();
    rl_resources.insert(
        "pkg".to_string(),
        ResourceLock {
            resource_type: ResourceType::Package,
            status: ResourceStatus::Converged,
            applied_at: Some("2026-03-08T12:00:00Z".to_string()),
            duration_seconds: Some(0.5),
            hash: "blake3:old".to_string(),
            details,
        },
    );

    let locks = make_lock("local", rl_resources);
    let reason = explain_why("pkg", &resource, "local", &locks);
    assert_eq!(reason.action, PlanAction::Update);
    assert!(reason
        .reasons
        .iter()
        .any(|r| r.contains("packages changed")));
}

// ── diff_resource_fields: no detail match → generic message ─────

#[test]
fn no_field_diffs_generic_message() {
    let mut resource = minimal_resource(ResourceType::File);
    resource.path = Some("/tmp/x".to_string());

    // Empty details → no specific diffs → generic "configuration fields changed"
    let mut rl_resources = IndexMap::new();
    rl_resources.insert(
        "f".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: Some("2026-03-08T12:00:00Z".to_string()),
            duration_seconds: Some(0.01),
            hash: "blake3:old".to_string(),
            details: HashMap::new(),
        },
    );

    let locks = make_lock("local", rl_resources);
    let reason = explain_why("f", &resource, "local", &locks);
    assert_eq!(reason.action, PlanAction::Update);
    assert!(reason
        .reasons
        .iter()
        .any(|r| r.contains("configuration fields changed")));
}

// ── format_why: multiple reasons ────────────────────────────────

#[test]
fn format_why_multiple_reasons() {
    let reason = super::why::ChangeReason {
        resource_id: "nginx".to_string(),
        machine: "web".to_string(),
        action: PlanAction::Update,
        reasons: vec![
            "hash changed: abc -> def".to_string(),
            "content changed".to_string(),
        ],
    };
    let output = format_why(&reason);
    assert!(output.contains("nginx"));
    assert!(output.contains("web"));
    assert!(output.contains("Update"));
    assert!(output.contains("  - hash changed"));
    assert!(output.contains("  - content changed"));
}

#[test]
fn format_why_noop() {
    let reason = super::why::ChangeReason {
        resource_id: "cfg".to_string(),
        machine: "local".to_string(),
        action: PlanAction::NoOp,
        reasons: vec!["hash unchanged (abc123)".to_string()],
    };
    let output = format_why(&reason);
    assert!(output.contains("NoOp"));
}
