//! FJ-012: Executor — orchestration loop for apply.
//!
//! Applies resources in topological order per machine:
//! parse → validate → DAG → plan → for each resource: codegen → transport → hash → state → events

mod helpers;
mod machine;
mod machine_wave;
mod output_verify;
mod resource_ops;
pub mod run_capture;
mod strategies;

mod machine_b;
#[cfg(test)]
mod test_fixtures;
#[cfg(test)]
mod tests_advanced;
#[cfg(test)]
mod tests_advanced_b;
#[cfg(test)]
mod tests_concurrent;
#[cfg(test)]
mod tests_converge;
#[cfg(test)]
mod tests_converge2;
#[cfg(test)]
mod tests_core;
#[cfg(test)]
mod tests_core_b;
#[cfg(test)]
mod tests_displaced_hash;
#[cfg(test)]
mod tests_drift;
#[cfg(test)]
mod tests_edge_apply;
#[cfg(test)]
mod tests_edge_apply_b;
#[cfg(test)]
mod tests_edge_details;
#[cfg(test)]
mod tests_edge_record;
#[cfg(test)]
mod tests_filters;
#[cfg(test)]
mod tests_filters_b;
#[cfg(test)]
mod tests_hooks;
#[cfg(test)]
mod tests_hooks_b;
#[cfg(test)]
mod tests_localhost;
#[cfg(test)]
mod tests_localhost2;
#[cfg(test)]
mod tests_parallel;
#[cfg(test)]
mod tests_rolling;
#[cfg(test)]
mod tests_run_capture;
#[cfg(test)]
mod tests_waves;
#[cfg(test)]
mod tests_waves_cov;

use super::codegen;
use super::conditions;
use super::planner;
use super::resolver;
use super::state;
use super::types::*;
use crate::copia;
use crate::transport;
use crate::tripwire::{eventlog, hasher, tracer};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Instant;

// Re-export the public API
pub use helpers::collect_machines;

// Re-export internal items for sibling submodule access via `use super::*;`
pub(crate) use crate::tripwire::eventlog::log_tripwire;
pub(crate) use helpers::copia_apply_file;
pub(crate) use helpers::{
    apply_and_record_outcome, build_resource_details, compute_resource_waves,
};
pub(crate) use machine::apply_machine;
pub(crate) use resource_ops::{
    apply_single_resource, record_failure, record_success, RecordCtx, ResourceOutcome,
};
pub(crate) use strategies::{
    apply_machines_parallel, apply_machines_rolling, apply_machines_sequential,
};

/// Configuration for an apply run.
pub struct ApplyConfig<'a> {
    /// Parsed forjar configuration.
    pub config: &'a ForjarConfig,
    /// State directory for lock files.
    pub state_dir: &'a std::path::Path,
    /// Force apply even if resources are converged.
    pub force: bool,
    /// Dry-run mode (plan only, no execution).
    pub dry_run: bool,
    /// Filter to a single machine.
    pub machine_filter: Option<&'a str>,
    /// Filter to a single resource.
    pub resource_filter: Option<&'a str>,
    /// Filter to resources with a specific tag.
    pub tag_filter: Option<&'a str>,
    /// FJ-281: Filter to resources in this group
    pub group_filter: Option<&'a str>,
    /// Global timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// FJ-266: Force-remove stale lock before apply
    pub force_unlock: bool,
    /// FJ-272: Show progress counter during apply
    pub progress: bool,
    /// FJ-283: Retry failed resources up to N times with exponential backoff
    pub retry: u32,
    /// FJ-290: Override parallel execution (None = use policy)
    pub parallel: Option<bool>,
    /// FJ-304: Per-resource timeout in seconds (kill if exceeded)
    pub resource_timeout: Option<u64>,
    /// FJ-310: Auto-rollback to previous lock state on any failure
    pub rollback_on_failure: bool,
    /// FJ-313: Max concurrent resources per wave (None = unlimited)
    pub max_parallel: Option<usize>,
    /// FJ-1397: Debug trace mode — print generated scripts before execution
    pub trace: bool,
    /// FJ-2301: Run ID for log capture (None = no capture)
    pub run_id: Option<String>,
    /// FJ-3010: Refresh mode — re-run check scripts, only re-apply what fails
    pub refresh: bool,
    /// FJ-3010: Selective force — only force resources matching this tag
    pub force_tag: Option<&'a str>,
}

/// Load existing locks for machines matching the filter.
fn load_machine_locks(
    cfg: &ApplyConfig,
    all_machines: &[String],
) -> Result<HashMap<String, StateLock>, String> {
    let mut locks = HashMap::with_capacity(all_machines.len());
    for machine_name in all_machines {
        if cfg.machine_filter.is_some_and(|f| machine_name != f) {
            continue;
        }
        if let Some(lock) = state::load_lock(cfg.state_dir, machine_name)? {
            locks.insert(machine_name.clone(), lock);
        }
    }
    Ok(locks)
}

