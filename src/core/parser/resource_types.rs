//! Type-specific required-field validation for each resource type.

use super::disk_budget_validate::validate_disk_budget;
use super::*;

/// Validate type-specific required fields for a resource.
pub(super) fn validate_resource_type(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    match resource.resource_type {
        ResourceType::Package => validate_package(id, resource, errors),
        ResourceType::File => validate_file(id, resource, errors),
        ResourceType::Service => validate_service(id, resource, errors),
        ResourceType::Mount => validate_mount(id, resource, errors),
        ResourceType::User => validate_user(id, resource, errors),
        ResourceType::Docker => validate_docker(id, resource, errors),
        ResourceType::Cron => validate_cron(id, resource, errors),
        ResourceType::Network => validate_network(id, resource, errors),
        ResourceType::Pepita => validate_pepita(id, resource, errors),
        ResourceType::Model => validate_model(id, resource, errors),
        ResourceType::Gpu => validate_gpu(id, resource, errors),
        ResourceType::Recipe => validate_recipe(id, resource, errors),
        ResourceType::Task => validate_task(id, resource, errors),
        ResourceType::WasmBundle | ResourceType::Image => validate_file(id, resource, errors),
        ResourceType::Build => validate_build(id, resource, errors),
        ResourceType::GithubRelease => validate_github_release(id, resource, errors),
        ResourceType::OverlayInterface => validate_overlay_interface(id, resource, errors),
        ResourceType::DiskBudget => validate_disk_budget(id, resource, errors),
    }
}

fn validate_package(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.packages.is_empty() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (package) has no packages"),
        });
    }
    if resource.provider.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (package) has no provider"),
        });
    }
    // FJ-48: Warn when linux-image-* is installed without matching linux-headers-*
    check_kernel_headers(id, resource, errors);
}

/// Warn when a kernel image package is listed without matching headers.
/// Without headers, DKMS modules (e.g. NVIDIA driver) fail to rebuild.
fn check_kernel_headers(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    for pkg in &resource.packages {
        let suffix = if let Some(s) = pkg.strip_prefix("linux-image-") {
            s
        } else {
            continue;
        };
        let expected_headers = format!("linux-headers-{suffix}");
        if !resource.packages.iter().any(|p| p == &expected_headers) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' installs '{pkg}' without '{expected_headers}' — DKMS modules will fail to build"
                ),
            });
        }
    }
}

fn validate_file(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.path.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (file) has no path"),
        });
    }
    if resource.content.is_some() && resource.source.is_some() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (file) has both content and source (pick one)"),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["file", "directory", "symlink", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (file) has invalid state '{}' (expected: {})",
                    id,
                    state,
                    valid.join(", ")
                ),
            });
        }
    }
    if resource.state.as_deref() == Some("symlink") && resource.target.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (file) state=symlink requires a target"),
        });
    }
}

fn validate_service(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (service) has no name"),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["running", "stopped", "enabled", "disabled"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (service) has invalid state '{}' (expected: {})",
                    id,
                    state,
                    valid.join(", ")
                ),
            });
        }
    }
}

fn validate_mount(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.source.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (mount) has no source"),
        });
    }
    if resource.path.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (mount) has no path"),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["mounted", "unmounted", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (mount) has invalid state '{}' (expected: {})",
                    id,
                    state,
                    valid.join(", ")
                ),
            });
        }
    }
}

fn validate_user(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (user) has no name"),
        });
    }
}

fn validate_docker(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (docker) has no name"),
        });
    }
    if resource.image.is_none() && resource.state.as_deref() != Some("absent") {
        errors.push(ValidationError {
            message: format!("resource '{id}' (docker) has no image"),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["running", "stopped", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' (docker) has invalid state '{}' (expected: {})",
                    id,
                    state,
                    valid.join(", ")
                ),
            });
        }
    }
}

