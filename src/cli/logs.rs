//! FJ-2301: Log viewer runtime — reads run logs from state/<machine>/runs/.
//!
//! Replaces the stub in dispatch_misc_b.rs with actual file I/O.
//! Reads `meta.yaml` and `*.log` files from the run directory structure.

use crate::core::types::RunMeta;
use std::path::Path;

/// A discovered run on disk.
#[derive(Debug)]
pub(crate) struct DiscoveredRun {
    pub(crate) machine: String,
    pub(crate) run_id: String,
    pub(crate) meta: RunMeta,
    pub(crate) run_dir: std::path::PathBuf,
}

/// Modification time of a run directory, in nanoseconds since the epoch.
///
/// Dogfood #208: the retention sort must be a TOTAL order. `started_at` alone
/// has second resolution (and is absent on runs written by older forjars), so
/// runs tie, the stable sort falls back to readdir/hash order, and `--gc`
/// deletes an arbitrary subset. mtime breaks the tie deterministically.
fn run_mtime_nanos(dir: &Path) -> u128 {
    std::fs::metadata(dir)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Whether a directory directly under the state dir names a machine.
///
/// Decides the one exclusion the walk has always made: forjar's own `images`
/// cache and any dotfile directory are siblings of the machine dirs but hold
/// no runs. Named so the walk in `discover_runs` reads as "for each machine"
/// instead of "for each directory, minus these two special cases".
fn is_machine_dir(name: &str) -> bool {
    name != "images" && !name.starts_with('.')
}

/// The `meta.yaml` of a run directory, or `None` when it cannot be used.
///
/// Decides whether a directory under `runs/` is a run we can report on. All
/// three failure modes — no `meta.yaml`, an unreadable one, and one that is
/// not valid `RunMeta` — mean the same thing to every caller: skip it. Exists
/// to keep that three-deep exists/read/parse ladder out of the discovery loop.
fn read_run_meta(run_dir: &Path) -> Option<RunMeta> {
    let meta_path = run_dir.join("meta.yaml");
    if !meta_path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&meta_path).ok()?;
    serde_yaml_ng::from_str::<RunMeta>(&content).ok()
}

/// Every run recorded under one machine's `runs/` directory that survives the
/// `--run` and `--failures-only` filters, in readdir order.
///
/// Exists so each function owns a single level of the two-level walk:
/// `discover_runs` iterates machines, this iterates that machine's runs.
fn runs_for_machine(
    machine_dir: &Path,
    machine_name: &str,
    run_filter: Option<&str>,
    failures_only: bool,
) -> Vec<DiscoveredRun> {
    let runs_dir = machine_dir.join("runs");
    if !runs_dir.is_dir() {
        return Vec::new();
    }

    let Ok(run_entries) = std::fs::read_dir(&runs_dir) else {
        return Vec::new();
    };

    run_entries
        .flatten()
        .filter_map(|entry| discovered_run(&entry, machine_name, run_filter, failures_only))
        .collect()
}

/// One directory under `runs/` turned into a reportable run, or `None` when it
/// is not one: not a directory at all, excluded by `--run`, carrying no usable
/// `meta.yaml`, or filtered out by `--failures-only`. Exists so the walk above
/// reads as "every run that survives the filters" rather than as five reasons
/// to `continue`.
fn discovered_run(
    run_entry: &std::fs::DirEntry,
    machine_name: &str,
    run_filter: Option<&str>,
    failures_only: bool,
) -> Option<DiscoveredRun> {
    let run_dir = run_entry.path();
    if !run_dir.is_dir() {
        return None;
    }

    let run_id = run_entry.file_name().to_string_lossy().to_string();
    if run_filter.is_some_and(|filter| run_id != filter) {
        return None;
    }

    let meta = read_run_meta(&run_dir)?;
    if failures_only && meta.summary.failed == 0 {
        return None;
    }

    Some(DiscoveredRun {
        machine: machine_name.to_string(),
        run_id,
        meta,
        run_dir,
    })
}

/// Discover all runs under a state directory, optionally filtered.
pub(crate) fn discover_runs(
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
        if !is_machine_dir(&machine_name) {
            continue;
        }

        if let Some(filter) = machine_filter {
            if machine_name != filter {
                continue;
            }
        }

        runs.extend(runs_for_machine(
            &machine_dir,
            &machine_name,
            run_filter,
            failures_only,
        ));
    }

    sort_runs_newest_first(&mut runs);
    runs
}

/// Sort runs newest-first under a total order: started_at, then directory
/// mtime, then run id. Deterministic even when timestamps tie or are absent.
pub(crate) fn sort_runs_newest_first(runs: &mut [DiscoveredRun]) {
    let mtimes: std::collections::HashMap<String, u128> = runs
        .iter()
        .map(|r| (r.run_id.clone(), run_mtime_nanos(&r.run_dir)))
        .collect();
    runs.sort_by(|a, b| {
        let ka = (
            a.meta.started_at.as_deref().unwrap_or(""),
            mtimes.get(&a.run_id).copied().unwrap_or(0),
            a.run_id.as_str(),
        );
        let kb = (
            b.meta.started_at.as_deref().unwrap_or(""),
            mtimes.get(&b.run_id).copied().unwrap_or(0),
            b.run_id.as_str(),
        );
        kb.cmp(&ka)
    });
}

