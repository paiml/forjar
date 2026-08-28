//! FJ-3010, second half: record what the host already satisfies.
//!
//! Split out of `refresh.rs` for the same reason `refresh.rs` was split out of
//! `mod.rs` — to keep each file small enough to read in one sitting.
//!
//! `refresh_locks` could only ever REMOVE lock entries whose live check failed,
//! so `--refresh` worked exactly when a lock existed and reality had drifted
//! from it, and was a no-op when the lock was EMPTY and reality was fine: there
//! was nothing to remove.
//!
//! An empty lock is not an edge case. It is every CI checkout, every reimaged
//! box, every `--state-dir` a run has not written yet. In all of those,
//! `--refresh` contacted the host, learned every check passed, and planned
//! `create` anyway — re-running every command against a machine already in its
//! declared state.
//!
//! Measured on 1.20.1: a `type: task` with `completion_check: "[ -d / ]"` and
//! `command: touch /tmp/marker`, applied with `--refresh` against a fresh state
//! dir, created the marker.
//!
//! It is also the only state in which forjar can express an ASSERTION — a guard
//! whose `completion_check` is the claim and whose `command` reports the
//! violation. Without this, every such guard ran its failure path on a healthy
//! host.

use super::super::codegen;
use super::super::resolver;
use super::super::types::*;
use super::ApplyConfig;
use crate::transport;

/// Did this resource's check DEFINITELY pass on `machine_name`?
///
/// Deliberately not `!refresh_check_fails`. That function treats "could not
/// observe" — exit 2, a transport error, an unresolvable machine — as
/// not-failing, which is the right call when deciding whether to RE-APPLY:
/// rebuilding the world because a host blinked is worse than waiting.
///
/// Seeding a lock entry is the opposite kind of claim. It records that the host
/// IS in its declared state, and "I could not look" is not evidence for that.
/// So this returns true only on a check that ran and exited 0.
pub(super) fn check_passes_on(cfg: &ApplyConfig, resource: &Resource, machine_name: &str) -> bool {
    if cfg.machine_filter.is_some_and(|f| machine_name != f) {
        return false;
    }
    let Ok(resolved) =
        resolver::resolve_resource_templates(resource, &cfg.config.params, &cfg.config.machines)
    else {
        return false;
    };
    let Ok(script) = codegen::check_script(&resolved) else {
        return false;
    };
    cfg.config
        .machines
        .get(machine_name)
        .and_then(|m| transport::exec_script(m, &script).ok())
        .is_some_and(|out| out.success())
}

/// FJ-3010, second half: record resources the HOST already satisfies.
///
/// `refresh_locks` could only ever REMOVE lock entries, so `--refresh` worked
/// exactly when a lock existed and reality had drifted from it, and was a no-op
/// when the lock was EMPTY and reality was fine — there was nothing to remove.
///
/// An empty lock is not an edge case. It is every CI checkout, every reimaged
/// box, every `--state-dir` a run has not written yet. In all of those,
/// `--refresh` contacted the host, learned that every check passed, and then
/// planned `create` for all of them anyway — re-running every command against a
/// machine already in its declared state.
///
/// Measured on forjar 1.20.1: a `type: task` with `completion_check: "[ -d / ]"`
/// and `command: touch /tmp/marker`, applied with `--refresh` against a fresh
/// state dir, created the marker. The check was satisfied before the run and
/// nothing consulted it.
///
/// "Re-run check scripts, only re-apply what fails" has two halves. This is the
/// one that says what PASSES is not re-applied.
fn should_seed(
    cfg: &ApplyConfig,
    machine_name: &str,
    id: &str,
    resource: &Resource,
    lock: &StateLock,
) -> bool {
    !lock.resources.contains_key(id)
        && super::refresh::refresh_in_scope(cfg, id, resource)
        && resource.machine.iter().any(|m| m == machine_name)
        && check_passes_on(cfg, resource, machine_name)
}

/// A lock entry for a resource the host already satisfies.
///
/// No apply happened, so there is no `applied_at` or `duration_seconds` to
/// record — writing one would date an event that never occurred. `observed` is
/// a digest of the state query's stdout, and the check script is not that
/// query; conflating the two is forjar#305.
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

pub(super) fn seed_converged(cfg: &ApplyConfig, machine_name: &str, lock: &mut StateLock) {
    let seeds: Vec<(String, ResourceLock)> = cfg
        .config
        .resources
        .iter()
        .filter(|(id, r)| should_seed(cfg, machine_name, id, r, lock))
        .map(|(id, r)| (id.clone(), converged_entry(r)))
        .collect();
    for (id, entry) in seeds {
        lock.resources.insert(id, entry);
    }
}
