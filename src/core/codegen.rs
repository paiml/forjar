//! FJ-005: Script generation — dispatch to resource handlers.
//! FJ-036: bashrs purification pipeline integrated (Invariant I8).
//!
//! Each resource type produces three scripts:
//! - check: read current state
//! - apply: converge to desired state
//! - state_query: query observable state for BLAKE3 hashing
//!
//! All scripts can be validated/purified via `core::purifier`.

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
        ResourceType::Pepita => Ok(resources::pepita::check_script(resource)),
        ResourceType::Model => Ok(resources::model::check_script(resource)),
        ResourceType::Gpu => Ok(resources::gpu::check_script(resource)),
        ResourceType::Recipe => {
            Err("codegen not implemented for recipe (expand first)".to_string())
        }
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
        ResourceType::Pepita => Ok(resources::pepita::apply_script(resource)),
        ResourceType::Model => Ok(resources::model::apply_script(resource)),
        ResourceType::Gpu => Ok(resources::gpu::apply_script(resource)),
        ResourceType::Recipe => {
            Err("codegen not implemented for recipe (expand first)".to_string())
        }
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
        ResourceType::Pepita => Ok(resources::pepita::state_query_script(resource)),
        ResourceType::Model => Ok(resources::model::state_query_script(resource)),
        ResourceType::Gpu => Ok(resources::gpu::state_query_script(resource)),
        ResourceType::Recipe => {
            Err("codegen not implemented for recipe (expand first)".to_string())
        }
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
            inputs: std::collections::HashMap::new(),
            arch: vec![],
            tags: vec![],
            resource_group: None,
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
            inputs: std::collections::HashMap::new(),
            arch: vec![],
            tags: vec![],
            resource_group: None,
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
            inputs: std::collections::HashMap::new(),
            arch: vec![],
            tags: vec![],
            resource_group: None,
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
            inputs: std::collections::HashMap::new(),
            arch: vec![],
            tags: vec![],
            resource_group: None,
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
    fn test_fj005_pepita_supported() {
        let mut r = make_package();
        r.resource_type = ResourceType::Pepita;
        r.name = Some("sandbox".to_string());
        r.netns = true;
        assert!(check_script(&r).is_ok());
        assert!(apply_script(&r).is_ok());
        assert!(state_query_script(&r).is_ok());
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
    fn test_fj040_pepita_codegen_check() {
        let mut r = make_package();
        r.resource_type = ResourceType::Pepita;
        r.name = Some("jail".to_string());
        r.chroot_dir = Some("/var/jail".to_string());
        let script = check_script(&r).unwrap();
        assert!(script.contains("chroot:present:jail"));
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

    #[test]
    fn test_fj132_check_script_all_types_succeed() {
        let resources = [
            make_package(),
            make_file(),
            make_service(),
            make_mount(),
            {
                let mut r = make_package();
                r.resource_type = ResourceType::User;
                r.name = Some("testuser".to_string());
                r
            },
            {
                let mut r = make_package();
                r.resource_type = ResourceType::Docker;
                r.name = Some("app".to_string());
                r.image = Some("nginx".to_string());
                r
            },
            {
                let mut r = make_package();
                r.resource_type = ResourceType::Cron;
                r.name = Some("backup".to_string());
                r.schedule = Some("0 2 * * *".to_string());
                r.command = Some("tar czf /backup.tar.gz /data".to_string());
                r
            },
            {
                let mut r = make_package();
                r.resource_type = ResourceType::Network;
                r.port = Some("443".to_string());
                r
            },
        ];
        for r in &resources {
            let result = check_script(r);
            assert!(
                result.is_ok(),
                "check_script for {:?} should succeed: {:?}",
                r.resource_type,
                result.err()
            );
        }
    }

    #[test]
    fn test_fj132_state_query_all_types_succeed() {
        let resources = [
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
                r.schedule = Some("* * * * *".to_string());
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
        for r in &resources {
            let result = state_query_script(r);
            assert!(
                result.is_ok(),
                "state_query for {:?} should succeed: {:?}",
                r.resource_type,
                result.err()
            );
        }
    }

    #[test]
    fn test_fj040_pepita_codegen_apply_netns() {
        let mut r = make_package();
        r.resource_type = ResourceType::Pepita;
        r.name = Some("net-sandbox".to_string());
        r.netns = true;
        let script = apply_script(&r).unwrap();
        assert!(script.contains("ip netns add 'forjar-net-sandbox'"));
    }

    // --- FJ-036: Additional codegen tests ---

    #[test]
    fn test_fj036_check_script_user_contains_name() {
        let mut r = make_package();
        r.resource_type = ResourceType::User;
        r.name = Some("operator".to_string());
        let script = check_script(&r).unwrap();
        assert!(
            script.contains("operator"),
            "user check script must reference the username 'operator': {script}"
        );
    }

    #[test]
    fn test_fj036_apply_docker_volumes_escaped() {
        let mut r = make_package();
        r.resource_type = ResourceType::Docker;
        r.name = Some("db".to_string());
        r.image = Some("postgres:15".to_string());
        r.volumes = vec![
            "/host/path with spaces:/container/data".to_string(),
            "/var/log:/logs".to_string(),
        ];
        let script = apply_script(&r).unwrap();
        // Volumes must be single-quoted to prevent shell word splitting
        assert!(
            script.contains("-v '/host/path with spaces:/container/data'"),
            "volume with spaces must be properly quoted: {script}"
        );
        assert!(
            script.contains("-v '/var/log:/logs'"),
            "second volume must be properly quoted: {script}"
        );
    }

    #[test]
    fn test_fj036_state_query_mount_contains_findmnt() {
        let mut r = make_mount();
        r.path = Some("/mnt/nfs-share".to_string());
        let script = state_query_script(&r).unwrap();
        assert!(
            script.contains("findmnt"),
            "mount state_query must use findmnt to query mount details: {script}"
        );
    }

    #[test]
    fn test_codegen_package_absent() {
        let mut r = make_package();
        r.state = Some("absent".to_string());
        let script = apply_script(&r).unwrap();
        assert!(
            script.contains("apt-get remove"),
            "package with state=absent should generate removal script: {script}"
        );
    }

    #[test]
    fn test_codegen_file_with_owner_and_mode() {
        let mut r = make_file();
        r.owner = Some("www-data".to_string());
        r.mode = Some("0644".to_string());
        r.content = Some("hello".to_string());
        let script = apply_script(&r).unwrap();
        assert!(
            script.contains("chown 'www-data"),
            "file script should set owner to www-data: {script}"
        );
        assert!(
            script.contains("chmod '0644'"),
            "file script should set mode to 0644: {script}"
        );
        assert!(
            script.contains("hello"),
            "file script should contain content 'hello': {script}"
        );
    }

    #[test]
    fn test_codegen_mount_with_options() {
        let mut r = make_mount();
        r.source = Some("/dev/sdb1".to_string());
        r.fs_type = Some("ext4".to_string());
        r.options = Some("noatime,errors=remount-ro".to_string());
        let script = apply_script(&r).unwrap();
        assert!(
            script.contains("mount -t 'ext4'"),
            "mount script should contain fstype ext4: {script}"
        );
        assert!(
            script.contains("noatime,errors=remount-ro"),
            "mount script should contain options: {script}"
        );
        assert!(
            script.contains("/dev/sdb1"),
            "mount script should reference the device: {script}"
        );
    }

    #[test]
    fn test_codegen_service_disabled() {
        let mut r = make_service();
        r.state = Some("stopped".to_string());
        r.enabled = Some(false);
        let script = apply_script(&r).unwrap();
        assert!(
            script.contains("systemctl stop"),
            "stopped service should generate stop command: {script}"
        );
        assert!(
            script.contains("systemctl disable"),
            "disabled service should generate disable command: {script}"
        );
    }

    // -- Coverage boost tests --

    #[test]
    fn test_codegen_user_absent() {
        let mut r = make_package();
        r.resource_type = ResourceType::User;
        r.name = Some("staleuser".to_string());
        r.state = Some("absent".to_string());
        let script = apply_script(&r).unwrap();
        assert!(
            script.contains("userdel"),
            "user absent should generate userdel: {script}"
        );
        assert!(
            !script.contains("useradd"),
            "user absent must not generate useradd: {script}"
        );
        assert!(
            !script.contains("usermod"),
            "user absent must not generate usermod: {script}"
        );
        assert!(
            script.contains("'staleuser'"),
            "userdel must reference the username: {script}"
        );
    }

    #[test]
    fn test_codegen_docker_with_ports_and_env() {
        let mut r = make_package();
        r.resource_type = ResourceType::Docker;
        r.name = Some("webapp".to_string());
        r.image = Some("myapp:v2".to_string());
        r.state = Some("running".to_string());
        r.ports = vec!["8080:80".to_string(), "8443:443".to_string()];
        r.environment = vec!["DB_HOST=db.local".to_string(), "LOG_LEVEL=info".to_string()];
        r.volumes = vec!["/data:/app/data".to_string()];
        let check = check_script(&r).unwrap();
        assert!(
            check.contains("docker inspect"),
            "docker check must use inspect: {check}"
        );
        let apply = apply_script(&r).unwrap();
        assert!(
            apply.contains("-p '8080:80'"),
            "apply must map port 8080:80: {apply}"
        );
        assert!(
            apply.contains("-p '8443:443'"),
            "apply must map port 8443:443: {apply}"
        );
        assert!(
            apply.contains("-e 'DB_HOST=db.local'"),
            "apply must set env DB_HOST: {apply}"
        );
        assert!(
            apply.contains("-e 'LOG_LEVEL=info'"),
            "apply must set env LOG_LEVEL: {apply}"
        );
        assert!(
            apply.contains("-v '/data:/app/data'"),
            "apply must mount volume: {apply}"
        );
        assert!(
            apply.contains("docker run -d"),
            "apply must run in detached mode: {apply}"
        );
        let query = state_query_script(&r).unwrap();
        assert!(
            query.contains("docker inspect 'webapp'"),
            "state_query must inspect container: {query}"
        );
    }

    #[test]
    fn test_codegen_cron_with_schedule() {
        let mut r = make_package();
        r.resource_type = ResourceType::Cron;
        r.name = Some("nightly-backup".to_string());
        r.owner = Some("deploy".to_string());
        r.schedule = Some("30 2 * * *".to_string());
        r.command = Some("/opt/backup/run.sh".to_string());
        let check = check_script(&r).unwrap();
        assert!(
            check.contains("crontab"),
            "cron check must use crontab: {check}"
        );
        assert!(
            check.contains("forjar:nightly-backup"),
            "cron check must reference job name: {check}"
        );
        let apply = apply_script(&r).unwrap();
        assert!(
            apply.contains("30 2 * * *"),
            "cron apply must include schedule: {apply}"
        );
        assert!(
            apply.contains("/opt/backup/run.sh"),
            "cron apply must include command: {apply}"
        );
        assert!(
            apply.contains("crontab -u 'deploy'"),
            "cron apply must target user 'deploy': {apply}"
        );
        let query = state_query_script(&r).unwrap();
        assert!(
            query.contains("crontab -u 'deploy' -l"),
            "cron state_query must list deploy's crontab: {query}"
        );
    }

    #[test]
    fn test_codegen_network_reject() {
        let mut r = make_package();
        r.resource_type = ResourceType::Network;
        r.port = Some("25".to_string());
        r.protocol = Some("tcp".to_string());
        r.action = Some("reject".to_string());
        r.from_addr = Some("0.0.0.0/0".to_string());
        let check = check_script(&r).unwrap();
        assert!(
            check.contains("ufw status"),
            "network check must query ufw: {check}"
        );
        assert!(
            check.contains("reject"),
            "network check must include action 'reject': {check}"
        );
        assert!(
            check.contains("25/tcp"),
            "network check must include port/proto: {check}"
        );
        let apply = apply_script(&r).unwrap();
        assert!(
            apply.contains("ufw reject"),
            "network apply must use 'ufw reject': {apply}"
        );
        assert!(
            apply.contains("from '0.0.0.0/0'"),
            "network apply must include from_addr: {apply}"
        );
        assert!(
            apply.contains("port '25'"),
            "network apply must include port: {apply}"
        );
        let query = state_query_script(&r).unwrap();
        assert!(
            query.contains("ufw status verbose"),
            "network state_query must use ufw status verbose: {query}"
        );
    }
}