fn validate_cron(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (cron) has no name"),
        });
    }
    if resource.schedule.is_none() && resource.state.as_deref() != Some("absent") {
        errors.push(ValidationError {
            message: format!("resource '{id}' (cron) has no schedule"),
        });
    }
    if let Some(ref sched) = resource.schedule {
        // Skip templates and cron keywords (@daily, @weekly, etc.)
        let is_keyword = sched.starts_with('@');
        let is_template = sched.contains("{{");
        if !is_keyword && !is_template {
            let fields: Vec<&str> = sched.split_whitespace().collect();
            if fields.len() != 5 {
                errors.push(ValidationError {
                    message: format!(
                        "resource '{id}' (cron) schedule '{sched}' must have exactly 5 fields (min hour dom mon dow)"
                    ),
                });
            }
        }
    }
    if resource.command.is_none() && resource.state.as_deref() != Some("absent") {
        errors.push(ValidationError {
            message: format!("resource '{id}' (cron) has no command"),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["present", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (cron) has invalid state '{state}' (expected: present, absent)"
                ),
            });
        }
    }
}

fn validate_network(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.port.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (network) has no port"),
        });
    }
    if let Some(ref proto) = resource.protocol {
        let valid = ["tcp", "udp"];
        if !valid.contains(&proto.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (network) has invalid protocol '{proto}' (expected: tcp, udp)"
                ),
            });
        }
    }
    if let Some(ref action) = resource.action {
        let valid = ["allow", "deny", "reject"];
        if !valid.contains(&action.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (network) has invalid action '{action}' (expected: allow, deny, reject)"
                ),
            });
        }
    }
}

fn validate_pepita(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (pepita) has no name"),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["present", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (pepita) has invalid state '{state}' (expected: present, absent)"
                ),
            });
        }
    }
    if resource.overlay_merged.is_some()
        && resource.overlay_lower.is_none()
        && resource.overlay_upper.is_none()
    {
        // overlay_merged without explicit dirs uses defaults -- valid but warn-worthy
    }
    if let Some(ref cpuset) = resource.cpuset {
        if cpuset.is_empty() {
            errors.push(ValidationError {
                message: format!("resource '{id}' (pepita) has empty cpuset"),
            });
        }
    }
}

fn validate_model(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.name.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (model) has no name"),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["present", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (model) has invalid state '{state}' (expected: present, absent)"
                ),
            });
        }
    }
}

fn validate_gpu(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.driver_version.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (gpu) has no driver_version"),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["present", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (gpu) has invalid state '{state}' (expected: present, absent)"
                ),
            });
        }
    }
}

fn validate_recipe(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.recipe.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (recipe) has no recipe name"),
        });
    }
}

fn validate_task(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    // FJ-2700: Pipeline tasks use stages instead of command
    let is_pipeline = resource
        .task_mode
        .as_ref()
        .is_some_and(|m| *m == crate::core::types::TaskMode::Pipeline);
    // FJ-2725: a phony target need not have a command. make's `all:
    // $(BUILD)/app` is a grouping node — prerequisites, no recipe — and a name
    // listed in `.PHONY` with no rule at all is legal too: `make deny` simply
    // prints "Nothing to be done". forjar's own Makefile has exactly that
    // (a stale `.PHONY` entry), which is how this was found. Inventing a no-op
    // command to satisfy this rule would be less honest than allowing it.
    if resource.command.is_none() && !is_pipeline && !resource.phony {
        errors.push(ValidationError {
            message: format!("resource '{id}' (task) has no command"),
        });
    }
    if is_pipeline && resource.stages.is_empty() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (task pipeline) has no stages"),
        });
    }
    if let Some(ref timeout) = resource.timeout {
        if *timeout == 0 {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (task) has timeout of 0 (use no timeout or a positive value)"
                ),
            });
        }
    }
}

fn validate_build(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.build_machine.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (build) has no build_machine — specify which machine performs the build"),
        });
    }
    if resource.command.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (build) has no command — specify the build command"),
        });
    }
    if resource.source.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (build) has no source — specify the artifact path on the build machine"),
        });
    }
    if resource.target.is_none() {
        errors.push(ValidationError {
            message: format!(
                "resource '{id}' (build) has no target — specify where to deploy the artifact"
            ),
        });
    }
}

fn validate_overlay_interface(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
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

fn validate_github_release(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if resource.repo.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (github_release) has no repo — specify owner/repo (e.g., paiml/forjar)"),
        });
    }
    if resource.binary.is_none() {
        errors.push(ValidationError {
            message: format!("resource '{id}' (github_release) has no binary — specify the binary name to install"),
        });
    }
    if let Some(ref state) = resource.state {
        let valid = ["present", "absent"];
        if !valid.contains(&state.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (github_release) has invalid state '{state}' (expected: present, absent)"
                ),
            });
        }
    }
}
