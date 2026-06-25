//! FJ-004: Plan generation — diff desired state against lock state.

use super::conditions;
use super::resolver;
use super::types::*;
use crate::tripwire::hasher;

/// Generate an execution plan by comparing desired config to lock state.
pub fn plan(
    config: &ForjarConfig,
    execution_order: &[String],
    locks: &std::collections::HashMap<String, StateLock>,
    tag_filter: Option<&str>,
) -> ExecutionPlan {
    // FJ-1210: Apply moved blocks — rename resource keys in lock state
    let locks = moved::apply_moved_blocks(&config.moved, locks);

    let mut changes = Vec::with_capacity(execution_order.len());
    let mut to_create = 0u32;
    let mut to_update = 0u32;
    let mut to_destroy = 0u32;
    let mut unchanged = 0u32;

    for resource_id in execution_order {
        let resource = match config.resources.get(resource_id) {
            Some(r) => r,
            None => continue,
        };

        if !passes_tag_filter(resource, tag_filter) {
            continue;
        }

        // Resolve templates before hashing so planner hash matches executor hash
        let resolved = resolve_or_fallback(resource_id, resource, config);

        for machine_name in resource.machine.iter() {
            if !passes_machine_filters(resource, machine_name, resource_id, config) {
                continue;
            }

            let action = determine_action(resource_id, &resolved, machine_name, &locks);
            let description = describe_action(resource_id, resource, &action);

            match action {
                PlanAction::Create => to_create += 1,
                PlanAction::Update => to_update += 1,
                PlanAction::Destroy => to_destroy += 1,
                PlanAction::NoOp => unchanged += 1,
            }

            changes.push(PlannedChange {
                resource_id: resource_id.clone(),
                machine: machine_name.to_owned(),
                resource_type: resource.resource_type.clone(),
                action,
                description,
            });
        }
    }

    // idempotent-apply-v1 contract: action counters partition the change
    // set — every planned change is counted exactly once, so a fully
    // converged stack shows to_create = to_update = to_destroy = 0
    // (f(f(x)) = f(x) at the plan level).
    debug_assert_eq!(
        (to_create + to_update + to_destroy + unchanged) as usize,
        changes.len(),
        "IDEMPOTENT-APPLY violated: action counters do not partition the change set"
    );

    ExecutionPlan {
        name: config.name.clone(),
        changes,
        execution_order: execution_order.to_vec(),
        to_create,
        to_update,
        to_destroy,
        unchanged,
    }
}

/// Check if a resource passes the tag filter.
fn passes_tag_filter(resource: &Resource, tag_filter: Option<&str>) -> bool {
    match tag_filter {
        Some(tag) => resource.tags.iter().any(|t| t == tag),
        None => true,
    }
}

/// Resolve resource templates, falling back to unresolved resource on error.
///
/// FJ-154 / #19: Resolve with the SAME `SecretsConfig` the executor uses
/// (`resolve_resource_templates_with_secrets(.., &config.secrets)`), so the
/// planner-computed desired hash matches the executor-stored hash. Resolving
/// with the default (env) provider here made every secret-bearing resource
/// replan as a spurious Update forever, violating f(f(x)) = f(x).
fn resolve_or_fallback(resource_id: &str, resource: &Resource, config: &ForjarConfig) -> Resource {
    resolver::resolve_resource_templates_with_secrets(
        resource,
        &config.params,
        &config.machines,
        &config.secrets,
    )
    .unwrap_or_else(|e| {
        eprintln!("warning: template resolution failed for {resource_id}: {e}");
        resource.clone()
    })
}

