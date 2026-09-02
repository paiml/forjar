//! FJ-004: Plan generation — diff desired state against lock state.

use super::conditions;
use super::resolver;
use super::types::*;

/// Generate an execution plan by comparing desired config to lock state.
pub fn plan(
    config: &ForjarConfig,
    execution_order: &[String],
    locks: &std::collections::HashMap<String, StateLock>,
    tag_filter: Option<&str>,
) -> ExecutionPlan {
    // v1.11.0 forwarded an EMPTY map here, which made every READ path
    // (plan/check/drift/observe) blind to the staleness that `apply` acts on.
    // Probing here means one answer for both.
    plan_with_probes(
        config,
        execution_order,
        locks,
        tag_filter,
        &crate::core::task::probe_config(config),
    )
}

/// FJ-2710 (PMAT-197): plan with world-derived staleness.
///
/// `probes` carries the observed on-disk state of each resource's declared
/// `task_inputs` / `output_artifacts`. Without it the planner compares only
/// desired-config hashes, so a task whose sources changed plans as `NoOp` and
/// forjar reports success over a stale artifact.
///
/// The planner stays PURE — it never touches the filesystem or a transport.
/// The caller probes and hands the result in.
pub fn plan_with_probes(
    config: &ForjarConfig,
    execution_order: &[String],
    locks: &std::collections::HashMap<String, StateLock>,
    tag_filter: Option<&str>,
    probes: &std::collections::HashMap<String, crate::core::task::IoDigest>,
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

            let action = determine_action(resource_id, &resolved, machine_name, &locks, probes);
            // GH-212: describe the RESOLVED resource. `determine_action` was
            // already given `resolved`, but the description was rendered from
            // the raw config, so `plan` named the file it would create as the
            // literal `{{params.sandbox}}/a.txt` while `show` and `apply` both
            // used the real path — the pre-flight review surface disagreed with
            // what the apply actually wrote.
            let description = describe_action(resource_id, &resolved, &action);

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

    // FJ-2711 (PMAT-197): a rebuilt prerequisite must invalidate its dependents.
    // See planner::propagation for why this cannot live in the probe.
    let promoted = self::propagation::propagate_changes(config, execution_order, &mut changes);
    unchanged -= promoted;
    to_update += promoted;

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
    resolver::resolve_or_fallback(
        resource_id,
        resource,
        &config.params,
        &config.machines,
        &config.secrets,
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
        | ResourceType::OverlayInterface
        | ResourceType::DiskBudget
        | ResourceType::BackupSync
        | ResourceType::NasArchive => "present",
    }
}

