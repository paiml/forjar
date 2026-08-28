//! FJ-005: Script generation — dispatch to resource handlers.
//! FJ-036: bashrs purification pipeline integrated (Invariant I8).
//!
//! Each resource type produces three scripts:
//! - check: read current state
//! - apply: converge to desired state
//! - state_query: query observable state for BLAKE3 hashing
//!
//! All scripts can be validated/purified via `core::purifier`.
//!
//! # Why one table instead of three matches
//!
//! `check_script`, `apply_script` and `state_query_script` used to hold three
//! parallel 21-arm matches over `ResourceType`. codegen-dispatch-v1's symmetry
//! obligation — "all three functions handle the same set of types" — was then a
//! property three separate lists had to keep agreeing on, checked only by
//! FALSIFY-CD-002 at test time. Routing each type ONCE, to a row carrying all
//! three generators, makes that obligation structural: a type cannot be handled
//! by one function and missed by another, because there is one row.
//!
//! The privilege context was left outside that row and cost the same way:
//! `sudo: true` wrapped the apply and neither of the other two, so the check
//! asked a different question than the apply answered (#349). It is resolved
//! once now, in `in_declared_privilege_context`, for all three.

use crate::core::types::{Resource, ResourceType};
use crate::resources;
use provable_contracts_macros::contract;

/// The three generators a resource type routes to.
#[derive(Clone, Copy)]
struct ScriptHandlers {
    /// Read current state.
    check: fn(&Resource) -> String,
    /// Converge to desired state.
    apply: fn(&Resource) -> String,
    /// Query observable state for hashing.
    state_query: fn(&Resource) -> String,
}

/// Row for a resource type whose module exposes all three generators.
macro_rules! from_module {
    ($($seg:ident)::+) => {
        ScriptHandlers {
            check: $($seg)::+::check_script,
            apply: $($seg)::+::apply_script,
            state_query: $($seg)::+::state_query_script,
        }
    };
}

/// Returned for a type with no codegen — the same message all three entry
/// points returned before.
const NOT_DISPATCHABLE: &str = "codegen not implemented for recipe (expand first)";

/// Route a resource type to its generators, or `None` when it has no codegen.
///
/// The groups exist only to keep each `match` readable; every variant is named
/// in exactly one of them, so adding a `ResourceType` still fails to compile
/// until it is routed.
fn handlers(resource_type: &ResourceType) -> Option<ScriptHandlers> {
    match resource_type {
        ResourceType::Package
        | ResourceType::File
        | ResourceType::Service
        | ResourceType::Mount
        | ResourceType::User
        | ResourceType::Cron
        | ResourceType::Network => handlers_host(resource_type),

        ResourceType::Docker
        | ResourceType::Pepita
        | ResourceType::Model
        | ResourceType::Gpu
        | ResourceType::Task
        | ResourceType::WasmBundle
        | ResourceType::Image => handlers_workload(resource_type),

        ResourceType::Build
        | ResourceType::GithubRelease
        | ResourceType::OverlayInterface
        | ResourceType::DiskBudget
        | ResourceType::BackupSync
        | ResourceType::NasArchive => handlers_fleet(resource_type),

        // A recipe is expanded into concrete resources before codegen runs.
        ResourceType::Recipe => None,
    }
}

/// Host configuration types: packages, files, units, mounts, accounts, jobs,
/// firewall rules.
fn handlers_host(resource_type: &ResourceType) -> Option<ScriptHandlers> {
    let row = match resource_type {
        // `package` has no `check_script`; the read side lives in its own module.
        ResourceType::Package => ScriptHandlers {
            check: resources::package_check::check_script,
            apply: resources::package::apply_script,
            state_query: resources::package::state_query_script,
        },
        ResourceType::File => from_module!(resources::file),
        ResourceType::Service => from_module!(resources::service),
        ResourceType::Mount => from_module!(resources::mount),
        ResourceType::User => from_module!(resources::user),
        ResourceType::Cron => from_module!(resources::cron),
        ResourceType::Network => from_module!(resources::network),

        // Routed to a sibling group by `handlers`. Named rather than caught by
        // `_` so a new ResourceType is a compile error here too.
        ResourceType::Docker
        | ResourceType::Pepita
        | ResourceType::Model
        | ResourceType::Gpu
        | ResourceType::Task
        | ResourceType::WasmBundle
        | ResourceType::Image
        | ResourceType::Build
        | ResourceType::GithubRelease
        | ResourceType::OverlayInterface
        | ResourceType::DiskBudget
        | ResourceType::BackupSync
        | ResourceType::NasArchive
        | ResourceType::Recipe => return None,
    };
    Some(row)
}