/// Read a specific log file content for a resource in a run.
fn read_log_file(run_dir: &Path, resource_id: &str, action: &str) -> Option<String> {
    let log_path = run_dir.join(format!("{resource_id}.{action}.log"));
    std::fs::read_to_string(&log_path).ok()
}

/// Actions actually recorded on disk for `resource_id` in this run.
///
/// Dogfood #208 (logs-resource-filter-drops-the-matching-resource): the filter
/// used to probe a hardcoded `apply`/`check`/`destroy` action list, but forjar
/// writes the PLANNED action (`create`, `update`, `delete`, …). Every probe
/// missed, so `--resource <existing>` was byte-identical to
/// `--resource <nonexistent>`: all rows suppressed, rc=0. Discover the actions
/// from the run directory instead of guessing them.
pub(crate) fn actions_for_resource(run_dir: &Path, resource_id: &str) -> Vec<String> {
    list_log_files(run_dir)
        .into_iter()
        .filter(|(res, _)| res == resource_id)
        .map(|(_, action)| action)
        .collect()
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

/// The one-word status shown beside a resource in `forjar logs`.
fn run_status_label(status: Option<&crate::core::types::ResourceRunStatus>) -> &'static str {
    match status {
        Some(crate::core::types::ResourceRunStatus::Noop) => "noop",
        Some(crate::core::types::ResourceRunStatus::Converged { failed: true, .. }) => "FAILED",
        Some(crate::core::types::ResourceRunStatus::Converged { .. }) => "converged",
        Some(crate::core::types::ResourceRunStatus::Skipped { .. }) => "skipped",
        None => "unknown",
    }
}

/// Dogfood #208 (logs-script-flag-noop): --script must add the executed script
/// to the output. It used to be byte-identical to plain `logs`.
fn print_recorded_script(run_dir: &Path, res_id: &str) {
    match read_script_file(run_dir, res_id) {
        Some(script) if !script.is_empty() => {
            println!("    --- {res_id}.script ---");
            for line in script.lines() {
                println!("    {line}");
            }
        }
        _ => println!("    (no script recorded)"),
    }
}

/// Print every resource log recorded for one run.
fn print_run_log_files(run: &DiscoveredRun, show_script: bool) {
    let log_files = list_log_files(&run.run_dir);
    for (res_id, action) in &log_files {
        let status_str = run_status_label(run.meta.resources.get(res_id));
        println!("  {res_id} ({action}) — {status_str}");
        if show_script {
            print_recorded_script(&run.run_dir, res_id);
        }
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
            print_run_log_files(run, show_script);
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
    let actions = actions_for_resource(run_dir, resource_id);
    if actions.is_empty() {
        println!("  (no log for resource '{resource_id}' in this run)");
        return;
    }
    for action in &actions {
        if let Some(content) = read_log_file(run_dir, resource_id, action) {
            println!("\n--- {resource_id}.{action}.log ---");
            println!("{content}");
        }
    }
    if show_script {
        match read_script_file(run_dir, resource_id) {
            Some(script) if !script.is_empty() => {
                println!("\n--- {resource_id}.script ---");
                println!("{script}");
            }
            _ => println!("  (no script recorded for '{resource_id}')"),
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
            if show_script {
                let mut scripts = serde_json::Map::new();
                for (res_id, _) in &log_files {
                    let body = read_script_file(&run.run_dir, res_id).unwrap_or_default();
                    scripts.insert(res_id.clone(), serde_json::Value::String(body));
                }
                run_obj["scripts"] = serde_json::Value::Object(scripts);
            }
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

/// FJ-2301: Follow mode — tail a run's log directory until interrupted.
///
/// Dogfood #208 (logs-follow-does-not-follow-and-ignores-run): this used to
/// print a "watching …" banner and return in ~5ms without streaming a byte,
/// and it resolved the target as "newest" before consulting `--run`, so
/// `--follow --run <older>` silently watched a different run.
pub(crate) fn cmd_logs_follow(
    state_dir: &Path,
    machine: Option<&str>,
    run: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let Some(target) = resolve_follow_target(state_dir, machine, run, json)? else {
        return Ok(());
    };
    super::logs_follow::tail_run_dir(
        &target.run_dir,
        json,
        &mut super::logs_follow::Forever,
        std::time::Duration::from_millis(400),
    );
    Ok(())
}

/// Resolve which run `--follow` should watch, and print the banner.
///
/// Dogfood #208: `--run` is consulted BEFORE "newest wins". An explicit run id
/// that matches nothing is an error, not a silent fallback to another run.
/// Returns `Ok(None)` when there is simply nothing to follow yet.
pub(crate) fn resolve_follow_target(
    state_dir: &Path,
    machine: Option<&str>,
    run: Option<&str>,
    json: bool,
) -> Result<Option<DiscoveredRun>, String> {
    let mut runs = discover_runs(state_dir, machine, run, false);
    if runs.is_empty() {
        if let Some(requested) = run {
            return Err(format!(
                "no run logs found for run id '{requested}' (see `forjar logs` for known run ids)"
            ));
        }
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
        return Ok(None);
    }

    let latest = runs.remove(0);
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
    Ok(Some(latest))
}
