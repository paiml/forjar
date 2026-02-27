use super::*;
use super::tests_helpers::make_config;
use std::collections::HashMap;

#[test]
fn test_fj004_plan_all_create() {
let config = make_config();
let order = vec!["pkg".to_string(), "conf".to_string(), "svc".to_string()];
let locks = HashMap::new();
let plan = plan(&config, &order, &locks, None);

assert_eq!(plan.to_create, 3);
assert_eq!(plan.to_update, 0);
assert_eq!(plan.unchanged, 0);
assert_eq!(plan.changes.len(), 3);
assert!(plan.changes.iter().all(|c| c.action == PlanAction::Create));
}

#[test]
fn test_fj004_plan_all_unchanged() {
let config = make_config();
let order = vec!["pkg".to_string(), "conf".to_string(), "svc".to_string()];

// Create locks that match desired state
let mut resources = indexmap::IndexMap::new();
for id in &order {
    let resource = &config.resources[id];
    resources.insert(
        id.clone(),
        ResourceLock {
            resource_type: resource.resource_type.clone(),
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: hash_desired_state(resource),
            details: HashMap::new(),
        },
    );
}
let lock = StateLock {
    schema: "1.0".to_string(),
    machine: "m1".to_string(),
    hostname: "m1".to_string(),
    generated_at: "2026-01-01T00:00:00Z".to_string(),
    generator: "forjar".to_string(),
    blake3_version: "1.8".to_string(),
    resources,
};
let mut locks = HashMap::new();
locks.insert("m1".to_string(), lock);

let plan = plan(&config, &order, &locks, None);
assert_eq!(plan.unchanged, 3);
assert_eq!(plan.to_create, 0);
}

#[test]
fn test_fj004_plan_update_on_hash_mismatch() {
let config = make_config();
let order = vec!["pkg".to_string()];

let mut resources = indexmap::IndexMap::new();
resources.insert(
    "pkg".to_string(),
    ResourceLock {
        resource_type: ResourceType::Package,
        status: ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        hash: "blake3:stale_hash".to_string(),
        details: HashMap::new(),
    },
);
let lock = StateLock {
    schema: "1.0".to_string(),
    machine: "m1".to_string(),
    hostname: "m1".to_string(),
    generated_at: "2026-01-01T00:00:00Z".to_string(),
    generator: "forjar".to_string(),
    blake3_version: "1.8".to_string(),
    resources,
};
let mut locks = HashMap::new();
locks.insert("m1".to_string(), lock);

let plan = plan(&config, &order, &locks, None);
assert_eq!(plan.to_update, 1);
}

#[test]
fn test_fj004_plan_destroy() {
let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  old-file:\n    type: file\n    machine: m1\n    path: /tmp/gone\n    state: absent\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["old-file".to_string()];

let mut resources = indexmap::IndexMap::new();
resources.insert(
    "old-file".to_string(),
    ResourceLock {
        resource_type: ResourceType::File,
        status: ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        hash: "blake3:xxx".to_string(),
        details: HashMap::new(),
    },
);
let lock = StateLock {
    schema: "1.0".to_string(),
    machine: "m1".to_string(),
    hostname: "m1".to_string(),
    generated_at: "2026-01-01T00:00:00Z".to_string(),
    generator: "forjar".to_string(),
    blake3_version: "1.8".to_string(),
    resources,
};
let mut locks = HashMap::new();
locks.insert("m1".to_string(), lock);

let plan = plan(&config, &order, &locks, None);
assert_eq!(plan.to_destroy, 1);
}

#[test]
fn test_fj004_plan_failed_resource_gets_retried() {
let config = make_config();
let order = vec!["pkg".to_string()];

let mut resources = indexmap::IndexMap::new();
resources.insert(
    "pkg".to_string(),
    ResourceLock {
        resource_type: ResourceType::Package,
        status: ResourceStatus::Failed,
        applied_at: None,
        duration_seconds: None,
        hash: "blake3:xxx".to_string(),
        details: HashMap::new(),
    },
);
let lock = StateLock {
    schema: "1.0".to_string(),
    machine: "m1".to_string(),
    hostname: "m1".to_string(),
    generated_at: "2026-01-01T00:00:00Z".to_string(),
    generator: "forjar".to_string(),
    blake3_version: "1.8".to_string(),
    resources,
};
let mut locks = HashMap::new();
locks.insert("m1".to_string(), lock);

let plan = plan(&config, &order, &locks, None);
assert_eq!(plan.to_update, 1);
}

#[test]
fn test_fj004_absent_not_in_lock_is_noop() {
let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  gone-file:\n    type: file\n    machine: m1\n    path: /tmp/gone\n    state: absent\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["gone-file".to_string()];
let locks = HashMap::new(); // no lock — resource never existed
let plan = plan(&config, &order, &locks, None);
assert_eq!(
    plan.unchanged, 1,
    "absent resource not in lock should be NoOp"
);
}

#[test]
fn test_fj004_plan_with_broken_template_fallback() {
// Template resolution error triggers fallback to unresolved resource
let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.1.1.1\nresources:\n  config:\n    type: file\n    machine: m1\n    path: /etc/test\n    content: \"{{params.missing_key}}\"\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["config".to_string()];
let locks = HashMap::new();
// Should not panic — falls back to unresolved resource
let plan = plan(&config, &order, &locks, None);
assert_eq!(plan.to_create, 1);
}

