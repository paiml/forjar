//! Resource detail.

use super::helpers::*;
use crate::core::types;
use std::collections::HashMap;
use std::path::Path;

/// FJ-593: Show resource timeline from event log.
pub(crate) fn cmd_status_resource_timeline(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut events: Vec<(String, String, String, String)> = Vec::new(); // (timestamp, machine, resource, status)

    for m in &machines {
        if let Some(filter) = machine {
            if m != filter {
                continue;
            }
        }
        let log_path = state_dir.join(m).join("events.jsonl");
        if !log_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&log_path).unwrap_or_default();
        for line in content.lines() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                let ts = val
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let resource = val
                    .get("resource")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let status = val
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                events.push((ts, m.clone(), resource, status));
            }
        }
    }

    if json {
        let items: Vec<String> = events
            .iter()
            .map(|(ts, m, r, s)| {
                format!(r#"{{"timestamp":"{ts}","machine":"{m}","resource":"{r}","status":"{s}"}}"#)
            })
            .collect();
        println!(
            r#"{{"timeline":[{}],"count":{}}}"#,
            items.join(","),
            events.len()
        );
    } else if events.is_empty() {
        println!("No timeline events found");
    } else {
        println!("Resource timeline ({} events):", events.len());
        for (ts, m, r, s) in &events {
            println!("  [{ts}] {m}:{r} — {s}");
        }
    }
    Ok(())
}

/// FJ-627: Show runtime dependency graph from lock files.
pub(crate) fn cmd_status_resource_dependencies(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut deps: Vec<(String, String, usize)> = Vec::new(); // (machine, resource, dep_count)

    for m in &machines {
        if let Some(filter) = machine {
            if m != filter {
                continue;
            }
        }
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
            for (rname, rl) in &lock.resources {
                deps.push((m.clone(), rname.clone(), rl.details.len()));
            }
        }
    }

    if json {
        let items: Vec<String> = deps
            .iter()
            .map(|(m, r, d)| format!(r#"{{"machine":"{m}","resource":"{r}","dependencies":{d}}}"#))
            .collect();
        println!(
            r#"{{"resource_dependencies":[{}],"count":{}}}"#,
            items.join(","),
            deps.len()
        );
    } else if deps.is_empty() {
        println!("No resource dependency data available");
    } else {
        println!("Resource dependencies ({} resources):", deps.len());
        for (m, r, d) in &deps {
            println!("  {m}:{r} — {d} detail(s)");
        }
    }
    Ok(())
}

/// FJ-712: Show resource input fields per resource
fn collect_resource_inputs(
    state_dir: &Path,
    targets: &[&String],
) -> Vec<(String, String, String, usize)> {
    let mut result = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        if let Ok(data) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&data) {
                for (name, rl) in &lock.resources {
                    result.push((
                        m.to_string(),
                        name.clone(),
                        format!("{:?}", rl.resource_type),
                        rl.details.len(),
                    ));
                }
            }
        }
    }
    result
}

pub(crate) fn cmd_status_resource_inputs(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let inputs = collect_resource_inputs(state_dir, &targets);
    if json {
        let entries: Vec<String> = inputs
            .iter()
            .map(|(m, name, rtype, count)| {
                format!(
                    "{{\"machine\":\"{m}\",\"resource\":\"{name}\",\"type\":\"{rtype}\",\"input_count\":{count}}}"
                )
            })
            .collect();
        println!("{{\"resource_inputs\":[{}]}}", entries.join(","));
    } else {
        println!("Resource inputs:");
        let mut current_machine = String::new();
        for (m, name, rtype, count) in &inputs {
            if *m != current_machine {
                println!("  Machine: {m}");
                current_machine = m.clone();
            }
            println!("    {name} ({rtype}) — {count} input(s)");
        }
    }
    Ok(())
}

/// FJ-727: Show count per resource type
fn tally_resource_types(state_dir: &Path, targets: &[&String]) -> Vec<(String, usize)> {
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for m in targets {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        if let Ok(data) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&data) {
                for rl in lock.resources.values() {
                    let rtype = format!("{:?}", rl.resource_type);
                    *type_counts.entry(rtype).or_default() += 1;
                }
            }
        }
    }
    let mut sorted: Vec<_> = type_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    sorted
}

pub(crate) fn cmd_status_resource_types_summary(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let sorted = tally_resource_types(state_dir, &targets);
    if json {
        let entries: Vec<String> = sorted
            .iter()
            .map(|(t, c)| format!("{{\"type\":\"{t}\",\"count\":{c}}}"))
            .collect();
        println!("{{\"resource_types\":[{}]}}", entries.join(","));
    } else if sorted.is_empty() {
        println!("No resources found.");
    } else {
        println!("Resource types summary:");
        for (rtype, count) in &sorted {
            println!("  {rtype} — {count}");
        }
    }
    Ok(())
}

/// Collect per-resource health entries from lock files.
fn collect_resource_health(state_dir: &Path, targets: &[&String]) -> Vec<(String, String, String)> {
    let mut entries: Vec<(String, String, String)> = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        if let Ok(data) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&data) {
                for (name, rl) in &lock.resources {
                    entries.push((m.to_string(), name.clone(), format!("{:?}", rl.status)));
                }
            }
        }
    }
    entries
}

