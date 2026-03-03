//! FJ-012: Executor — orchestration loop for apply.
//!
//! Applies resources in topological order per machine:
//! parse → validate → DAG → plan → for each resource: codegen → transport → hash → state → events

mod helpers;
mod machine;
mod machine_wave;
mod resource_ops;
mod strategies;

mod machine_b;
#[cfg(test)]
mod test_fixtures;
#[cfg(test)]
mod tests_advanced;
#[cfg(test)]
mod tests_concurrent;
#[cfg(test)]
mod tests_converge;
#[cfg(test)]
mod tests_converge2;
#[cfg(test)]
mod tests_core;
#[cfg(test)]
mod tests_drift;
#[cfg(test)]
mod tests_edge_apply;
#[cfg(test)]
mod tests_edge_details;
#[cfg(test)]
mod tests_edge_record;
#[cfg(test)]
mod tests_filters;
#[cfg(test)]
mod tests_hooks;
#[cfg(test)]
mod tests_localhost;
#[cfg(test)]
mod tests_localhost2;
#[cfg(test)]
mod tests_parallel;
#[cfg(test)]
mod tests_rolling;
#[cfg(test)]
mod tests_waves;

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
pub(crate) use helpers::{
    apply_and_record_outcome, build_resource_details, compute_resource_waves,
};
pub(crate) use machine::apply_machine;
pub(crate) use resource_ops::{
    apply_single_resource, copia_apply_file, log_tripwire, record_failure, record_success,
    RecordCtx, ResourceOutcome,
};
pub(crate) use strategies::{
    apply_machines_parallel, apply_machines_rolling, apply_machines_sequential,
};

/// Configuration for an apply run.
pub struct ApplyConfig<'a> {
    pub config: &'a ForjarConfig,
    pub state_dir: &'a std::path::Path,
    pub force: bool,
    pub dry_run: bool,
    pub machine_filter: Option<&'a str>,
    pub resource_filter: Option<&'a str>,
    pub tag_filter: Option<&'a str>,
    /// FJ-281: Filter to resources in this group
    pub group_filter: Option<&'a str>,
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
}

/// Load existing locks for machines matching the filter.
fn load_machine_locks(
    cfg: &ApplyConfig,
    all_machines: &[String],
) -> Result<HashMap<String, StateLock>, String> {
    let mut locks = HashMap::new();
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

    let plan = planner::plan(cfg.config, &execution_order, &locks, cfg.tag_filter);

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
