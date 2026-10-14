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
/// So only [`CheckVerdict::Passed`] — a check that ran and exited 0 — seeds.
///
/// forjar#590: this used to return a bare `bool`, collapsing "the check ran and
/// failed" and "the check could not be run at all" into the same `false`. Both
/// still decline to seed, but the second now says so on stderr, because a
/// guard that silently re-runs its command for a reason nobody can see is the
/// shape that hid #590 for a release.
pub(super) fn check_on(cfg: &ApplyConfig, resource: &Resource, machine_name: &str) -> CheckVerdict {
    if cfg.machine_filter.is_some_and(|f| machine_name != f) {
        return CheckVerdict::OutOfScope;
    }
    // The executor's own resolver, with the executor's own secrets: anything
    // else hashes and checks a resource the apply would never run.
    let resolved = match resolver::resolve_resource_templates_with_secrets(
        resource,
        &cfg.config.params,
        &cfg.config.machines,
        &cfg.config.secrets,
    ) {
        Ok(r) => r,
        Err(e) => return CheckVerdict::Unmeasured(format!("template resolution: {e}")),
    };
    let script = match codegen::check_script(&resolved) {
        Ok(s) => s,
        Err(e) => return CheckVerdict::Unmeasured(format!("no check script: {e}")),
    };
    let Some(machine) = cfg.config.machines.get(machine_name) else {
        return CheckVerdict::Unmeasured(format!("unknown machine {machine_name}"));
    };
    match transport::exec_script(machine, &script) {
        Ok(out) if out.success() => CheckVerdict::Passed(Box::new(resolved)),
        Ok(_) => CheckVerdict::Failed,
        Err(e) => CheckVerdict::Unmeasured(format!("transport: {e}")),
    }
}

/// What one `--refresh` check established about one resource on one host.
pub(super) enum CheckVerdict {
    /// The check ran and exited 0. Carries the RESOLVED resource the check ran
    /// against, so the seeded lock entry is hashed over what the planner and
    /// executor hash — never over the raw, still-templated config (forjar#590).
    Passed(Box<Resource>),
    /// The check ran and exited non-zero.
    Failed,
    /// The check could not be run; the reason is reported, never swallowed.
    Unmeasured(String),
    /// `-m` excludes this machine: nothing was asked.
    OutOfScope,
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
pub(super) fn seed_converged(cfg: &ApplyConfig, machine_name: &str, lock: &mut StateLock) {
    let unlocked: Vec<String> = cfg
        .config
        .resources
        .keys()
        .filter(|id| !lock.resources.contains_key(*id))
        .cloned()
        .collect();
    record_converged(cfg, machine_name, lock, unlocked);
}

/// Does the HOST — not the lock — report this resource as being in its declared
/// state right now? `Some(resolved)` when it does.
///
/// Shared by seeding (an entry that does not exist) and unlatching (an entry
/// that exists and says the resource is broken), because the evidence required
/// is identical: the resource is in this apply's scope, it is declared for this
/// machine, and its check ran here and exited 0.
///
/// A check that could not be run declines to seed AND says why on stderr —
/// "could not measure" is reported, never folded silently into "failed".
fn host_says_converged(
    cfg: &ApplyConfig,
    machine_name: &str,
    id: &str,
    resource: &Resource,
) -> Option<Resource> {
    if !(super::refresh::refresh_in_scope(cfg, id, resource)
        && resource.machine.iter().any(|m| m == machine_name))
    {
        return None;
    }
    match check_on(cfg, resource, machine_name) {
        CheckVerdict::Passed(resolved) => Some(*resolved),
        CheckVerdict::Unmeasured(why) => {
            eprintln!(
                "warning: --refresh could not check {id} on {machine_name} ({why}); \
                 not recording it as converged"
            );
            None
        }
        CheckVerdict::Failed | CheckVerdict::OutOfScope => None,
    }
}

/// A lock entry for a resource the host already satisfies.
///
/// `resolved` MUST be the template-resolved resource (forjar#590). The planner
/// compares `rl.hash` against `hash_desired_state` of the RESOLVED resource and
/// every executor write stores that same hash; an entry hashed over the raw
/// config matches only when the resource has no template, so every templated
/// guard was seeded "converged" and then re-planned as an Update that ran its
/// command anyway.
///
/// No apply happened, so there is no `applied_at` or `duration_seconds` to
/// record — writing one would date an event that never occurred. `observed` is
/// a digest of the state query's stdout, and the check script is not that
/// query; conflating the two is forjar#305.
pub(super) fn converged_entry(resolved: &Resource) -> ResourceLock {
    ResourceLock {
        resource_type: resolved.resource_type.clone(),
        status: ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        hash: crate::core::planner::hashing::hash_desired_state(resolved),
        observed: None,
        details: Default::default(),
    }
}

/// Record, as converged, each candidate the host says is already converged.
///
/// The one writer both halves of `--refresh` go through: seeding (candidates
/// with no lock entry) and unlatching (candidates the lock calls broken) differ
/// only in which entries they nominate, never in the evidence required.
fn record_converged(
    cfg: &ApplyConfig,
    machine_name: &str,
    lock: &mut StateLock,
    candidates: Vec<String>,
) {
    let entries: Vec<(String, ResourceLock)> = candidates
        .into_iter()
        .filter_map(|id| cfg.config.resources.get(&id).map(|r| (id, r)))
        .filter_map(|(id, r)| {
            host_says_converged(cfg, machine_name, &id, r)
                .map(|resolved| (id, converged_entry(&resolved)))
        })
        .collect();
    for (id, entry) in entries {
        lock.resources.insert(id, entry);
    }
}

/// PMAT-214 (forjar#487): re-check the entries the lock records as BROKEN.
///
/// `refresh_locks` evicts an entry whose live check FAILS, and `seed_converged`
/// adds one for a resource that has NO entry. Between them sits the case
/// neither covers: an entry that EXISTS, says `failed`, and whose check now
/// passes. Nothing re-ran that check, so `--refresh` planned an `Update`, and
/// the generated script is command-then-check —
///
/// ```sh
/// set -euo pipefail
/// <command>                   # a guard's command exits 1 by design
/// if ! { <completion_check> } # never reached
/// ```
///
/// — so the command re-failed and the failure was re-recorded. A latch.
///
/// It closes on the resources forjar's own idioms require: a guard whose command
/// cannot succeed because the action needs a secret no config may hold, and whose
/// job is to refuse loudly and name the make target. Measured on yoga 2026-09-07
/// against a correctly registered runner — check PASS by hand inside forjar's own
/// wrapper, `drift` skipping the entry, `apply --refresh -r runner-registered`
/// still "0 converged, 0 unchanged, 2 failed". On gx10 it also took three
/// dependents with it under `policy.failure: stop_on_first`.
///
/// NOT a forgiveness rule: the promotion demands a fresh check that exits 0, so
/// nothing reaches `converged` without the host saying so.
pub(super) fn unlatch_failed(cfg: &ApplyConfig, machine_name: &str, lock: &mut StateLock) {
    let broken: Vec<String> = lock
        .resources
        .iter()
        .filter(|(_, rl)| rl.status != ResourceStatus::Converged)
        .map(|(id, _)| id.clone())
        .collect();
    record_converged(cfg, machine_name, lock, broken);
}
