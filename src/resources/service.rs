//! FJ-008: Systemd service resource handler.

use crate::core::types::Resource;

/// Generate shell to check service state.
pub fn check_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    format!(
        "systemctl is-active '{}' 2>/dev/null && echo 'active:{}' || echo 'inactive:{}'\n\
         systemctl is-enabled '{}' 2>/dev/null && echo 'enabled:{}' || echo 'disabled:{}'",
        name, name, name, name, name, name
    )
}

/// Generate shell to converge service to desired state.
pub fn apply_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let state = resource.state.as_deref().unwrap_or("running");
    let enabled = resource.enabled.unwrap_or(true);

    let mut lines = vec!["set -euo pipefail".to_string()];

    match state {
        "running" => {
            lines.push(format!(
                "if ! systemctl is-active --quiet '{}'; then\n  systemctl start '{}'\nfi",
                name, name
            ));
        }
        "stopped" => {
            lines.push(format!(
                "if systemctl is-active --quiet '{}'; then\n  systemctl stop '{}'\nfi",
                name, name
            ));
        }
        _ => {}
    }

    if enabled {
        lines.push(format!(
            "if ! systemctl is-enabled --quiet '{}'; then\n  systemctl enable '{}'\nfi",
            name, name
        ));
    } else {
        lines.push(format!(
            "if systemctl is-enabled --quiet '{}'; then\n  systemctl disable '{}'\nfi",
            name, name
        ));
    }

    // Reload if needed (after config changes)
    if !resource.restart_on.is_empty() {
        lines.push(format!("systemctl reload-or-restart '{}'", name));
    }

    lines.join("\n")
}

/// Generate shell to query service state (for hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    format!(
        "echo \"active=$(systemctl is-active '{}' 2>/dev/null || echo 'unknown')\"\n\
         echo \"enabled=$(systemctl is-enabled '{}' 2>/dev/null || echo 'unknown')\"",
        name, name
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn make_service_resource(name: &str, state: &str) -> Resource {
        Resource {
            resource_type: ResourceType::Service,
            machine: MachineTarget::Single("m1".to_string()),
            state: Some(state.to_string()),
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
            name: Some(name.to_string()),
            enabled: Some(true),
            restart_on: vec![],
            fs_type: None,
            options: None,
        }
    }

    #[test]
    fn test_fj008_check_service() {
        let r = make_service_resource("nfs-kernel-server", "running");
        let script = check_script(&r);
        assert!(script.contains("systemctl is-active 'nfs-kernel-server'"));
        assert!(script.contains("systemctl is-enabled 'nfs-kernel-server'"));
    }

    #[test]
    fn test_fj008_apply_start() {
        let r = make_service_resource("nfs-kernel-server", "running");
        let script = apply_script(&r);
        assert!(script.contains("systemctl start"));
        assert!(script.contains("systemctl enable"));
    }

    #[test]
    fn test_fj008_apply_stop() {
        let r = make_service_resource("nfs-kernel-server", "stopped");
        let script = apply_script(&r);
        assert!(script.contains("systemctl stop"));
    }

    #[test]
    fn test_fj008_disable() {
        let mut r = make_service_resource("svc", "running");
        r.enabled = Some(false);
        let script = apply_script(&r);
        assert!(script.contains("systemctl disable"));
    }

    #[test]
    fn test_fj008_restart_on() {
        let mut r = make_service_resource("svc", "running");
        r.restart_on = vec!["config-file".to_string()];
        let script = apply_script(&r);
        assert!(script.contains("systemctl reload-or-restart"));
    }
}
