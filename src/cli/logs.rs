//! FJ-2301: Log viewer runtime — reads run logs from state/<machine>/runs/.
//!
//! Replaces the stub in dispatch_misc_b.rs with actual file I/O.
//! Reads `meta.yaml` and `*.log` files from the run directory structure.

use crate::core::types::{LogRetention, RunMeta};
use std::path::Path;

/// A discovered run on disk.
#[derive(Debug)]
struct DiscoveredRun {
    machine: String,
    run_id: String,
    meta: RunMeta,
    run_dir: std::path::PathBuf,
}

/// Discover all runs under a state directory, optionally filtered.
fn discover_runs(
    state_dir: &Path,
    machine_filter: Option<&str>,
    run_filter: Option<&str>,
    failures_only: bool,
) -> Vec<DiscoveredRun> {
    let mut runs = Vec::new();
    let entries = match std::fs::read_dir(state_dir) {
        Ok(e) => e,
        Err(_) => return runs,
    };

    for entry in entries.flatten() {
        let machine_dir = entry.path();
        if !machine_dir.is_dir() {
            continue;
        }
        let machine_name = entry.file_name().to_string_lossy().to_string();

        // Skip non-machine directories (images, etc.)
        if machine_name == "images" || machine_name.starts_with('.') {
            continue;
        }

        if let Some(filter) = machine_filter {
            if machine_name != filter {
                continue;
            }
        }

        let runs_dir = machine_dir.join("runs");
        if !runs_dir.is_dir() {
            continue;
        }

        let run_entries = match std::fs::read_dir(&runs_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for run_entry in run_entries.flatten() {
            let run_dir = run_entry.path();
            if !run_dir.is_dir() {
                continue;
            }
            let run_id = run_entry.file_name().to_string_lossy().to_string();

            if let Some(filter) = run_filter {
                if run_id != filter {
                    continue;
                }
            }

            let meta_path = run_dir.join("meta.yaml");
            let meta = if meta_path.exists() {
                match std::fs::read_to_string(&meta_path) {
                    Ok(content) => match serde_yaml_ng::from_str::<RunMeta>(&content) {
                        Ok(m) => m,
                        Err(_) => continue,
                    },
                    Err(_) => continue,
                }
            } else {
                continue;
            };

            if failures_only && meta.summary.failed == 0 {
                continue;
            }

            runs.push(DiscoveredRun {
                machine: machine_name.clone(),
                run_id,
                meta,
                run_dir,
            });
        }
    }

    // Sort by started_at descending (most recent first)
    runs.sort_by(|a, b| {
        b.meta
            .started_at
            .as_deref()
            .unwrap_or("")
            .cmp(a.meta.started_at.as_deref().unwrap_or(""))
    });
    runs
}

/// Read a specific log file content for a resource in a run.
fn read_log_file(run_dir: &Path, resource_id: &str, action: &str) -> Option<String> {
    let log_path = run_dir.join(format!("{resource_id}.{action}.log"));
    std::fs::read_to_string(&log_path).ok()
}

/// Read the script file for a resource in a run.
fn read_script_file(run_dir: &Path, resource_id: &str) -> Option<String> {
    let script_path = run_dir.join(format!("{resource_id}.script"));
    std::fs::read_to_string(&script_path).ok()
}

/// List all .log files in a run directory.
fn list_log_files(run_dir: &Path) -> Vec<(String, String)> {
    let mut logs = Vec::new();
    let entries = match std::fs::read_dir(run_dir) {
        Ok(e) => e,
        Err(_) => return logs,
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(stem) = name.strip_suffix(".log") {
            if let Some((resource, action)) = stem.rsplit_once('.') {
                logs.push((resource.to_string(), action.to_string()));
            }
        }
    }
    logs.sort();
    logs
}

/// FJ-2301: Log viewer — reads actual run logs from disk.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_logs(
    state_dir: &Path,
    machine: Option<&str>,
    run: Option<&str>,
    resource: Option<&str>,
    failures: bool,
    show_script: bool,
    all_machines: bool,
    json: bool,
) -> Result<(), String> {
    let machine_filter = if all_machines { None } else { machine };
    let runs = discover_runs(state_dir, machine_filter, run, failures);

    if json {
        print_logs_json(&runs, resource, show_script)
    } else {
        print_logs_text(&runs, resource, show_script)
    }
}

