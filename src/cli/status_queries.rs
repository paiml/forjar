//! Status query variants.

use super::helpers::*;
use super::status_core::*;
use crate::core::{state, types};
use std::path::Path;

/// Collect resource rows from state directory: (machine_name, resource_name, ResourceLock).
#[allow(clippy::type_complexity)]
fn collect_resources(
    state_dir: &Path,
    machine: Option<&str>,
) -> Result<Vec<(String, Vec<(String, types::ResourceLock)>)>, String> {
    let mut result = Vec::new();
    if !state_dir.exists() {
        return Ok(result);
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
        if let Some(filter) = machine {
            if m_name != filter {
                continue;
            }
        }
        if let Ok(Some(lock)) = state::load_lock(state_dir, &m_name) {
            let resources: Vec<(String, types::ResourceLock)> =
                lock.resources.into_iter().collect();
            result.push((m_name, resources));
        }
    }
    Ok(result)
}

// ── FJ-392: status --count ──

pub(crate) fn cmd_status_count(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let mut converged = 0usize;
    let mut failed = 0usize;
    let mut drifted = 0usize;
    let mut unknown = 0usize;

    let machines = collect_resources(state_dir, machine)?;
    for (_m_name, resources) in &machines {
        for (_name, rl) in resources {
            match rl.status {
                types::ResourceStatus::Converged => converged += 1,
                types::ResourceStatus::Failed => failed += 1,
                types::ResourceStatus::Drifted => drifted += 1,
                types::ResourceStatus::Unknown => unknown += 1,
            }
        }
    }

    let total = converged + failed + drifted + unknown;
    if json {
        println!(
            "{{\"total\":{},\"converged\":{},\"failed\":{},\"drifted\":{},\"unknown\":{}}}",
            total, converged, failed, drifted, unknown
        );
    } else {
        println!("{}", bold("Resource Count by Status"));
        println!("  {} converged: {}", green("●"), converged);
        println!("  {} failed:    {}", red("●"), failed);
        println!("  {} drifted:   {}", yellow("●"), drifted);
        println!("  {} unknown:   {}", dim("●"), unknown);
        println!("  ─────────────");
        println!("  total:      {}", total);
    }
    Ok(())
}

// ── FJ-397: status --format ──

/// Emit all resources in JSON array format.
fn format_json_output(state_dir: &Path, machine: Option<&str>) -> Result<(), String> {
    let machines = collect_resources(state_dir, machine)?;
    let mut all = Vec::new();
    for (m_name, resources) in &machines {
        for (name, rl) in resources {
            all.push(format!(
                "{{\"machine\":\"{}\",\"resource\":\"{}\",\"status\":\"{:?}\",\"applied_at\":{}}}",
                m_name,
                name,
                rl.status,
                rl.applied_at
                    .as_deref()
                    .map(|s| format!("\"{}\"", s))
                    .unwrap_or_else(|| "null".to_string()),
            ));
        }
    }
    println!("[{}]", all.join(","));
    Ok(())
}

/// Emit all resources in CSV format.
fn format_csv_output(state_dir: &Path, machine: Option<&str>) -> Result<(), String> {
    println!("machine,resource,status,applied_at");
    let machines = collect_resources(state_dir, machine)?;
    for (m_name, resources) in &machines {
        for (name, rl) in resources {
            println!(
                "{},{},{:?},{}",
                m_name,
                name,
                rl.status,
                rl.applied_at.as_deref().unwrap_or(""),
            );
        }
    }
    Ok(())
}

pub(crate) fn cmd_status_format(
    state_dir: &Path,
    machine: Option<&str>,
    fmt: &str,
) -> Result<(), String> {
    match fmt {
        "json" => format_json_output(state_dir, machine),
        "csv" => format_csv_output(state_dir, machine),
        "table" => cmd_status(state_dir, machine, false, None, false),
        _ => Err(format!("unknown format '{}'. Use table, json, or csv", fmt)),
    }
}

// ── FJ-452: status --compact ──

/// Build a compact summary line for a single machine.
fn build_compact_line(m_name: &str, lock: &types::StateLock, json: bool) -> String {
    let total = lock.resources.len();
    let converged = lock
        .resources
        .values()
        .filter(|rl| rl.status == types::ResourceStatus::Converged)
        .count();
    let failed = lock
        .resources
        .values()
        .filter(|rl| rl.status == types::ResourceStatus::Failed)
        .count();
    if json {
        format!(
            "{{\"machine\":\"{}\",\"total\":{},\"converged\":{},\"failed\":{}}}",
            m_name, total, converged, failed
        )
    } else {
        let status = if failed > 0 {
            red(&format!("{}F", failed))
        } else {
            green("OK")
        };
        format!("{}: {}/{} converged [{}]", m_name, converged, total, status)
    }
}

