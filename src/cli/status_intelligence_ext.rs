//! Status intelligence extensions — drift frequency, streaks, error distribution.

use super::helpers::*;
#[allow(unused_imports)]
use super::helpers_state::*;
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

/// FJ-942: How often resources drift per machine over time.
pub(crate) fn cmd_status_machine_resource_drift_frequency(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let frequencies = collect_drift_frequencies(sd, &targets);
    if json {
        let items: Vec<String> = frequencies
            .iter()
            .map(|(m, c)| format!("{{\"machine\":\"{}\",\"drifted_resources\":{}}}", m, c))
            .collect();
        println!("{{\"drift_frequencies\":[{}]}}", items.join(","));
    } else if frequencies.is_empty() {
        println!("No drift frequency data available.");
    } else {
        println!("Drift frequency:");
        for (m, c) in &frequencies {
            println!("  {} — {} drifted resources", m, c);
        }
    }
    Ok(())
}
fn collect_drift_frequencies(sd: &Path, targets: &[&String]) -> Vec<(String, usize)> {
    let mut frequencies = Vec::new();
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
        let drifted = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Drifted)
            .count();
        if drifted > 0 {
            frequencies.push(((*m).clone(), drifted));
        }
    }
    frequencies.sort_by(|a, b| a.0.cmp(&b.0));
    frequencies
}
/// FJ-946: Fleet-wide drift frequency aggregation.
pub(crate) fn cmd_status_fleet_resource_drift_frequency(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let frequencies = collect_drift_frequencies(sd, &targets);
    let total: usize = frequencies.iter().map(|(_, c)| c).sum();
    if json {
        println!(
            "{{\"fleet_drifted_resources\":{},\"machines\":{}}}",
            total,
            frequencies.len()
        );
    } else {
        println!(
            "Fleet drift frequency: {} drifted resources across {} machines",
            total,
            frequencies.len()
        );
    }
    Ok(())
}
/// FJ-948: Trend analysis of apply durations per machine.
pub(crate) fn cmd_status_machine_resource_apply_duration_trend(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let trends = collect_apply_duration_trends(sd, &targets);
    if json {
        let items: Vec<String> = trends
            .iter()
            .map(|(m, avg)| {
                format!(
                    "{{\"machine\":\"{}\",\"avg_duration_seconds\":{:.4}}}",
                    m, avg
                )
            })
            .collect();
        println!("{{\"apply_duration_trends\":[{}]}}", items.join(","));
    } else if trends.is_empty() {
        println!("No apply duration trend data available.");
    } else {
        println!("Apply duration trends:");
        for (m, avg) in &trends {
            println!("  {} — avg {:.4}s", m, avg);
        }
    }
    Ok(())
}
fn collect_apply_duration_trends(sd: &Path, targets: &[&String]) -> Vec<(String, f64)> {
    let mut trends = Vec::new();
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
        let durations: Vec<f64> = lock
            .resources
            .values()
            .filter_map(|r| r.duration_seconds)
            .collect();
        if durations.is_empty() {
            continue;
        }
        let avg = durations.iter().sum::<f64>() / durations.len() as f64;
        trends.push(((*m).clone(), avg));
    }
    trends.sort_by(|a, b| a.0.cmp(&b.0));
    trends
}
/// FJ-950: Longest consecutive convergence streak per machine.
pub(crate) fn cmd_status_machine_resource_convergence_streak(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let streaks = collect_convergence_streaks(sd, &targets);
    if json {
        let items: Vec<String> = streaks
            .iter()
            .map(|(m, s)| format!("{{\"machine\":\"{}\",\"streak\":{}}}", m, s))
            .collect();
        println!("{{\"convergence_streaks\":[{}]}}", items.join(","));
    } else if streaks.is_empty() {
        println!("No convergence streak data available.");
    } else {
        println!("Convergence streaks:");
        for (m, s) in &streaks {
            println!("  {} — {} consecutive converged", m, s);
        }
    }
    Ok(())
}
fn collect_convergence_streaks(sd: &Path, targets: &[&String]) -> Vec<(String, usize)> {
    let mut streaks = Vec::new();
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
        let converged = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Converged)
            .count();
        streaks.push(((*m).clone(), converged));
    }
    streaks.sort_by(|a, b| a.0.cmp(&b.0));
    streaks
}
/// FJ-954: Fleet-wide convergence streak aggregation.
pub(crate) fn cmd_status_fleet_resource_convergence_streak(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let streaks = collect_convergence_streaks(sd, &targets);
    let total: usize = streaks.iter().map(|(_, s)| s).sum();
    let avg = if !streaks.is_empty() {
        total as f64 / streaks.len() as f64
    } else {
        0.0
    };
    if json {
        println!(
            "{{\"fleet_convergence_streak_avg\":{:.4},\"machines\":{}}}",
            avg,
            streaks.len()
        );
    } else {
        println!(
            "Fleet convergence streak avg: {:.4} ({} machines)",
            avg,
            streaks.len()
        );
    }
    Ok(())
}