/// FJ-732: Show per-resource health status (converged/failed/drifted).
pub(crate) fn cmd_status_resource_health(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let entries = collect_resource_health(state_dir, &targets);
    if json {
        let items: Vec<String> = entries
            .iter()
            .map(|(m, n, s)| {
                format!("{{\"machine\":\"{m}\",\"resource\":\"{n}\",\"status\":\"{s}\"}}")
            })
            .collect();
        println!("{{\"resource_health\":[{}]}}", items.join(","));
    } else if entries.is_empty() {
        println!("No resources found.");
    } else {
        println!("Resource health ({} resources):", entries.len());
        for (m, name, status) in &entries {
            println!("  [{m}] {name} — {status}");
        }
    }
    Ok(())
}

/// Tally status counts for a single machine lock file.
pub(crate) fn tally_machine_health(
    state_dir: &Path,
    machine: &str,
) -> (usize, usize, usize, usize) {
    let lock_path = state_dir.join(format!("{machine}.lock.yaml"));
    let (mut total, mut converged, mut failed, mut drifted) = (0, 0, 0, 0);
    if let Ok(data) = std::fs::read_to_string(&lock_path) {
        if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&data) {
            for rl in lock.resources.values() {
                total += 1;
                match format!("{:?}", rl.status).as_str() {
                    "Converged" => converged += 1,
                    "Failed" => failed += 1,
                    "Drifted" => drifted += 1,
                    _ => {}
                }
            }
        }
    }
    (total, converged, failed, drifted)
}

/// FJ-737: Show per-machine health overview.
pub(crate) fn cmd_status_machine_health_summary(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let summaries: Vec<(String, usize, usize, usize, usize)> = targets
        .iter()
        .map(|m| {
            let (t, c, f, d) = tally_machine_health(state_dir, m);
            (m.to_string(), t, c, f, d)
        })
        .collect();
    if json {
        let items: Vec<String> = summaries
            .iter()
            .map(|(m, t, c, f, d)| {
                format!("{{\"machine\":\"{m}\",\"total\":{t},\"converged\":{c},\"failed\":{f},\"drifted\":{d}}}")
            })
            .collect();
        println!("{{\"machine_health\":[{}]}}", items.join(","));
    } else if summaries.is_empty() {
        println!("No machines found.");
    } else {
        println!("Machine health summary:");
        for (m, total, converged, failed, drifted) in &summaries {
            let pct = if *total > 0 {
                *converged * 100 / *total
            } else {
                0
            };
            println!(
                "  {m} — {total} total, {converged} converged ({pct}%), {failed} failed, {drifted} drifted"
            );
        }
    }
    Ok(())
}

/// FJ-746: Show last apply success/failure per machine.
pub(crate) fn cmd_status_last_apply_status(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let statuses = collect_last_apply(state_dir, &targets);
    if json {
        let items: Vec<String> = statuses
            .iter()
            .map(|(m, s, t)| {
                format!("{{\"machine\":\"{m}\",\"status\":\"{s}\",\"timestamp\":\"{t}\"}}")
            })
            .collect();
        println!("{{\"last_apply_status\":[{}]}}", items.join(","));
    } else if statuses.is_empty() {
        println!("No apply history found.");
    } else {
        println!("Last apply status:");
        for (m, s, t) in &statuses {
            println!("  {m} — {s} ({t})");
        }
    }
    Ok(())
}

/// Extract timestamp prefix from an event line.
fn extract_ts(line: &str) -> String {
    line.split_whitespace().next().unwrap_or("—").to_string()
}

/// Parse last apply status from event log content.
fn parse_last_apply(data: &str) -> (String, String) {
    let last_line = data.lines().rev().find(|l| l.contains("apply"));
    match last_line {
        Some(l) if l.contains("success") => ("success".to_string(), extract_ts(l)),
        Some(l) if l.contains("fail") => ("failed".to_string(), extract_ts(l)),
        _ => ("unknown".to_string(), "—".to_string()),
    }
}

/// Read last apply status from event log for each machine.
fn collect_last_apply(state_dir: &Path, targets: &[&String]) -> Vec<(String, String, String)> {
    let mut results = Vec::new();
    for m in targets {
        let log_path = state_dir.join(m).join("events.jsonl");
        let (status, ts) = match std::fs::read_to_string(&log_path) {
            Ok(data) => parse_last_apply(&data),
            Err(_) => ("no_history".to_string(), "—".to_string()),
        };
        results.push((m.to_string(), status, ts));
    }
    results
}

/// FJ-748: Show time since last successful apply per resource.
pub(crate) fn cmd_status_resource_staleness(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let entries = collect_staleness(state_dir, &targets);
    if json {
        let items: Vec<String> = entries
            .iter()
            .map(|(m, n, ts)| {
                format!("{{\"machine\":\"{m}\",\"resource\":\"{n}\",\"last_apply\":\"{ts}\"}}")
            })
            .collect();
        println!("{{\"resource_staleness\":[{}]}}", items.join(","));
    } else if entries.is_empty() {
        println!("No resources found.");
    } else {
        println!("Resource staleness:");
        for (m, name, ts) in &entries {
            println!("  [{m}] {name} — last apply: {ts}");
        }
    }
    Ok(())
}

/// Collect last apply timestamp per resource from lock files.
fn collect_staleness(state_dir: &Path, targets: &[&String]) -> Vec<(String, String, String)> {
    let mut entries = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        if let Ok(data) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&data) {
                for (name, rl) in &lock.resources {
                    let ts = rl.applied_at.as_deref().unwrap_or("never");
                    entries.push((m.to_string(), name.clone(), ts.to_string()));
                }
            }
        }
    }
    entries
}
