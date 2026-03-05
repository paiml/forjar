//! FJ-005: Script generation — dispatch to resource handlers.
//! FJ-036: bashrs purification pipeline integrated (Invariant I8).
//!
//! Each resource type produces three scripts:
//! - check: read current state
//! - apply: converge to desired state
//! - state_query: query observable state for BLAKE3 hashing
//!
//! All scripts can be validated/purified via `core::purifier`.

use crate::core::types::{Resource, ResourceType};
use crate::resources;
use provable_contracts_macros::contract;

/// Generate a check script for a resource.
#[contract("codegen-dispatch-v1", equation = "check_script")]
pub fn check_script(resource: &Resource) -> Result<String, String> {
    match &resource.resource_type {
        ResourceType::Package => Ok(resources::package::check_script(resource)),
        ResourceType::File => Ok(resources::file::check_script(resource)),
        ResourceType::Service => Ok(resources::service::check_script(resource)),
        ResourceType::Mount => Ok(resources::mount::check_script(resource)),
        ResourceType::User => Ok(resources::user::check_script(resource)),
        ResourceType::Docker => Ok(resources::docker::check_script(resource)),
        ResourceType::Cron => Ok(resources::cron::check_script(resource)),
        ResourceType::Network => Ok(resources::network::check_script(resource)),
        ResourceType::Pepita => Ok(resources::pepita::check_script(resource)),
        ResourceType::Model => Ok(resources::model::check_script(resource)),
        ResourceType::Gpu => Ok(resources::gpu::check_script(resource)),
        ResourceType::Task => Ok(resources::task::check_script(resource)),
        ResourceType::WasmBundle | ResourceType::Image => Ok(resources::file::check_script(resource)),
        ResourceType::Recipe => {
            Err("codegen not implemented for recipe (expand first)".to_string())
        }
    }
}

/// Generate an apply script for a resource.
///
/// FJ-1394: If `resource.sudo` is true, wraps the entire script in a sudo
/// heredoc so all commands run with elevated privileges.
#[contract("codegen-dispatch-v1", equation = "apply_script")]
pub fn apply_script(resource: &Resource) -> Result<String, String> {
    let script = match &resource.resource_type {
        ResourceType::Package => Ok(resources::package::apply_script(resource)),
        ResourceType::File => Ok(resources::file::apply_script(resource)),
        ResourceType::Service => Ok(resources::service::apply_script(resource)),
        ResourceType::Mount => Ok(resources::mount::apply_script(resource)),
        ResourceType::User => Ok(resources::user::apply_script(resource)),
        ResourceType::Docker => Ok(resources::docker::apply_script(resource)),
        ResourceType::Cron => Ok(resources::cron::apply_script(resource)),
        ResourceType::Network => Ok(resources::network::apply_script(resource)),
        ResourceType::Pepita => Ok(resources::pepita::apply_script(resource)),
        ResourceType::Model => Ok(resources::model::apply_script(resource)),
        ResourceType::Gpu => Ok(resources::gpu::apply_script(resource)),
        ResourceType::Task => Ok(resources::task::apply_script(resource)),
        ResourceType::WasmBundle | ResourceType::Image => Ok(resources::file::apply_script(resource)),
        ResourceType::Recipe => {
            Err("codegen not implemented for recipe (expand first)".to_string())
        }
    }?;
    Ok(sudo_wrap(resource, script))
}

/// FJ-1394 / FJ-29: Wrap script with sudo if the resource has `sudo: true`.
///
/// Uses heredoc to pass the script to sudo bash — avoids single-quote escaping
/// that triggers bashrs SC2075 false positives.
fn sudo_wrap(resource: &Resource, script: String) -> String {
    if !resource.sudo {
        return script;
    }
    // Wrap: if already root, run as-is; otherwise elevate via sudo bash with heredoc
    format!(
        "if [ \"$(id -u)\" -eq 0 ]; then\n{script}\nelse\nsudo bash <<'FORJAR_SUDO'\n{script}\nFORJAR_SUDO\nfi"
    )
}

/// Generate a state query script for a resource.
#[contract("codegen-dispatch-v1", equation = "state_query_script")]
pub fn state_query_script(resource: &Resource) -> Result<String, String> {
    match &resource.resource_type {
        ResourceType::Package => Ok(resources::package::state_query_script(resource)),
        ResourceType::File => Ok(resources::file::state_query_script(resource)),
        ResourceType::Service => Ok(resources::service::state_query_script(resource)),
        ResourceType::Mount => Ok(resources::mount::state_query_script(resource)),
        ResourceType::User => Ok(resources::user::state_query_script(resource)),
        ResourceType::Docker => Ok(resources::docker::state_query_script(resource)),
        ResourceType::Cron => Ok(resources::cron::state_query_script(resource)),
        ResourceType::Network => Ok(resources::network::state_query_script(resource)),
        ResourceType::Pepita => Ok(resources::pepita::state_query_script(resource)),
        ResourceType::Model => Ok(resources::model::state_query_script(resource)),
        ResourceType::Gpu => Ok(resources::gpu::state_query_script(resource)),
        ResourceType::Task => Ok(resources::task::state_query_script(resource)),
        ResourceType::WasmBundle | ResourceType::Image => Ok(resources::file::state_query_script(resource)),
        ResourceType::Recipe => {
            Err("codegen not implemented for recipe (expand first)".to_string())
        }
    }
}
