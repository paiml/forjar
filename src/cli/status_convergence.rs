//! Convergence analytics.

use super::helpers::*;
use super::helpers_time::*;
use crate::core::{state, types};
use std::path::Path;

// FJ-364: Status timeline
pub(crate) fn cmd_status_timeline(
    state_dir: &Path,
    machine_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let entries =
        std::fs::read_dir(state_dir).map_err(|e| format!("cannot read state dir: {e}"))?;

    let mut timeline: Vec<serde_json::Value> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter) = machine_filter {
            if name != filter {
                continue;
            }
        }
        if !entry.path().is_dir() {
            continue;
        }
        if let Some(lock) = state::load_lock(state_dir, &name)? {
            for (id, rl) in &lock.resources {
                timeline.push(serde_json::json!({
                    "resource": id,
                    "machine": lock.machine,
                    "status": format!("{:?}", rl.status),
                    "applied_at": rl.applied_at,
                    "duration_seconds": rl.duration_seconds,
                }));
            }
        }
    }

    timeline.sort_by(|a, b| {
        let ta = a["applied_at"].as_str().unwrap_or("");
        let tb = b["applied_at"].as_str().unwrap_or("");
        ta.cmp(tb)
    });

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&timeline).unwrap_or_else(|_| "[]".to_string())
        );
    } else {
        print_timeline_text(&timeline);
    }

    Ok(())
}

/// Print timeline entries in text format.
fn print_timeline_text(timeline: &[serde_json::Value]) {
    println!("Convergence Timeline:\n");
    if timeline.is_empty() {
        println!("  (no data)");
        return;
    }
    for t in timeline {
        let status_icon = match t["status"].as_str().unwrap_or("") {
            "Converged" => green("✓"),
            "Failed" => red("✗"),
            "Drifted" => yellow("~"),
            _ => dim("?"),
        };
        println!(
            "  {} {} {} on {} ({}s)",
            t["applied_at"].as_str().unwrap_or("-"),
            status_icon,
            t["resource"].as_str().unwrap_or("?"),
            t["machine"].as_str().unwrap_or("?"),
            t["duration_seconds"]
                .as_f64()
                .map(|d| format!("{d:.2}"))
                .unwrap_or_else(|| "-".to_string()),
        );
    }
}

// FJ-372: Show resources changed since a git commit
pub(crate) fn cmd_status_changes_since(
    state_dir: &Path,
    commit: &str,
    json: bool,
) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .args([
            "diff",
            "--name-only",
            commit,
            "--",
            &state_dir.display().to_string(),
        ])
        .output()
        .map_err(|e| format!("git diff failed: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let changed: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&changed).unwrap_or_else(|_| "[]".to_string())
        );
    } else {
        println!("Resources changed since {}:\n", bold(commit));
        if changed.is_empty() {
            println!("  {} No changes.", green("✓"));
        } else {
            for c in &changed {
                println!("  {} {}", yellow("~"), c);
            }
        }
    }

    Ok(())
}

// ── FJ-437: status --since ──

/// Collect resources modified within a time window.
fn collect_recent_resources(
    state_dir: &Path,
    machine: Option<&str>,
    cutoff: u64,
) -> Result<Vec<(String, String, String)>, String> {
    let mut results = Vec::new();
    if !state_dir.exists() {
        return Ok(results);
    }
    let entries = std::fs::read_dir(state_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let m_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if m_name.starts_with('.') {
            continue;
        }
        if let Some(filter) = machine {
            if m_name != filter {
                continue;
            }
        }
        collect_recent_from_machine(state_dir, &m_name, &path, cutoff, &mut results)?;
    }
    Ok(results)
}

/// Collect recent resources for a single machine.
fn collect_recent_from_machine(
    state_dir: &Path,
    m_name: &str,
    path: &Path,
    cutoff: u64,
    results: &mut Vec<(String, String, String)>,
) -> Result<(), String> {
    let lock_path = path.join("lock.yaml");
    let meta = match std::fs::metadata(&lock_path) {
        Ok(m) => m,
        Err(_) => return Ok(()),
    };
    let mod_secs = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if mod_secs < cutoff {
        return Ok(());
    }
    if let Ok(Some(lock)) = state::load_lock(state_dir, m_name) {
        for (rname, rl) in &lock.resources {
            results.push((
                m_name.to_string(),
                rname.clone(),
                format!("{:?}", rl.status),
            ));
        }
    }
    Ok(())
}

pub(crate) fn cmd_status_since(
    state_dir: &Path,
    machine: Option<&str>,
    duration: &str,
    json: bool,
) -> Result<(), String> {
    let seconds = parse_duration_secs(duration)?;
    let cutoff = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(seconds);

    let results = collect_recent_resources(state_dir, machine, cutoff)?;

    if json {
        let items: Vec<String> = results
            .iter()
            .map(|(m, r, s)| {
                format!(
                    "{{\"machine\":\"{m}\",\"resource\":\"{r}\",\"status\":\"{s}\"}}"
                )
            })
            .collect();
        println!("[{}]", items.join(","));
    } else if results.is_empty() {
        println!("No resources changed within {duration}.");
    } else {
        println!("{} resource(s) changed within {}:", results.len(), duration);
        for (m, r, s) in &results {
            println!("  {m}/{r}: {s}");
        }
    }
    Ok(())
}

// FJ-376: Status summary-by dimension
/// Resolve the grouping key for a resource lock entry.
fn summary_dimension_key(
    dimension: &str,
    lock: &types::StateLock,
    rl: &types::ResourceLock,
) -> Result<String, String> {
    match dimension {
        "machine" => Ok(lock.machine.clone()),
        "type" => Ok(format!("{:?}", rl.resource_type)),
        "status" => Ok(format!("{:?}", rl.status)),
        _ => Err(format!(
            "Unknown dimension '{dimension}'. Use: machine, type, status"
        )),
    }
}