/// Build sorted target machine list (cheapest first).
fn build_target_machines<'a>(cfg: &ApplyConfig, all_machines: &'a [String]) -> Vec<&'a String> {
    let mut targets: Vec<&String> = all_machines
        .iter()
        .filter(|m| cfg.machine_filter.is_none_or(|f| *m == f))
        .collect();
    targets.sort_by_key(|m| {
        cfg.config
            .machines
            .get(*m)
            .map(|machine| machine.cost)
            .unwrap_or(0)
    });
    targets
}

/// Rollback locks to snapshots if any machine had failures.
fn rollback_on_failure(
    cfg: &ApplyConfig,
    results: &[ApplyResult],
    snapshots: &HashMap<String, StateLock>,
) {
    if !cfg.rollback_on_failure || snapshots.is_empty() {
        return;
    }
    let any_failed = results.iter().any(|r| r.resources_failed > 0);
    if any_failed {
        for snapshot in snapshots.values() {
            let _ = state::save_lock(cfg.state_dir, snapshot);
        }
    }
}

/// FJ-3010: Build locks with entries removed for resources matching force_tag.
///
/// Resources tagged with `force_tag` get their lock entry stripped (forcing re-apply).
/// All other resources keep their lock entries (hash comparison works normally).
fn selective_force_locks(
    locks: &HashMap<String, StateLock>,
    config: &ForjarConfig,
    tag: &str,
) -> HashMap<String, StateLock> {
    // Collect resource IDs that match the force tag
    let forced_ids: std::collections::HashSet<&str> = config
        .resources
        .iter()
        .filter(|(_, r)| r.tags.iter().any(|t| t == tag))
        .map(|(id, _)| id.as_str())
        .collect();

    let mut result = HashMap::with_capacity(locks.len());
    for (machine, lock) in locks {
        let mut new_lock = lock.clone();
        new_lock
            .resources
            .retain(|rid, _| !forced_ids.contains(rid.as_str()));
        result.insert(machine.clone(), new_lock);
    }
    result
}

/// FJ-129: Count resources that the lock reports as unchanged at apply
/// time, *before* `--force` clears the lock to nuke-and-pave them. The
/// caller invokes this only when `cfg.force` is true; the count answers
/// "how many resources did `--force` re-run that the lock said were
/// already converged?" which is the missing piece in the apply summary
/// that makes claim **C3** observable through `--force`.
///
/// This is the runtime side of contract
/// `apply-summary-distinguishability-v1`: the apply summary MUST be
/// able to distinguish a forced re-converge of a fully-converged stack
/// from a legitimate re-converge after drift.
///
/// Returns 0 if the locks are not yet loaded (first apply on a fresh
/// state directory) or on lock-load failure — both are correct: there
/// is no "forced no-op" if there was nothing to be forced over.
///
/// # GH-210: the lock alone cannot answer the question
///
/// The count used to be `shadow_plan.unchanged` — purely a config-vs-lock
/// comparison — and the caller invoked it AFTER the apply, when the lock had
/// already been rewritten to match the config. Both mistakes point the same
/// way: everything looks unchanged. Measured on 1.12.3, a file tampered with
/// on disk and restored by `apply --force` reported
///
/// ```text
///   note: --force re-ran 3 resource(s) the lock reported as unchanged
///         (0 actual change(s), 3 forced no-op(s))
/// ```
///
/// while `forjar drift` called the same resource DRIFTED and the file's hash
/// demonstrably changed. That is precisely the discrimination contract
/// `apply-summary-distinguishability-v1` requires, failing.
///
/// A resource is a genuine forced no-op only if the lock says NoOp **and** the
/// live machine still matches the lock. The live half costs a drift probe, so
/// this runs only under `--force`, and only for a real apply.
pub fn forced_noop_count(cfg: &ApplyConfig) -> u32 {
    let execution_order = match resolver::build_execution_order(cfg.config) {
        Ok(o) => o,
        Err(_) => return 0,
    };
    let all_machines = collect_machines(cfg.config);
    let real_locks = match load_machine_locks(cfg, &all_machines) {
        Ok(l) => l,
        Err(_) => return 0,
    };
    let shadow_plan = planner::plan(cfg.config, &execution_order, &real_locks, cfg.tag_filter);
    // GH-208 REGRESSION FIX: this briefly subtracted live-filesystem drift
    // (`count_forced_noops(&changes, &live_drifted_resources(..))`) as a fix for
    // "--force restores a tampered file but reports 0 actual changes".
    //
    // That misread the contract. apply-summary-distinguishability-v1 defines:
    //
    //     forced_noop_count(cfg) =
    //       if cfg.force then shadow_plan(config, real_locks).unchanged else 0
    //
    // — LOCK-based, deliberately. The contract even states the reported
    // behaviour as an invariant, not a bug:
    //   "actual_changes = 0 ∧ f > 0 ⇒ stack was fully converged before --force ran"
    //
    // This is the Q1/Q2 split that tests/test_fj129_force_distinguishability.rs
    // documents: Q1 "how many did --force re-run that the LOCK called
    // unchanged?" is cheap and deterministic; Q2 "how many have live drift?" is
    // what `forjar drift` answers. Conflating them was the ORIGINAL bug, and
    // subtracting drift here re-introduced it — FJ-129 shape 4 went 2 -> 1.
    count_forced_noops(&shadow_plan.changes, &Default::default())
}