/// Check if a resource passes arch and when-condition filters for a machine.
fn passes_machine_filters(
    resource: &Resource,
    machine_name: &str,
    resource_id: &str,
    config: &ForjarConfig,
) -> bool {
    // FJ-064: Skip resource if arch filter doesn't match machine
    if !resource.arch.is_empty() {
        if let Some(machine) = config.machines.get(machine_name) {
            if !resource.arch.contains(&machine.arch) {
                return false;
            }
        }
    }

    // FJ-202: Skip resource if `when:` condition evaluates to false
    if let Some(ref when_expr) = resource.when {
        if let Some(machine) = config.machines.get(machine_name) {
            match conditions::evaluate_when(when_expr, &config.params, machine) {
                Ok(false) => return false,
                Err(e) => {
                    eprintln!(
                        "warning: when condition failed for {resource_id} on {machine_name}: {e}"
                    );
                    return false;
                }
                Ok(true) => {} // condition met, proceed
            }
        }
    }

    true
}

/// Get the default desired state for a resource type.
fn default_state(resource_type: &ResourceType) -> &'static str {
    match resource_type {
        ResourceType::Package => "present",
        ResourceType::File => "file",
        ResourceType::Service => "running",
        ResourceType::Mount => "mounted",
        ResourceType::User
        | ResourceType::Docker
        | ResourceType::Pepita
        | ResourceType::Network
        | ResourceType::Cron
        | ResourceType::Model
        | ResourceType::Gpu
        | ResourceType::Task
        | ResourceType::Recipe
        | ResourceType::WasmBundle
        | ResourceType::Image
        | ResourceType::Build
        | ResourceType::GithubRelease
        | ResourceType::OverlayInterface => "present",
    }
}

/// Determine what action to take for a resource on a machine.
fn determine_action(
    resource_id: &str,
    resource: &Resource,
    machine_name: &str,
    locks: &std::collections::HashMap<String, StateLock>,
) -> PlanAction {
    let state = resource
        .state
        .as_deref()
        .unwrap_or_else(|| default_state(&resource.resource_type));

    if state == "absent" {
        let action = determine_absent_action(resource_id, machine_name, locks);

        // FJ-1220: prevent_destroy blocks Destroy actions
        if action == PlanAction::Destroy {
            if let Some(ref lifecycle) = resource.lifecycle {
                if lifecycle.prevent_destroy {
                    eprintln!("warning: {resource_id} has prevent_destroy — skipping destroy");
                    return PlanAction::NoOp;
                }
            }
        }

        return action;
    }

    determine_present_action(resource_id, resource, machine_name, locks)
}

/// Determine action for a resource with state=absent.
fn determine_absent_action(
    resource_id: &str,
    machine_name: &str,
    locks: &std::collections::HashMap<String, StateLock>,
) -> PlanAction {
    if let Some(lock) = locks.get(machine_name) {
        if lock.resources.contains_key(resource_id) {
            return PlanAction::Destroy;
        }
    }
    PlanAction::NoOp
}

/// Determine action for a resource with a present/running/mounted state.
///
/// # Postcondition (FJ-2200)
///
/// If status is `Converged` and `rl.hash == hash_desired_state(resource)`,
/// the result MUST be `NoOp`. This is the idempotency contract.
fn determine_present_action(
    resource_id: &str,
    resource: &Resource,
    machine_name: &str,
    locks: &std::collections::HashMap<String, StateLock>,
) -> PlanAction {
    let lock = match locks.get(machine_name) {
        Some(l) => l,
        None => return PlanAction::Create,
    };
    let rl = match lock.resources.get(resource_id) {
        Some(r) => r,
        None => return PlanAction::Create,
    };

    if rl.status != ResourceStatus::Converged {
        return PlanAction::Update; // Previously failed or drifted
    }

    let desired_hash = hash_desired_state(resource);
    let result = if rl.hash == desired_hash {
        PlanAction::NoOp
    } else {
        PlanAction::Update
    };

    // FJ-2200 / idempotent-apply-v1 contract: idempotency postcondition —
    // converged + matching hash → NoOp
    debug_assert!(
        rl.status != ResourceStatus::Converged
            || rl.hash != desired_hash
            || result == PlanAction::NoOp,
        "idempotency violation: converged resource with matching hash must be NoOp"
    );

    result
}

