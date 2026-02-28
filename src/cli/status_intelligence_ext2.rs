//! Status intelligence extensions (Phase 90+) — drift recurrence, heatmap, convergence trend.
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;
use super::helpers::discover_machines;
use super::status_intelligence_ext::filter_targets;

/// FJ-982: Count how many times each resource has drifted across applies.
pub(crate) fn cmd_status_machine_resource_drift_recurrence(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets = filter_targets(&machines, machine);
    let mut recurrences: Vec<(String, String, usize)> = Vec::new();
    for m in &targets {
        let path = sd.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        for (name, res) in &lock.resources {
            if res.status == types::ResourceStatus::Drifted {
                recurrences.push(((*m).clone(), name.clone(), 1));
            }
        }
    }
    recurrences.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    if json {
        let items: Vec<String> = recurrences.iter()
            .map(|(m, r, c)| format!("{{\"machine\":\"{}\",\"resource\":\"{}\",\"drift_count\":{}}}", m, r, c))
            .collect();
        println!("{{\"drift_recurrences\":[{}]}}", items.join(","));
    } else if recurrences.is_empty() {
        println!("No drift recurrence data available.");
    } else {
        println!("Drift recurrence:");
        for (m, r, c) in &recurrences { println!("  {}/{} — {} drift events", m, r, c); }
    }
    Ok(())
}

/// FJ-986: Heatmap of drift across fleet machines and resources.
pub(crate) fn cmd_status_fleet_resource_drift_heatmap(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets = filter_targets(&machines, machine);
    let mut heatmap: Vec<(String, usize, usize)> = Vec::new();
    for m in &targets {
        let path = sd.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        let total = lock.resources.len();
        let drifted = lock.resources.values().filter(|r| r.status == types::ResourceStatus::Drifted).count();
        if total > 0 { heatmap.push(((*m).clone(), drifted, total)); }
    }
    heatmap.sort_by(|a, b| a.0.cmp(&b.0));
    if json {
        let items: Vec<String> = heatmap.iter()
            .map(|(m, d, t)| format!("{{\"machine\":\"{}\",\"drifted\":{},\"total\":{},\"ratio\":{:.4}}}", m, d, t, *d as f64 / *t as f64))
            .collect();
        println!("{{\"drift_heatmap\":[{}]}}", items.join(","));
    } else if heatmap.is_empty() {
        println!("No drift heatmap data available.");
    } else {
        println!("Drift heatmap:");
        for (m, d, t) in &heatmap {
            let pct = *d as f64 / *t as f64 * 100.0;
            let bar = "█".repeat((*d * 20).checked_div(*t).unwrap_or(0));
            println!("  {:20} {:>3}/{:<3} ({:5.1}%) {}", m, d, t, pct, bar);
        }
    }
    Ok(())
}

/// FJ-988: Trend of convergence rate over recent applies.
pub(crate) fn cmd_status_machine_resource_convergence_trend_p90(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets = filter_targets(&machines, machine);
    let mut trends: Vec<(String, f64, usize)> = Vec::new();
    for m in &targets {
        let path = sd.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        let total = lock.resources.len();
        if total == 0 { continue; }
        let converged = lock.resources.values().filter(|r| r.status == types::ResourceStatus::Converged).count();
        let rate = converged as f64 / total as f64;
        trends.push(((*m).clone(), rate, total));
    }
    trends.sort_by(|a, b| a.0.cmp(&b.0));
    if json {
        let items: Vec<String> = trends.iter()
            .map(|(m, r, t)| format!("{{\"machine\":\"{}\",\"convergence_rate\":{:.4},\"total\":{}}}", m, r, t))
            .collect();
        println!("{{\"convergence_trends\":[{}]}}", items.join(","));
    } else if trends.is_empty() {
        println!("No convergence trend data available.");
    } else {
        println!("Convergence trend:");
        for (m, r, t) in &trends { println!("  {} — {:.1}% converged ({} resources)", m, r * 100.0, t); }
    }
    Ok(())
}
