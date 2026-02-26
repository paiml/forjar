//! FJ-044: Docker → pepita migration.
//!
//! Converts Docker container resources to pepita kernel isolation resources.
//! Docker uses a container runtime; pepita uses kernel primitives directly.
//!
//! Mapping:
//! - Docker `image`        → pepita `overlay_lower` (image rootfs concept)
//! - Docker `ports`        → pepita `netns: true` (network namespace isolation)
//! - Docker `volumes`      → preserved as overlay mount hints
//! - Docker `name`         → pepita `name`
//! - Docker `state`        → pepita `state` (running→present, absent→absent)
//! - Docker restart/env    → noted in migration warnings

use super::types::{ForjarConfig, Resource, ResourceType};
use indexmap::IndexMap;

/// Result of migrating a single Docker resource.
#[derive(Debug)]
pub struct MigrationResult {
    /// The converted pepita resource.
    pub resource: Resource,
    /// Warnings about features that don't map cleanly.
    pub warnings: Vec<String>,
}

/// Migrate a single Docker resource to a pepita resource.
pub fn docker_to_pepita(id: &str, docker: &Resource) -> MigrationResult {
    let mut warnings = Vec::new();
    let mut pepita = docker.clone();
    pepita.resource_type = ResourceType::Pepita;

    // Map Docker state → pepita state
    match docker.state.as_deref() {
        Some("running") | None => pepita.state = Some("present".to_string()),
        Some("stopped") => {
            pepita.state = Some("absent".to_string());
            warnings.push(format!(
                "{}: Docker 'stopped' has no pepita equivalent — mapped to 'absent'",
                id
            ));
        }
        Some("absent") => pepita.state = Some("absent".to_string()),
        Some(other) => {
            warnings.push(format!(
                "{}: unknown Docker state '{}' — defaulting to 'present'",
                id, other
            ));
            pepita.state = Some("present".to_string());
        }
    }

    // Enable network namespace if ports are exposed
    if !docker.ports.is_empty() {
        pepita.netns = true;
        warnings.push(format!(
            "{}: Docker ports {:?} require manual iptables/nftables rules in pepita namespace",
            id, docker.ports
        ));
    }

    // Image → overlay lower hint
    if let Some(ref image) = docker.image {
        warnings.push(format!(
            "{}: Docker image '{}' — extract rootfs to use as overlay_lower",
            id, image
        ));
        // Clear docker-specific fields
        pepita.image = None;
    }

    // Volumes → warning
    if !docker.volumes.is_empty() {
        warnings.push(format!(
            "{}: Docker volumes {:?} — use bind mounts or overlay directories in pepita",
            id, docker.volumes
        ));
    }

    // Environment → warning
    if !docker.environment.is_empty() {
        warnings.push(format!(
            "{}: Docker environment {:?} — set in chroot /etc/environment or exec wrapper",
            id, docker.environment
        ));
    }

    // Restart policy → warning
    if docker.restart.is_some() {
        warnings.push(format!(
            "{}: Docker restart policy '{}' — use systemd unit with pepita chroot instead",
            id,
            docker.restart.as_deref().unwrap_or("")
        ));
        pepita.restart = None;
    }

    // Clear Docker-specific fields
    pepita.ports = vec![];
    pepita.environment = vec![];
    pepita.volumes = vec![];

    MigrationResult {
        resource: pepita,
        warnings,
    }
}

