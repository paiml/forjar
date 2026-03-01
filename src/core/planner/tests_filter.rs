#![allow(unused_imports)]
use super::*;
use super::tests_helpers::{make_config, make_base_resource};
use std::collections::HashMap;

#[test]
fn test_tag_filter_excludes_untagged_from_plan() {
let yaml = "\nversion: \"1.0\"\nname: tag-plan-test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  tagged:\n    type: file\n    machine: m\n    path: /tmp/tagged\n    content: \"yes\"\n    tags: [web, critical]\n  untagged:\n    type: file\n    machine: m\n    path: /tmp/untagged\n    content: \"no\"\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["tagged".to_string(), "untagged".to_string()];
let locks = HashMap::new();

// With tag filter: only tagged resource
let filtered = plan(&config, &order, &locks, Some("web"));
assert_eq!(filtered.changes.len(), 1);
assert_eq!(filtered.changes[0].resource_id, "tagged");

// Without tag filter: both resources
let unfiltered = plan(&config, &order, &locks, None);
assert_eq!(unfiltered.changes.len(), 2);
}

#[test]
fn test_tag_filter_no_matches_empty_plan() {
let yaml = "\nversion: \"1.0\"\nname: tag-plan-empty\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  f:\n    type: file\n    machine: m\n    path: /tmp/test\n    content: \"hello\"\n    tags: [web]\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["f".to_string()];
let locks = HashMap::new();

let plan = plan(&config, &order, &locks, Some("db"));
assert_eq!(plan.changes.len(), 0);
}

#[test]
fn test_fj004_arch_filter_skips_mismatched_machine() {
let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  x86:\n    hostname: x86\n    addr: 1.1.1.1\n    arch: x86_64\n  arm:\n    hostname: arm\n    addr: 2.2.2.2\n    arch: aarch64\nresources:\n  intel-only:\n    type: package\n    machine: [x86, arm]\n    provider: apt\n    packages: [intel-microcode]\n    arch: [x86_64]\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["intel-only".to_string()];
let locks = HashMap::new();
let plan = plan(&config, &order, &locks, None);
// Only x86 should get a planned change, arm is skipped
assert_eq!(plan.changes.len(), 1);
assert_eq!(plan.changes[0].machine, "x86");
}

#[test]
fn test_fj004_arch_filter_with_existing_lock() {
// Even if arm has a lock entry, arch filter still skips it
let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  x86:\n    hostname: x86\n    addr: 1.1.1.1\n    arch: x86_64\n  arm:\n    hostname: arm\n    addr: 2.2.2.2\n    arch: aarch64\nresources:\n  intel-only:\n    type: package\n    machine: [x86, arm]\n    provider: apt\n    packages: [intel-microcode]\n    arch: [x86_64]\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["intel-only".to_string()];

// Simulate arm having a converged lock for this resource
let mut arm_resources = indexmap::IndexMap::new();
arm_resources.insert(
    "intel-only".to_string(),
    ResourceLock {
        resource_type: ResourceType::Package,
        status: ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        hash: "blake3:old".to_string(),
        details: HashMap::new(),
    },
);
let mut locks = HashMap::new();
locks.insert(
    "arm".to_string(),
    StateLock {
        schema: "1.0".to_string(),
        machine: "arm".to_string(),
        hostname: "arm".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar".to_string(),
        blake3_version: "1.8".to_string(),
        resources: arm_resources,
    },
);

let plan = plan(&config, &order, &locks, None);
// arm skipped by arch filter, x86 is create
assert_eq!(plan.changes.len(), 1);
assert_eq!(plan.changes[0].machine, "x86");
assert_eq!(plan.changes[0].action, PlanAction::Create);
}

#[test]
fn test_fj004_arch_and_tag_filter_combined() {
let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  x86:\n    hostname: x86\n    addr: 1.1.1.1\n    arch: x86_64\n  arm:\n    hostname: arm\n    addr: 2.2.2.2\n    arch: aarch64\nresources:\n  driver:\n    type: package\n    machine: [x86, arm]\n    provider: apt\n    packages: [intel-microcode]\n    arch: [x86_64]\n    tags: [infra]\n  generic:\n    type: file\n    machine: [x86, arm]\n    path: /etc/test\n    content: hello\n    tags: [infra]\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["driver".to_string(), "generic".to_string()];
let locks = HashMap::new();

// Tag filter "infra" + arch filter: driver only on x86, generic on both
let plan = plan(&config, &order, &locks, Some("infra"));
assert_eq!(plan.changes.len(), 3); // driver:x86, generic:x86, generic:arm
assert_eq!(plan.to_create, 3);
}

#[test]
fn test_fj132_plan_tag_filter_excludes_untagged() {
let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  tagged:\n    type: file\n    machine: m1\n    path: /tmp/tagged\n    content: \"tagged\"\n    tags: [web]\n  untagged:\n    type: file\n    machine: m1\n    path: /tmp/untagged\n    content: \"untagged\"\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["tagged".to_string(), "untagged".to_string()];
let locks = HashMap::new();
let p = plan(&config, &order, &locks, Some("web"));
// Only tagged resource should appear in plan
assert_eq!(p.to_create, 1);
assert_eq!(p.changes.len(), 1);
assert_eq!(p.changes[0].resource_id, "tagged");
}

#[test]
fn test_fj132_plan_tag_filter_no_match() {
let config = make_config();
let order = vec!["pkg".to_string(), "conf".to_string(), "svc".to_string()];
let locks = HashMap::new();
let p = plan(&config, &order, &locks, Some("nonexistent-tag"));
assert_eq!(p.to_create, 0);
assert_eq!(p.changes.len(), 0);
}

#[test]
fn test_fj132_plan_arch_filter_skips_mismatched() {
let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  x86:\n    hostname: x86\n    addr: 127.0.0.1\n    arch: x86_64\n  arm:\n    hostname: arm\n    addr: 10.0.0.1\n    arch: aarch64\nresources:\n  intel-only:\n    type: file\n    machine: [x86, arm]\n    path: /tmp/intel\n    content: \"intel\"\n    arch: [x86_64]\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["intel-only".to_string()];
let locks = HashMap::new();
let p = plan(&config, &order, &locks, None);
// Should create for x86 but skip arm
assert_eq!(p.to_create, 1);
assert_eq!(p.changes.len(), 1);
}