#[test]
fn test_fj004_multi_machine() {
let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  a:\n    hostname: a\n    addr: 1.1.1.1\n  b:\n    hostname: b\n    addr: 2.2.2.2\nresources:\n  tools:\n    type: package\n    machine: [a, b]\n    provider: cargo\n    packages: [batuta]\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["tools".to_string()];
let locks = HashMap::new();
let plan = plan(&config, &order, &locks, None);
// One resource on two machines = 2 planned changes
assert_eq!(plan.changes.len(), 2);
assert_eq!(plan.to_create, 2);
}

#[test]
fn test_fj004_multi_machine_partial_lock() {
// One machine converged, other is new
let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  a:\n    hostname: a\n    addr: 1.1.1.1\n  b:\n    hostname: b\n    addr: 2.2.2.2\nresources:\n  pkg:\n    type: package\n    machine: [a, b]\n    provider: apt\n    packages: [curl]\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["pkg".to_string()];
let resource = &config.resources["pkg"];

let mut a_resources = indexmap::IndexMap::new();
a_resources.insert(
    "pkg".to_string(),
    ResourceLock {
        resource_type: ResourceType::Package,
        status: ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        hash: hash_desired_state(resource),
        details: HashMap::new(),
    },
);
let mut locks = HashMap::new();
locks.insert(
    "a".to_string(),
    StateLock {
        schema: "1.0".to_string(),
        machine: "a".to_string(),
        hostname: "a".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar".to_string(),
        blake3_version: "1.8".to_string(),
        resources: a_resources,
    },
);

let plan = plan(&config, &order, &locks, None);
assert_eq!(plan.changes.len(), 2);
// a is unchanged, b is create
assert_eq!(plan.unchanged, 1);
assert_eq!(plan.to_create, 1);
}

#[test]
fn test_fj004_empty_execution_order() {
let config = make_config();
let order: Vec<String> = vec![];
let locks = HashMap::new();
let plan = plan(&config, &order, &locks, None);
assert_eq!(plan.changes.len(), 0);
assert_eq!(plan.to_create, 0);
}

#[test]
fn test_fj004_nonexistent_resource_in_order_skipped() {
let config = make_config();
let order = vec!["nonexistent".to_string(), "pkg".to_string()];
let locks = HashMap::new();
let plan = plan(&config, &order, &locks, None);
// Only pkg should be planned
assert_eq!(plan.changes.len(), 1);
assert_eq!(plan.changes[0].resource_id, "pkg");
}

#[test]
fn test_fj004_plan_name_from_config() {
let config = make_config();
let order = vec![];
let locks = HashMap::new();
let plan = plan(&config, &order, &locks, None);
assert_eq!(plan.name, "test");
}

#[test]
fn test_fj004_determine_action_mount_default_state() {
// Exercises `ResourceType::Mount => "mounted"` default state branch
let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  nfs-share:\n    type: mount\n    machine: m1\n    source: \"nas:/data\"\n    path: /mnt/data\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["nfs-share".to_string()];
let locks = HashMap::new();
let plan = plan(&config, &order, &locks, None);
assert_eq!(plan.to_create, 1);
}

#[test]
fn test_fj004_determine_action_service_default_state() {
// Exercises `ResourceType::Service => "running"` default state branch
// (no explicit state field)
let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  web:\n    type: service\n    machine: m1\n    name: nginx\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["web".to_string()];
let locks = HashMap::new();
let plan = plan(&config, &order, &locks, None);
assert_eq!(plan.to_create, 1);
}

#[test]
fn test_fj004_determine_action_default_state_non_standard_type() {
// Exercises the `_ => "present"` default state branch for non-standard
// resource types (User, Docker, Network, etc.)
let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  my-user:\n    type: user\n    machine: m1\n    name: deploy\n";
let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
let order = vec!["my-user".to_string()];
let locks = HashMap::new();
let plan = plan(&config, &order, &locks, None);
assert_eq!(plan.to_create, 1);
}

#[test]
fn test_fj132_plan_empty_order() {
let config = make_config();
let order: Vec<String> = vec![];
let locks = HashMap::new();
let p = plan(&config, &order, &locks, None);
assert_eq!(p.to_create, 0);
assert_eq!(p.to_update, 0);
assert_eq!(p.unchanged, 0);
assert_eq!(p.changes.len(), 0);
}

#[test]
fn test_fj132_plan_order_with_unknown_resource() {
let config = make_config();
let order = vec!["nonexistent".to_string()];
let locks = HashMap::new();
let p = plan(&config, &order, &locks, None);
// Unknown resource ID should be silently skipped
assert_eq!(p.to_create, 0);
assert_eq!(p.changes.len(), 0);
}

#[test]
fn test_fj132_plan_mixed_actions() {
// Config with one new resource and one unchanged resource
let config = make_config();
let order = vec!["pkg".to_string(), "conf".to_string()];

let mut resources = indexmap::IndexMap::new();
// pkg matches its hash — unchanged
resources.insert(
    "pkg".to_string(),
    ResourceLock {
        resource_type: ResourceType::Package,
        status: ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        hash: hash_desired_state(&config.resources["pkg"]),
        details: HashMap::new(),
    },
);
let lock = StateLock {
    schema: "1.0".to_string(),
    machine: "m1".to_string(),
    hostname: "m1".to_string(),
    generated_at: "2026-01-01T00:00:00Z".to_string(),
    generator: "forjar".to_string(),
    blake3_version: "1.8".to_string(),
    resources,
};
let mut locks = HashMap::new();
locks.insert("m1".to_string(), lock);

let p = plan(&config, &order, &locks, None);
assert_eq!(p.unchanged, 1, "pkg should be unchanged");
assert_eq!(p.to_create, 1, "conf should be created");
}
