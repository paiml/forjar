//! FJ-032: Network/firewall resource handler.
//!
//! Manages firewall rules via ufw (Uncomplicated Firewall).

use crate::core::types::Resource;

/// Generate shell script to check if a firewall rule exists.
pub fn check_script(resource: &Resource) -> String {
    let port = resource.port.as_deref().unwrap_or("0");
    let protocol = resource.protocol.as_deref().unwrap_or("tcp");
    let action = resource.action.as_deref().unwrap_or("allow");
    format!(
        "ufw status numbered 2>/dev/null | grep -q '{}.*{}/{}' && echo 'exists:{}' || echo 'missing:{}'",
        action, port, protocol, port, port
    )
}

/// Generate shell script to add/remove a firewall rule.
pub fn apply_script(resource: &Resource) -> String {
    let port = resource.port.as_deref().unwrap_or("0");
    let protocol = resource.protocol.as_deref().unwrap_or("tcp");
    let action = resource.action.as_deref().unwrap_or("allow");
    let state = resource.state.as_deref().unwrap_or("present");

    let mut lines = vec![
        "set -euo pipefail".to_string(),
        "SUDO=\"\"".to_string(),
        "[ \"$(id -u)\" -ne 0 ] && SUDO=\"sudo\"".to_string(),
        // Ensure ufw is enabled
        "$SUDO ufw --force enable".to_string(),
    ];

    match state {
        "absent" => {
            let mut rule_parts = vec![];
            if let Some(ref from) = resource.from_addr {
                rule_parts.push(format!("from '{}'", from));
            }
            rule_parts.push(format!("to any port '{}' proto '{}'", port, protocol));

            lines.push(format!(
                "$SUDO ufw delete {} {} || true",
                action,
                rule_parts.join(" ")
            ));
        }
        _ => {
            let mut rule_parts = vec![];
            if let Some(ref from) = resource.from_addr {
                rule_parts.push(format!("from '{}'", from));
            }
            rule_parts.push(format!("to any port '{}' proto '{}'", port, protocol));

            if let Some(ref comment) = resource.name {
                lines.push(format!(
                    "$SUDO ufw {} {} comment '{}'",
                    action,
                    rule_parts.join(" "),
                    comment
                ));
            } else {
                lines.push(format!("$SUDO ufw {} {}", action, rule_parts.join(" ")));
            }
        }
    }

    lines.join("\n")
}

/// Generate shell to query firewall state (for BLAKE3 hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let port = resource.port.as_deref().unwrap_or("0");
    format!(
        "ufw status verbose 2>/dev/null | grep '{}' || echo 'rule=MISSING:{}'",
        port, port
    )
}

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
}
