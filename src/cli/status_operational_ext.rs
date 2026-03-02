//! Status operational extensions (Phase 95) — apply success rate, drift recurrence, type drift heatmap.

use super::helpers::discover_machines;
use super::status_intelligence_ext::filter_targets;
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::collections::HashMap;
use std::path::Path;

// ────────────────────────────────────────────────────────────────────────────
// FJ-1021: Rolling apply success rate per machine from event logs.
// ────────────────────────────────────────────────────────────────────────────

/// Parse event log lines and count ok/fail results for a single machine.
fn count_apply_results(sd: &Path, machine: &str) -> (usize, usize) {
    let path = sd.join(machine).join("events.log");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return (0, 0),
    };
    let mut ok = 0usize;
    let mut fail = 0usize;
    for line in content.lines() {
        let parsed: serde_json::Value =
            serde_json::from_str(line).unwrap_or(serde_json::Value::Null);
        match parsed.get("result").and_then(|v| v.as_str()) {
            Some("ok") => ok += 1,
            Some("fail") => fail += 1,
            _ => {}
        }
    }
    (ok, fail)
}

/// FJ-1021: Compute rolling apply success rate per machine.
pub(crate) fn cmd_status_fleet_apply_success_rate_trend(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets = filter_targets(&machines, machine);
    let mut rates: Vec<(String, f64, usize, usize)> = Vec::new();
    for m in &targets {
        let (ok, fail) = count_apply_results(sd, m);
        let total = ok + fail;
        if total == 0 {
            continue;
        }
        let rate = ok as f64 / total as f64 * 100.0;
        rates.push(((*m).clone(), rate, ok, total));
    }
    rates.sort_by(|a, b| a.0.cmp(&b.0));
    print_apply_success_rates(&rates, json);
    Ok(())
}

fn print_apply_success_rates(rates: &[(String, f64, usize, usize)], json: bool) {
    if json {
        let items: Vec<String> = rates
            .iter()
            .map(|(m, rate, ok, total)| {
                format!(
                    "{{\"machine\":\"{}\",\"success_rate\":{:.1},\"ok\":{},\"total\":{}}}",
                    m, rate, ok, total
                )
            })
            .collect();
        println!(
            "{{\"fleet_apply_success_rate_trend\":[{}]}}",
            items.join(",")
        );
    } else if rates.is_empty() {
        println!("No apply events found.");
    } else {
        for (m, rate, ok, total) in rates {
            println!("{}: {:.1}% ({}/{})", m, rate, ok, total);
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// FJ-1024: Identify resources that repeatedly drift.
// ────────────────────────────────────────────────────────────────────────────

/// FJ-1024: Identify resources with status "drifted" from state locks.
pub(crate) fn cmd_status_machine_resource_drift_flapping(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets = filter_targets(&machines, machine);
    let drifted = collect_drifted_resources(sd, &targets);
    print_drift_recurrence(&drifted, json);
    Ok(())
}

fn collect_drifted_resources(sd: &Path, targets: &[&String]) -> Vec<(String, String, String)> {
    let mut results: Vec<(String, String, String)> = Vec::new();
    for m in targets {
        let path = sd.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        for (name, res) in &lock.resources {
            if res.status == types::ResourceStatus::Drifted {
                let rtype = format!("{}", res.resource_type);
                results.push((name.clone(), rtype, "drifted".to_string()));
            }
        }
    }
    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}

fn print_drift_recurrence(drifted: &[(String, String, String)], json: bool) {
    if json {
        let items: Vec<String> = drifted
            .iter()
            .map(|(r, t, s)| {
                format!(
                    "{{\"resource\":\"{}\",\"type\":\"{}\",\"status\":\"{}\"}}",
                    r, t, s
                )
            })
            .collect();
        println!("{{\"drift_recurrence\":[{}]}}", items.join(","));
    } else if drifted.is_empty() {
        println!("No recurring drift detected.");
    } else {
        for (r, t, _) in drifted {
            println!("{}: drifted ({})", r, t);
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// FJ-1027: Heatmap of drift by resource type across fleet.
// ────────────────────────────────────────────────────────────────────────────

/// FJ-1027: Aggregate drifted resources by type across all machines.
pub(crate) fn cmd_status_fleet_resource_type_drift_heatmap(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets = filter_targets(&machines, machine);
    let heatmap = collect_type_drift_heatmap(sd, &targets);
    print_type_drift_heatmap(&heatmap, json);
    Ok(())
}

/// Collect drift counts grouped by resource type, tracking which machines contribute.
fn collect_type_drift_heatmap(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    let mut type_machines: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
    for m in targets {
        let path = sd.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        for res in lock.resources.values() {
            if res.status == types::ResourceStatus::Drifted {
                let rtype = format!("{}", res.resource_type);
                *type_counts.entry(rtype.clone()).or_insert(0) += 1;
                type_machines.entry(rtype).or_default().insert((*m).clone());
            }
        }
    }
    let mut results: Vec<(String, usize, usize)> = type_counts
        .into_iter()
        .map(|(rtype, count)| {
            let machine_count = type_machines.get(&rtype).map_or(0, |s| s.len());
            (rtype, count, machine_count)
        })
        .collect();
    results.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    results
}

fn print_type_drift_heatmap(heatmap: &[(String, usize, usize)], json: bool) {
    if json {
        let items: Vec<String> = heatmap
            .iter()
            .map(|(rtype, count, machines)| {
                format!(
                    "{{\"resource_type\":\"{}\",\"drift_count\":{},\"machine_count\":{}}}",
                    rtype, count, machines
                )
            })
            .collect();
        println!("{{\"type_drift_heatmap\":[{}]}}", items.join(","));
    } else if heatmap.is_empty() {
        println!("No drift detected across fleet.");
    } else {
        for (rtype, count, machines) in heatmap {
            println!("{}: {} drifted across {} machines", rtype, count, machines);
        }
    }
}