/// Migrate all Docker resources in a config to pepita.
/// Returns the modified config and collected warnings.
pub fn migrate_config(config: &ForjarConfig) -> (ForjarConfig, Vec<String>) {
    let mut new_config = config.clone();
    let mut all_warnings = Vec::new();
    let mut new_resources = IndexMap::new();

    for (id, resource) in &config.resources {
        if resource.resource_type == ResourceType::Docker {
            let result = docker_to_pepita(id, resource);
            new_resources.insert(id.clone(), result.resource);
            all_warnings.extend(result.warnings);
        } else {
            new_resources.insert(id.clone(), resource.clone());
        }
    }

    new_config.resources = new_resources;
    (new_config, all_warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::MachineTarget;
    use std::collections::HashMap;

    fn make_docker(name: &str, image: &str) -> Resource {
        Resource {
            resource_type: ResourceType::Docker,
            machine: MachineTarget::Single("m1".to_string()),
            state: Some("running".to_string()),
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
            name: Some(name.to_string()),
            enabled: None,
            restart_on: vec![],
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
            image: Some(image.to_string()),
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
        }
    }

    #[test]
    fn test_fj044_basic_conversion() {
        let docker = make_docker("web", "nginx:latest");
        let result = docker_to_pepita("web", &docker);
        assert_eq!(result.resource.resource_type, ResourceType::Pepita);
        assert_eq!(result.resource.name.as_deref(), Some("web"));
        assert_eq!(result.resource.state.as_deref(), Some("present"));
        assert!(
            result.resource.image.is_none(),
            "docker image should be cleared"
        );
    }

    #[test]
    fn test_fj044_ports_enable_netns() {
        let mut docker = make_docker("web", "nginx:latest");
        docker.ports = vec!["8080:80".to_string()];
        let result = docker_to_pepita("web", &docker);
        assert!(result.resource.netns, "ports should enable netns");
        assert!(result.resource.ports.is_empty(), "ports should be cleared");
        assert!(result.warnings.iter().any(|w| w.contains("iptables")));
    }

    #[test]
    fn test_fj044_absent_state() {
        let mut docker = make_docker("old", "nginx:latest");
        docker.state = Some("absent".to_string());
        let result = docker_to_pepita("old", &docker);
        assert_eq!(result.resource.state.as_deref(), Some("absent"));
    }

    #[test]
    fn test_fj044_stopped_maps_to_absent() {
        let mut docker = make_docker("app", "myapp:v1");
        docker.state = Some("stopped".to_string());
        let result = docker_to_pepita("app", &docker);
        assert_eq!(result.resource.state.as_deref(), Some("absent"));
        assert!(result.warnings.iter().any(|w| w.contains("stopped")));
    }

    #[test]
    fn test_fj044_image_warning() {
        let docker = make_docker("web", "nginx:latest");
        let result = docker_to_pepita("web", &docker);
        assert!(result.warnings.iter().any(|w| w.contains("nginx:latest")));
        assert!(result.warnings.iter().any(|w| w.contains("overlay_lower")));
    }

    #[test]
    fn test_fj044_volumes_warning() {
        let mut docker = make_docker("db", "postgres:16");
        docker.volumes = vec!["/data:/var/lib/postgresql".to_string()];
        let result = docker_to_pepita("db", &docker);
        assert!(result.resource.volumes.is_empty());
        assert!(result.warnings.iter().any(|w| w.contains("volumes")));
    }

    #[test]
    fn test_fj044_environment_warning() {
        let mut docker = make_docker("app", "myapp:v1");
        docker.environment = vec!["DB_HOST=localhost".to_string()];
        let result = docker_to_pepita("app", &docker);
        assert!(result.resource.environment.is_empty());
        assert!(result.warnings.iter().any(|w| w.contains("environment")));
    }

    #[test]
    fn test_fj044_restart_policy_warning() {
        let mut docker = make_docker("web", "nginx:latest");
        docker.restart = Some("unless-stopped".to_string());
        let result = docker_to_pepita("web", &docker);
        assert!(result.resource.restart.is_none());
        assert!(result.warnings.iter().any(|w| w.contains("restart")));
    }

    #[test]
    fn test_fj044_preserves_name_and_machine() {
        let docker = make_docker("api-server", "api:v3");
        let result = docker_to_pepita("api", &docker);
        assert_eq!(result.resource.name.as_deref(), Some("api-server"));
        assert!(matches!(result.resource.machine, MachineTarget::Single(ref m) if m == "m1"));
    }

    #[test]
    fn test_fj044_preserves_depends_on() {
        let mut docker = make_docker("app", "myapp:v1");
        docker.depends_on = vec!["db".to_string()];
        let result = docker_to_pepita("app", &docker);
        assert_eq!(result.resource.depends_on, vec!["db".to_string()]);
    }

    #[test]
    fn test_fj044_preserves_tags() {
        let mut docker = make_docker("web", "nginx:latest");
        docker.tags = vec!["web".to_string(), "critical".to_string()];
        let result = docker_to_pepita("web", &docker);
        assert_eq!(result.resource.tags, vec!["web", "critical"]);
    }

    #[test]
    fn test_fj044_full_docker_migration() {
        let mut docker = make_docker("full-app", "myapp:v2");
        docker.ports = vec!["8080:80".to_string(), "443:443".to_string()];
        docker.environment = vec!["NODE_ENV=production".to_string()];
        docker.volumes = vec!["/data:/app/data".to_string()];
        docker.restart = Some("always".to_string());
        docker.depends_on = vec!["db".to_string()];

        let result = docker_to_pepita("full-app", &docker);
        assert_eq!(result.resource.resource_type, ResourceType::Pepita);
        assert!(result.resource.netns);
        assert!(result.resource.ports.is_empty());
        assert!(result.resource.environment.is_empty());
        assert!(result.resource.volumes.is_empty());
        assert!(result.resource.restart.is_none());
        assert!(result.resource.image.is_none());
        assert_eq!(result.resource.depends_on, vec!["db"]);
        assert!(
            result.warnings.len() >= 4,
            "should have warnings for ports, image, volumes, env, restart"
        );
    }

    #[test]
    fn test_fj044_migrate_config() {
        use crate::core::types::{ForjarConfig, Policy};

        let mut resources = IndexMap::new();
        resources.insert("web".to_string(), make_docker("web", "nginx:latest"));

        let mut pkg = make_docker("tools", "unused");
        pkg.resource_type = ResourceType::Package;
        pkg.provider = Some("apt".to_string());
        pkg.packages = vec!["curl".to_string()];
        resources.insert("tools".to_string(), pkg);

        let config = ForjarConfig {
            version: "1.0".to_string(),
            name: "test".to_string(),
            description: None,
            params: HashMap::new(),
            machines: IndexMap::new(),
            resources,
            policy: Policy::default(),
            outputs: indexmap::IndexMap::new(),
            policies: vec![],
            data: indexmap::IndexMap::new(),
        };

        let (migrated, warnings) = migrate_config(&config);
        assert_eq!(
            migrated.resources["web"].resource_type,
            ResourceType::Pepita
        );
        assert_eq!(
            migrated.resources["tools"].resource_type,
            ResourceType::Package,
            "non-docker resources should be unchanged"
        );
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_fj044_no_docker_no_warnings() {
        use crate::core::types::{ForjarConfig, Policy};

        let mut resources = IndexMap::new();
        let mut pkg = make_docker("tools", "unused");
        pkg.resource_type = ResourceType::Package;
        pkg.provider = Some("apt".to_string());
        pkg.packages = vec!["curl".to_string()];
        resources.insert("tools".to_string(), pkg);

        let config = ForjarConfig {
            version: "1.0".to_string(),
            name: "test".to_string(),
            description: None,
            params: HashMap::new(),
            machines: IndexMap::new(),
            resources,
            policy: Policy::default(),
            outputs: indexmap::IndexMap::new(),
            policies: vec![],
            data: indexmap::IndexMap::new(),
        };

        let (migrated, warnings) = migrate_config(&config);
        assert!(warnings.is_empty(), "no docker = no warnings");
        assert_eq!(
            migrated.resources["tools"].resource_type,
            ResourceType::Package
        );
    }

    #[test]
    fn test_fj044_default_state_maps_to_present() {
        let mut docker = make_docker("web", "nginx:latest");
        docker.state = None;
        let result = docker_to_pepita("web", &docker);
        assert_eq!(result.resource.state.as_deref(), Some("present"));
    }

    #[test]
    fn test_fj044_unknown_state_warning() {
        let mut docker = make_docker("web", "nginx:latest");
        docker.state = Some("restarting".to_string());
        let result = docker_to_pepita("web", &docker);
        assert_eq!(result.resource.state.as_deref(), Some("present"));
        assert!(result.warnings.iter().any(|w| w.contains("restarting")));
    }
}
