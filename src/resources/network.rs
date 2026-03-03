//! FJ-032: Network/firewall resource handler.
//!
//! Manages firewall rules via ufw (Uncomplicated Firewall).
//! PMAT-038: Includes ufw availability guard — gracefully skips in
//! environments without ufw (e.g. Docker containers).

use crate::core::types::Resource;

/// Shell guard that detects ufw availability.
/// If ufw is not found, prints a warning and exits 0 (skip).
const UFW_GUARD: &str = "\
if ! command -v ufw >/dev/null 2>&1; then\n  \
  echo 'FORJAR_WARN: ufw not found - skipping network resource (no firewall)'\n  \
  exit 0\n\
fi";

/// Generate shell script to check if a firewall rule exists.
pub fn check_script(resource: &Resource) -> String {
    let port = resource.port.as_deref().unwrap_or("0");
    let protocol = resource.protocol.as_deref().unwrap_or("tcp");
    let action = resource.action.as_deref().unwrap_or("allow");
    format!(
        "{}\nufw status numbered 2>/dev/null | grep -q '{}.*{}/{}' && echo 'exists:{}' || echo 'missing:{}'",
        UFW_GUARD, action, port, protocol, port, port
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
        UFW_GUARD.to_string(),
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
        "{}\nufw status verbose 2>/dev/null | grep '{}' || echo 'rule=MISSING:{}'",
        UFW_GUARD, port, port
    )
}