/// Determine what action to take for a resource on a machine.
fn determine_action(
    resource_id: &str,
    resource: &Resource,
    machine_name: &str,
    locks: &std::collections::HashMap<String, StateLock>,
    probes: &std::collections::HashMap<String, crate::core::task::IoDigest>,
) -> PlanAction {
    let state = resource
        .state
        .as_deref()
        .unwrap_or_else(|| default_state(&resource.resource_type));

    if state == "absent" {
        let action = determine_absent_action(resource_id, resource, machine_name, locks);

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

    determine_present_action(resource_id, resource, machine_name, locks, probes)
}

/// Determine action for a resource with state=absent.
///
/// GH-229: this used to return `Destroy` for any resource present in the lock,
/// using "is in the lock" as a proxy for "still exists on the machine". That
/// proxy is false — a SUCCESSFUL destroy writes the resource back into the lock
/// as `converged`, so the plan re-emitted `Destroy` on every subsequent run and
/// never reached a fixed point (observed as a permanent "N to destroy" on
/// infra's lambda-labs, pending for days across two unrelated resource sets).
///
/// Two situations look identical under `status: converged` and must not be
/// conflated:
///   (A) converged as PRESENT, now redeclared absent  -> must Destroy
///   (B) converged TO ABSENT (destroy already ran)    -> must NoOp
///
/// The stored hash separates them, because `state` is itself a component of
/// `hash_desired_state` (see hashing.rs `push_opt(components, &resource.state)`).
/// A lock written by a present-state apply cannot match the absent form's hash.
/// This is the same rule `determine_present_action` already applies, so the two
/// branches now share one idempotency contract rather than one having none.
///
/// The FJ-2200 idempotency postcondition (converged + matching hash → NoOp) is
/// structural here — it is the literal final branch — so it needs no
/// `debug_assert` to restate it.
pub(super) fn determine_absent_action(
    resource_id: &str,
    resource: &Resource,
    machine_name: &str,
    locks: &std::collections::HashMap<String, StateLock>,
) -> PlanAction {
    let Some(rl) = locks
        .get(machine_name)
        .and_then(|lock| lock.resources.get(resource_id))
    else {
        // GH-339: NO LOCK ENTRY MEANS UNKNOWN, NOT ABSENT.
        //
        // This returned NoOp, on the reasoning stated in why.rs — "resource not
        // in lock, nothing to destroy". That is a claim about the LOCK, not
        // about the machine, and it is backwards for the common case: the whole
        // reason to declare a file absent is normally that it exists and forjar
        // did NOT create it. A legacy file, a leftover, a stale drop-in. Those
        // are exactly the resources with no lock entry, so `absent` worked only
        // for files forjar had made itself — the case where you would simply
        // delete the declaration instead.
        //
        // It shipped a green "Apply complete" over a dormant NOPASSWD:ALL
        // sudoers grant on the fleet controller (paiml/infra#317).
        //
        // Destroy is safe here BECAUSE of the hash check below, which GH-229
        // added: the destroy runs `rm -rf` (idempotent on a missing path),
        // records the absent-form hash, and the next plan takes the (B) branch
        // and no-ops. Fixed point after one apply. Before that check existed,
        // returning Destroy for an unlocked resource is what re-emitted forever.
        return PlanAction::Destroy;
    };

    // A destroy that failed or drifted must be retried.
    if rl.status != ResourceStatus::Converged {
        return PlanAction::Destroy;
    }

    if rl.hash == hash_desired_state(resource) {
        PlanAction::NoOp // (B) already converged to absent
    } else {
        PlanAction::Destroy // (A) lock holds a present-state hash
    }
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
    probes: &std::collections::HashMap<String, crate::core::task::IoDigest>,
) -> PlanAction {
    // FJ-2725 (PMAT-199): a phony resource that reaches the planner was named
    // as an explicit goal — `strip_unrequested_phony` removed every other one
    // before planning. It names an ACTION with no observable artifact, so it
    // runs unconditionally: no lock read, no hash compare, no probe.
    //
    // This does not weaken the idempotency contract. `plan` over a config with
    // no goals contains no phony resources at all, so the plan fixed point is
    // untouched; requesting an action by name is the user asking for it to
    // happen, which is a different thing from convergence.
    if resource.phony {
        return PlanAction::Update;
    }

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

    // FJ-2710 (PMAT-197): world-derived staleness OVERRIDES a matching config
    // hash. A build task whose sources changed has an identical desired state
    // — only the filesystem knows it must re-run. Checked before the hash
    // comparison because a stale artifact is a correctness bug, not a
    // preference.
    if let Some(probe) = probes.get(resource_id) {
        let stored_in = rl.details.get("input_hash").and_then(|v| v.as_str());
        let stored_out = rl.details.get("output_hash").and_then(|v| v.as_str());
        if let Some(reason) = crate::core::task::staleness_reason(probe, stored_in, stored_out) {
            eprintln!("  {resource_id}: stale — {reason}");
            return PlanAction::Update;
        }
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
            | ResourceType::OverlayInterface
            | ResourceType::DiskBudget
            | ResourceType::BackupSync
            | ResourceType::NasArchive => format!("{resource_id}: create"),
        },
        PlanAction::Update => format!("{resource_id}: update (state changed)"),
        PlanAction::Destroy => format!("{resource_id}: destroy"),
        PlanAction::NoOp => format!("{resource_id}: no changes"),
    }
}

pub mod hashing;
pub use hashing::hash_desired_state;
pub mod minimal_changeset;
pub mod moved;
pub mod proof_obligation;
pub mod propagation;
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
mod tests_hash_overlay;
#[cfg(test)]
mod tests_hash_source;
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
mod tests_sat_deps_b;
#[cfg(test)]
mod tests_when;
#[cfg(test)]
mod tests_why;
#[cfg(test)]
mod tests_why_cov;