/// Push an optional field's value onto the components list.
fn push_opt<'a>(components: &mut Vec<&'a str>, field: &'a Option<String>) {
    if let Some(ref val) = *field {
        components.push(val);
    }
}

/// Push all items from a Vec<String> onto the components list.
fn push_list<'a>(components: &mut Vec<&'a str>, items: &'a [String]) {
    for item in items {
        components.push(item);
    }
}

/// Collect core resource fields (phase 1) into hash components.
///
/// Field order is stable and must not change — it determines hash identity.
fn collect_core_fields<'a>(components: &mut Vec<&'a str>, resource: &'a Resource) {
    push_opt(components, &resource.state);
    push_opt(components, &resource.provider);
    push_list(components, &resource.packages);
    push_opt(components, &resource.path);
    push_opt(components, &resource.content);
    push_opt(components, &resource.source);
    push_opt(components, &resource.name);
    push_opt(components, &resource.owner);
    push_opt(components, &resource.group);
    push_opt(components, &resource.mode);
    push_opt(components, &resource.fs_type);
    push_opt(components, &resource.options);
    push_opt(components, &resource.target);
    push_opt(components, &resource.version);
}

/// Canonicalize `overlay_hosts` (a `HashMap`) into a stable, hashable string.
///
/// FJ-035: the `/etc/hosts` managed block is part of the converged state, so
/// two overlay_interface resources differing ONLY in `overlay_hosts` MUST hash
/// differently or `plan` will false-report `NoOp` and never rewrite the block.
/// `HashMap` iteration order is non-deterministic, so we sort by (name, ip) to
/// get a deterministic, order-independent serialization. Returns an empty
/// string when the map is absent/empty (no contribution to the hash).
fn canonical_overlay_hosts(resource: &Resource) -> String {
    let Some(hosts) = resource.overlay_hosts.as_ref() else {
        return String::new();
    };
    if hosts.is_empty() {
        return String::new();
    }
    let mut entries: Vec<(&String, &String)> = hosts.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0).then(a.1.cmp(b.1)));
    let mut out = String::from("overlay_hosts:");
    for (name, ip) in entries {
        out.push_str(name);
        out.push('=');
        out.push_str(ip);
        out.push(';');
    }
    out
}

/// Collect phase 2 resource fields into hash components.
///
/// Field order is stable and must not change — it determines hash identity.
/// `overlay_hosts_canon` is the pre-computed canonical serialization of
/// `overlay_hosts` (see `canonical_overlay_hosts`); it is threaded in as an
/// owned `&str` because the source map cannot be borrowed as a single slice.
fn collect_phase2_fields<'a>(
    components: &mut Vec<&'a str>,
    resource: &'a Resource,
    overlay_hosts_canon: &'a str,
) {
    push_opt(components, &resource.image);
    push_opt(components, &resource.command);
    push_opt(components, &resource.schedule);
    push_opt(components, &resource.restart);
    push_opt(components, &resource.port);
    push_opt(components, &resource.protocol);
    push_opt(components, &resource.action);
    push_opt(components, &resource.from_addr);
    push_opt(components, &resource.shell);
    push_opt(components, &resource.home);
    if let Some(ref enabled) = resource.enabled {
        components.push(if *enabled { "enabled" } else { "disabled" });
    }
    push_list(components, &resource.ports);
    push_list(components, &resource.environment);
    push_list(components, &resource.volumes);
    push_list(components, &resource.restart_on);
    // FJ-035: overlay_interface identity-bearing fields. overlay_ip changes the
    // bound address / ExecStart line, overlay_iface changes the target NIC,
    // overlay_firewall changes the converged ufw state, and overlay_hosts
    // changes the managed /etc/hosts block — all must alter the desired-state
    // hash so plan does not wrongly report NoOp.
    push_opt(components, &resource.overlay_ip);
    push_opt(components, &resource.overlay_iface);
    if let Some(fw) = resource.overlay_firewall {
        components.push(if fw {
            "overlay_fw_on"
        } else {
            "overlay_fw_off"
        });
    }
    // FJ-035 MATERIAL FIX: overlay_hosts was omitted from the hash collector, so
    // two resources differing ONLY in their /etc/hosts map hashed equal and plan
    // false-reported NoOp. Folding the canonical serialization closes that hole.
    if !overlay_hosts_canon.is_empty() {
        components.push(overlay_hosts_canon);
    }
}