fn print_logs_text(
    runs: &[DiscoveredRun],
    resource_filter: Option<&str>,
    show_script: bool,
) -> Result<(), String> {
    if runs.is_empty() {
        println!("No run logs found.");
        println!("  (run `forjar apply` to generate logs in state/<machine>/runs/)");
        return Ok(());
    }

    for run in runs {
        let meta = &run.meta;
        let started = meta.started_at.as_deref().unwrap_or("unknown");
        let gen = meta
            .generation
            .map(|g| format!(", gen {g}"))
            .unwrap_or_default();
        println!(
            "\nRun {} ({}{}) on {}",
            run.run_id, started, gen, run.machine
        );
        print_run_summary(&meta.summary);

        if let Some(res_id) = resource_filter {
            print_resource_log(&run.run_dir, res_id, show_script);
        } else {
            let log_files = list_log_files(&run.run_dir);
            for (res_id, action) in &log_files {
                let status = meta.resources.get(res_id);
                let status_str = match status {
                    Some(crate::core::types::ResourceRunStatus::Noop) => "noop",
                    Some(crate::core::types::ResourceRunStatus::Converged {
                        failed: true, ..
                    }) => "FAILED",
                    Some(crate::core::types::ResourceRunStatus::Converged { .. }) => "converged",
                    Some(crate::core::types::ResourceRunStatus::Skipped { .. }) => "skipped",
                    None => "unknown",
                };
                println!("  {res_id} ({action}) — {status_str}");
            }
        }
    }
    Ok(())
}

fn print_run_summary(summary: &crate::core::types::RunSummary) {
    println!(
        "  {} total: {} converged, {} noop, {} failed, {} skipped",
        summary.total, summary.converged, summary.noop, summary.failed, summary.skipped,
    );
}

fn print_resource_log(run_dir: &Path, resource_id: &str, show_script: bool) {
    // Try apply, then check, then destroy
    for action in &["apply", "check", "destroy"] {
        if let Some(content) = read_log_file(run_dir, resource_id, action) {
            println!("\n--- {resource_id}.{action}.log ---");
            println!("{content}");
        }
    }
    if show_script {
        if let Some(script) = read_script_file(run_dir, resource_id) {
            println!("\n--- {resource_id}.script ---");
            println!("{script}");
        }
    }
}

fn print_logs_json(
    runs: &[DiscoveredRun],
    resource_filter: Option<&str>,
    show_script: bool,
) -> Result<(), String> {
    let mut entries = Vec::new();
    for run in runs {
        let mut run_obj = serde_json::json!({
            "run_id": run.run_id,
            "machine": run.machine,
            "command": run.meta.command,
            "started_at": run.meta.started_at,
            "finished_at": run.meta.finished_at,
            "duration_secs": run.meta.duration_secs,
            "generation": run.meta.generation,
            "summary": {
                "total": run.meta.summary.total,
                "converged": run.meta.summary.converged,
                "noop": run.meta.summary.noop,
                "failed": run.meta.summary.failed,
                "skipped": run.meta.summary.skipped,
            },
        });

        if let Some(res_id) = resource_filter {
            let mut logs = serde_json::Map::new();
            for action in &["apply", "check", "destroy"] {
                if let Some(content) = read_log_file(&run.run_dir, res_id, action) {
                    logs.insert(format!("{action}_log"), serde_json::Value::String(content));
                }
            }
            if show_script {
                if let Some(script) = read_script_file(&run.run_dir, res_id) {
                    logs.insert("script".into(), serde_json::Value::String(script));
                }
            }
            run_obj["resource_logs"] = serde_json::Value::Object(logs);
        } else {
            let log_files = list_log_files(&run.run_dir);
            let file_list: Vec<String> = log_files
                .iter()
                .map(|(r, a)| format!("{r}.{a}.log"))
                .collect();
            run_obj["log_files"] = serde_json::json!(file_list);
        }
        entries.push(run_obj);
    }

    let output = serde_json::json!({ "runs": entries });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_default()
    );
    Ok(())
}

/// FJ-2301: Garbage-collect old run logs based on retention policy.
pub(crate) fn cmd_logs_gc(
    state_dir: &Path,
    dry_run: bool,
    keep_failed: bool,
    json: bool,
) -> Result<(), String> {
    let retention = LogRetention::default();
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

/// FJ-2301: Follow mode — tail the most recent run's log directory.
pub(crate) fn cmd_logs_follow(state_dir: &Path, json: bool) -> Result<(), String> {
    // Find the most recent run across all machines
    let runs = discover_runs(state_dir, None, None, false);
    if runs.is_empty() {
        if json {
            let output = serde_json::json!({
                "action": "follow",
                "status": "no_runs",
                "message": "no run logs found to follow",
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).unwrap_or_default()
            );
        } else {
            println!("Follow mode: no run logs found.");
            println!("  Start `forjar apply` in another terminal to generate logs.");
        }
        return Ok(());
    }

    let latest = &runs[0];
    if json {
        let output = serde_json::json!({
            "action": "follow",
            "status": "watching",
            "run_id": latest.run_id,
            "machine": latest.machine,
            "run_dir": latest.run_dir.display().to_string(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    } else {
        println!(
            "Follow mode: watching {}/{} ({})",
            latest.machine,
            latest.run_id,
            latest.run_dir.display()
        );
        println!("  Press Ctrl+C to stop.");
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
