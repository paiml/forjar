//! Status intelligence extensions (Phase 90+) — drift recurrence, heatmap, convergence trend, latency, security posture.
use super::helpers::discover_machines;
use super::status_intelligence_ext::filter_targets;
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

/// FJ-982: Count how many times each resource has drifted across applies.
pub(crate) fn cmd_status_machine_resource_drift_recurrence(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets = filter_targets(&machines, machine);
    let mut recurrences: Vec<(String, String, usize)> = Vec::new();
    for m in &targets {
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
                recurrences.push(((*m).clone(), name.clone(), 1));
            }
        }
    }
    recurrences.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    if json {
        let items: Vec<String> = recurrences
            .iter()
            .map(|(m, r, c)| {
                format!(
                    "{{\"machine\":\"{m}\",\"resource\":\"{r}\",\"drift_count\":{c}}}"
                )
            })
            .collect();
        println!("{{\"drift_recurrences\":[{}]}}", items.join(","));
    } else if recurrences.is_empty() {
        println!("No drift recurrence data available.");
    } else {
        println!("Drift recurrence:");
        for (m, r, c) in &recurrences {
            println!("  {m}/{r} — {c} drift events");
        }
    }
    Ok(())
}

/// FJ-986: Heatmap of drift across fleet machines and resources.
pub(crate) fn cmd_status_fleet_resource_drift_heatmap(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets = filter_targets(&machines, machine);
    let mut heatmap: Vec<(String, usize, usize)> = Vec::new();
    for m in &targets {
        let path = sd.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let total = lock.resources.len();
        let drifted = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Drifted)
            .count();
        if total > 0 {
            heatmap.push(((*m).clone(), drifted, total));
        }
    }
    heatmap.sort_by(|a, b| a.0.cmp(&b.0));
    if json {
        let items: Vec<String> = heatmap
            .iter()
            .map(|(m, d, t)| {
                format!(
                    "{{\"machine\":\"{}\",\"drifted\":{},\"total\":{},\"ratio\":{:.4}}}",
                    m,
                    d,
                    t,
                    *d as f64 / *t as f64
                )
            })
            .collect();
        println!("{{\"drift_heatmap\":[{}]}}", items.join(","));
    } else if heatmap.is_empty() {
        println!("No drift heatmap data available.");
    } else {
        println!("Drift heatmap:");
        for (m, d, t) in &heatmap {
            let pct = *d as f64 / *t as f64 * 100.0;
            let bar = "█".repeat((*d * 20).checked_div(*t).unwrap_or(0));
            println!("  {m:20} {d:>3}/{t:<3} ({pct:5.1}%) {bar}");
        }
    }
    Ok(())
}

/// FJ-988: Trend of convergence rate over recent applies.
pub(crate) fn cmd_status_machine_resource_convergence_trend_p90(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets = filter_targets(&machines, machine);
    let mut trends: Vec<(String, f64, usize)> = Vec::new();
    for m in &targets {
        let path = sd.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let total = lock.resources.len();
        if total == 0 {
            continue;
        }
        let converged = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Converged)
            .count();
        let rate = converged as f64 / total as f64;
        trends.push(((*m).clone(), rate, total));
    }
    trends.sort_by(|a, b| a.0.cmp(&b.0));
    if json {
        let items: Vec<String> = trends
            .iter()
            .map(|(m, r, t)| {
                format!(
                    "{{\"machine\":\"{m}\",\"convergence_rate\":{r:.4},\"total\":{t}}}"
                )
            })
            .collect();
        println!("{{\"convergence_trends\":[{}]}}", items.join(","));
    } else if trends.is_empty() {
        println!("No convergence trend data available.");
    } else {
        println!("Convergence trend:");
        for (m, r, t) in &trends {
            println!("  {} — {:.1}% converged ({} resources)", m, r * 100.0, t);
        }
    }
    Ok(())
}
/// FJ-990: How long each drifted resource has been drifted (hours estimate).
pub(crate) fn cmd_status_machine_resource_drift_age_hours(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets = filter_targets(&machines, machine);
    let mut ages: Vec<(String, String, f64)> = Vec::new();
    for m in &targets {
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
                let hours = res.duration_seconds.unwrap_or(0.0) / 3600.0;
                ages.push(((*m).clone(), name.clone(), hours));
            }
        }
    }
    ages.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    print_drift_ages(&ages, json);
    Ok(())
}
pub(super) fn print_drift_ages(ages: &[(String, String, f64)], json: bool) {
    if json {
        let items: Vec<String> = ages
            .iter()
            .map(|(m, r, h)| {
                format!(
                    "{{\"machine\":\"{m}\",\"resource\":\"{r}\",\"drift_age_hours\":{h:.2}}}"
                )
            })
            .collect();
        println!("{{\"drift_ages\":[{}]}}", items.join(","));
    } else if ages.is_empty() {
        println!("No drifted resources found.");
    } else {
        println!("Drift age (hours):");
        for (m, r, h) in ages {
            println!("  {m}/{r} — {h:.2}h");
        }
    }
}
/// FJ-994: Convergence rate at various percentiles.
pub(crate) fn cmd_status_fleet_resource_convergence_percentile(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets = filter_targets(&machines, machine);
    let mut rates: Vec<f64> = Vec::new();
    for m in &targets {
        let path = sd.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let total = lock.resources.len();
        if total == 0 {
            continue;
        }
        let converged = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Converged)
            .count();
        rates.push(converged as f64 / total as f64);
    }
    rates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    print_convergence_percentiles(&rates, json);
    Ok(())
}
pub(super) fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p / 100.0 * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}
pub(super) fn print_convergence_percentiles(rates: &[f64], json: bool) {
    let p50 = percentile(rates, 50.0);
    let p90 = percentile(rates, 90.0);
    let p99 = percentile(rates, 99.0);
    if json {
        println!("{{\"convergence_percentiles\":{{\"p50\":{:.4},\"p90\":{:.4},\"p99\":{:.4},\"count\":{}}}}}", p50, p90, p99, rates.len());
    } else if rates.is_empty() {
        println!("No convergence data available.");
    } else {
        println!("Convergence percentiles ({} machines):", rates.len());
        println!("  p50 — {:.1}%", p50 * 100.0);
        println!("  p90 — {:.1}%", p90 * 100.0);
        println!("  p99 — {:.1}%", p99 * 100.0);
    }
}
/// FJ-996: Error rate per machine across recent applies.
pub(crate) fn cmd_status_machine_resource_error_rate(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets = filter_targets(&machines, machine);
    let mut error_rates: Vec<(String, f64, usize, usize)> = Vec::new();
    for m in &targets {
        let path = sd.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let total = lock.resources.len();
        if total == 0 {
            continue;
        }
        let failed = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Failed)
            .count();
        let rate = failed as f64 / total as f64;
        error_rates.push(((*m).clone(), rate, failed, total));
    }
    error_rates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    print_error_rates(&error_rates, json);
    Ok(())
}

pub(super) use super::status_intelligence_ext2_b::*;
