//! FJ-008/081: Systemd service resource handler.
//!
//! Generates shell scripts for systemd service management.
//! Includes runtime systemd detection (FJ-081) — gracefully skips
//! when systemctl is unavailable (e.g. inside containers without systemd).

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;
use crate::resources::verdict;

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
    let n = sh_squote(name);
    // The check must assert the state `apply_script` converges TO, not a fixed
    // "active and enabled". A service declared `state: stopped` is converged
    // when it is NOT running; asserting the opposite would turn a correct host
    // into a permanent check failure.
    let state = resource.state.as_deref().unwrap_or("running");
    let enabled = resource.enabled.unwrap_or(true);

    let active = if state == "stopped" {
        verdict::assert_that(
            &format!("! systemctl is-active --quiet {n} 2>/dev/null"),
            &format!("inactive:{name}"),
            &format!("active:{name}"),
        )
    } else {
        verdict::assert_that(
            &format!("systemctl is-active --quiet {n} 2>/dev/null"),
            &format!("active:{name}"),
            &format!("inactive:{name}"),
        )
    };

    let enablement = if enabled {
        verdict::assert_that(
            &format!("systemctl is-enabled --quiet {n} 2>/dev/null"),
            &format!("enabled:{name}"),
            &format!("disabled:{name}"),
        )
    } else {
        verdict::assert_that(
            &format!("! systemctl is-enabled --quiet {n} 2>/dev/null"),
            &format!("disabled:{name}"),
            &format!("enabled:{name}"),
        )
    };

    // SYSTEMD_GUARD still exits 0 when systemd is absent. That is a genuine
    // "not applicable on this host" skip (FJ-081, containers without systemd),
    // not a claim of convergence, and removing it would fail every service
    // resource inside a container.
    format!(
        "{SYSTEMD_GUARD}\n{}",
        verdict::check_script_from(&[active, enablement])
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
