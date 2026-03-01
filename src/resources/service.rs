//! FJ-008/081: Systemd service resource handler.
//!
//! Generates shell scripts for systemd service management.
//! Includes runtime systemd detection (FJ-081) — gracefully skips
//! when systemctl is unavailable (e.g. inside containers without systemd).

use crate::core::types::Resource;

/// Shell preamble that detects systemd availability.
/// If systemctl is not found, prints a warning and exits 0 (skip).
const SYSTEMD_GUARD: &str = "\
if ! command -v systemctl >/dev/null 2>&1; then\n  \
  echo 'FORJAR_WARN: systemctl not found — skipping service resource (no systemd)'\n  \
  exit 0\n\
fi";

/// Generate shell to check service state.
pub fn check_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    format!(
        "{}\n\
         systemctl is-active '{}' 2>/dev/null && echo 'active:{}' || echo 'inactive:{}'\n\
         systemctl is-enabled '{}' 2>/dev/null && echo 'enabled:{}' || echo 'disabled:{}'",
        SYSTEMD_GUARD, name, name, name, name, name, name
    )
}

/// Generate shell to converge service to desired state.
pub fn apply_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let state = resource.state.as_deref().unwrap_or("running");
    let enabled = resource.enabled.unwrap_or(true);

    let mut lines = vec!["set -euo pipefail".to_string(), SYSTEMD_GUARD.to_string()];

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
        "{}\n\
         echo \"active=$(systemctl is-active '{}' 2>/dev/null || echo 'unknown')\"\n\
         echo \"enabled=$(systemctl is-enabled '{}' 2>/dev/null || echo 'unknown')\"",
        SYSTEMD_GUARD, name, name
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
            version: None,
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
            gpu_backend: None,
            driver_version: None,
            cuda_version: None,
            rocm_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
            pre_apply: None,
            post_apply: None,
        lifecycle: None,
        }
    }

    #[test]
    fn test_fj008_check_service() {
        let r = make_service_resource("nfs-kernel-server", "running");
        let script = check_script(&r);
        assert!(script.contains("systemctl is-active 'nfs-kernel-server'"));
        assert!(script.contains("systemctl is-enabled 'nfs-kernel-server'"));
        assert!(
            script.contains("command -v systemctl"),
            "must include systemd guard"
        );
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

    #[test]
    fn test_fj008_state_query_script() {
        let r = make_service_resource("nginx", "running");
        let script = state_query_script(&r);
        assert!(script.contains("systemctl is-active 'nginx'"));
        assert!(script.contains("systemctl is-enabled 'nginx'"));
        assert!(
            script.contains("command -v systemctl"),
            "must include systemd guard"
        );
    }

    /// FJ-081: Verify all service scripts include systemd detection guard.
    #[test]
    fn test_fj081_systemd_guard_in_all_scripts() {
        let r = make_service_resource("test-svc", "running");

        let check = check_script(&r);
        assert!(check.contains("FORJAR_WARN: systemctl not found"));
        assert!(check.contains("exit 0"));

        let apply = apply_script(&r);
        assert!(apply.contains("FORJAR_WARN: systemctl not found"));
        assert!(apply.contains("exit 0"));

        let query = state_query_script(&r);
        assert!(query.contains("FORJAR_WARN: systemctl not found"));
        assert!(query.contains("exit 0"));
    }

    #[test]
    fn test_fj008_apply_pipefail() {
        let r = make_service_resource("nginx", "running");
        let script = apply_script(&r);
        assert!(
            script.starts_with("set -euo pipefail"),
            "service apply must start with pipefail"
        );
    }

    #[test]
    fn test_fj008_apply_idempotent_start() {
        // start should check is-active first (conditional)
        let r = make_service_resource("nginx", "running");
        let script = apply_script(&r);
        assert!(script.contains("if ! systemctl is-active --quiet"));
        assert!(script.contains("systemctl start 'nginx'"));
    }

    #[test]
    fn test_fj008_apply_idempotent_stop() {
        // stop should check is-active first (conditional)
        let r = make_service_resource("nginx", "stopped");
        let script = apply_script(&r);
        assert!(script.contains("if systemctl is-active --quiet"));
        assert!(script.contains("systemctl stop 'nginx'"));
    }

    #[test]
    fn test_fj008_apply_idempotent_enable() {
        // enable should check is-enabled first
        let r = make_service_resource("nginx", "running");
        let script = apply_script(&r);
        assert!(script.contains("if ! systemctl is-enabled --quiet"));
        assert!(script.contains("systemctl enable 'nginx'"));
    }

    #[test]
    fn test_fj008_default_state_and_enabled() {
        // Default: state=running, enabled=true
        let mut r = make_service_resource("svc", "running");
        r.state = None;
        r.enabled = None;
        let script = apply_script(&r);
        assert!(
            script.contains("systemctl start"),
            "default state should be running"
        );
        assert!(
            script.contains("systemctl enable"),
            "default enabled should be true"
        );
    }

    #[test]
    fn test_fj008_stopped_and_disabled() {
        let mut r = make_service_resource("svc", "stopped");
        r.enabled = Some(false);
        let script = apply_script(&r);
        assert!(script.contains("systemctl stop"));
        assert!(script.contains("systemctl disable"));
        assert!(!script.contains("systemctl start"));
        assert!(!script.contains("systemctl enable 'svc'\nfi"));
    }

    // ── Edge-case tests (FJ-123) ─────────────────────────────────

    #[test]
    fn test_fj008_invalid_state_no_op() {
        // Unknown state like "restarted" hits the _ => {} branch — no start/stop emitted
        let r = make_service_resource("svc", "restarted");
        let script = apply_script(&r);
        assert!(!script.contains("systemctl start"));
        assert!(!script.contains("systemctl stop"));
        // But enabled/disabled still emitted
        assert!(script.contains("systemctl enable"));
    }

    #[test]
    fn test_fj008_restart_on_with_disabled() {
        // Service with restart_on + disabled: reload-or-restart still emitted
        let mut r = make_service_resource("svc", "running");
        r.enabled = Some(false);
        r.restart_on = vec!["config-file".to_string()];
        let script = apply_script(&r);
        assert!(script.contains("systemctl reload-or-restart 'svc'"));
        assert!(script.contains("systemctl disable"));
    }

    #[test]
    fn test_fj008_no_name_defaults_to_unknown() {
        let mut r = make_service_resource("placeholder", "running");
        r.name = None;
        let script = apply_script(&r);
        assert!(script.contains("systemctl start 'unknown'"));
        let check = check_script(&r);
        assert!(check.contains("is-active 'unknown'"));
        let query = state_query_script(&r);
        assert!(query.contains("is-active 'unknown'"));
    }

    #[test]
    fn test_fj008_multiple_restart_on_single_reload() {
        // Multiple restart_on deps should still produce only one reload-or-restart
        let mut r = make_service_resource("app", "running");
        r.restart_on = vec!["cfg1".to_string(), "cfg2".to_string()];
        let script = apply_script(&r);
        let count = script.matches("systemctl reload-or-restart").count();
        assert_eq!(count, 1, "should emit exactly one reload-or-restart");
    }

    // --- FJ-132: Service edge case tests ---

    #[test]
    fn test_fj132_check_script_idempotent() {
        // check_script output should be identical across calls
        let r = make_service_resource("nginx", "running");
        let s1 = check_script(&r);
        let s2 = check_script(&r);
        assert_eq!(s1, s2, "check_script must be idempotent");
    }

    #[test]
    fn test_fj132_state_query_captures_active_and_enabled() {
        // state_query_script should capture both active and enabled state
        let r = make_service_resource("nginx", "running");
        let script = state_query_script(&r);
        assert!(
            script.contains("systemctl is-active 'nginx'"),
            "state_query should check is-active"
        );
        assert!(
            script.contains("systemctl is-enabled 'nginx'"),
            "state_query should check is-enabled"
        );
    }

    #[test]
    fn test_fj132_apply_running_enabled_explicit() {
        // Explicitly verify the golden path: running + enabled=true
        let mut r = make_service_resource("app", "running");
        r.enabled = Some(true);
        let script = apply_script(&r);
        assert!(script.contains("systemctl start 'app'"));
        assert!(script.contains("systemctl enable 'app'"));
        assert!(!script.contains("systemctl stop"));
        assert!(!script.contains("systemctl disable"));
    }

    #[test]
    fn test_fj132_restart_on_stopped_service() {
        // restart_on should still emit reload-or-restart even when state=stopped
        let mut r = make_service_resource("worker", "stopped");
        r.restart_on = vec!["config".to_string()];
        let script = apply_script(&r);
        assert!(
            script.contains("systemctl reload-or-restart 'worker'"),
            "restart_on should trigger even for stopped services"
        );
        assert!(script.contains("systemctl stop"));
    }

    // ── FJ-036: Additional service tests ─────────────────────────────

    #[test]
    fn test_fj036_service_apply_enable_true() {
        // enabled=true must generate `systemctl enable`
        let mut r = make_service_resource("caddy", "running");
        r.enabled = Some(true);
        let script = apply_script(&r);
        assert!(
            script.contains("systemctl enable 'caddy'"),
            "enabled=true must emit systemctl enable"
        );
        assert!(
            !script.contains("systemctl disable"),
            "enabled=true must not emit disable"
        );
    }

    #[test]
    fn test_fj036_service_apply_stopped() {
        // state="stopped" must generate `systemctl stop`
        let r = make_service_resource("redis", "stopped");
        let script = apply_script(&r);
        assert!(
            script.contains("systemctl stop 'redis'"),
            "state=stopped must emit systemctl stop with service name"
        );
        assert!(
            !script.contains("systemctl start"),
            "state=stopped must not emit start"
        );
    }

    #[test]
    fn test_fj036_service_apply_restart_on() {
        // restart_on deps should cause reload-or-restart to appear
        let mut r = make_service_resource("myapp", "running");
        r.restart_on = vec!["etc-myapp-conf".to_string(), "myapp-env".to_string()];
        let script = apply_script(&r);
        assert!(
            script.contains("systemctl reload-or-restart 'myapp'"),
            "restart_on with deps must trigger reload-or-restart"
        );
    }

    #[test]
    fn test_fj153_stopped_disabled_no_restart() {
        let mut r = make_service_resource("old-app", "stopped");
        r.enabled = Some(false);
        r.restart_on = vec![];
        let script = apply_script(&r);
        assert!(script.contains("systemctl stop 'old-app'"));
        assert!(script.contains("systemctl disable 'old-app'"));
        assert!(!script.contains("systemctl start"));
        assert!(!script.contains("systemctl enable 'old-app'\nfi"));
        assert!(!script.contains("reload-or-restart"));
    }

    #[test]
    fn test_fj153_unknown_state_still_enables() {
        let mut r = make_service_resource("custom", "custom-state");
        r.enabled = Some(true);
        let script = apply_script(&r);
        assert!(!script.contains("systemctl start"));
        assert!(!script.contains("systemctl stop"));
        assert!(script.contains("systemctl enable 'custom'"));
    }

    #[test]
    fn test_fj153_unknown_state_disables() {
        let mut r = make_service_resource("custom", "custom-state");
        r.enabled = Some(false);
        let script = apply_script(&r);
        assert!(script.contains("systemctl disable 'custom'"));
    }

    #[test]
    fn test_fj153_running_disabled_restart_on() {
        let mut r = make_service_resource("worker", "running");
        r.enabled = Some(false);
        r.restart_on = vec!["config".to_string(), "env".to_string()];
        let script = apply_script(&r);
        assert!(script.contains("systemctl start 'worker'"));
        assert!(script.contains("systemctl disable 'worker'"));
        assert!(script.contains("systemctl reload-or-restart 'worker'"));
    }

    #[test]
    fn test_fj036_service_state_query_active() {
        // state_query must check is-active for the named service
        let r = make_service_resource("postgresql", "running");
        let script = state_query_script(&r);
        assert!(
            script.contains("systemctl is-active 'postgresql'"),
            "state_query must check is-active for the service"
        );
        assert!(
            script.contains("active="),
            "state_query must capture active state into variable"
        );
    }
}
