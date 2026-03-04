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
  echo 'FORJAR_WARN: systemctl not found - skipping service resource (no systemd)'\n  \
  exit 0\n\
fi";

/// Generate shell to check service state.
pub fn check_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    format!(
        "{SYSTEMD_GUARD}\n\
         systemctl is-active '{name}' 2>/dev/null && echo 'active:{name}' || echo 'inactive:{name}'\n\
         systemctl is-enabled '{name}' 2>/dev/null && echo 'enabled:{name}' || echo 'disabled:{name}'"
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
                "if ! systemctl is-active --quiet '{name}'; then\n  systemctl start '{name}'\nfi"
            ));
        }
        "stopped" => {
            lines.push(format!(
                "if systemctl is-active --quiet '{name}'; then\n  systemctl stop '{name}'\nfi"
            ));
        }
        _ => {}
    }

    if enabled {
        lines.push(format!(
            "if ! systemctl is-enabled --quiet '{name}'; then\n  systemctl enable '{name}'\nfi"
        ));
    } else {
        lines.push(format!(
            "if systemctl is-enabled --quiet '{name}'; then\n  systemctl disable '{name}'\nfi"
        ));
    }

    // Reload if needed (after config changes)
    if !resource.restart_on.is_empty() {
        lines.push(format!("systemctl reload-or-restart '{name}'"));
    }

    lines.join("\n")
}

/// Generate shell to query service state (for hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    format!(
        "{SYSTEMD_GUARD}\n\
         echo \"active=$(systemctl is-active '{name}' 2>/dev/null || echo 'unknown')\"\n\
         echo \"enabled=$(systemctl is-enabled '{name}' 2>/dev/null || echo 'unknown')\""
    )
}
