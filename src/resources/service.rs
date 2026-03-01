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