/// Workload types: containers, sandboxes, accelerators, and the artifacts a run
/// produces.
fn handlers_workload(resource_type: &ResourceType) -> Option<ScriptHandlers> {
    let row = match resource_type {
        ResourceType::Docker => from_module!(resources::docker),
        ResourceType::Pepita => from_module!(resources::pepita),
        ResourceType::Model => from_module!(resources::model),
        ResourceType::Gpu => from_module!(resources::gpu),
        ResourceType::Task => from_module!(resources::task),
        ResourceType::WasmBundle => from_module!(resources::wasm_bundle),
        // An image is materialised as a file on the target.
        ResourceType::Image => from_module!(resources::file),

        // Routed to a sibling group by `handlers`.
        ResourceType::Package
        | ResourceType::File
        | ResourceType::Service
        | ResourceType::Mount
        | ResourceType::User
        | ResourceType::Cron
        | ResourceType::Network
        | ResourceType::Build
        | ResourceType::GithubRelease
        | ResourceType::OverlayInterface
        | ResourceType::DiskBudget
        | ResourceType::BackupSync
        | ResourceType::NasArchive
        | ResourceType::Recipe => return None,
    };
    Some(row)
}

/// Fleet lifecycle types (FJ-33 … FJ-38): build/deploy pipelines, release
/// installation, overlay networking, disk budget, backup and archival.
fn handlers_fleet(resource_type: &ResourceType) -> Option<ScriptHandlers> {
    let row = match resource_type {
        ResourceType::Build => from_module!(resources::build),
        ResourceType::GithubRelease => from_module!(resources::github_release),
        ResourceType::OverlayInterface => from_module!(resources::overlay_interface),
        ResourceType::DiskBudget => from_module!(resources::disk_budget),
        ResourceType::BackupSync => from_module!(resources::backup_sync),
        ResourceType::NasArchive => from_module!(resources::nas_archive),

        // Routed to a sibling group by `handlers`.
        ResourceType::Package
        | ResourceType::File
        | ResourceType::Service
        | ResourceType::Mount
        | ResourceType::User
        | ResourceType::Cron
        | ResourceType::Network
        | ResourceType::Docker
        | ResourceType::Pepita
        | ResourceType::Model
        | ResourceType::Gpu
        | ResourceType::Task
        | ResourceType::WasmBundle
        | ResourceType::Image
        | ResourceType::Recipe => return None,
    };
    Some(row)
}

/// Generate a check script for a resource.
///
/// Runs in the privilege context the resource declares — see
/// [`in_declared_privilege_context`].
#[contract("codegen-dispatch-v1", equation = "check_script")]
pub fn check_script(resource: &Resource) -> Result<String, String> {
    // Contract: codegen-dispatch-v1.yaml precondition (pv codegen)
    contract_pre_check_script!(resource);
    let row = handlers(&resource.resource_type).ok_or_else(|| NOT_DISPATCHABLE.to_string())?;
    Ok(in_declared_privilege_context(
        resource,
        (row.check)(resource),
    ))
}