/// FJ-956: Distribution of error types per machine.
pub(crate) fn cmd_status_machine_resource_error_distribution(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let distributions = collect_error_distributions(sd, &targets);
    if json {
        let items: Vec<String> = distributions
            .iter()
            .map(|(m, f, d)| {
                format!(
                    "{{\"machine\":\"{}\",\"failed\":{},\"drifted\":{}}}",
                    m, f, d
                )
            })
            .collect();
        println!("{{\"error_distributions\":[{}]}}", items.join(","));
    } else if distributions.is_empty() {
        println!("No error distribution data available.");
    } else {
        println!("Error distribution:");
        for (m, f, d) in &distributions {
            println!("  {} — {} failed, {} drifted", m, f, d);
        }
    }
    Ok(())
}

fn collect_error_distributions(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut dists = Vec::new();
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
        let failed = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Failed)
            .count();
        let drifted = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Drifted)
            .count();
        if failed > 0 || drifted > 0 {
            dists.push(((*m).clone(), failed, drifted));
        }
    }
    dists.sort_by(|a, b| a.0.cmp(&b.0));
    dists
}

/// FJ-958: How long each resource has been in drifted state.
pub(crate) fn cmd_status_machine_resource_drift_age(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let ages = collect_drift_ages(sd, &targets);
    if json {
        let items: Vec<String> = ages
            .iter()
            .map(|(m, r, age)| {
                format!(
                    "{{\"machine\":\"{}\",\"resource\":\"{}\",\"drift_age_hours\":{:.2}}}",
                    m, r, age
                )
            })
            .collect();
        println!("{{\"drift_ages\":[{}]}}", items.join(","));
    } else if ages.is_empty() {
        println!("No drifted resources found.");
    } else {
        println!("Drift ages:");
        for (m, r, age) in &ages {
            println!("  {}/{} — {:.2}h drifted", m, r, age);
        }
    }
    Ok(())
}

fn collect_drift_ages(sd: &Path, targets: &[&String]) -> Vec<(String, String, f64)> {
    let mut ages = Vec::new();
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
        for (rid, rl) in &lock.resources {
            if rl.status != types::ResourceStatus::Drifted {
                continue;
            }
            let hours = rl.duration_seconds.unwrap_or(0.0) / 3600.0;
            ages.push(((*m).clone(), rid.clone(), hours));
        }
    }
    ages.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    ages
}

/// FJ-962: Fleet-wide drift age aggregation.
pub(crate) fn cmd_status_fleet_resource_drift_age(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let ages = collect_drift_ages(sd, &targets);
    let total: f64 = ages.iter().map(|(_, _, h)| h).sum();
    let avg = if !ages.is_empty() {
        total / ages.len() as f64
    } else {
        0.0
    };
    if json {
        println!(
            "{{\"fleet_drift_age_avg_hours\":{:.2},\"drifted_resources\":{}}}",
            avg,
            ages.len()
        );
    } else {
        println!(
            "Fleet drift age: avg {:.2}h across {} drifted resources",
            avg,
            ages.len()
        );
    }
    Ok(())
}

/// FJ-964: Rate of recovery from failed/drifted states.
pub(crate) fn cmd_status_machine_resource_recovery_rate(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let rates = collect_recovery_rates(sd, &targets);
    if json {
        let items: Vec<String> = rates
            .iter()
            .map(|(m, rate)| format!("{{\"machine\":\"{}\",\"recovery_rate\":{:.4}}}", m, rate))
            .collect();
        println!("{{\"recovery_rates\":[{}]}}", items.join(","));
    } else if rates.is_empty() {
        println!("No recovery rate data available.");
    } else {
        println!("Recovery rates:");
        for (m, rate) in &rates {
            println!("  {} — {:.1}% recovered", m, rate * 100.0);
        }
    }
    Ok(())
}