/// Compute a hash of the desired state for comparison.
///
/// FJ-2200: Contract — determinism: same resource always produces same hash.
pub fn hash_desired_state(resource: &Resource) -> String {
    let type_str = resource.resource_type.to_string();
    // Owned canonicalization of overlay_hosts; kept alive for the borrow below.
    let overlay_hosts_canon = canonical_overlay_hosts(resource);
    let mut components: Vec<&str> = vec![&type_str];

    collect_core_fields(&mut components, resource);
    collect_phase2_fields(&mut components, resource, &overlay_hosts_canon);

    let joined = components.join("\0");
    let result = hasher::hash_string(&joined);

    // FJ-2200 / idempotent-apply-v1 contract: determinism postcondition —
    // calling again must produce the same hash
    debug_assert_eq!(
        result,
        hasher::hash_string(&joined),
        "hash_desired_state: determinism violated"
    );

    result
}

/// Generate a human-readable description of a planned action.
fn describe_action(resource_id: &str, resource: &Resource, action: &PlanAction) -> String {
    match action {
        PlanAction::Create => match resource.resource_type {
            ResourceType::Package => {
                let pkgs = resource.packages.join(", ");
                format!("{resource_id}: install {pkgs}")
            }
            ResourceType::File => {
                let path = resource.path.as_deref().unwrap_or("?");
                format!("{resource_id}: create {path}")
            }
            ResourceType::Service => {
                let name = resource.name.as_deref().unwrap_or("?");
                let verb = match resource.state.as_deref() {
                    Some("stopped") => "stop",
                    _ => "start",
                };
                format!("{resource_id}: {verb} {name}")
            }
            ResourceType::Mount => {
                let path = resource.path.as_deref().unwrap_or("?");
                format!("{resource_id}: mount {path}")
            }
            ResourceType::User
            | ResourceType::Docker
            | ResourceType::Pepita
            | ResourceType::Network
            | ResourceType::Cron
            | ResourceType::Model
            | ResourceType::Gpu
            | ResourceType::Task
            | ResourceType::Recipe
            | ResourceType::WasmBundle
            | ResourceType::Image
            | ResourceType::Build
            | ResourceType::GithubRelease
            | ResourceType::OverlayInterface => format!("{resource_id}: create"),
        },
        PlanAction::Update => format!("{resource_id}: update (state changed)"),
        PlanAction::Destroy => format!("{resource_id}: destroy"),
        PlanAction::NoOp => format!("{resource_id}: no changes"),
    }
}

pub mod minimal_changeset;
pub mod moved;
pub mod proof_obligation;
pub mod reversibility;
pub mod sat_deps;
pub mod why;

#[cfg(test)]
mod tests_advanced;
#[cfg(test)]
mod tests_describe;
#[cfg(test)]
mod tests_determine;
#[cfg(test)]
mod tests_filter;
#[cfg(test)]
mod tests_hash;
#[cfg(test)]
mod tests_hash_b;
#[cfg(test)]
mod tests_helpers;
#[cfg(test)]
mod tests_lifecycle;
#[cfg(test)]
mod tests_plan;
#[cfg(test)]
mod tests_plan_secrets;
#[cfg(test)]
mod tests_proof_cov;
#[cfg(test)]
mod tests_reversibility;
#[cfg(test)]
mod tests_when;
#[cfg(test)]
mod tests_why;
#[cfg(test)]
mod tests_why_cov;
