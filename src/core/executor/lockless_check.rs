//! forjar#615: a resource with NO lock entry asks the host before it plans `create`.
//!
//! The planner is lock-relative: no entry means `Create`, and `Create` runs the
//! command. For most resources that is harmless — a file is rewritten with the
//! bytes it already holds. For a `task` it is not: the command is whatever the
//! author wrote, and the `completion_check` beside it is the author's own
//! statement of "already done".
//!
//! Measured on paiml/infra's lambda-labs with forjar 1.32.0 and no lambda-labs
//! lock in the state dir: `apply -r ollama-model-qwen35-4b` pulled in its
//! dependency `ollama-binary` and prompted "Apply 2 change(s) (2 create)".
//! `ollama-binary`'s check returned 0 on the box; its command runs
//! `rm -rf /usr/local/lib/ollama`, re-extracts, and restarts the shared daemon.
//!
//! `--refresh` already seeds every lockless resource whose check passes
//! (`refresh_seed::seed_converged`). The DEFAULT path did not, so the one state
//! in which forjar knows nothing — an empty lock — was the one in which it
//! trusted itself most. This is the default path's version, narrowed to what the
//! defect needs: resources that DECLARE a `completion_check`, and only where the
//! lock has no entry. A resource with an entry keeps the lock's word, exactly as
//! before; a check that cannot run, or runs and does not exit 0, leaves the
//! resource planned `create`, exactly as before.
//!
//! The seeded entries are the planner's VIEW, never written: nothing was applied,
//! so there is nothing to record (the same rule `--refresh`'s seeding follows).

use super::super::codegen;
use super::super::resolver;
use super::super::types::*;
use crate::transport;
use std::collections::HashMap;

/// Did `resource`'s declared check run on `machine_name` and exit 0?
///
/// "Could not look" — an unresolvable template, an undeclared machine, a
/// transport error — is NOT a pass: seeding records that the host IS in its
/// declared state, and absence of evidence is not that.
pub(crate) fn host_check_passes(
    config: &ForjarConfig,
    resource: &Resource,
    machine_name: &str,
) -> bool {
    let Ok(resolved) =
        resolver::resolve_resource_templates(resource, &config.params, &config.machines)
    else {
        return false;
    };
    let Ok(script) = codegen::check_script(&resolved) else {
        return false;
    };
    config
        .machines
        .get(machine_name)
        .and_then(|m| transport::exec_script(m, &script).ok())
        .is_some_and(|out| out.success())
}

/// Is this resource one the lockless check applies to at all?
fn declares_a_check(resource: &Resource, tag_filter: Option<&str>) -> bool {
    resource.completion_check.is_some()
        && !resource.phony
        && tag_filter.is_none_or(|t| resource.tags.iter().any(|x| x == t))
}

/// A lock entry for a resource the host already satisfies. No apply happened,
/// so there is no `applied_at` or `duration_seconds` to record.
fn converged_entry(resource: &Resource) -> ResourceLock {
    ResourceLock {
        resource_type: resource.resource_type.clone(),
        status: ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        hash: crate::core::planner::hashing::hash_desired_state(resource),
        observed: None,
        details: Default::default(),
    }
}

fn empty_lock(machine: &str) -> StateLock {
    StateLock {
        schema: "1".to_string(),
        machine: machine.to_string(),
        hostname: machine.to_string(),
        generated_at: crate::tripwire::eventlog::now_iso8601(),
        generator: format!("forjar-lockless-check {}", env!("CARGO_PKG_VERSION")),
        created_by: None,
        blake3_version: "1.5".to_string(),
        resources: indexmap::IndexMap::new(),
    }
}

/// The locks the PLANNER should read: `locks`, plus a converged entry for every
/// `(machine, resource)` that has no entry, declares a `completion_check`, and
/// whose check passes on that machine right now.
///
/// `config` is the apply's SELECTION (`resolve_selection` has already pruned
/// it), so only selected resources cost a host round-trip; `machine_filter` and
/// `tag_filter` narrow it the same way the planner will.
pub(crate) fn seeded(
    config: &ForjarConfig,
    machine_filter: Option<&str>,
    tag_filter: Option<&str>,
    locks: &HashMap<String, StateLock>,
) -> HashMap<String, StateLock> {
    let mut out = locks.clone();
    for (id, resource) in &config.resources {
        if !declares_a_check(resource, tag_filter) {
            continue;
        }
        for machine in resource.machine.iter() {
            if machine_filter.is_some_and(|f| machine != f) {
                continue;
            }
            let has_entry = out
                .get(machine)
                .is_some_and(|l| l.resources.contains_key(id));
            if has_entry || !host_check_passes(config, resource, machine) {
                continue;
            }
            out.entry(machine.to_string())
                .or_insert_with(|| empty_lock(machine))
                .resources
                .insert(id.clone(), converged_entry(resource));
        }
    }
    out
}

#[cfg(test)]
#[path = "tests_lockless_check.rs"]
mod tests_lockless_check;
