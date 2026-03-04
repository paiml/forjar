//! Status intelligence — MTTR estimates, convergence forecasting, budget projections.

use super::helpers::*;
#[allow(unused_imports)]
use super::helpers_state::*;
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

pub(super) fn pct(num: usize, den: usize) -> f64 {
    if den > 0 {
        (num as f64 / den as f64) * 100.0
    } else {
        0.0
    }
}

/// FJ-910: Estimate MTTR per machine based on failure/recovery patterns.
pub(crate) fn cmd_status_machine_resource_mttr_estimate(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let estimates = collect_mttr_estimates(sd, &targets);
    if json {
        let items: Vec<String> = estimates
            .iter()
            .map(|(m, s)| format!("{{\"machine\":\"{m}\",\"mttr_estimate\":\"{s}\"}}"))
            .collect();
        println!("{{\"machine_mttr_estimates\":[{}]}}", items.join(","));
    } else if estimates.is_empty() {
        println!("No MTTR estimate data available.");
    } else {
        println!("Machine MTTR estimates:");
        for (m, s) in &estimates {
            println!("  {m} — {s}");
        }
    }
    Ok(())
}

pub(super) fn collect_mttr_estimates(sd: &Path, targets: &[&String]) -> Vec<(String, String)> {
    let mut estimates = Vec::new();
    for m in targets {
        // Read events to compute actual MTTR
        let events_path = sd.join(m).join("events.jsonl");
        let events_content = std::fs::read_to_string(&events_path).ok();

        // Read lock to get current failure count
        let lock_path = sd.join(m).join("state.lock.yaml");
        let lock_content = std::fs::read_to_string(&lock_path).ok();

        let failed = lock_content
            .as_ref()
            .and_then(|c| serde_yaml_ng::from_str::<types::StateLock>(c).ok())
            .map(|lock| {
                lock.resources
                    .values()
                    .filter(|r| matches!(r.status, types::ResourceStatus::Failed))
                    .count()
            })
            .unwrap_or(0);

        let est = match events_content {
            Some(ref content) if !content.is_empty() => {
                let mttr = compute_event_mttr(content);
                match (mttr, failed) {
                    (Some(seconds), f) if f > 0 => format_mttr(seconds, f),
                    (Some(seconds), _) => format_mttr_healthy(seconds),
                    (None, f) if f > 0 => {
                        format!("{f} currently failed — no prior recovery data")
                    }
                    _ => "all healthy — no recovery needed".to_string(),
                }
            }
            _ if failed > 0 => format!("{failed} failed — no event history for MTTR"),
            _ => "no data".to_string(),
        };
        estimates.push(((*m).clone(), est));
    }
    estimates.sort_by(|a, b| a.0.cmp(&b.0));
    estimates
}

fn compute_event_mttr(content: &str) -> Option<f64> {
    let mut fail_times: std::collections::HashMap<String, f64> =
        std::collections::HashMap::new();
    let mut recovery_durations: Vec<f64> = Vec::new();

    for line in content.lines() {
        let val: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let event = val.get("event").and_then(|v| v.as_str()).unwrap_or("");
        let resource = val.get("resource").and_then(|v| v.as_str()).unwrap_or("");
        let ts_str = val.get("ts").and_then(|v| v.as_str()).unwrap_or("");
        let ts = match parse_event_ts(ts_str) {
            Some(t) => t,
            None => continue,
        };

        if event == "resource_failed" || event == "resource_drifted" {
            fail_times.insert(resource.to_string(), ts);
        } else if event == "resource_converged" {
            if let Some(fail_ts) = fail_times.remove(resource) {
                let duration = ts - fail_ts;
                if duration > 0.0 {
                    recovery_durations.push(duration);
                }
            }
        }
    }

    if recovery_durations.is_empty() {
        None
    } else {
        Some(recovery_durations.iter().sum::<f64>() / recovery_durations.len() as f64)
    }
}

fn parse_event_ts(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split('T').collect();
    if parts.len() != 2 {
        return None;
    }
    let date: Vec<u32> = parts[0].split('-').filter_map(|p| p.parse().ok()).collect();
    let time_str = parts[1].trim_end_matches('Z');
    let time: Vec<f64> = time_str.split(':').filter_map(|p| p.parse().ok()).collect();
    if date.len() != 3 || time.len() != 3 {
        return None;
    }
    let days = (date[0] as f64 - 1970.0) * 365.25 + (date[1] as f64 - 1.0) * 30.44 + date[2] as f64;
    Some(days * 86400.0 + time[0] * 3600.0 + time[1] * 60.0 + time[2])
}

fn format_mttr(seconds: f64, current_failed: usize) -> String {
    let time = if seconds < 60.0 {
        format!("{seconds:.1}s")
    } else if seconds < 3600.0 {
        format!("{:.1}m", seconds / 60.0)
    } else {
        format!("{:.1}h", seconds / 3600.0)
    };
    format!("MTTR {time} avg — {current_failed} currently failed")
}

fn format_mttr_healthy(seconds: f64) -> String {
    let time = if seconds < 60.0 {
        format!("{seconds:.1}s")
    } else if seconds < 3600.0 {
        format!("{:.1}m", seconds / 60.0)
    } else {
        format!("{:.1}h", seconds / 3600.0)
    };
    format!("MTTR {time} avg — all healthy now")
}

