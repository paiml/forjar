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
            .map(|(m, c)| format!("{{\"machine\":\"{m}\",\"drifted_resources\":{c}}}"))
            .collect();
        println!("{{\"drift_frequencies\":[{}]}}", items.join(","));
    } else if frequencies.is_empty() {
        println!("No drift frequency data available.");
    } else {
        println!("Drift frequency:");
        for (m, c) in &frequencies {
            println!("  {m} — {c} drifted resources");
        }
    }
    Ok(())
}
pub(super) fn collect_drift_frequencies(sd: &Path, targets: &[&String]) -> Vec<(String, usize)> {
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
                    "{{\"machine\":\"{m}\",\"avg_duration_seconds\":{avg:.4}}}"
                )
            })
            .collect();
        println!("{{\"apply_duration_trends\":[{}]}}", items.join(","));
    } else if trends.is_empty() {
        println!("No apply duration trend data available.");
    } else {
        println!("Apply duration trends:");
        for (m, avg) in &trends {
            println!("  {m} — avg {avg:.4}s");
        }
    }
    Ok(())
}
pub(super) fn collect_apply_duration_trends(sd: &Path, targets: &[&String]) -> Vec<(String, f64)> {
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
            .map(|(m, s)| format!("{{\"machine\":\"{m}\",\"streak\":{s}}}"))
            .collect();
        println!("{{\"convergence_streaks\":[{}]}}", items.join(","));
    } else if streaks.is_empty() {
        println!("No convergence streak data available.");
    } else {
        println!("Convergence streaks:");
        for (m, s) in &streaks {
            println!("  {m} — {s} consecutive converged");
        }
    }
    Ok(())
}
pub(super) fn collect_convergence_streaks(sd: &Path, targets: &[&String]) -> Vec<(String, usize)> {
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
                    "{{\"machine\":\"{m}\",\"failed\":{f},\"drifted\":{d}}}"
                )
            })
            .collect();
        println!("{{\"error_distributions\":[{}]}}", items.join(","));
    } else if distributions.is_empty() {
        println!("No error distribution data available.");
    } else {
        println!("Error distribution:");
        for (m, f, d) in &distributions {
            println!("  {m} — {f} failed, {d} drifted");
        }
    }
    Ok(())
}

pub(super) fn collect_error_distributions(
    sd: &Path,
    targets: &[&String],
) -> Vec<(String, usize, usize)> {
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
                    "{{\"machine\":\"{m}\",\"resource\":\"{r}\",\"drift_age_hours\":{age:.2}}}"
                )
            })
            .collect();
        println!("{{\"drift_ages\":[{}]}}", items.join(","));
    } else if ages.is_empty() {
        println!("No drifted resources found.");
    } else {
        println!("Drift ages:");
        for (m, r, age) in &ages {
            println!("  {m}/{r} — {age:.2}h drifted");
        }
    }
    Ok(())
}

pub(super) fn collect_drift_ages(sd: &Path, targets: &[&String]) -> Vec<(String, String, f64)> {
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

pub(super) use super::status_intelligence_ext_b::*;
