//! FJ-004: Plan generation — diff desired state against lock state.

use super::types::*;
use crate::tripwire::hasher;

/// Generate an execution plan by comparing desired config to lock state.
pub fn plan(
    config: &ForjarConfig,
    execution_order: &[String],
    locks: &std::collections::HashMap<String, StateLock>,
) -> ExecutionPlan {
    let mut changes = Vec::new();
    let mut to_create = 0u32;
    let mut to_update = 0u32;
    let mut to_destroy = 0u32;
    let mut unchanged = 0u32;

    for resource_id in execution_order {
        let resource = match config.resources.get(resource_id) {
            Some(r) => r,
            None => continue,
        };

        for machine_name in resource.machine.to_vec() {
            let action = determine_action(resource_id, resource, &machine_name, locks);
            let description = describe_action(resource_id, resource, &action);

            match action {
                PlanAction::Create => to_create += 1,
                PlanAction::Update => to_update += 1,
                PlanAction::Destroy => to_destroy += 1,
                PlanAction::NoOp => unchanged += 1,
            }

            changes.push(PlannedChange {
                resource_id: resource_id.clone(),
                machine: machine_name,
                resource_type: resource.resource_type.clone(),
                action,
                description,
            });
        }
    }

    ExecutionPlan {
        name: config.name.clone(),
        changes,
        execution_order: execution_order.to_vec(),
        to_create,
        to_update,
        to_destroy,
        unchanged,
    }
}

/// Determine what action to take for a resource on a machine.
fn determine_action(
    resource_id: &str,
    resource: &Resource,
    machine_name: &str,
    locks: &std::collections::HashMap<String, StateLock>,
) -> PlanAction {
    let state = resource.state.as_deref().unwrap_or(match resource.resource_type {
        ResourceType::Package => "present",
        ResourceType::File => "file",
        ResourceType::Service => "running",
        ResourceType::Mount => "mounted",
        _ => "present",
    });

    // Check if this is a destroy action
    if state == "absent" {
        if let Some(lock) = locks.get(machine_name) {
            if lock.resources.contains_key(resource_id) {
                return PlanAction::Destroy;
            }
        }
        return PlanAction::NoOp;
    }

    // Check lock for existing state
    if let Some(lock) = locks.get(machine_name) {
        if let Some(rl) = lock.resources.get(resource_id) {
            if rl.status == ResourceStatus::Converged {
                // Check if desired state hash matches
                let desired_hash = hash_desired_state(resource);
                if rl.hash == desired_hash {
                    return PlanAction::NoOp;
                }
                return PlanAction::Update;
            }
            // Previously failed or drifted — re-apply
            return PlanAction::Update;
        }
    }

    PlanAction::Create
}

/// Compute a hash of the desired state for comparison.
pub fn hash_desired_state(resource: &Resource) -> String {
    let mut components = Vec::new();
    components.push(resource.resource_type.to_string());

    if let Some(ref s) = resource.state {
        components.push(s.clone());
    }
    if let Some(ref p) = resource.provider {
        components.push(p.clone());
    }
    for pkg in &resource.packages {
        components.push(pkg.clone());
    }
    if let Some(ref path) = resource.path {
        components.push(path.clone());
    }
    if let Some(ref content) = resource.content {
        components.push(content.clone());
    }
    if let Some(ref source) = resource.source {
        components.push(source.clone());
    }
    if let Some(ref name) = resource.name {
        components.push(name.clone());
    }
    if let Some(ref owner) = resource.owner {
        components.push(owner.clone());
    }
    if let Some(ref group) = resource.group {
        components.push(group.clone());
    }
    if let Some(ref mode) = resource.mode {
        components.push(mode.clone());
    }
    if let Some(ref fs_type) = resource.fs_type {
        components.push(fs_type.clone());
    }
    if let Some(ref options) = resource.options {
        components.push(options.clone());
    }

    let joined = components.join("\0");
    hasher::hash_string(&joined)
}

/// Generate a human-readable description of a planned action.
fn describe_action(resource_id: &str, resource: &Resource, action: &PlanAction) -> String {
    match action {
        PlanAction::Create => match resource.resource_type {
            ResourceType::Package => {
                let pkgs = resource.packages.join(", ");
                format!("{}: install {}", resource_id, pkgs)
            }
            ResourceType::File => {
                let path = resource.path.as_deref().unwrap_or("?");
                format!("{}: create {}", resource_id, path)
            }
            ResourceType::Service => {
                let name = resource.name.as_deref().unwrap_or("?");
                format!("{}: start {}", resource_id, name)
            }
            ResourceType::Mount => {
                let path = resource.path.as_deref().unwrap_or("?");
                format!("{}: mount {}", resource_id, path)
            }
            _ => format!("{}: create", resource_id),
        },
        PlanAction::Update => format!("{}: update (state changed)", resource_id),
        PlanAction::Destroy => format!("{}: destroy", resource_id),
        PlanAction::NoOp => format!("{}: no changes", resource_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::MachineTarget;
    use std::collections::HashMap;

    fn make_config() -> ForjarConfig {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  conf:
    type: file
    machine: m1
    path: /etc/test.conf
    content: "hello"
    depends_on: [pkg]
  svc:
    type: service
    machine: m1
    name: nginx
    state: running
    depends_on: [conf]
"#;
        serde_yaml::from_str(yaml).unwrap()
    }

    #[test]
    fn test_fj004_plan_all_create() {
        let config = make_config();
        let order = vec!["pkg".to_string(), "conf".to_string(), "svc".to_string()];
        let locks = HashMap::new();
        let plan = plan(&config, &order, &locks);

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

        let plan = plan(&config, &order, &locks);
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

        let plan = plan(&config, &order, &locks);
        assert_eq!(plan.to_update, 1);
    }

    #[test]
    fn test_fj004_plan_destroy() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  old-file:
    type: file
    machine: m1
    path: /tmp/gone
    state: absent
"#;
        let config: ForjarConfig = serde_yaml::from_str(yaml).unwrap();
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

        let plan = plan(&config, &order, &locks);
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

        let plan = plan(&config, &order, &locks);
        assert_eq!(plan.to_update, 1);
    }

    #[test]
    fn test_fj004_hash_deterministic() {
        let r = Resource {
            resource_type: ResourceType::Package,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: Some("apt".to_string()),
            packages: vec!["curl".to_string()],
            path: None,
            content: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            fs_type: None,
            options: None,
        };
        let h1 = hash_desired_state(&r);
        let h2 = hash_desired_state(&r);
        assert_eq!(h1, h2);
        assert!(h1.starts_with("blake3:"));
    }

    #[test]
    fn test_fj004_describe_action() {
        let r = Resource {
            resource_type: ResourceType::Package,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: Some("apt".to_string()),
            packages: vec!["curl".to_string(), "wget".to_string()],
            path: None,
            content: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            fs_type: None,
            options: None,
        };
        let desc = describe_action("test-pkg", &r, &PlanAction::Create);
        assert!(desc.contains("curl, wget"));
    }

    #[test]
    fn test_fj004_multi_machine() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  a:
    hostname: a
    addr: 1.1.1.1
  b:
    hostname: b
    addr: 2.2.2.2
resources:
  tools:
    type: package
    machine: [a, b]
    provider: cargo
    packages: [batuta]
"#;
        let config: ForjarConfig = serde_yaml::from_str(yaml).unwrap();
        let order = vec!["tools".to_string()];
        let locks = HashMap::new();
        let plan = plan(&config, &order, &locks);
        // One resource on two machines = 2 planned changes
        assert_eq!(plan.changes.len(), 2);
        assert_eq!(plan.to_create, 2);
    }
}
