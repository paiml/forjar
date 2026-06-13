//! FJ-032: Network/firewall resource handler.
//!
//! Manages firewall rules via ufw (Uncomplicated Firewall).
//! PMAT-038: Includes ufw availability guard — gracefully skips in
//! environments without ufw (e.g. Docker containers).

use crate::core::shell_escape::{is_valid_ufw_action, sh_squote};
use crate::core::types::Resource;

/// Shell guard that detects ufw availability.
/// If ufw is not found, prints a warning and exits 0 (skip).
const UFW_GUARD: &str = "\
if ! command -v ufw >/dev/null 2>&1; then\n  \
  echo 'FORJAR_WARN: ufw not found - skipping network resource (no firewall)'\n  \
  exit 0\n\
fi";

/// FJ-154: resolve the ufw action, falling back to `allow` for any value that
/// is not one of the fixed ufw verbs. This lets the action be interpolated as
/// a bare (unquoted) token without risking command injection.
fn safe_action(resource: &Resource) -> &str {
    let action = resource.action.as_deref().unwrap_or("allow");
    if is_valid_ufw_action(action) {
        action
    } else {
        "allow"
    }
}

/// Build the `to any port '<port>' proto '<proto>'` rule fragment, with the
/// optional `from '<addr>'` prefix. All data fields are shell-escaped.
fn rule_parts(resource: &Resource) -> String {
    let port = resource.port.as_deref().unwrap_or("0");
    let protocol = resource.protocol.as_deref().unwrap_or("tcp");
    let mut parts = vec![];
    if let Some(ref from) = resource.from_addr {
        parts.push(format!("from {}", sh_squote(from)));
    }
    parts.push(format!(
        "to any port {} proto {}",
        sh_squote(port),
        sh_squote(protocol)
    ));
    parts.join(" ")
}

/// Generate shell script to check if a firewall rule exists.
pub fn check_script(resource: &Resource) -> String {
    let port = resource.port.as_deref().unwrap_or("0");
    let protocol = resource.protocol.as_deref().unwrap_or("tcp");
    let action = safe_action(resource);
    let grep = sh_squote(&format!("{action}.*{port}/{protocol}"));
    format!(
        "{UFW_GUARD}\nufw status numbered 2>/dev/null | grep -q {grep} && echo {} || echo {}",
        sh_squote(&format!("exists:{port}")),
        sh_squote(&format!("missing:{port}"))
    )
}

/// Generate shell script to add/remove a firewall rule.
pub fn apply_script(resource: &Resource) -> String {
    let action = safe_action(resource);
    let state = resource.state.as_deref().unwrap_or("present");
    let parts = rule_parts(resource);

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
            lines.push(format!("$SUDO ufw delete {action} {parts} || true"));
        }
        _ => {
            if let Some(ref comment) = resource.name {
                lines.push(format!(
                    "$SUDO ufw {action} {parts} comment {}",
                    sh_squote(comment)
                ));
            } else {
                lines.push(format!("$SUDO ufw {action} {parts}"));
            }
        }
    }

    lines.join("\n")
}

/// Generate shell to query firewall state (for BLAKE3 hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let port = resource.port.as_deref().unwrap_or("0");
    format!(
        "{UFW_GUARD}\nufw status verbose 2>/dev/null | grep {} || echo {}",
        sh_squote(port),
        sh_squote(&format!("rule=MISSING:{port}"))
    )
}

#[cfg(test)]
mod fj154_tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn net_resource() -> Resource {
        Resource {
            resource_type: ResourceType::Network,
            machine: MachineTarget::Single("m1".to_string()),
            port: Some("443".to_string()),
            protocol: Some("tcp".to_string()),
            action: Some("allow".to_string()),
            from_addr: Some("10.0.0.0/8".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn fj154_injected_action_falls_back_to_allow() {
        // Defect #15: `allow; reboot #` is not a ufw verb → forced to `allow`.
        let mut r = net_resource();
        r.action = Some("allow; reboot #".to_string());
        let script = apply_script(&r);
        assert!(script.contains("$SUDO ufw allow "), "{script}");
        assert!(!script.contains("reboot"), "{script}");
    }

    #[test]
    fn fj154_from_addr_quote_neutralized() {
        let mut r = net_resource();
        r.from_addr = Some("10.0.0.1';reboot;'".to_string());
        let script = apply_script(&r);
        assert!(script.contains("'\\''"), "{script}");
        assert!(!script.contains("from '10.0.0.1';reboot"), "{script}");
    }

    #[test]
    fn fj154_comment_quote_neutralized() {
        let mut r = net_resource();
        r.name = Some("c';reboot;'".to_string());
        let script = apply_script(&r);
        assert!(script.contains("'\\''"), "{script}");
        assert!(!script.contains("comment 'c';reboot"), "{script}");
    }

    #[test]
    fn fj154_benign_unchanged() {
        let r = net_resource();
        let script = apply_script(&r);
        assert!(
            script.contains("$SUDO ufw allow from '10.0.0.0/8' to any port '443' proto 'tcp'"),
            "{script}"
        );
        assert!(check_script(&r).contains("exists:443"));
        assert!(state_query_script(&r).contains("rule=MISSING:443"));
    }

    #[test]
    fn fj154_valid_verbs_preserved() {
        for verb in ["allow", "deny", "reject", "limit"] {
            let mut r = net_resource();
            r.action = Some(verb.to_string());
            let script = apply_script(&r);
            assert!(script.contains(&format!("$SUDO ufw {verb} ")), "{script}");
        }
    }
}
