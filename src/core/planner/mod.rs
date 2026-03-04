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
    let locks = apply_moved_blocks(&config.moved, locks);

    let mut changes = Vec::new();
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

        for machine_name in resource.machine.to_vec() {
            if !passes_machine_filters(resource, &machine_name, resource_id, config) {
                continue;
            }

            let action = determine_action(resource_id, &resolved, &machine_name, &locks);
            let description = describe_action(resource_id, resource, &action);

            match action {
                PlanAction::Create => to_create += 1,
                PlanAction::Update => to_update += 1,
                PlanAction::Destroy => to_destroy += 1,
                PlanAction::NoOp => unchanged += 1,
            }

            changes.push(PlannedChange {
                resource_id: resource_id.clone(),
                machine: machine_name,
                resource_type: resource.resource_type.clone(),
                action,
                description,
            });
        }
    }

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
fn resolve_or_fallback(resource_id: &str, resource: &Resource, config: &ForjarConfig) -> Resource {
    resolver::resolve_resource_templates(resource, &config.params, &config.machines).unwrap_or_else(
        |e| {
            eprintln!(
                "warning: template resolution failed for {}: {}",
                resource_id, e
            );
            resource.clone()
        },
    )
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
                        "warning: when condition failed for {} on {}: {}",
                        resource_id, machine_name, e
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
        | ResourceType::Recipe => "present",
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
                    eprintln!(
                        "warning: {} has prevent_destroy — skipping destroy",
                        resource_id
                    );
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
    if rl.hash == desired_hash {
        PlanAction::NoOp
    } else {
        PlanAction::Update
    }
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

/// Collect phase 2 resource fields into hash components.
///
/// Field order is stable and must not change — it determines hash identity.
fn collect_phase2_fields<'a>(components: &mut Vec<&'a str>, resource: &'a Resource) {
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
}

/// Compute a hash of the desired state for comparison.
pub fn hash_desired_state(resource: &Resource) -> String {
    let type_str = resource.resource_type.to_string();
    let mut components: Vec<&str> = vec![&type_str];

    collect_core_fields(&mut components, resource);
    collect_phase2_fields(&mut components, resource);

    let joined = components.join("\0");
    hasher::hash_string(&joined)
}

/// Generate a human-readable description of a planned action.
fn describe_action(resource_id: &str, resource: &Resource, action: &PlanAction) -> String {
    match action {
        PlanAction::Create => match resource.resource_type {
            ResourceType::Package => {
                let pkgs = resource.packages.join(", ");
                format!("{}: install {}", resource_id, pkgs)
            }
            ResourceType::File => {
                let path = resource.path.as_deref().unwrap_or("?");
                format!("{}: create {}", resource_id, path)
            }
            ResourceType::Service => {
                let name = resource.name.as_deref().unwrap_or("?");
                format!("{}: start {}", resource_id, name)
            }
            ResourceType::Mount => {
                let path = resource.path.as_deref().unwrap_or("?");
                format!("{}: mount {}", resource_id, path)
            }
            ResourceType::User
            | ResourceType::Docker
            | ResourceType::Pepita
            | ResourceType::Network
            | ResourceType::Cron
            | ResourceType::Model
            | ResourceType::Gpu
            | ResourceType::Task
            | ResourceType::Recipe => format!("{}: create", resource_id),
        },
        PlanAction::Update => format!("{}: update (state changed)", resource_id),
        PlanAction::Destroy => format!("{}: destroy", resource_id),
        PlanAction::NoOp => format!("{}: no changes", resource_id),
    }
}

/// FJ-1210: Apply moved blocks to rename resource keys in lock state.
///
/// Returns a new lock map with resource keys renamed according to moved entries.
/// This prevents moved resources from appearing as destroy+create in the plan.
fn apply_moved_blocks(
    moved: &[crate::core::types::MovedEntry],
    locks: &std::collections::HashMap<String, StateLock>,
) -> std::collections::HashMap<String, StateLock> {
    if moved.is_empty() {
        return locks.clone();
    }

    let mut result = std::collections::HashMap::new();
    for (machine, lock) in locks {
        let mut new_lock = lock.clone();
        for entry in moved {
            if let Some(rl) = new_lock.resources.swap_remove(&entry.from) {
                new_lock.resources.insert(entry.to.clone(), rl);
                eprintln!(
                    "info: moved {} → {} in state for {}",
                    entry.from, entry.to, machine
                );
            }
        }
        result.insert(machine.clone(), new_lock);
    }
    result
}

pub mod proof_obligation;
pub mod reversibility;
pub mod sat_deps;
pub mod why;

#[cfg(test)]
mod tests_proof_obligation;
#[cfg(test)]
mod tests_reversibility;
#[cfg(test)]
mod tests_why;
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
mod tests_when;
