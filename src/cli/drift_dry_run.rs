//! `forjar drift --dry-run`: what a drift check would inspect, without connecting.
//! Split out of `drift.rs` to keep that file under the 500-line ratchet.

use super::drift_lockless::dry_run_lockless;
use super::drift_state::{machine_state_dirs, refuse_out_of_scope};
use crate::core::{state, types};
use crate::tripwire::drift;
use std::path::Path;

/// Records what a drift check would inspect on one machine: in JSON mode each
/// resource is appended to `checks`, otherwise the machine and its resources are
/// printed. Returns the number of resources accounted for.
fn record_dry_run_checks(
    name: &str,
    lock: &types::StateLock,
    json: bool,
    checks: &mut Vec<serde_json::Value>,
) -> usize {
    if !json {
        println!("Machine: {} ({} resources)", name, lock.resources.len());
    }
    for (res_id, res_state) in &lock.resources {
        if json {
            checks.push(serde_json::json!({
                "machine": name,
                "resource": res_id,
                "status": res_state.status,
                "hash": res_state.hash,
            }));
        } else {
            println!("  would check: {} (status: {})", res_id, res_state.status);
        }
    }
    lock.resources.len()
}

/// Emits the dry-run result: a JSON report, or a human-readable total.
pub(super) fn print_dry_run_report(
    json: bool,
    total: usize,
    checks: &[serde_json::Value],
) -> Result<(), String> {
    if json {
        let report = serde_json::json!({
            "dry_run": true,
            "total_checks": total,
            "checks": checks,
        });
        let output =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{output}");
    } else {
        println!();
        println!("Dry run: {total} resource(s) would be checked");
    }
    Ok(())
}

/// Dry-run mode for drift: lists resources that would be checked without connecting.
///
/// forjar#385: takes the config, because with an ABSENT state dir the preview
/// has to come from the same place the run does. A preview that dies where the
/// run succeeds is a worse answer than no preview at all.
pub(crate) fn cmd_drift_dry_run(
    config: Option<&types::ForjarConfig>,
    state_dir: &Path,
    machine_filter: Option<&str>,
    scope: Option<&[String]>,
    json: bool,
    no_task_checks: bool,
) -> Result<(), String> {
    // forjar#488: THE PREVIEW REFUSES WHAT THE RUN REFUSES.
    //
    // The scope guard lived in `collect_machine_locks`, which the preview does
    // not go through — it calls `machine_state_dirs` directly. So
    // `drift --dry-run -m <undeclared>` scanned zero machines and printed
    // "0 resource(s) would be checked", exit 0: the same false green the guard
    // exists to prevent, in the command an operator reaches for FIRST when
    // they are unsure. Found by two of three review lanes independently.
    refuse_out_of_scope(machine_filter, scope)?;
    let Some(names) = machine_state_dirs(state_dir, machine_filter, scope)? else {
        let opts = drift::DriftOptions {
            run_task_checks: !no_task_checks,
            ..drift::DriftOptions::default()
        };
        return dry_run_lockless(state_dir, machine_filter, config, json, opts);
    };
    let mut checks: Vec<serde_json::Value> = Vec::new();
    let mut total = 0usize;

    for name in names {
        if let Some(lock) = state::load_lock(state_dir, &name)? {
            total += record_dry_run_checks(&name, &lock, json, &mut checks);
        }
    }

    print_dry_run_report(json, total, &checks)
}
