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

    // Group by machine
    let mut by_machine: std::collections::HashMap<String, Vec<&DiscoveredRun>> =
        std::collections::HashMap::new();
    for run in &runs {
        by_machine.entry(run.machine.clone()).or_default().push(run);
    }

    let mut total_cleaned = 0u64;
    let mut total_deleted = 0u32;

    for (machine, machine_runs) in &by_machine {
        let to_keep = retention.keep_runs as usize;
        if machine_runs.len() <= to_keep {
            continue;
        }

        for run in machine_runs.iter().skip(to_keep) {
            if keep_failed && run.meta.summary.failed > 0 {
                continue;
            }
            let size = dir_size(&run.run_dir);
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
            total_cleaned += size;
            total_deleted += 1;
        }
    }

    if json {
        let output = serde_json::json!({
            "action": if dry_run { "dry_run" } else { "gc" },
            "state_dir": state_dir.display().to_string(),
            "deleted_runs": total_deleted,
            "freed_bytes": total_cleaned,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    } else if total_deleted == 0 {
        println!(
            "Log garbage collection: nothing to clean (within retention: {} runs/machine)",
            retention.keep_runs
        );
    } else {
        let verb = if dry_run { "would delete" } else { "deleted" };
        println!(
            "Log garbage collection: {} {} runs, {} bytes freed",
            verb, total_deleted, total_cleaned
        );
    }
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
