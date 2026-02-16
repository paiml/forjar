//! FJ-005: Script generation — dispatch to resource handlers.
//!
//! Each resource type produces three scripts:
//! - check: read current state
//! - apply: converge to desired state
//! - state_query: query observable state for BLAKE3 hashing

use super::types::{Resource, ResourceType};
use crate::resources;

/// Generate a check script for a resource.
pub fn check_script(resource: &Resource) -> Result<String, String> {
    match &resource.resource_type {
        ResourceType::Package => Ok(resources::package::check_script(resource)),
        ResourceType::File => Ok(resources::file::check_script(resource)),
        ResourceType::Service => Ok(resources::service::check_script(resource)),
        ResourceType::Mount => Ok(resources::mount::check_script(resource)),
        other => Err(format!("codegen not implemented for {} (Phase 2+)", other)),
    }
}

/// Generate an apply script for a resource.
pub fn apply_script(resource: &Resource) -> Result<String, String> {
    match &resource.resource_type {
        ResourceType::Package => Ok(resources::package::apply_script(resource)),
        ResourceType::File => Ok(resources::file::apply_script(resource)),
        ResourceType::Service => Ok(resources::service::apply_script(resource)),
        ResourceType::Mount => Ok(resources::mount::apply_script(resource)),
        other => Err(format!("codegen not implemented for {} (Phase 2+)", other)),
    }
}

/// Generate a state query script for a resource.
pub fn state_query_script(resource: &Resource) -> Result<String, String> {
    match &resource.resource_type {
        ResourceType::Package => Ok(resources::package::state_query_script(resource)),
        ResourceType::File => Ok(resources::file::state_query_script(resource)),
        ResourceType::Service => Ok(resources::service::state_query_script(resource)),
        ResourceType::Mount => Ok(resources::mount::state_query_script(resource)),
        other => Err(format!("codegen not implemented for {} (Phase 2+)", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::MachineTarget;

    fn make_package() -> Resource {
        Resource {
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
        }
    }

    fn make_file() -> Resource {
        Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            path: Some("/etc/test.conf".to_string()),
            content: Some("key=value".to_string()),
            source: None,
            target: None,
            owner: Some("root".to_string()),
            group: Some("root".to_string()),
            mode: Some("0644".to_string()),
            name: None,
            enabled: None,
            restart_on: vec![],
            fs_type: None,
            options: None,
        }
    }

    fn make_service() -> Resource {
        Resource {
            resource_type: ResourceType::Service,
            machine: MachineTarget::Single("m1".to_string()),
            state: Some("running".to_string()),
            depends_on: vec![],
            provider: None,
            packages: vec![],
            path: None,
            content: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: Some("nginx".to_string()),
            enabled: Some(true),
            restart_on: vec![],
            fs_type: None,
            options: None,
        }
    }

    fn make_mount() -> Resource {
        Resource {
            resource_type: ResourceType::Mount,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            path: Some("/mnt/data".to_string()),
            content: None,
            source: Some("192.168.1.1:/data".to_string()),
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            fs_type: Some("nfs".to_string()),
            options: Some("ro,hard".to_string()),
        }
    }

    #[test]
    fn test_fj005_check_dispatches_package() {
        let r = make_package();
        let script = check_script(&r).unwrap();
        assert!(script.contains("dpkg"));
    }

    #[test]
    fn test_fj005_check_dispatches_file() {
        let r = make_file();
        let script = check_script(&r).unwrap();
        assert!(script.contains("test -f"));
    }

    #[test]
    fn test_fj005_check_dispatches_service() {
        let r = make_service();
        let script = check_script(&r).unwrap();
        assert!(script.contains("systemctl"));
    }

    #[test]
    fn test_fj005_check_dispatches_mount() {
        let r = make_mount();
        let script = check_script(&r).unwrap();
        assert!(script.contains("mountpoint"));
    }

    #[test]
    fn test_fj005_apply_dispatches_package() {
        let r = make_package();
        let script = apply_script(&r).unwrap();
        assert!(script.contains("apt-get install"));
    }

    #[test]
    fn test_fj005_apply_dispatches_file() {
        let r = make_file();
        let script = apply_script(&r).unwrap();
        assert!(script.contains("cat >"));
        assert!(script.contains("FORJAR_EOF"));
    }

    #[test]
    fn test_fj005_apply_dispatches_service() {
        let r = make_service();
        let script = apply_script(&r).unwrap();
        assert!(script.contains("systemctl start"));
    }

    #[test]
    fn test_fj005_apply_dispatches_mount() {
        let r = make_mount();
        let script = apply_script(&r).unwrap();
        assert!(script.contains("mount -t 'nfs'"));
    }

    #[test]
    fn test_fj005_state_query_dispatches() {
        let r = make_package();
        let script = state_query_script(&r).unwrap();
        assert!(script.contains("dpkg-query"));
    }

    #[test]
    fn test_fj005_state_query_file() {
        let mut r = make_package();
        r.resource_type = ResourceType::File;
        r.path = Some("/etc/conf".to_string());
        let script = state_query_script(&r).unwrap();
        assert!(script.contains("stat") || script.contains("/etc/conf"));
    }

    #[test]
    fn test_fj005_state_query_service() {
        let mut r = make_package();
        r.resource_type = ResourceType::Service;
        r.name = Some("nginx".to_string());
        let script = state_query_script(&r).unwrap();
        assert!(script.contains("nginx"));
    }

    #[test]
    fn test_fj005_unsupported_type() {
        let mut r = make_package();
        r.resource_type = ResourceType::Docker;
        assert!(check_script(&r).is_err());
        assert!(apply_script(&r).is_err());
        assert!(state_query_script(&r).is_err());
    }
}
