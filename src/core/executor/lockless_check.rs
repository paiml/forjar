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
//! The apply path RECORDS what it seeded (`record`), as the converge it replaces
//! would have; the previews (prompt, `--dry-run`) only read (`seeded`).

use super::super::codegen;
use super::super::resolver;
use super::super::state;
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

/// Add a converged entry to `locks` for every `(machine, resource)` that has no
/// entry, declares a `completion_check`, and whose check passes on that machine
/// right now. The entry is `refresh_seed::converged_entry`: no `applied_at`, no
/// `observed`, because nothing was applied and the check is not the state query.
///
/// `config` is the apply's SELECTION (`resolve_selection` has already pruned
/// it), so only selected resources cost a host round-trip; `machine_filter` and
/// `tag_filter` narrow it the same way the planner will.
fn add_passing(
    config: &ForjarConfig,
    machine_filter: Option<&str>,
    tag_filter: Option<&str>,
    locks: &mut HashMap<String, StateLock>,
) {
    for (id, resource) in &config.resources {
        if !declares_a_check(resource, tag_filter) {
            continue;
        }
        for machine in resource.machine.iter() {
            if machine_filter.is_some_and(|f| machine != f)
                // The planner skips an arch mismatch or a false `when:`; asking
                // (and recording) a resource it will not plan is a false record.
                || !crate::core::planner::passes_machine_filters(resource, machine, id, config)
            {
                continue;
            }
            let has_entry = locks
                .get(machine)
                .is_some_and(|l| l.resources.contains_key(id));
            if has_entry || !host_check_passes(config, resource, machine) {
                continue;
            }
            let hostname = config
                .machines
                .get(machine)
                .map_or(machine, |m| m.hostname.as_str());
            locks
                .entry(machine.to_string())
                .or_insert_with(|| state::new_lock(machine, hostname))
                .resources
                .insert(
                    id.clone(),
                    super::refresh_seed::converged_entry(config, id, resource),
                );
        }
    }
}

/// PREVIEW: the locks the planner will read, without touching `locks`. For the
/// confirmation prompt and `--dry-run`, which must show what the apply will do
/// and must not change what it will write.
pub(crate) fn seeded(
    config: &ForjarConfig,
    machine_filter: Option<&str>,
    tag_filter: Option<&str>,
    locks: &HashMap<String, StateLock>,
) -> HashMap<String, StateLock> {
    let mut out = locks.clone();
    add_passing(config, machine_filter, tag_filter, &mut out);
    out
}

/// APPLY: record the passing entries in the locks this apply WRITES, and return
/// the planner view (identical to them).
///
/// Recorded, not merely planned around, because the converge this replaces
/// recorded one: before forjar#615 an apply over a satisfied guard ran its
/// command, re-ran the check (GH-254) and wrote `status: converged`. Skipping
/// the command must not also skip the record, or the lock never learns the
/// resource exists and `drift` declines it for ever ("inspected 0 of 1
/// declared; no lock holds them" —
/// `falsification_drift_is_not_blind_to_task_guards`). The evidence is the one
/// the old record rested on: the check ran on the host and exited 0.
pub(crate) fn record(
    config: &ForjarConfig,
    machine_filter: Option<&str>,
    tag_filter: Option<&str>,
    locks: &mut HashMap<String, StateLock>,
) -> HashMap<String, StateLock> {
    add_passing(config, machine_filter, tag_filter, locks);
    locks.clone()
}

#[cfg(test)]
#[path = "tests_lockless_check.rs"]
mod tests_lockless_check;