pub(crate) fn cmd_status_summary_by(
    state_dir: &Path,
    machine_filter: Option<&str>,
    dimension: &str,
    json: bool,
) -> Result<(), String> {
    let entries =
        std::fs::read_dir(state_dir).map_err(|e| format!("cannot read state dir: {e}"))?;

    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter) = machine_filter {
            if name != filter {
                continue;
            }
        }
        if !entry.path().is_dir() {
            continue;
        }
        if let Some(lock) = state::load_lock(state_dir, &name)? {
            for (id, rl) in &lock.resources {
                let key = summary_dimension_key(dimension, &lock, rl)?;
                groups.entry(key).or_default().push(id.clone());
            }
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&groups).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("Summary by {}:\n", bold(dimension));
        for (group, resources) in &groups {
            println!("  {} ({}):", bold(group), resources.len());
            for r in resources {
                println!("    {r}");
            }
        }
    }

    Ok(())
}

// ── FJ-487: status --convergence-rate ──

/// Count total and converged resources across machines.
fn count_convergence(state_dir: &Path, machines: &[String]) -> Result<(usize, usize), String> {
    let mut total = 0usize;
    let mut converged = 0usize;
    for m in machines {
        if let Some(lock) = state::load_lock(state_dir, m).map_err(|e| e.to_string())? {
            for rl in lock.resources.values() {
                total += 1;
                if rl.status == types::ResourceStatus::Converged {
                    converged += 1;
                }
            }
        }
    }
    Ok((total, converged))
}

pub(crate) fn cmd_status_convergence_rate(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let all_machines = discover_machines(state_dir);
    let machines: Vec<String> = if let Some(m) = machine {
        all_machines.into_iter().filter(|n| n == m).collect()
    } else {
        all_machines
    };
    let (total, converged) = count_convergence(state_dir, &machines)?;
    let rate = if total > 0 {
        (converged as f64 / total as f64 * 100.0).round()
    } else {
        100.0
    };
    if json {
        let result = serde_json::json!({
            "convergence_rate": rate,
            "converged": converged,
            "total": total,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else {
        let indicator = if rate >= 90.0 {
            green("✓")
        } else if rate >= 50.0 {
            yellow("⚠")
        } else {
            red("✗")
        };
        println!(
            "{indicator} Convergence rate: {rate:.0}% ({converged}/{total})"
        );
    }
    Ok(())
}

/// Collect convergence duration data per resource.
fn collect_convergence_times(
    state_dir: &Path,
    machines: &[String],
    machine: Option<&str>,
) -> Vec<(String, String, f64)> {
    let mut times = Vec::new();
    for m in machines {
        if let Some(filter) = machine {
            if m != filter {
                continue;
            }
        }
        let lock_path = state_dir.join(format!("{m}.lock.yaml"));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
            for (rname, rlock) in &lock.resources {
                if let Some(duration) = rlock.duration_seconds {
                    times.push((m.clone(), rname.clone(), duration));
                }
            }
        }
    }
    times
}

/// FJ-587: Show average time to convergence per resource.
pub(crate) fn cmd_status_convergence_time(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let times = collect_convergence_times(state_dir, &machines, machine);
    let avg = if !times.is_empty() {
        times.iter().map(|(_, _, d)| d).sum::<f64>() / times.len() as f64
    } else {
        0.0
    };

    if json {
        let items: Vec<String> = times
            .iter()
            .map(|(m, r, d)| {
                format!(
                    r#"{{"machine":"{m}","resource":"{r}","seconds":{d:.3}}}"#
                )
            })
            .collect();
        println!(
            r#"{{"convergence_times":[{}],"average":{:.3},"count":{}}}"#,
            items.join(","),
            avg,
            times.len()
        );
    } else if times.is_empty() {
        println!("No convergence time data available");
    } else {
        println!("Convergence times (avg: {avg:.3}s):");
        for (m, r, d) in &times {
            println!("  {m}:{r} — {d:.3}s");
        }
    }
    Ok(())
}

/// Compute convergence stats for a single machine lock.
fn compute_convergence_stats(lock: &types::StateLock) -> (usize, usize, f64) {
    let total = lock.resources.len();
    let converged = lock
        .resources
        .values()
        .filter(|r| format!("{:?}", r.status) == "Converged")
        .count();
    let rate = if total > 0 {
        converged as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    (total, converged, rate)
}

/// Load convergence stats for each target machine.
fn load_convergence_history(
    state_dir: &Path,
    targets: &[&String],
) -> Vec<(String, usize, usize, f64)> {
    let mut results = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{m}.lock.yaml"));
        if let Ok(data) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&data) {
                let (total, converged, rate) = compute_convergence_stats(&lock);
                results.push((m.to_string(), total, converged, rate));
            }
        }
    }
    results
}

/// FJ-707: Show convergence trend over time
pub(crate) fn cmd_status_convergence_history(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let history = load_convergence_history(state_dir, &targets);
    if json {
        let entries: Vec<String> = history
            .iter()
            .map(|(m, total, converged, rate)| {
                format!(
                    "{{\"machine\":\"{m}\",\"total\":{total},\"converged\":{converged},\"rate\":{rate:.1}}}"
                )
            })
            .collect();
        println!("{{\"convergence_history\":[{}]}}", entries.join(","));
    } else {
        println!("Convergence history:");
        for (m, total, converged, rate) in &history {
            println!("  {m} — {converged}/{total} converged ({rate:.1}%)");
        }
    }
    Ok(())
}
