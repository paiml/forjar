//! FJ-035: `overlay_interface` declaration validation.
//!
//! Split out of `resource_types.rs` to match the shape the newer types already
//! use (`backup_sync_validate`, `disk_budget_validate`, `nas_archive_validate`):
//! that file is a dispatch table, and a type's own rules belong with the type.

use super::*;

pub(super) fn validate_overlay_interface(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    use crate::core::shell_escape::{is_valid_iface, is_valid_overlay_ip};
    match resource.overlay_ip.as_deref() {
        None => errors.push(ValidationError {
            message: format!(
                "resource '{id}' (overlay_interface) has no overlay_ip — specify an IPv4/CIDR (e.g., 10.42.0.11/24)"
            ),
        }),
        Some(ip) if !ip.contains("{{") && !is_valid_overlay_ip(ip) => {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (overlay_interface) overlay_ip '{ip}' is not a valid IPv4/CIDR (e.g., 10.42.0.11/24)"
                ),
            });
        }
        _ => {}
    }
    if let Some(iface) = resource.overlay_iface.as_deref() {
        if !iface.contains("{{") && !is_valid_iface(iface) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (overlay_interface) interface '{iface}' is not a valid network interface name"
                ),
            });
        }
    }
    if let Some(ref state) = resource.state {
        let valid = ["present", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (overlay_interface) has invalid state '{state}' (expected: present, absent)"
                ),
            });
        }
    }
}

/// Cross-resource invariant: overlay_ip is INJECTIVE across all present
/// `overlay_interface` resources targeting the same machine.
///
/// QUORUM MUST-1 (plan/validate leg of the RFC 5227 defense): two
/// `overlay_interface` resources that claim the same overlay address would make
/// the per-host self-heal loops fight over one IP. The runtime `arping -D` probe
/// is the last line of defense; this is the FIRST — it rejects the conflict at
/// `forjar plan`/`validate` time, naming BOTH resource ids, before anything is
/// applied. The conflict key is the bound address (CIDR prefix stripped), since
/// `10.42.0.11/24` and `10.42.0.11/16` collide on the same address. `absent`
/// resources are excluded (they are being torn down, not claiming the address).
pub(super) fn validate_overlay_ip_injective(
    config: &ForjarConfig,
    errors: &mut Vec<ValidationError>,
) {
    use std::collections::BTreeMap;
    // (machine, address) -> first resource id that claimed it.
    let mut seen: BTreeMap<(String, String), String> = BTreeMap::new();
    // Deterministic iteration so the *first* claimant (and thus the error
    // wording) is stable regardless of HashMap ordering.
    let mut ids: Vec<&String> = config.resources.keys().collect();
    ids.sort();
    for id in ids {
        let resource = &config.resources[id];
        if resource.resource_type != ResourceType::OverlayInterface {
            continue;
        }
        if resource.state.as_deref() == Some("absent") {
            continue;
        }
        let Some(ip_cidr) = resource.overlay_ip.as_deref() else {
            continue;
        };
        // Skip unresolved templates and malformed IPs (reported elsewhere).
        if ip_cidr.contains("{{") {
            continue;
        }
        let addr = ip_cidr.split('/').next().unwrap_or(ip_cidr).to_string();
        for machine in resource.machine.iter() {
            let key = (machine.to_string(), addr.clone());
            if let Some(prev) = seen.get(&key) {
                errors.push(ValidationError {
                    message: format!(
                        "overlay_interface resources '{prev}' and '{id}' both claim overlay_ip {addr} on machine '{machine}' — overlay addresses must be injective (no two resources may share one IP, or their self-heal loops will fight over it)"
                    ),
                });
            } else {
                seen.insert(key, id.clone());
            }
        }
    }
}