/// Generate an apply script for a resource.
///
/// FJ-1394: If `resource.sudo` is true, wraps the entire script in a sudo
/// heredoc so all commands run with elevated privileges.
#[contract("codegen-dispatch-v1", equation = "apply_script")]
pub fn apply_script(resource: &Resource) -> Result<String, String> {
    // Contract: codegen-dispatch-v1.yaml precondition (pv codegen)
    contract_pre_apply_script!(resource);

    // A `{{secrets.*}}` that survives template resolution is a
    // credential-shaped PLACEHOLDER, not a credential. `resolve_or_fallback`
    // hands the unresolved resource back by design, so without this the literal
    // string is spliced into the generated script and written to the machine —
    // demonstrated with a file resource whose `content` became
    // `API_KEY={{secrets.some-token}}` on disk, behind only a stderr warning.
    //
    // `backup_sync` has guarded its own token since FJ-037; this generalises
    // that to every resource type, at the one place every apply script is born.
    // Failing HERE fails this resource only, leaving `policy.failure` to decide
    // the run — an unrelated unresolvable secret must not block a whole machine.
    if crate::core::resolver::has_unresolved_secret(resource) {
        return Err(format!(
            "resource carries an unresolved secret template — the secrets provider \
             did not supply it, and applying would write the placeholder as if it \
             were the credential (type: {})",
            resource.resource_type
        ));
    }

    // FJ-2722 (PMAT-199): `state: absent` must never RUN the thing.
    //
    // `destroy` converges every resource to `state: absent`. For a task, build
    // or wasm_bundle these handlers ignore `state` entirely, so `forjar destroy`
    // executed the task's command — running a build, a training job or a deploy
    // as its way of "removing" it, then reporting `- <id> (task)` as a success.
    // These types describe an ACTION, and an action has no absent form; the
    // artifacts they produce are removed by whatever file resource declares
    // them.
    if resource.state.as_deref() == Some("absent")
        && matches!(
            resource.resource_type,
            ResourceType::Task | ResourceType::Build | ResourceType::WasmBundle
        )
    {
        return Ok(format!(
            "echo 'forjar: {} resources have no absent form — nothing to remove'",
            resource.resource_type
        ));
    }

    let row = handlers(&resource.resource_type).ok_or_else(|| NOT_DISPATCHABLE.to_string())?;
    let script = (row.apply)(resource);
    Ok(in_declared_privilege_context(resource, script))
}

/// FJ-1394 / FJ-29: run a generated script in the privilege context the
/// resource DECLARES.
///
/// Called by all three entry points, not just `apply`. `sudo` is a property of
/// the RESOURCE, not of the apply phase: a check that runs as a different user
/// than the apply is not checking the apply, it is asking a different question.
/// Measured (#349): a `file` at `/etc/audit/rules.d/50-cargo-bin.rules` with
/// `sudo: true` was written correctly and then probed with a bare `test -f`,
/// which cannot traverse `drwxr-x--- root root /etc/audit`. apply exited 0, the
/// bytes on disk were right, and forjar reported `missing:file` forever — then
/// jidoka skipped the dependents that arm kernel auditing.
///
/// The same argument the module header makes for one dispatch row: the
/// privilege context is resolved ONCE, so a resource cannot be elevated on one
/// path and unelevated on another.
///
/// Uses a heredoc to pass the script to `sudo bash` — avoids the single-quote
/// escaping that triggers bashrs SC2075 false positives.
fn in_declared_privilege_context(resource: &Resource, script: String) -> String {
    if !resource.sudo {
        return script;
    }
    // Wrap: if already root, run as-is; otherwise elevate via sudo bash with heredoc
    format!(
        "if [ \"$(id -u)\" -eq 0 ]; then\n{script}\nelse\nsudo bash <<'FORJAR_SUDO'\n{script}\nFORJAR_SUDO\nfi"
    )
}

/// Generate a state query script for a resource.
///
/// Runs in the privilege context the resource declares — see
/// [`in_declared_privilege_context`]. Without it `live_hash`/`observed` record
/// the digest of the literal string `MISSING` for a root-only path.
#[contract("codegen-dispatch-v1", equation = "state_query_script")]
pub fn state_query_script(resource: &Resource) -> Result<String, String> {
    // Contract: codegen-dispatch-v1.yaml precondition (pv codegen)
    contract_pre_state_query_script!(resource);
    let row = handlers(&resource.resource_type).ok_or_else(|| NOT_DISPATCHABLE.to_string())?;
    Ok(in_declared_privilege_context(
        resource,
        (row.state_query)(resource),
    ))
}