/// FJ-914: Forecast convergence trajectory based on current state.
pub(crate) fn cmd_status_fleet_resource_convergence_forecast(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let forecasts = collect_convergence_forecasts(sd, &targets);
    if json {
        let items: Vec<String> = forecasts
            .iter()
            .map(|(m, c, t)| {
                format!(
                    "{{\"machine\":\"{}\",\"converged\":{},\"total\":{},\"forecast\":\"{}\"}}",
                    m,
                    c,
                    t,
                    forecast_label(*c, *t)
                )
            })
            .collect();
        println!("{{\"convergence_forecast\":[{}]}}", items.join(","));
    } else if forecasts.is_empty() {
        println!("No convergence forecast data available.");
    } else {
        println!("Fleet convergence forecast:");
        for (m, c, t) in &forecasts {
            println!(
                "  {} — {}/{} converged ({})",
                m,
                c,
                t,
                forecast_label(*c, *t)
            );
        }
    }
    Ok(())
}

pub(super) fn forecast_label(converged: usize, total: usize) -> String {
    if total == 0 {
        return "no resources".to_string();
    }
    let rate = pct(converged, total);
    if rate >= 100.0 {
        "fully converged".to_string()
    } else if rate >= 80.0 {
        "near convergence".to_string()
    } else if rate >= 50.0 {
        "partial convergence".to_string()
    } else {
        "low convergence".to_string()
    }
}

pub(super) fn collect_convergence_forecasts(
    sd: &Path,
    targets: &[&String],
) -> Vec<(String, usize, usize)> {
    let mut forecasts = Vec::new();
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
        let converged = lock
            .resources
            .values()
            .filter(|r| matches!(r.status, types::ResourceStatus::Converged))
            .count();
        forecasts.push(((*m).clone(), converged, total));
    }
    forecasts.sort_by(|a, b| a.0.cmp(&b.0));
    forecasts
}

/// FJ-916: Forecast error budget depletion based on current failure rate.
pub(crate) fn cmd_status_machine_resource_error_budget_forecast(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let forecasts = collect_error_budget_forecasts(sd, &targets);
    if json {
        let items: Vec<String> = forecasts
            .iter()
            .map(|(m, f, t)| {
                format!(
                    "{{\"machine\":\"{}\",\"failed\":{},\"total\":{},\"budget_pct\":{:.1}}}",
                    m,
                    f,
                    t,
                    pct(*f, *t)
                )
            })
            .collect();
        println!("{{\"error_budget_forecast\":[{}]}}", items.join(","));
    } else if forecasts.is_empty() {
        println!("No error budget forecast data available.");
    } else {
        println!("Machine error budget forecast:");
        for (m, f, t) in &forecasts {
            let remaining = 100.0 - pct(*f, *t);
            println!(
                "  {m} — {remaining:.1}% budget remaining ({f}/{t} failed)"
            );
        }
    }
    Ok(())
}

pub(super) fn collect_error_budget_forecasts(
    sd: &Path,
    targets: &[&String],
) -> Vec<(String, usize, usize)> {
    let mut forecasts = Vec::new();
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
        let failed = lock
            .resources
            .values()
            .filter(|r| matches!(r.status, types::ResourceStatus::Failed))
            .count();
        forecasts.push(((*m).clone(), failed, total));
    }
    forecasts.sort_by(|a, b| a.0.cmp(&b.0));
    forecasts
}

/// FJ-918: Detect lag between dependent resource convergence.
pub(crate) fn cmd_status_machine_resource_dependency_lag(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let lags = collect_dependency_lag(sd, &targets);
    if json {
        let items: Vec<String> = lags
            .iter()
            .map(|(m, c, f)| {
                format!(
                    "{{\"machine\":\"{}\",\"converged\":{},\"failed\":{},\"lag_detected\":{}}}",
                    m,
                    c,
                    f,
                    *f > 0
                )
            })
            .collect();
        println!("{{\"dependency_lag\":[{}]}}", items.join(","));
    } else if lags.is_empty() {
        println!("No dependency lag data available.");
    } else {
        println!("Machine dependency convergence lag:");
        for (m, c, f) in &lags {
            let lag = if *f > 0 { "lag detected" } else { "in sync" };
            println!("  {} — {}/{} converged ({})", m, c, c + f, lag);
        }
    }
    Ok(())
}

pub(super) fn collect_dependency_lag(
    sd: &Path,
    targets: &[&String],
) -> Vec<(String, usize, usize)> {
    let mut lags = Vec::new();
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
            .filter(|r| matches!(r.status, types::ResourceStatus::Converged))
            .count();
        let failed = lock
            .resources
            .values()
            .filter(|r| matches!(r.status, types::ResourceStatus::Failed))
            .count();
        lags.push(((*m).clone(), converged, failed));
    }
    lags.sort_by(|a, b| a.0.cmp(&b.0));
    lags
}

/// FJ-922: Fleet-wide dependency convergence lag analysis.
pub(crate) fn cmd_status_fleet_resource_dependency_lag(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let lags = collect_dependency_lag(sd, &targets);
    let total_converged: usize = lags.iter().map(|(_, c, _)| c).sum();
    let total_failed: usize = lags.iter().map(|(_, _, f)| f).sum();
    let total = total_converged + total_failed;
    if json {
        println!("{{\"fleet_dependency_lag\":{{\"total_converged\":{},\"total_failed\":{},\"total\":{},\"lag_pct\":{:.1}}}}}", total_converged, total_failed, total, pct(total_failed, total));
    } else {
        println!(
            "Fleet dependency lag: {}/{} resources converged ({:.1}% lagging)",
            total_converged,
            total,
            pct(total_failed, total)
        );
    }
    Ok(())
}

pub(super) use super::status_intelligence_b::*;