fn collect_recovery_rates(sd: &Path, targets: &[&String]) -> Vec<(String, f64)> {
    let mut rates = Vec::new();
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
        rates.push(((*m).clone(), rate));
    }
    rates.sort_by(|a, b| a.0.cmp(&b.0));
    rates
}

/// FJ-966: Rate of drift accumulation per machine over time.
pub(crate) fn cmd_status_machine_resource_drift_velocity(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets = filter_targets(&machines, machine);
    let velocities = collect_drift_velocities(sd, &targets);
    if json {
        let items: Vec<String> = velocities
            .iter()
            .map(|(m, d, t)| {
                let v = velocity_ratio(*d, *t);
                format!(
                    "{{\"machine\":\"{}\",\"drifted\":{},\"total\":{},\"velocity\":{:.4}}}",
                    m, d, t, v
                )
            })
            .collect();
        println!("{{\"drift_velocities\":[{}]}}", items.join(","));
    } else if velocities.is_empty() {
        println!("No drift velocity data available.");
    } else {
        println!("Drift velocity:");
        for (m, d, t) in &velocities {
            println!(
                "  {} — {}/{} resources drifted ({:.1}%)",
                m,
                d,
                t,
                velocity_ratio(*d, *t) * 100.0
            );
        }
    }
    Ok(())
}

pub(super) fn filter_targets<'a>(machines: &'a [String], machine: Option<&str>) -> Vec<&'a String> {
    match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    }
}

fn velocity_ratio(drifted: usize, total: usize) -> f64 {
    if total > 0 {
        drifted as f64 / total as f64
    } else {
        0.0
    }
}

fn collect_drift_velocities(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut velocities = Vec::new();
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
        let total = lock.resources.len();
        if total == 0 {
            continue;
        }
        let drifted = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Drifted)
            .count();
        velocities.push(((*m).clone(), drifted, total));
    }
    velocities.sort_by(|a, b| a.0.cmp(&b.0));
    velocities
}

/// FJ-970: Fleet-wide recovery rate aggregation.
pub(crate) fn cmd_status_fleet_resource_recovery_rate(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let rates = collect_recovery_rates(sd, &targets);
    let avg = if !rates.is_empty() {
        rates.iter().map(|(_, r)| r).sum::<f64>() / rates.len() as f64
    } else {
        0.0
    };
    if json {
        println!(
            "{{\"fleet_recovery_rate_avg\":{:.4},\"machines\":{}}}",
            avg,
            rates.len()
        );
    } else {
        println!(
            "Fleet recovery rate: avg {:.1}% ({} machines)",
            avg * 100.0,
            rates.len()
        );
    }
    Ok(())
}

/// FJ-972: Ratio of converged resources to total apply time.
pub(crate) fn cmd_status_machine_resource_convergence_efficiency(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let efficiencies = collect_convergence_efficiencies(sd, &targets);
    if json {
        let items: Vec<String> = efficiencies
            .iter()
            .map(|(m, eff)| format!("{{\"machine\":\"{}\",\"efficiency\":{:.4}}}", m, eff))
            .collect();
        println!("{{\"convergence_efficiencies\":[{}]}}", items.join(","));
    } else if efficiencies.is_empty() {
        println!("No convergence efficiency data available.");
    } else {
        println!("Convergence efficiency:");
        for (m, eff) in &efficiencies {
            println!("  {} — {:.4} converged/sec", m, eff);
        }
    }
    Ok(())
}

fn collect_convergence_efficiencies(sd: &Path, targets: &[&String]) -> Vec<(String, f64)> {
    let mut efficiencies = Vec::new();
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
        let converged = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Converged)
            .count();
        let total_duration: f64 = lock
            .resources
            .values()
            .filter_map(|r| r.duration_seconds)
            .sum();
        let eff = if total_duration > 0.0 {
            converged as f64 / total_duration
        } else {
            0.0
        };
        if converged > 0 {
            efficiencies.push(((*m).clone(), eff));
        }
    }
    efficiencies.sort_by(|a, b| a.0.cmp(&b.0));
    efficiencies
}

