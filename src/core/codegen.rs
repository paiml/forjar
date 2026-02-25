//! FJ-005: Script generation — dispatch to resource handlers.
//!
//! Each resource type produces three scripts:
//! - check: read current state
//! - apply: converge to desired state
//! - state_query: query observable state for BLAKE3 hashing

use super::types::{Resource, ResourceType};
use crate::resources;
use provable_contracts_macros::contract;

/// Generate a check script for a resource.
#[contract("codegen-dispatch-v1", equation = "check_script")]
pub fn check_script(resource: &Resource) -> Result<String, String> {
    match &resource.resource_type {
        ResourceType::Package => Ok(resources::package::check_script(resource)),
        ResourceType::File => Ok(resources::file::check_script(resource)),
        ResourceType::Service => Ok(resources::service::check_script(resource)),
        ResourceType::Mount => Ok(resources::mount::check_script(resource)),
        ResourceType::User => Ok(resources::user::check_script(resource)),
        ResourceType::Docker => Ok(resources::docker::check_script(resource)),
        ResourceType::Cron => Ok(resources::cron::check_script(resource)),
        ResourceType::Network => Ok(resources::network::check_script(resource)),
        other => Err(format!("codegen not implemented for {} (Phase 3+)", other)),
    }
}

/// Generate an apply script for a resource.
#[contract("codegen-dispatch-v1", equation = "apply_script")]
pub fn apply_script(resource: &Resource) -> Result<String, String> {
    match &resource.resource_type {
        ResourceType::Package => Ok(resources::package::apply_script(resource)),
        ResourceType::File => Ok(resources::file::apply_script(resource)),
        ResourceType::Service => Ok(resources::service::apply_script(resource)),
        ResourceType::Mount => Ok(resources::mount::apply_script(resource)),
        ResourceType::User => Ok(resources::user::apply_script(resource)),
        ResourceType::Docker => Ok(resources::docker::apply_script(resource)),
        ResourceType::Cron => Ok(resources::cron::apply_script(resource)),
        ResourceType::Network => Ok(resources::network::apply_script(resource)),
        other => Err(format!("codegen not implemented for {} (Phase 3+)", other)),
    }
}

