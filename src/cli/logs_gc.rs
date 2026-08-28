//! `forjar logs --gc` — run-log retention.
//!
//! Split out of `logs.rs` to keep both modules under the 500-line gate.
//!
//! Dogfood #208 (`logs-gc-deletes-arbitrary-runs-including-the-newest`): the
//! retention cut depends entirely on `discover_runs` returning a TOTAL order
//! newest-first. See `logs::sort_runs_newest_first`.

use super::logs::{discover_runs, DiscoveredRun};
use crate::core::types::LogRetention;
use std::path::Path;

/// Runs bucketed by machine, keeping `discover_runs`' newest-first order within
/// each bucket — the retention cut is "everything after the first N", so that
/// order is load-bearing (see the module docs).
fn group_runs_by_machine(
    runs: &[DiscoveredRun],
) -> std::collections::HashMap<String, Vec<&DiscoveredRun>> {
    let mut by_machine: std::collections::HashMap<String, Vec<&DiscoveredRun>> =
        std::collections::HashMap::new();
    for run in runs {
        by_machine.entry(run.machine.clone()).or_default().push(run);
    }
    by_machine
}

/// Frees one run: deleted outright, or merely named when this is a dry run.
fn discard_run(machine: &str, run: &DiscoveredRun, size: u64, dry_run: bool, json: bool) {
    if dry_run {
        if !json {
            println!(
                "  would delete: {}/{} ({} bytes)",
                machine, run.run_id, size
            );
        }
    } else {
        let _ = std::fs::remove_dir_all(&run.run_dir);
    }
}

/// Applies the retention cut to one machine: everything past the newest
/// `keep_runs` goes, except failed runs when `keep_failed` is set. Returns
/// (bytes freed, runs discarded).
fn gc_machine_runs(
    machine: &str,
    machine_runs: &[&DiscoveredRun],
    keep_runs: usize,
    dry_run: bool,
    keep_failed: bool,
    json: bool,
) -> (u64, u32) {
    let mut cleaned = 0u64;
    let mut deleted = 0u32;
    for run in machine_runs.iter().skip(keep_runs) {
        if keep_failed && run.meta.summary.failed > 0 {
            continue;
        }
        let size = dir_size(&run.run_dir);
        discard_run(machine, run, size, dry_run, json);
        cleaned += size;
        deleted += 1;
    }
    (cleaned, deleted)
}

/// Reports the outcome of a GC pass as JSON, as "nothing to clean", or as a
/// count of what was (or would be) removed.
fn print_gc_report(
    state_dir: &Path,
    dry_run: bool,
    json: bool,
    deleted: u32,
    cleaned: u64,
    keep_runs: u32,
) {
    if json {
        let output = serde_json::json!({
            "action": if dry_run { "dry_run" } else { "gc" },
            "state_dir": state_dir.display().to_string(),
            "deleted_runs": deleted,
            "freed_bytes": cleaned,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    } else if deleted == 0 {
        println!(
            "Log garbage collection: nothing to clean (within retention: {keep_runs} runs/machine)"
        );
    } else {
        let verb = if dry_run { "would delete" } else { "deleted" };
        println!("Log garbage collection: {verb} {deleted} runs, {cleaned} bytes freed");
    }
}

/// FJ-2301: Garbage-collect old run logs based on retention policy.
pub(crate) fn cmd_logs_gc(
    state_dir: &Path,
    dry_run: bool,
    keep_failed: bool,
    json: bool,
    retention: Option<&LogRetention>,
) -> Result<(), String> {
    let default_retention = LogRetention::default();
    let retention = retention.unwrap_or(&default_retention);
    let runs = discover_runs(state_dir, None, None, false);

    let mut total_cleaned = 0u64;
    let mut total_deleted = 0u32;

    for (machine, machine_runs) in &group_runs_by_machine(&runs) {
        let (cleaned, deleted) = gc_machine_runs(
            machine,
            machine_runs,
            retention.keep_runs as usize,
            dry_run,
            keep_failed,
            json,
        );
        total_cleaned += cleaned;
        total_deleted += deleted;
    }

    print_gc_report(
        state_dir,
        dry_run,
        json,
        total_deleted,
        total_cleaned,
        retention.keep_runs,
    );
    Ok(())
}

fn dir_size(path: &Path) -> u64 {
    let mut size = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                size += meta.len();
            }
        }
    }
    size
}