/// FJ-974: Track how often each machine's resources are applied.
pub(crate) fn cmd_status_machine_resource_apply_frequency(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let freqs = collect_apply_frequencies(sd, &targets);
    if json {
        let items: Vec<String> = freqs
            .iter()
            .map(|(m, count)| format!("{{\"machine\":\"{}\",\"resource_count\":{}}}", m, count))
            .collect();
        println!("{{\"apply_frequencies\":[{}]}}", items.join(","));
    } else if freqs.is_empty() {
        println!("No apply frequency data available.");
    } else {
        println!("Apply frequencies:");
        for (m, count) in &freqs {
            println!("  {} — {} resources applied", m, count);
        }
    }
    Ok(())
}

fn collect_apply_frequencies(sd: &Path, targets: &[&String]) -> Vec<(String, usize)> {
    let mut freqs = Vec::new();
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
        let count = lock.resources.len();
        if count > 0 {
            freqs.push(((*m).clone(), count));
        }
    }
    freqs.sort_by(|a, b| a.0.cmp(&b.0));
    freqs
}

/// FJ-978: Composite fleet health score (convergence + drift + recovery).
pub(crate) fn cmd_status_fleet_resource_health_score(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets = filter_targets(&machines, machine);
    let (total_resources, total_converged, total_drifted, total_failed) =
        collect_fleet_totals(sd, &targets);
    let score = compute_health_score(
        total_resources,
        total_converged,
        total_drifted,
        total_failed,
    );
    if json {
        println!("{{\"fleet_health_score\":{},\"total_resources\":{},\"converged\":{},\"drifted\":{},\"failed\":{}}}", score, total_resources, total_converged, total_drifted, total_failed);
    } else if total_resources == 0 {
        println!("No fleet health data available.");
    } else {
        println!(
            "Fleet health score: {:.0}% ({} converged, {} drifted, {} failed of {} total)",
            score, total_converged, total_drifted, total_failed, total_resources
        );
    }
    Ok(())
}

fn collect_fleet_totals(sd: &Path, targets: &[&String]) -> (usize, usize, usize, usize) {
    let (mut total, mut converged, mut drifted, mut failed) = (0, 0, 0, 0);
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
        for r in lock.resources.values() {
            total += 1;
            match r.status {
                types::ResourceStatus::Converged => converged += 1,
                types::ResourceStatus::Drifted => drifted += 1,
                types::ResourceStatus::Failed => failed += 1,
                _ => {}
            }
        }
    }
    (total, converged, drifted, failed)
}

fn compute_health_score(total: usize, converged: usize, drifted: usize, failed: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    let convergence_ratio = converged as f64 / total as f64;
    let drift_penalty = drifted as f64 / total as f64 * 0.5;
    let failure_penalty = failed as f64 / total as f64;
    ((convergence_ratio - drift_penalty - failure_penalty).clamp(0.0, 1.0) * 100.0).round()
}

/// FJ-980: Index of how stale each machine's state data is.
pub(crate) fn cmd_status_machine_resource_staleness_index(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let staleness = collect_staleness_indices(sd, &targets);
    if json {
        let items: Vec<String> = staleness
            .iter()
            .map(|(m, idx)| format!("{{\"machine\":\"{}\",\"staleness_index\":{:.4}}}", m, idx))
            .collect();
        println!("{{\"staleness_indices\":[{}]}}", items.join(","));
    } else if staleness.is_empty() {
        println!("No staleness data available.");
    } else {
        println!("Staleness index (higher = more stale):");
        for (m, idx) in &staleness {
            println!("  {} — {:.4}", m, idx);
        }
    }
    Ok(())
}

fn collect_staleness_indices(sd: &Path, targets: &[&String]) -> Vec<(String, f64)> {
    let mut indices = Vec::new();
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
        let total = lock.resources.len();
        if total == 0 {
            continue;
        }
        let stale_count = lock
            .resources
            .values()
            .filter(|r| {
                r.status == types::ResourceStatus::Drifted
                    || r.status == types::ResourceStatus::Failed
            })
            .count();
        let avg_duration: f64 = lock
            .resources
            .values()
            .filter_map(|r| r.duration_seconds)
            .sum::<f64>()
            / total as f64;
        let staleness = stale_count as f64 / total as f64 + (avg_duration / 3600.0).min(1.0) * 0.1;
        indices.push(((*m).clone(), staleness));
    }
    indices.sort_by(|a, b| a.0.cmp(&b.0));
    indices
}