/// Generate a state query script for a resource.
#[contract("codegen-dispatch-v1", equation = "state_query_script")]
pub fn state_query_script(resource: &Resource) -> Result<String, String> {
    match &resource.resource_type {
        ResourceType::Package => Ok(resources::package::state_query_script(resource)),
        ResourceType::File => Ok(resources::file::state_query_script(resource)),
        ResourceType::Service => Ok(resources::service::state_query_script(resource)),
        ResourceType::Mount => Ok(resources::mount::state_query_script(resource)),
        ResourceType::User => Ok(resources::user::state_query_script(resource)),
        ResourceType::Docker => Ok(resources::docker::state_query_script(resource)),
        ResourceType::Cron => Ok(resources::cron::state_query_script(resource)),
        ResourceType::Network => Ok(resources::network::state_query_script(resource)),
        other => Err(format!("codegen not implemented for {} (Phase 3+)", other)),
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
            inputs: std::collections::HashMap::new(),
            arch: vec![],
            tags: vec![],
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
            version: None,
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
            inputs: std::collections::HashMap::new(),
            arch: vec![],
            tags: vec![],
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
            version: None,
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
            inputs: std::collections::HashMap::new(),
            arch: vec![],
            tags: vec![],
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
            version: None,
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
            inputs: std::collections::HashMap::new(),
            arch: vec![],
            tags: vec![],
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
        assert!(script.contains("$SUDO apt-get install"));
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
        r.resource_type = ResourceType::Pepita;
        assert!(check_script(&r).is_err());
        assert!(apply_script(&r).is_err());
        assert!(state_query_script(&r).is_err());
    }

    // ── Falsification tests (Codegen Dispatch Contract) ─────────

    /// FALSIFY-CD-001: All Phase 1 types produce Ok for all three functions.
    #[test]
    fn falsify_cd_001_dispatch_completeness() {
        let resources = [make_package(), make_file(), make_service(), make_mount()];
        for r in &resources {
            assert!(
                check_script(r).is_ok(),
                "check_script failed for {:?}",
                r.resource_type
            );
            assert!(
                apply_script(r).is_ok(),
                "apply_script failed for {:?}",
                r.resource_type
            );
            assert!(
                state_query_script(r).is_ok(),
                "state_query_script failed for {:?}",
                r.resource_type
            );
        }
    }

    #[test]
    fn test_fj005_dispatch_user() {
        let mut r = make_package();
        r.resource_type = ResourceType::User;
        r.name = Some("deploy".to_string());
        assert!(check_script(&r).unwrap().contains("deploy"));
        assert!(apply_script(&r).unwrap().contains("useradd"));
        assert!(state_query_script(&r).unwrap().contains("deploy"));
    }

    #[test]
    fn test_fj005_dispatch_docker() {
        let mut r = make_package();
        r.resource_type = ResourceType::Docker;
        r.image = Some("nginx:latest".to_string());
        r.name = Some("web".to_string());
        assert!(check_script(&r).unwrap().contains("docker"));
        assert!(apply_script(&r).unwrap().contains("docker"));
        assert!(state_query_script(&r).unwrap().contains("docker"));
    }

    #[test]
    fn test_fj005_dispatch_cron() {
        let mut r = make_package();
        r.resource_type = ResourceType::Cron;
        r.schedule = Some("0 2 * * *".to_string());
        r.command = Some("/opt/backup.sh".to_string());
        assert!(check_script(&r).unwrap().contains("crontab"));
        assert!(apply_script(&r).unwrap().contains("backup.sh"));
        assert!(state_query_script(&r).unwrap().contains("crontab"));
    }

    #[test]
    fn test_fj005_dispatch_network() {
        let mut r = make_package();
        r.resource_type = ResourceType::Network;
        r.port = Some("443".to_string());
        r.action = Some("allow".to_string());
        assert!(check_script(&r).unwrap().contains("ufw"));
        assert!(apply_script(&r).unwrap().contains("ufw"));
        assert!(state_query_script(&r).unwrap().contains("ufw"));
    }

    #[test]
    fn test_fj005_apply_scripts_contain_pipefail() {
        // All apply scripts should start with set -euo pipefail
        let resources = [make_package(), make_file(), make_service(), make_mount()];
        for r in &resources {
            let script = apply_script(r).unwrap();
            assert!(
                script.contains("set -euo pipefail"),
                "apply script for {:?} must contain pipefail",
                r.resource_type
            );
        }
    }

    #[test]
    fn test_fj005_unsupported_type_error_message() {
        let mut r = make_package();
        r.resource_type = ResourceType::Pepita;
        let err = check_script(&r).unwrap_err();
        assert!(
            err.contains("Phase 3+"),
            "error should mention Phase 3+: {}",
            err
        );
        assert!(
            err.contains("pepita"),
            "error should name the type: {}",
            err
        );
    }

    #[test]
    fn test_fj005_recipe_type_not_dispatchable() {
        let mut r = make_package();
        r.resource_type = ResourceType::Recipe;
        assert!(
            check_script(&r).is_err(),
            "recipe type should not be directly dispatchable"
        );
        assert!(apply_script(&r).is_err());
        assert!(state_query_script(&r).is_err());
    }

    #[test]
    fn test_fj005_all_phase1_check_scripts_nonempty() {
        let types_and_resources = [
            make_package(),
            make_file(),
            make_service(),
            make_mount(),
            {
                let mut r = make_package();
                r.resource_type = ResourceType::User;
                r.name = Some("u".to_string());
                r
            },
            {
                let mut r = make_package();
                r.resource_type = ResourceType::Docker;
                r.name = Some("c".to_string());
                r.image = Some("img".to_string());
                r
            },
            {
                let mut r = make_package();
                r.resource_type = ResourceType::Cron;
                r.name = Some("j".to_string());
                r.schedule = Some("0 * * * *".to_string());
                r.command = Some("echo".to_string());
                r
            },
            {
                let mut r = make_package();
                r.resource_type = ResourceType::Network;
                r.port = Some("80".to_string());
                r
            },
        ];
        for r in &types_and_resources {
            let script = check_script(r).unwrap();
            assert!(
                !script.is_empty(),
                "check_script for {:?} should not be empty",
                r.resource_type
            );
        }
    }

    #[test]
    fn test_fj005_all_phase1_state_query_scripts_nonempty() {
        let types_and_resources = [
            make_package(),
            make_file(),
            make_service(),
            make_mount(),
            {
                let mut r = make_package();
                r.resource_type = ResourceType::User;
                r.name = Some("u".to_string());
                r
            },
            {
                let mut r = make_package();
                r.resource_type = ResourceType::Docker;
                r.name = Some("c".to_string());
                r
            },
            {
                let mut r = make_package();
                r.resource_type = ResourceType::Cron;
                r.name = Some("j".to_string());
                r
            },
            {
                let mut r = make_package();
                r.resource_type = ResourceType::Network;
                r.port = Some("80".to_string());
                r
            },
        ];
        for r in &types_and_resources {
            let script = state_query_script(r).unwrap();
            assert!(
                !script.is_empty(),
                "state_query_script for {:?} should not be empty",
                r.resource_type
            );
        }
    }

    /// FALSIFY-CD-002: Dispatch is symmetric — same types handled by all three functions.
    #[test]
    fn falsify_cd_002_dispatch_symmetry() {
        let all_types = [
            ResourceType::Package,
            ResourceType::File,
            ResourceType::Service,
            ResourceType::Mount,
            ResourceType::Docker,
            ResourceType::User,
            ResourceType::Network,
            ResourceType::Cron,
            ResourceType::Pepita,
        ];

        for rt in &all_types {
            let mut r = make_package();
            r.resource_type = rt.clone();

            let check_ok = check_script(&r).is_ok();
            let apply_ok = apply_script(&r).is_ok();
            let query_ok = state_query_script(&r).is_ok();

            assert_eq!(
                check_ok, apply_ok,
                "check/apply asymmetry for {:?}: check={}, apply={}",
                rt, check_ok, apply_ok
            );
            assert_eq!(
                apply_ok, query_ok,
                "apply/query asymmetry for {:?}: apply={}, query={}",
                rt, apply_ok, query_ok
            );
        }
    }

    // --- FJ-132: Codegen edge case tests ---

    #[test]
    fn test_fj132_state_query_docker_contains_inspect() {
        let mut r = make_package();
        r.resource_type = ResourceType::Docker;
        r.name = Some("web".to_string());
        r.image = Some("nginx:latest".to_string());
        let script = state_query_script(&r).unwrap();
        assert!(
            script.contains("docker inspect"),
            "docker state_query should use docker inspect"
        );
    }

    #[test]
    fn test_fj132_state_query_cron_contains_crontab() {
        let mut r = make_package();
        r.resource_type = ResourceType::Cron;
        r.name = Some("backup".to_string());
        r.schedule = Some("0 2 * * *".to_string());
        r.command = Some("/opt/backup.sh".to_string());
        let script = state_query_script(&r).unwrap();
        assert!(
            script.contains("crontab"),
            "cron state_query should read crontab"
        );
    }

    #[test]
    fn test_fj132_state_query_network_contains_ufw() {
        let mut r = make_package();
        r.resource_type = ResourceType::Network;
        r.port = Some("443".to_string());
        r.action = Some("allow".to_string());
        let script = state_query_script(&r).unwrap();
        assert!(script.contains("ufw"), "network state_query should use ufw");
    }

    #[test]
    fn test_fj132_state_query_user_contains_id() {
        let mut r = make_package();
        r.resource_type = ResourceType::User;
        r.name = Some("deploy".to_string());
        let script = state_query_script(&r).unwrap();
        assert!(
            script.contains("id ") || script.contains("getent"),
            "user state_query should use id or getent"
        );
    }

    #[test]
    fn test_fj132_all_phase1_apply_pipefail() {
        // Verify ALL Phase 1 types have pipefail in apply scripts
        let types_and_resources = [
            make_package(),
            make_file(),
            make_service(),
            make_mount(),
            {
                let mut r = make_package();
                r.resource_type = ResourceType::User;
                r.name = Some("u".to_string());
                r
            },
            {
                let mut r = make_package();
                r.resource_type = ResourceType::Docker;
                r.name = Some("c".to_string());
                r.image = Some("img".to_string());
                r
            },
            {
                let mut r = make_package();
                r.resource_type = ResourceType::Cron;
                r.name = Some("j".to_string());
                r.schedule = Some("0 * * * *".to_string());
                r.command = Some("echo".to_string());
                r
            },
            {
                let mut r = make_package();
                r.resource_type = ResourceType::Network;
                r.port = Some("80".to_string());
                r
            },
        ];
        for r in &types_and_resources {
            let script = apply_script(r).unwrap();
            assert!(
                script.contains("set -euo pipefail"),
                "apply script for {:?} must contain pipefail",
                r.resource_type
            );
        }
    }
}
