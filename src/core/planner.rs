//! FJ-004: Plan generation — diff desired state against lock state.

use super::conditions;
use super::resolver;
use super::types::*;
use crate::tripwire::hasher;

/// Generate an execution plan by comparing desired config to lock state.
pub fn plan(
    config: &ForjarConfig,
    execution_order: &[String],
    locks: &std::collections::HashMap<String, StateLock>,
    tag_filter: Option<&str>,
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

        // Tag filtering: skip resource if tag filter specified and resource doesn't have the tag
        if let Some(tag) = tag_filter {
            if !resource.tags.iter().any(|t| t == tag) {
                continue;
            }
        }

        // Resolve templates before hashing so planner hash matches executor hash
        let resolved =
            resolver::resolve_resource_templates(resource, &config.params, &config.machines)
                .unwrap_or_else(|e| {
                    eprintln!(
                        "warning: template resolution failed for {}: {}",
                        resource_id, e
                    );
                    resource.clone()
                });

        for machine_name in resource.machine.to_vec() {
            // FJ-064: Skip resource if arch filter doesn't match machine
            if !resource.arch.is_empty() {
                if let Some(machine) = config.machines.get(&machine_name) {
                    if !resource.arch.contains(&machine.arch) {
                        continue;
                    }
                }
            }

            // FJ-202: Skip resource if `when:` condition evaluates to false
            if let Some(ref when_expr) = resource.when {
                if let Some(machine) = config.machines.get(&machine_name) {
                    match conditions::evaluate_when(when_expr, &config.params, machine) {
                        Ok(false) => continue,
                        Err(e) => {
                            eprintln!(
                                "warning: when condition failed for {} on {}: {}",
                                resource_id, machine_name, e
                            );
                            continue;
                        }
                        Ok(true) => {} // condition met, proceed
                    }
                }
            }

            let action = determine_action(resource_id, &resolved, &machine_name, locks);
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
    let state = resource
        .state
        .as_deref()
        .unwrap_or(match resource.resource_type {
            ResourceType::Package => "present",
            ResourceType::File => "file",
            ResourceType::Service => "running",
            ResourceType::Mount => "mounted",
            ResourceType::User
            | ResourceType::Docker
            | ResourceType::Pepita
            | ResourceType::Network
            | ResourceType::Cron
            | ResourceType::Model
            | ResourceType::Gpu
            | ResourceType::Recipe => "present",
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
    let type_str = resource.resource_type.to_string();
    let mut components: Vec<&str> = vec![&type_str];

    if let Some(ref s) = resource.state {
        components.push(s);
    }
    if let Some(ref p) = resource.provider {
        components.push(p);
    }
    for pkg in &resource.packages {
        components.push(pkg);
    }
    if let Some(ref path) = resource.path {
        components.push(path);
    }
    if let Some(ref content) = resource.content {
        components.push(content);
    }
    if let Some(ref source) = resource.source {
        components.push(source);
    }
    if let Some(ref name) = resource.name {
        components.push(name);
    }
    if let Some(ref owner) = resource.owner {
        components.push(owner);
    }
    if let Some(ref group) = resource.group {
        components.push(group);
    }
    if let Some(ref mode) = resource.mode {
        components.push(mode);
    }
    if let Some(ref fs_type) = resource.fs_type {
        components.push(fs_type);
    }
    if let Some(ref options) = resource.options {
        components.push(options);
    }
    if let Some(ref target) = resource.target {
        components.push(target);
    }
    if let Some(ref version) = resource.version {
        components.push(version);
    }
    // Phase 2 resource fields
    if let Some(ref image) = resource.image {
        components.push(image);
    }
    if let Some(ref command) = resource.command {
        components.push(command);
    }
    if let Some(ref schedule) = resource.schedule {
        components.push(schedule);
    }
    if let Some(ref restart) = resource.restart {
        components.push(restart);
    }
    if let Some(ref port) = resource.port {
        components.push(port);
    }
    if let Some(ref protocol) = resource.protocol {
        components.push(protocol);
    }
    if let Some(ref action) = resource.action {
        components.push(action);
    }
    if let Some(ref from_addr) = resource.from_addr {
        components.push(from_addr);
    }
    if let Some(ref shell) = resource.shell {
        components.push(shell);
    }
    if let Some(ref home) = resource.home {
        components.push(home);
    }
    if let Some(ref enabled) = resource.enabled {
        if *enabled {
            components.push("enabled");
        } else {
            components.push("disabled");
        }
    }
    for p in &resource.ports {
        components.push(p);
    }
    for e in &resource.environment {
        components.push(e);
    }
    for v in &resource.volumes {
        components.push(v);
    }
    for r in &resource.restart_on {
        components.push(r);
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
            ResourceType::User
            | ResourceType::Docker
            | ResourceType::Pepita
            | ResourceType::Network
            | ResourceType::Cron
            | ResourceType::Model
            | ResourceType::Gpu
            | ResourceType::Recipe => format!("{}: create", resource_id),
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
        serde_yaml_ng::from_str(yaml).unwrap()
    }

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
    fn test_fj004_hash_deterministic() {
        let r = Resource {
            resource_type: ResourceType::Package,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: Some("apt".to_string()),
            packages: vec!["curl".to_string()],
            version: None,
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
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
            format: None,
            quantization: None,
            checksum: None,
            cache_dir: None,
            driver_version: None,
            cuda_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
        pre_apply: None,
        post_apply: None,
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
            version: None,
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
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
            format: None,
            quantization: None,
            checksum: None,
            cache_dir: None,
            driver_version: None,
            cuda_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
        pre_apply: None,
        post_apply: None,
        };
        let desc = describe_action("test-pkg", &r, &PlanAction::Create);
        assert!(desc.contains("curl, wget"));
    }

    #[test]
    fn test_fj004_describe_action_file() {
        let r = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: Some("/etc/conf".to_string()),
            content: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
            format: None,
            quantization: None,
            checksum: None,
            cache_dir: None,
            driver_version: None,
            cuda_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
        pre_apply: None,
        post_apply: None,
        };
        assert!(describe_action("f", &r, &PlanAction::Create).contains("/etc/conf"));
        assert!(describe_action("f", &r, &PlanAction::Update).contains("update"));
        assert!(describe_action("f", &r, &PlanAction::Destroy).contains("destroy"));
        assert!(describe_action("f", &r, &PlanAction::NoOp).contains("no changes"));
    }

    #[test]
    fn test_fj004_describe_action_service() {
        let r = Resource {
            resource_type: ResourceType::Service,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: None,
            content: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: Some("nginx".to_string()),
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
            format: None,
            quantization: None,
            checksum: None,
            cache_dir: None,
            driver_version: None,
            cuda_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
        pre_apply: None,
        post_apply: None,
        };
        assert!(describe_action("svc", &r, &PlanAction::Create).contains("nginx"));
    }

    #[test]
    fn test_fj004_describe_action_mount() {
        let r = Resource {
            resource_type: ResourceType::Mount,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: Some("/mnt/data".to_string()),
            content: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
            format: None,
            quantization: None,
            checksum: None,
            cache_dir: None,
            driver_version: None,
            cuda_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
        pre_apply: None,
        post_apply: None,
        };
        assert!(describe_action("mnt", &r, &PlanAction::Create).contains("/mnt/data"));
    }

    #[test]
    fn test_fj004_hash_includes_all_fields() {
        let r1 = Resource {
            resource_type: ResourceType::Mount,
            machine: MachineTarget::Single("m1".to_string()),
            state: Some("mounted".to_string()),
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: Some("/mnt/data".to_string()),
            content: None,
            source: Some("192.168.1.1:/data".to_string()),
            target: Some("/mnt/target".to_string()),
            owner: Some("root".to_string()),
            group: Some("root".to_string()),
            mode: Some("0755".to_string()),
            name: Some("data-mount".to_string()),
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: Some("nfs".to_string()),
            options: Some("ro,hard".to_string()),
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
            format: None,
            quantization: None,
            checksum: None,
            cache_dir: None,
            driver_version: None,
            cuda_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
        pre_apply: None,
        post_apply: None,
        };
        // Changing any field should change the hash
        let mut r2 = r1.clone();
        r2.fs_type = Some("ext4".to_string());
        assert_ne!(hash_desired_state(&r1), hash_desired_state(&r2));

        let mut r3 = r1.clone();
        r3.options = Some("rw".to_string());
        assert_ne!(hash_desired_state(&r1), hash_desired_state(&r3));

        let mut r4 = r1.clone();
        r4.name = Some("other-mount".to_string());
        assert_ne!(hash_desired_state(&r1), hash_desired_state(&r4));
    }

    #[test]
    fn test_fj004_absent_not_in_lock_is_noop() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  gone-file:
    type: file
    machine: m1
    path: /tmp/gone
    state: absent
"#;
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
    fn test_fj004_determine_action_mount_default_state() {
        // Exercises `ResourceType::Mount => "mounted"` default state branch
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  nfs-share:
    type: mount
    machine: m1
    source: "nas:/data"
    path: /mnt/data
"#;
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
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  web:
    type: service
    machine: m1
    name: nginx
"#;
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
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  my-user:
    type: user
    machine: m1
    name: deploy
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = vec!["my-user".to_string()];
        let locks = HashMap::new();
        let plan = plan(&config, &order, &locks, None);
        assert_eq!(plan.to_create, 1);
    }

    #[test]
    fn test_fj004_plan_with_broken_template_fallback() {
        // Template resolution error triggers fallback to unresolved resource
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  config:
    type: file
    machine: m1
    path: /etc/test
    content: "{{params.missing_key}}"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = vec!["config".to_string()];
        let locks = HashMap::new();
        // Should not panic — falls back to unresolved resource
        let plan = plan(&config, &order, &locks, None);
        assert_eq!(plan.to_create, 1);
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
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = vec!["tools".to_string()];
        let locks = HashMap::new();
        let plan = plan(&config, &order, &locks, None);
        // One resource on two machines = 2 planned changes
        assert_eq!(plan.changes.len(), 2);
        assert_eq!(plan.to_create, 2);
    }

    #[test]
    fn test_tag_filter_excludes_untagged_from_plan() {
        let yaml = r#"
version: "1.0"
name: tag-plan-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  tagged:
    type: file
    machine: m
    path: /tmp/tagged
    content: "yes"
    tags: [web, critical]
  untagged:
    type: file
    machine: m
    path: /tmp/untagged
    content: "no"
"#;
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
        let yaml = r#"
version: "1.0"
name: tag-plan-empty
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m
    path: /tmp/test
    content: "hello"
    tags: [web]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = vec!["f".to_string()];
        let locks = HashMap::new();

        let plan = plan(&config, &order, &locks, Some("db"));
        assert_eq!(plan.changes.len(), 0);
    }

    #[test]
    fn test_fj004_arch_filter_skips_mismatched_machine() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  x86:
    hostname: x86
    addr: 1.1.1.1
    arch: x86_64
  arm:
    hostname: arm
    addr: 2.2.2.2
    arch: aarch64
resources:
  intel-only:
    type: package
    machine: [x86, arm]
    provider: apt
    packages: [intel-microcode]
    arch: [x86_64]
"#;
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
        let yaml = r#"
version: "1.0"
name: test
machines:
  x86:
    hostname: x86
    addr: 1.1.1.1
    arch: x86_64
  arm:
    hostname: arm
    addr: 2.2.2.2
    arch: aarch64
resources:
  intel-only:
    type: package
    machine: [x86, arm]
    provider: apt
    packages: [intel-microcode]
    arch: [x86_64]
"#;
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
    fn test_fj004_multi_machine_partial_lock() {
        // One machine converged, other is new
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
  pkg:
    type: package
    machine: [a, b]
    provider: apt
    packages: [curl]
"#;
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
    fn test_fj004_describe_action_file_no_path() {
        // File with no path should show "?"
        let r = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
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
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
            format: None,
            quantization: None,
            checksum: None,
            cache_dir: None,
            driver_version: None,
            cuda_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
        pre_apply: None,
        post_apply: None,
        };
        let desc = describe_action("f", &r, &PlanAction::Create);
        assert!(desc.contains("?"), "missing path should show ?");
    }

    #[test]
    fn test_fj004_describe_action_service_no_name() {
        let r = Resource {
            resource_type: ResourceType::Service,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
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
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
            format: None,
            quantization: None,
            checksum: None,
            cache_dir: None,
            driver_version: None,
            cuda_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
        pre_apply: None,
        post_apply: None,
        };
        let desc = describe_action("svc", &r, &PlanAction::Create);
        assert!(desc.contains("?"), "missing name should show ?");
    }

    #[test]
    fn test_fj004_describe_action_docker_type() {
        let r = Resource {
            resource_type: ResourceType::Docker,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
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
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
            format: None,
            quantization: None,
            checksum: None,
            cache_dir: None,
            driver_version: None,
            cuda_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
        pre_apply: None,
        post_apply: None,
        };
        let desc = describe_action("dock", &r, &PlanAction::Create);
        assert!(desc.contains("create"), "Docker create should say create");
    }

    #[test]
    fn test_fj004_hash_content_change_changes_hash() {
        let r1 = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: Some("/etc/test".to_string()),
            content: Some("version=1".to_string()),
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
            format: None,
            quantization: None,
            checksum: None,
            cache_dir: None,
            driver_version: None,
            cuda_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
        pre_apply: None,
        post_apply: None,
        };
        let mut r2 = r1.clone();
        r2.content = Some("version=2".to_string());
        assert_ne!(
            hash_desired_state(&r1),
            hash_desired_state(&r2),
            "content change must change hash"
        );
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
    fn test_fj004_arch_and_tag_filter_combined() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  x86:
    hostname: x86
    addr: 1.1.1.1
    arch: x86_64
  arm:
    hostname: arm
    addr: 2.2.2.2
    arch: aarch64
resources:
  driver:
    type: package
    machine: [x86, arm]
    provider: apt
    packages: [intel-microcode]
    arch: [x86_64]
    tags: [infra]
  generic:
    type: file
    machine: [x86, arm]
    path: /etc/test
    content: hello
    tags: [infra]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = vec!["driver".to_string(), "generic".to_string()];
        let locks = HashMap::new();

        // Tag filter "infra" + arch filter: driver only on x86, generic on both
        let plan = plan(&config, &order, &locks, Some("infra"));
        assert_eq!(plan.changes.len(), 3); // driver:x86, generic:x86, generic:arm
        assert_eq!(plan.to_create, 3);
    }

    // ── Phase 2 field hash sensitivity (FJ-127) ─────────────────

    #[test]
    fn test_hash_sensitive_to_image() {
        let mut r = make_base_resource(ResourceType::Docker);
        r.image = Some("myapp:v1".to_string());
        let h1 = hash_desired_state(&r);
        r.image = Some("myapp:v2".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "image change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_schedule() {
        let mut r = make_base_resource(ResourceType::Cron);
        r.schedule = Some("0 * * * *".to_string());
        let h1 = hash_desired_state(&r);
        r.schedule = Some("*/5 * * * *".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "schedule change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_port() {
        let mut r = make_base_resource(ResourceType::Network);
        r.port = Some("80".to_string());
        let h1 = hash_desired_state(&r);
        r.port = Some("443".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "port change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_restart_policy() {
        let mut r = make_base_resource(ResourceType::Docker);
        r.restart = Some("always".to_string());
        let h1 = hash_desired_state(&r);
        r.restart = Some("unless-stopped".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "restart policy change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_enabled() {
        let mut r = make_base_resource(ResourceType::Service);
        r.enabled = Some(true);
        let h1 = hash_desired_state(&r);
        r.enabled = Some(false);
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "enabled change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_ports_list() {
        let mut r = make_base_resource(ResourceType::Docker);
        r.ports = vec!["8080:80".to_string()];
        let h1 = hash_desired_state(&r);
        r.ports = vec!["8080:80".to_string(), "443:443".to_string()];
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "ports list change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_environment() {
        let mut r = make_base_resource(ResourceType::Docker);
        r.environment = vec!["KEY=val1".to_string()];
        let h1 = hash_desired_state(&r);
        r.environment = vec!["KEY=val2".to_string()];
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "environment change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_volumes() {
        let mut r = make_base_resource(ResourceType::Docker);
        r.volumes = vec!["/host:/container".to_string()];
        let h1 = hash_desired_state(&r);
        r.volumes = vec!["/other:/container".to_string()];
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "volumes change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_from_addr() {
        let mut r = make_base_resource(ResourceType::Network);
        r.from_addr = Some("10.0.0.0/8".to_string());
        let h1 = hash_desired_state(&r);
        r.from_addr = Some("192.168.0.0/16".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "from_addr change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_protocol() {
        let mut r = make_base_resource(ResourceType::Network);
        r.protocol = Some("tcp".to_string());
        let h1 = hash_desired_state(&r);
        r.protocol = Some("udp".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "protocol change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_action() {
        let mut r = make_base_resource(ResourceType::Network);
        r.action = Some("allow".to_string());
        let h1 = hash_desired_state(&r);
        r.action = Some("deny".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "action change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_shell() {
        let mut r = make_base_resource(ResourceType::User);
        r.shell = Some("/bin/bash".to_string());
        let h1 = hash_desired_state(&r);
        r.shell = Some("/bin/zsh".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "shell change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_home() {
        let mut r = make_base_resource(ResourceType::User);
        r.home = Some("/home/deploy".to_string());
        let h1 = hash_desired_state(&r);
        r.home = Some("/opt/deploy".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "home change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_target() {
        let mut r = make_base_resource(ResourceType::File);
        r.target = Some("/etc/nginx/nginx.conf".to_string());
        let h1 = hash_desired_state(&r);
        r.target = Some("/etc/nginx/sites.conf".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "target change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_source() {
        let mut r = make_base_resource(ResourceType::File);
        r.source = Some("/tmp/src1".to_string());
        let h1 = hash_desired_state(&r);
        r.source = Some("/tmp/src2".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "source change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_command() {
        let mut r = make_base_resource(ResourceType::Cron);
        r.command = Some("/usr/bin/backup.sh".to_string());
        let h1 = hash_desired_state(&r);
        r.command = Some("/usr/bin/cleanup.sh".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "command change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_fs_type() {
        let mut r = make_base_resource(ResourceType::Mount);
        r.fs_type = Some("nfs".to_string());
        let h1 = hash_desired_state(&r);
        r.fs_type = Some("ext4".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "fs_type change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_options() {
        let mut r = make_base_resource(ResourceType::Mount);
        r.options = Some("rw,noatime".to_string());
        let h1 = hash_desired_state(&r);
        r.options = Some("ro,noatime".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "options change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_version() {
        let mut r = make_base_resource(ResourceType::Package);
        r.version = Some("1.0.0".to_string());
        let h1 = hash_desired_state(&r);
        r.version = Some("2.0.0".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "version change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_restart() {
        let mut r = make_base_resource(ResourceType::Docker);
        r.restart = Some("always".to_string());
        let h1 = hash_desired_state(&r);
        r.restart = Some("unless-stopped".to_string());
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "restart policy change must change hash");
    }

    #[test]
    fn test_hash_sensitive_to_restart_on() {
        let mut r = make_base_resource(ResourceType::Service);
        r.restart_on = vec!["config-file".to_string()];
        let h1 = hash_desired_state(&r);
        r.restart_on = vec!["other-file".to_string()];
        let h2 = hash_desired_state(&r);
        assert_ne!(h1, h2, "restart_on change must change hash");
    }

    // ── FJ-132: Additional planner tests ────────────────────────

    #[test]
    fn test_fj132_plan_tag_filter_excludes_untagged() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  tagged:
    type: file
    machine: m1
    path: /tmp/tagged
    content: "tagged"
    tags: [web]
  untagged:
    type: file
    machine: m1
    path: /tmp/untagged
    content: "untagged"
"#;
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
        let yaml = r#"
version: "1.0"
name: test
machines:
  x86:
    hostname: x86
    addr: 127.0.0.1
    arch: x86_64
  arm:
    hostname: arm
    addr: 10.0.0.1
    arch: aarch64
resources:
  intel-only:
    type: file
    machine: [x86, arm]
    path: /tmp/intel
    content: "intel"
    arch: [x86_64]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = vec!["intel-only".to_string()];
        let locks = HashMap::new();
        let p = plan(&config, &order, &locks, None);
        // Should create for x86 but skip arm
        assert_eq!(p.to_create, 1);
        assert_eq!(p.changes.len(), 1);
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

    #[test]
    fn test_fj132_describe_action_user_type() {
        let r = make_base_resource(ResourceType::User);
        let desc = describe_action("myuser", &r, &PlanAction::Create);
        assert_eq!(desc, "myuser: create");
    }

    #[test]
    fn test_fj132_describe_action_cron_type() {
        let r = make_base_resource(ResourceType::Cron);
        let desc = describe_action("backup-job", &r, &PlanAction::Create);
        assert_eq!(desc, "backup-job: create");
    }

    #[test]
    fn test_fj132_describe_action_network_type() {
        let r = make_base_resource(ResourceType::Network);
        let desc = describe_action("fw-rule", &r, &PlanAction::Create);
        assert_eq!(desc, "fw-rule: create");
    }

    #[test]
    fn test_fj132_hash_deterministic() {
        // Same resource hashed twice should produce identical hash
        let r = make_base_resource(ResourceType::Package);
        let h1 = hash_desired_state(&r);
        let h2 = hash_desired_state(&r);
        assert_eq!(h1, h2, "hashing must be deterministic");
    }

    #[test]
    fn test_fj132_hash_format() {
        let r = make_base_resource(ResourceType::File);
        let h = hash_desired_state(&r);
        assert!(h.starts_with("blake3:"), "hash should have blake3: prefix");
        assert_eq!(h.len(), 71, "blake3 hash should be 71 chars");
    }

    fn make_base_resource(rt: ResourceType) -> Resource {
        Resource {
            resource_type: rt,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
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
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
            format: None,
            quantization: None,
            checksum: None,
            cache_dir: None,
            driver_version: None,
            cuda_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
        pre_apply: None,
        post_apply: None,
        }
    }

    #[test]
    fn test_fj132_determine_action_no_lock_creates() {
        let resource = make_base_resource(ResourceType::File);
        let locks = std::collections::HashMap::new();
        let action = determine_action("my-file", &resource, "web", &locks);
        assert_eq!(action, PlanAction::Create);
    }

    #[test]
    fn test_fj132_determine_action_converged_same_hash_noop() {
        let mut resource = make_base_resource(ResourceType::File);
        resource.path = Some("/etc/test.conf".to_string());
        resource.content = Some("hello".to_string());
        let desired = hash_desired_state(&resource);
        let mut locks = std::collections::HashMap::new();
        let mut lock_resources = indexmap::IndexMap::new();
        lock_resources.insert(
            "my-file".to_string(),
            ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: desired,
                details: std::collections::HashMap::new(),
            },
        );
        locks.insert(
            "web".to_string(),
            StateLock {
                schema: "1.0".to_string(),
                machine: "web".to_string(),
                hostname: "web".to_string(),
                generated_at: "now".to_string(),
                generator: "test".to_string(),
                blake3_version: "1.8".to_string(),
                resources: lock_resources,
            },
        );
        let action = determine_action("my-file", &resource, "web", &locks);
        assert_eq!(action, PlanAction::NoOp);
    }

    #[test]
    fn test_fj132_determine_action_hash_changed_updates() {
        let mut resource = make_base_resource(ResourceType::File);
        resource.path = Some("/etc/test.conf".to_string());
        resource.content = Some("new content".to_string());
        let mut locks = std::collections::HashMap::new();
        let mut lock_resources = indexmap::IndexMap::new();
        lock_resources.insert(
            "my-file".to_string(),
            ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:old_hash_value".to_string(),
                details: std::collections::HashMap::new(),
            },
        );
        locks.insert(
            "web".to_string(),
            StateLock {
                schema: "1.0".to_string(),
                machine: "web".to_string(),
                hostname: "web".to_string(),
                generated_at: "now".to_string(),
                generator: "test".to_string(),
                blake3_version: "1.8".to_string(),
                resources: lock_resources,
            },
        );
        let action = determine_action("my-file", &resource, "web", &locks);
        assert_eq!(action, PlanAction::Update);
    }

    #[test]
    fn test_fj132_determine_action_absent_with_lock_destroys() {
        let mut resource = make_base_resource(ResourceType::File);
        resource.state = Some("absent".to_string());
        let mut locks = std::collections::HashMap::new();
        let mut lock_resources = indexmap::IndexMap::new();
        lock_resources.insert(
            "old-file".to_string(),
            ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:abc".to_string(),
                details: std::collections::HashMap::new(),
            },
        );
        locks.insert(
            "web".to_string(),
            StateLock {
                schema: "1.0".to_string(),
                machine: "web".to_string(),
                hostname: "web".to_string(),
                generated_at: "now".to_string(),
                generator: "test".to_string(),
                blake3_version: "1.8".to_string(),
                resources: lock_resources,
            },
        );
        let action = determine_action("old-file", &resource, "web", &locks);
        assert_eq!(action, PlanAction::Destroy);
    }

    #[test]
    fn test_fj132_determine_action_absent_no_lock_noop() {
        let mut resource = make_base_resource(ResourceType::File);
        resource.state = Some("absent".to_string());
        let locks = std::collections::HashMap::new();
        let action = determine_action("old-file", &resource, "web", &locks);
        assert_eq!(action, PlanAction::NoOp);
    }

    #[test]
    fn test_fj132_determine_action_failed_retries() {
        let resource = make_base_resource(ResourceType::Package);
        let mut locks = std::collections::HashMap::new();
        let mut lock_resources = indexmap::IndexMap::new();
        lock_resources.insert(
            "my-pkg".to_string(),
            ResourceLock {
                resource_type: ResourceType::Package,
                status: ResourceStatus::Failed,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:abc".to_string(),
                details: std::collections::HashMap::new(),
            },
        );
        locks.insert(
            "web".to_string(),
            StateLock {
                schema: "1.0".to_string(),
                machine: "web".to_string(),
                hostname: "web".to_string(),
                generated_at: "now".to_string(),
                generator: "test".to_string(),
                blake3_version: "1.8".to_string(),
                resources: lock_resources,
            },
        );
        let action = determine_action("my-pkg", &resource, "web", &locks);
        assert_eq!(
            action,
            PlanAction::Update,
            "failed resources should be retried"
        );
    }

    // ── FJ-036 tests ─────────────────────────────────────────────

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
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  old-config:
    type: file
    machine: m1
    path: /etc/old.conf
    state: absent
"#;
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
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  gone-file:
    type: file
    machine: m1
    path: /tmp/gone.txt
    state: absent
"#;
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

    // ── FJ-202: Conditional resource tests ──────────────────────────

    #[test]
    fn test_fj202_when_true_includes_resource() {
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
    when: "true"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = vec!["pkg".to_string()];
        let locks = HashMap::new();
        let p = plan(&config, &order, &locks, None);
        assert_eq!(p.to_create, 1, "when: true should include the resource");
    }

    #[test]
    fn test_fj202_when_false_excludes_resource() {
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
    when: "false"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = vec!["pkg".to_string()];
        let locks = HashMap::new();
        let p = plan(&config, &order, &locks, None);
        assert_eq!(p.to_create, 0, "when: false should exclude the resource");
        assert_eq!(p.changes.len(), 0);
    }

    #[test]
    fn test_fj202_when_arch_match() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
    arch: x86_64
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [cuda]
    when: '{{machine.arch}} == "x86_64"'
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = vec!["pkg".to_string()];
        let locks = HashMap::new();
        let p = plan(&config, &order, &locks, None);
        assert_eq!(
            p.to_create, 1,
            "when arch matches, resource should be included"
        );
    }

    #[test]
    fn test_fj202_when_arch_mismatch() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
    arch: aarch64
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [cuda]
    when: '{{machine.arch}} == "x86_64"'
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = vec!["pkg".to_string()];
        let locks = HashMap::new();
        let p = plan(&config, &order, &locks, None);
        assert_eq!(
            p.to_create, 0,
            "when arch doesn't match, resource should be excluded"
        );
    }

    #[test]
    fn test_fj202_when_param_condition() {
        let yaml = r#"
version: "1.0"
name: test
params:
  env: staging
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  debug-pkg:
    type: package
    machine: m1
    provider: apt
    packages: [strace]
    when: '{{params.env}} != "production"'
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = vec!["debug-pkg".to_string()];
        let locks = HashMap::new();
        let p = plan(&config, &order, &locks, None);
        assert_eq!(p.to_create, 1, "env=staging != production, should include");
    }

    #[test]
    fn test_fj202_when_param_condition_false() {
        let yaml = r#"
version: "1.0"
name: test
params:
  env: production
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  debug-pkg:
    type: package
    machine: m1
    provider: apt
    packages: [strace]
    when: '{{params.env}} != "production"'
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = vec!["debug-pkg".to_string()];
        let locks = HashMap::new();
        let p = plan(&config, &order, &locks, None);
        assert_eq!(
            p.to_create, 0,
            "env=production != production is false, should exclude"
        );
    }

    #[test]
    fn test_fj202_when_no_condition_applies() {
        // Resource without when: should always be included
        let config = make_config();
        let order = vec!["pkg".to_string(), "conf".to_string(), "svc".to_string()];
        let locks = HashMap::new();
        let p = plan(&config, &order, &locks, None);
        assert_eq!(
            p.to_create, 3,
            "resources without when: should all be included"
        );
    }

    #[test]
    fn test_fj202_when_mixed_conditions() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  always:
    type: file
    machine: m1
    path: /tmp/always.txt
    content: "always"
  never:
    type: file
    machine: m1
    path: /tmp/never.txt
    content: "never"
    when: "false"
  conditional:
    type: file
    machine: m1
    path: /tmp/cond.txt
    content: "cond"
    when: "true"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = vec![
            "always".to_string(),
            "conditional".to_string(),
            "never".to_string(),
        ];
        let locks = HashMap::new();
        let p = plan(&config, &order, &locks, None);
        assert_eq!(
            p.to_create, 2,
            "only 'always' and 'conditional' should be included"
        );
    }

    // ================================================================
    // FJ-204/FJ-203: count + for_each planner integration
    // ================================================================

    #[test]
    fn test_fj204_count_plans_expanded_resources() {
        let yaml = r#"
version: "1.0"
name: test-count
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  shard:
    type: file
    machine: m1
    path: "/data/shard-{{index}}"
    content: "shard={{index}}"
    count: 3
"#;
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
        let yaml = r#"
version: "1.0"
name: test-foreach
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  vhost:
    type: file
    machine: m1
    path: "/etc/nginx/{{item}}.conf"
    content: "server {{item}}"
    for_each: [api, web]
"#;
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
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  base:
    type: file
    machine: m1
    path: "/etc/base.conf"
  node:
    type: file
    machine: m1
    path: "/data/node-{{index}}"
    content: "node={{index}}"
    count: 2
"#;
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
}
