#[allow(unused_imports)]
use super::network::*;
#[allow(unused_imports)]
use crate::core::types::Resource;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn make_network_resource(port: &str, action: &str) -> Resource {
        Resource {
            resource_type: ResourceType::Network,
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
            protocol: Some("tcp".to_string()),
            port: Some(port.to_string()),
            action: Some(action.to_string()),
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
            gpu_backend: None,
            driver_version: None,
            cuda_version: None,
            rocm_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
            output_artifacts: vec![],
            completion_check: None,
            timeout: None,
            working_dir: None,
            task_mode: None,
            task_inputs: vec![],
            stages: vec![],
            cache: false,
            gpu_device: None,
            restart_delay: None,
            pre_apply: None,
            post_apply: None,
            lifecycle: None,
            store: false,
            sudo: false,
            script: None,
            gather: vec![],
            scatter: vec![],
        }
    }

    #[test]
    fn test_fj032_check_rule() {
        let r = make_network_resource("22", "allow");
        let script = check_script(&r);
        assert!(script.contains("ufw status numbered"));
        assert!(script.contains("22/tcp"));
        assert!(script.contains("exists:22"));
        assert!(script.contains("missing:22"));
    }

    #[test]
    fn test_fj032_apply_allow() {
        let r = make_network_resource("80", "allow");
        let script = apply_script(&r);
        assert!(script.contains("set -euo pipefail"));
        assert!(script.contains("ufw --force enable"));
        assert!(script.contains("ufw allow"));
        assert!(script.contains("port '80'"));
        assert!(script.contains("proto 'tcp'"));
    }

    #[test]
    fn test_fj032_apply_deny() {
        let r = make_network_resource("3306", "deny");
        let script = apply_script(&r);
        assert!(script.contains("ufw deny"));
        assert!(script.contains("port '3306'"));
    }

    #[test]
    fn test_fj032_apply_with_source() {
        let mut r = make_network_resource("22", "allow");
        r.from_addr = Some("192.168.1.0/24".to_string());
        let script = apply_script(&r);
        assert!(script.contains("from '192.168.1.0/24'"));
    }

    #[test]
    fn test_fj032_apply_with_comment() {
        let mut r = make_network_resource("443", "allow");
        r.name = Some("https-rule".to_string());
        let script = apply_script(&r);
        assert!(script.contains("comment 'https-rule'"));
    }

    #[test]
    fn test_fj032_apply_absent() {
        let mut r = make_network_resource("8080", "allow");
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        assert!(script.contains("ufw delete allow"));
        assert!(script.contains("port '8080'"));
    }

    #[test]
    fn test_fj032_state_query() {
        let r = make_network_resource("80", "allow");
        let script = state_query_script(&r);
        assert!(script.contains("ufw status verbose"));
        assert!(script.contains("rule=MISSING:80"));
    }

    #[test]
    fn test_fj032_sudo_detection() {
        let r = make_network_resource("22", "allow");
        let script = apply_script(&r);
        assert!(script.contains("SUDO=\"\""));
        assert!(script.contains("$SUDO ufw"));
    }

    #[test]
    fn test_fj032_apply_reject() {
        let r = make_network_resource("25", "reject");
        let script = apply_script(&r);
        assert!(script.contains("ufw reject"));
        assert!(script.contains("port '25'"));
    }

    #[test]
    fn test_fj032_apply_udp_protocol() {
        let mut r = make_network_resource("53", "allow");
        r.protocol = Some("udp".to_string());
        let script = apply_script(&r);
        assert!(script.contains("proto 'udp'"));
        assert!(script.contains("port '53'"));
    }

    #[test]
    fn test_fj032_absent_with_source() {
        let mut r = make_network_resource("22", "allow");
        r.state = Some("absent".to_string());
        r.from_addr = Some("10.0.0.0/8".to_string());
        let script = apply_script(&r);
        assert!(script.contains("ufw delete allow"));
        assert!(script.contains("from '10.0.0.0/8'"));
        assert!(script.contains("port '22'"));
    }

    #[test]
    fn test_fj032_check_script_default_protocol() {
        let mut r = make_network_resource("80", "allow");
        r.protocol = None;
        let script = check_script(&r);
        // Should default to tcp
        assert!(script.contains("80/tcp"));
    }

    #[test]
    fn test_fj032_state_query_default_port() {
        let mut r = make_network_resource("443", "allow");
        r.port = None;
        let script = state_query_script(&r);
        assert!(script.contains("rule=MISSING:0"));
    }

    #[test]
    fn test_fj032_apply_from_cidr_range() {
        let mut r = make_network_resource("5432", "allow");
        r.from_addr = Some("172.16.0.0/12".to_string());
        let script = apply_script(&r);
        assert!(script.contains("from '172.16.0.0/12'"));
        assert!(script.contains("port '5432'"));
        // from comes before to
        let from_idx = script.find("from '172.16.0.0/12'").unwrap();
        let to_idx = script.find("to any port").unwrap();
        assert!(from_idx < to_idx, "from must come before to in ufw rule");
    }

    #[test]
    fn test_fj032_apply_script_pipefail() {
        let r = make_network_resource("22", "allow");
        let script = apply_script(&r);
        assert!(
            script.starts_with("set -euo pipefail"),
            "apply script must start with safety flags"
        );
    }

    // ── Edge-case tests (FJ-123) ─────────────────────────────────

    #[test]
    fn test_fj032_absent_with_from_addr() {
        // Absent state with from_addr should include from clause in delete
        let mut r = make_network_resource("22", "deny");
        r.state = Some("absent".to_string());
        r.from_addr = Some("192.168.1.100".to_string());
        let script = apply_script(&r);
        assert!(script.contains("ufw delete deny"));
        assert!(script.contains("from '192.168.1.100'"));
    }

    #[test]
    fn test_fj032_all_defaults() {
        // No port, no protocol, no action — all defaults
        let mut r = make_network_resource("80", "allow");
        r.port = None;
        r.protocol = None;
        r.action = None;
        let script = apply_script(&r);
        assert!(script.contains("ufw allow"));
        assert!(script.contains("port '0'"));
        assert!(script.contains("proto 'tcp'"));
    }

    #[test]
    fn test_fj032_present_no_comment_without_name() {
        // Without name, no comment clause should appear
        let r = make_network_resource("443", "allow");
        assert!(r.name.is_none());
        let script = apply_script(&r);
        assert!(!script.contains("comment"));
    }

    #[test]
    fn test_fj032_ufw_force_enable_always() {
        // ufw --force enable should appear in all apply states
        let r = make_network_resource("80", "allow");
        let script = apply_script(&r);
        assert!(script.contains("$SUDO ufw --force enable"));

        let mut r2 = make_network_resource("80", "allow");
        r2.state = Some("absent".to_string());
        let script2 = apply_script(&r2);
        assert!(script2.contains("$SUDO ufw --force enable"));
    }

    // ── FJ-132: Additional network edge case tests ─────────────

    #[test]
    fn test_fj132_check_script_action_in_pattern() {
        // check_script should include the action in its grep pattern
        let r = make_network_resource("443", "deny");
        let script = check_script(&r);
        assert!(script.contains("deny.*443/tcp"));
    }

    #[test]
    fn test_fj132_apply_absent_with_udp() {
        let mut r = make_network_resource("53", "allow");
        r.state = Some("absent".to_string());
        r.protocol = Some("udp".to_string());
        let script = apply_script(&r);
        assert!(script.contains("ufw delete allow"));
        assert!(script.contains("proto 'udp'"));
    }

    #[test]
    fn test_fj132_apply_absent_idempotent() {
        // Delete rule should use || true for idempotency
        let mut r = make_network_resource("80", "allow");
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        assert!(script.contains("|| true"), "delete should be idempotent");
    }

    #[test]
    fn test_fj132_state_query_port_in_pattern() {
        let r = make_network_resource("8443", "allow");
        let script = state_query_script(&r);
        assert!(script.contains("grep '8443'"));
    }

    #[test]
    fn test_fj132_apply_high_port() {
        let r = make_network_resource("65535", "allow");
        let script = apply_script(&r);
        assert!(script.contains("port '65535'"));
    }

    // ── FJ-036: Additional network resource tests ────────────────

    #[test]
    fn test_fj036_network_apply_with_comment() {
        // When name (comment) and from_addr are set, both appear in the ufw rule
        let mut r = make_network_resource("5432", "allow");
        r.name = Some("postgres-access".to_string());
        r.from_addr = Some("10.0.1.0/24".to_string());
        let script = apply_script(&r);
        assert!(
            script.contains("comment 'postgres-access'"),
            "comment must appear in ufw rule"
        );
        assert!(
            script.contains("from '10.0.1.0/24'"),
            "from_addr must appear in ufw rule"
        );
        assert!(script.contains("ufw allow"), "action must be allow");
        assert!(
            script.contains("port '5432'"),
            "port must be present in rule"
        );
    }

    #[test]
    fn test_fj153_network_all_defaults() {
        let mut r = make_network_resource("0", "allow");
        r.port = None;
        r.protocol = None;
        r.action = None;
        r.state = None;
        let script = apply_script(&r);
        assert!(script.contains("ufw allow"));
        assert!(script.contains("port '0'"));
        assert!(script.contains("proto 'tcp'"));
    }

    #[test]
    fn test_fj153_network_absent_with_from() {
        let mut r = make_network_resource("22", "deny");
        r.state = Some("absent".to_string());
        r.from_addr = Some("10.0.0.0/8".to_string());
        let script = apply_script(&r);
        assert!(script.contains("ufw delete deny"));
        assert!(script.contains("from '10.0.0.0/8'"));
        assert!(script.contains("port '22'"));
        assert!(script.contains("|| true"));
    }

    #[test]
    fn test_fj153_network_no_name_no_comment() {
        let mut r = make_network_resource("443", "allow");
        r.name = None;
        let script = apply_script(&r);
        assert!(!script.contains("comment"));
        assert!(script.contains("ufw allow"));
    }

    #[test]
    fn test_fj153_network_check_defaults() {
        let mut r = make_network_resource("0", "allow");
        r.port = None;
        r.protocol = None;
        r.action = None;
        let script = check_script(&r);
        assert!(script.contains("0/tcp"));
        assert!(script.contains("allow"));
    }

    // ── PMAT-038: ufw guard for containers ─────────────

    #[test]
    fn test_pmat038_apply_guards_ufw_availability() {
        // apply_script should check for ufw before running it,
        // gracefully skipping when ufw is not available (e.g. containers).
        let r = make_network_resource("80", "allow");
        let script = apply_script(&r);
        assert!(
            script.contains("command -v ufw"),
            "apply must check ufw availability before running it"
        );
    }

    #[test]
    fn test_pmat038_check_guards_ufw_availability() {
        let r = make_network_resource("80", "allow");
        let script = check_script(&r);
        assert!(
            script.contains("command -v ufw"),
            "check must guard against missing ufw"
        );
    }

    #[test]
    fn test_pmat038_state_query_guards_ufw_availability() {
        let r = make_network_resource("80", "allow");
        let script = state_query_script(&r);
        assert!(
            script.contains("command -v ufw"),
            "state_query must guard against missing ufw"
        );
    }

    #[test]
    fn test_fj036_network_check_contains_ufw_status() {
        // check_script must query ufw status to determine rule existence
        let r = make_network_resource("443", "allow");
        let script = check_script(&r);
        assert!(
            script.contains("ufw status"),
            "check script must query ufw status"
        );
        assert!(
            script.contains("443/tcp"),
            "check script must include port/protocol pattern"
        );
        assert!(
            script.contains("allow"),
            "check script must include action in grep pattern"
        );
        assert!(
            script.contains("exists:443"),
            "check script must emit exists token"
        );
        assert!(
            script.contains("missing:443"),
            "check script must emit missing token"
        );
    }
}