/// A planned NoOp is a genuine forced no-op only if the machine still agrees.
///
/// Split out from [`forced_noop_count`] so the discrimination that contract
/// `apply-summary-distinguishability-v1` requires can be tested without a
/// machine: the shipped code was `shadow_plan.unchanged`, which counts a
/// drifted resource as a no-op.
pub fn count_forced_noops(
    changes: &[crate::core::types::PlannedChange],
    drifted: &std::collections::HashSet<String>,
) -> u32 {
    changes
        .iter()
        .filter(|c| c.action == crate::core::types::PlanAction::NoOp)
        .filter(|c| !drifted.contains(&c.resource_id))
        .count() as u32
}

/// Execute the apply loop.
pub fn apply(cfg: &ApplyConfig) -> Result<Vec<ApplyResult>, String> {
    let start = Instant::now();

    // FJ-266: State locking
    if !cfg.dry_run {
        if cfg.force_unlock {
            state::force_unlock(cfg.state_dir)?;
        }
        state::acquire_process_lock(cfg.state_dir)?;
    }

    let execution_order = resolver::build_execution_order(cfg.config)?;
    let all_machines = collect_machines(cfg.config);
    let mut locks = load_machine_locks(cfg, &all_machines)?;

    // FJ-310: Snapshot locks for rollback
    let lock_snapshots: HashMap<String, StateLock> = if cfg.rollback_on_failure {
        locks.clone()
    } else {
        HashMap::new()
    };

    // FJ-2300/FJ-3010: Force mode selection
    // --force: nuclear — empty locks, all resources re-applied
    // --force-tag: selective — empty locks only for resources matching tag
    // --refresh: re-run checks but use real locks (planner plans normally,
    //   check scripts re-evaluate live state during execution)
    let plan_locks = if cfg.force {
        HashMap::new()
    } else if let Some(tag) = cfg.force_tag {
        selective_force_locks(&locks, cfg.config, tag)
    } else {
        locks.clone()
    };
    // FJ-2710 (PMAT-197): probe declared build I/O BEFORE planning, so a task
    // whose sources changed on disk plans as Update rather than NoOp.
    // Probing is controller-local, so only resources targeting a local machine
    // are probed — hashing this host's files for a remote target would compare
    // the wrong tree and produce confidently wrong build decisions.
    // Resources MUST be template-resolved before probing: `working_dir` is
    // routinely `{{params.proj}}`, and probing the raw form makes every
    // declared artifact look missing, which rebuilds the world on every apply.
    // This is the same class as the drift bug fixed alongside it — hence the
    // shared resolver.
    let resolved_for_probe = crate::core::resolver::resolve_all(
        &cfg.config.resources,
        &cfg.config.params,
        &cfg.config.machines,
        &cfg.config.secrets,
    );
    let probes = crate::core::task::probe_all(&resolved_for_probe, |m| {
        cfg.config
            .machines
            .get(m)
            .is_some_and(crate::transport::machine_is_local)
    });
    let plan = planner::plan_with_probes(
        cfg.config,
        &execution_order,
        &plan_locks,
        cfg.tag_filter,
        &probes,
    );

    if cfg.dry_run {
        return Ok(vec![ApplyResult {
            machine: "dry-run".to_string(),
            resources_converged: 0,
            resources_unchanged: plan.unchanged,
            resources_failed: 0,
            total_duration: start.elapsed(),
            resource_reports: Vec::new(),
        }]);
    }

    let target_machines = build_target_machines(cfg, &all_machines);
    let localhost_machine = Machine {
        hostname: "localhost".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    };

    let result = dispatch_apply(cfg, &target_machines, &localhost_machine, &plan, &mut locks);

    if let Ok(ref results) = result {
        rollback_on_failure(cfg, results, &lock_snapshots);
    }

    if !cfg.dry_run {
        state::release_process_lock(cfg.state_dir);
    }

    result
}

/// Dispatch to the appropriate machine apply strategy.
fn dispatch_apply(
    cfg: &ApplyConfig,
    target_machines: &[&String],
    localhost_machine: &Machine,
    plan: &ExecutionPlan,
    locks: &mut HashMap<String, StateLock>,
) -> Result<Vec<ApplyResult>, String> {
    if let Some(batch_size) = cfg.config.policy.serial {
        let batch_size = batch_size.max(1);
        apply_machines_rolling(
            cfg,
            target_machines,
            localhost_machine,
            plan,
            locks,
            batch_size,
        )
    } else if cfg.config.policy.parallel_machines && target_machines.len() > 1 {
        apply_machines_parallel(cfg, target_machines, localhost_machine, plan, locks)
    } else {
        apply_machines_sequential(cfg, target_machines, localhost_machine, plan, locks)
    }
}
