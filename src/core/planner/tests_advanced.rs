#![allow(unused_imports)]
use super::tests_helpers::{make_base_resource, make_config};
use super::*;
use std::collections::HashMap;

#[test]
fn test_fj036_plan_all_noop_when_converged() {
    // Config where all resources have matching hashes -> plan shows all NoOp
    let config = make_config();
    let order = vec!["pkg".to_string(), "conf".to_string(), "svc".to_string()];

    // Build locks with matching hashes for every resource
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

    let p = plan(&config, &order, &locks, None);

    assert_eq!(p.unchanged, 3, "all 3 resources should be unchanged");
    assert_eq!(p.to_create, 0, "nothing to create");
    assert_eq!(p.to_update, 0, "nothing to update");
    assert_eq!(p.to_destroy, 0, "nothing to destroy");
    assert_eq!(p.changes.len(), 3, "all 3 resources should appear in plan");
    assert!(
        p.changes.iter().all(|c| c.action == PlanAction::NoOp),
        "every action should be NoOp"
    );
}

#[test]
fn test_fj036_plan_respects_resource_filter() {
    // With 3 resources, filter execution_order to 1, verify only that 1 appears in plan
    let config = make_config();
    let locks = HashMap::new();

    // Only include "conf" in execution order (not pkg or svc)
    let filtered_order = vec!["conf".to_string()];
    let p = plan(&config, &filtered_order, &locks, None);

    assert_eq!(p.changes.len(), 1, "only 1 resource should appear in plan");
    assert_eq!(
        p.changes[0].resource_id, "conf",
        "the planned resource should be 'conf'"
    );
    assert_eq!(
        p.changes[0].action,
        PlanAction::Create,
        "conf should be Create since no locks exist"
    );
}

#[test]
fn test_fj036_plan_absent_resource_destroy() {
    // Resource with state=absent that exists in lock -> plan shows Destroy action
    let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  old-config:\n    type: file\n    machine: m1\n    path: /etc/old.conf\n    state: absent\n";
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let order = vec!["old-config".to_string()];

    // Create a lock entry for this resource (it exists on the machine)
    let mut lock_resources = indexmap::IndexMap::new();
    lock_resources.insert(
        "old-config".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:some_existing_hash".to_string(),
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
        resources: lock_resources,
    };
    let mut locks = HashMap::new();
    locks.insert("m1".to_string(), lock);

    let p = plan(&config, &order, &locks, None);

    assert_eq!(p.to_destroy, 1, "should have 1 destroy action");
    assert_eq!(p.changes.len(), 1);
    assert_eq!(
        p.changes[0].action,
        PlanAction::Destroy,
        "absent resource with existing lock should be Destroy"
    );
    assert_eq!(p.changes[0].resource_id, "old-config");
}

#[test]
fn test_plan_absent_resource_no_lock() {
    let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  gone-file:\n    type: file\n    machine: m1\n    path: /tmp/gone.txt\n    state: absent\n";
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let order = vec!["gone-file".to_string()];
    let locks = HashMap::new();

    let p = plan(&config, &order, &locks, None);

    assert_eq!(
        p.unchanged, 1,
        "absent resource with no lock should be NoOp (counted as unchanged)"
    );
    assert_eq!(p.to_destroy, 0, "nothing to destroy when no lock exists");
    assert_eq!(p.changes.len(), 1);
    assert_eq!(p.changes[0].action, PlanAction::NoOp);
}

#[test]
fn test_plan_converged_hash_match() {
    let config = make_config();
    let order = vec!["conf".to_string()];

    let resource = &config.resources["conf"];
    let desired_hash = hash_desired_state(resource);

    let mut resources = indexmap::IndexMap::new();
    resources.insert(
        "conf".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: desired_hash,
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

    assert_eq!(
        p.unchanged, 1,
        "converged resource with matching hash should be NoOp"
    );
    assert_eq!(p.to_update, 0);
    assert_eq!(p.to_create, 0);
    assert_eq!(p.changes[0].action, PlanAction::NoOp);
}

#[test]
fn test_plan_converged_hash_mismatch() {
    let config = make_config();
    let order = vec!["conf".to_string()];

    let mut resources = indexmap::IndexMap::new();
    resources.insert(
        "conf".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:old_stale_hash_that_does_not_match".to_string(),
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

    assert_eq!(
        p.to_update, 1,
        "converged resource with mismatched hash should be Update"
    );
    assert_eq!(p.unchanged, 0);
    assert_eq!(p.changes[0].action, PlanAction::Update);
}

#[test]
fn test_fj204_count_plans_expanded_resources() {
    let yaml = "\nversion: \"1.0\"\nname: test-count\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  shard:\n    type: file\n    machine: m1\n    path: \"/data/shard-{{index}}\"\n    content: \"shard={{index}}\"\n    count: 3\n";
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    super::super::parser::expand_resources(&mut config);
    let order = vec![
        "shard-0".to_string(),
        "shard-1".to_string(),
        "shard-2".to_string(),
    ];
    let locks = HashMap::new();
    let p = plan(&config, &order, &locks, None);
    assert_eq!(p.to_create, 3);
    assert_eq!(p.changes.len(), 3);
    assert_eq!(p.changes[0].resource_id, "shard-0");
    assert_eq!(p.changes[1].resource_id, "shard-1");
    assert_eq!(p.changes[2].resource_id, "shard-2");
}

#[test]
fn test_fj203_for_each_plans_expanded_resources() {
    let yaml = "\nversion: \"1.0\"\nname: test-foreach\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  vhost:\n    type: file\n    machine: m1\n    path: \"/etc/nginx/{{item}}.conf\"\n    content: \"server {{item}}\"\n    for_each: [api, web]\n";
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    super::super::parser::expand_resources(&mut config);
    let order = vec!["vhost-api".to_string(), "vhost-web".to_string()];
    let locks = HashMap::new();
    let p = plan(&config, &order, &locks, None);
    assert_eq!(p.to_create, 2);
    assert_eq!(p.changes[0].resource_id, "vhost-api");
    assert_eq!(p.changes[1].resource_id, "vhost-web");
}

#[test]
fn test_fj204_count_mixed_with_regular() {
    let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    type: file\n    machine: m1\n    path: \"/etc/base.conf\"\n  node:\n    type: file\n    machine: m1\n    path: \"/data/node-{{index}}\"\n    content: \"node={{index}}\"\n    count: 2\n";
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    super::super::parser::expand_resources(&mut config);

    let order = vec![
        "base".to_string(),
        "node-0".to_string(),
        "node-1".to_string(),
    ];
    let locks = HashMap::new();
    let p = plan(&config, &order, &locks, None);
    assert_eq!(p.to_create, 3, "base + node-0 + node-1");
}