pub(crate) fn cmd_status_compact(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    if !state_dir.exists() {
        println!("No state directory found.");
        return Ok(());
    }
    let entries = std::fs::read_dir(state_dir).map_err(|e| e.to_string())?;
    let mut lines = Vec::new();
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
        if let Ok(Some(lock)) = state::load_lock(state_dir, &m_name) {
            lines.push(build_compact_line(&m_name, &lock, json));
        }
    }
    for line in &lines {
        println!("{}", line);
    }
    Ok(())
}

// ── FJ-432: status --json-lines ──

pub(crate) fn cmd_status_json_lines(state_dir: &Path, machine: Option<&str>) -> Result<(), String> {
    if !state_dir.exists() {
        return Ok(());
    }
    let machines = collect_resources(state_dir, machine)?;
    for (m_name, resources) in &machines {
        if m_name.starts_with('.') {
            continue;
        }
        for (rname, rl) in resources {
            println!(
                "{{\"machine\":\"{}\",\"resource\":\"{}\",\"status\":\"{:?}\",\"hash\":\"{}\"}}",
                m_name, rname, rl.status, rl.hash
            );
        }
    }
    Ok(())
}

// ── FJ-417: status --machines-only ──

/// Gather per-machine summary stats: (name, total, converged, failed).
fn gather_machine_stats(
    state_dir: &Path,
    machine: Option<&str>,
) -> Result<Vec<(String, usize, usize, usize)>, String> {
    let mut machines = Vec::new();
    if !state_dir.exists() {
        return Ok(machines);
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
        if let Ok(Some(lock)) = state::load_lock(state_dir, &m_name) {
            let total = lock.resources.len();
            let converged = lock
                .resources
                .values()
                .filter(|r| r.status == types::ResourceStatus::Converged)
                .count();
            let failed = lock
                .resources
                .values()
                .filter(|r| r.status == types::ResourceStatus::Failed)
                .count();
            machines.push((m_name, total, converged, failed));
        }
    }
    Ok(machines)
}

pub(crate) fn cmd_status_machines_only(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = gather_machine_stats(state_dir, machine)?;

    if json {
        let entries: Vec<String> = machines
            .iter()
            .map(|(m, t, c, f)| {
                format!(
                    "{{\"machine\":\"{}\",\"total\":{},\"converged\":{},\"failed\":{}}}",
                    m, t, c, f
                )
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else {
        println!("{}", bold("Machine Summary"));
        for (m, total, converged, failed) in &machines {
            let status = if *failed > 0 {
                red("DEGRADED")
            } else {
                green("HEALTHY")
            };
            println!(
                "  {} — {} ({} resources, {} converged, {} failed)",
                m, status, total, converged, failed
            );
        }
        if machines.is_empty() {
            println!("  {}", dim("No machines found in state"));
        }
    }
    Ok(())
}

// ── FJ-412: status --resources-by-type ──

pub(crate) fn cmd_status_resources_by_type(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let mut by_type: std::collections::HashMap<String, Vec<(String, String, String)>> =
        std::collections::HashMap::new();

    let machines = collect_resources(state_dir, machine)?;
    for (m_name, resources) in &machines {
        for (name, rl) in resources {
            let rtype = format!("{:?}", rl.resource_type);
            by_type.entry(rtype).or_default().push((
                m_name.clone(),
                name.clone(),
                format!("{:?}", rl.status),
            ));
        }
    }

    if json {
        let entries: Vec<String> = by_type
            .iter()
            .map(|(t, resources)| {
                let items: Vec<String> = resources
                    .iter()
                    .map(|(m, n, s)| {
                        format!(
                            "{{\"machine\":\"{}\",\"resource\":\"{}\",\"status\":\"{}\"}}",
                            m, n, s
                        )
                    })
                    .collect();
                format!("\"{}\":[{}]", t, items.join(","))
            })
            .collect();
        println!("{{{}}}", entries.join(","));
    } else {
        for (rtype, resources) in &by_type {
            println!("{} ({}):", bold(rtype), resources.len());
            for (m, n, s) in resources {
                println!("  {}/{} — {}", m, n, s);
            }
        }
    }
    Ok(())
}
