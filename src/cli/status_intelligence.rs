//! Status intelligence — MTTR estimates, convergence forecasting, budget projections.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;
use super::helpers::*;
#[allow(unused_imports)]
use super::helpers_state::*;

fn pct(num: usize, den: usize) -> f64 {
    if den > 0 { (num as f64 / den as f64) * 100.0 } else { 0.0 }
}

/// FJ-910: Estimate MTTR per machine based on failure/recovery patterns.
pub(crate) fn cmd_status_machine_resource_mttr_estimate(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let estimates = collect_mttr_estimates(sd, &targets);
    if json {
        let items: Vec<String> = estimates.iter()
            .map(|(m, s)| format!("{{\"machine\":\"{}\",\"mttr_estimate\":\"{}\"}}", m, s))
            .collect();
        println!("{{\"machine_mttr_estimates\":[{}]}}", items.join(","));
    } else if estimates.is_empty() {
        println!("No MTTR estimate data available.");
    } else {
        println!("Machine MTTR estimates:");
        for (m, s) in &estimates { println!("  {} — {}", m, s); }
    }
    Ok(())
}

fn collect_mttr_estimates(sd: &Path, targets: &[&String]) -> Vec<(String, String)> {
    let mut estimates = Vec::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => { estimates.push(((*m).clone(), "no data".to_string())); continue } };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => { estimates.push(((*m).clone(), "parse error".to_string())); continue } };
        let failed = lock.resources.values().filter(|r| matches!(r.status, types::ResourceStatus::Failed)).count();
        let est = if failed > 0 { format!("{} failed resources — estimated recovery needed", failed) } else { "all healthy — no recovery needed".to_string() };
        estimates.push(((*m).clone(), est));
    }
    estimates.sort_by(|a, b| a.0.cmp(&b.0));
    estimates
}

/// FJ-914: Forecast convergence trajectory based on current state.
pub(crate) fn cmd_status_fleet_resource_convergence_forecast(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let forecasts = collect_convergence_forecasts(sd, &targets);
    if json {
        let items: Vec<String> = forecasts.iter()
            .map(|(m, c, t)| format!("{{\"machine\":\"{}\",\"converged\":{},\"total\":{},\"forecast\":\"{}\"}}", m, c, t, forecast_label(*c, *t)))
            .collect();
        println!("{{\"convergence_forecast\":[{}]}}", items.join(","));
    } else if forecasts.is_empty() {
        println!("No convergence forecast data available.");
    } else {
        println!("Fleet convergence forecast:");
        for (m, c, t) in &forecasts { println!("  {} — {}/{} converged ({})", m, c, t, forecast_label(*c, *t)); }
    }
    Ok(())
}

fn forecast_label(converged: usize, total: usize) -> String {
    if total == 0 { return "no resources".to_string(); }
    let rate = pct(converged, total);
    if rate >= 100.0 { "fully converged".to_string() }
    else if rate >= 80.0 { "near convergence".to_string() }
    else if rate >= 50.0 { "partial convergence".to_string() }
    else { "low convergence".to_string() }
}

fn collect_convergence_forecasts(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut forecasts = Vec::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        let total = lock.resources.len();
        let converged = lock.resources.values().filter(|r| matches!(r.status, types::ResourceStatus::Converged)).count();
        forecasts.push(((*m).clone(), converged, total));
    }
    forecasts.sort_by(|a, b| a.0.cmp(&b.0));
    forecasts
}

/// FJ-916: Forecast error budget depletion based on current failure rate.
pub(crate) fn cmd_status_machine_resource_error_budget_forecast(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let forecasts = collect_error_budget_forecasts(sd, &targets);
    if json {
        let items: Vec<String> = forecasts.iter()
            .map(|(m, f, t)| format!("{{\"machine\":\"{}\",\"failed\":{},\"total\":{},\"budget_pct\":{:.1}}}", m, f, t, pct(*f, *t)))
            .collect();
        println!("{{\"error_budget_forecast\":[{}]}}", items.join(","));
    } else if forecasts.is_empty() {
        println!("No error budget forecast data available.");
    } else {
        println!("Machine error budget forecast:");
        for (m, f, t) in &forecasts {
            let remaining = 100.0 - pct(*f, *t);
            println!("  {} — {:.1}% budget remaining ({}/{} failed)", m, remaining, f, t);
        }
    }
    Ok(())
}

fn collect_error_budget_forecasts(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut forecasts = Vec::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        let total = lock.resources.len();
        let failed = lock.resources.values().filter(|r| matches!(r.status, types::ResourceStatus::Failed)).count();
        forecasts.push(((*m).clone(), failed, total));
    }
    forecasts.sort_by(|a, b| a.0.cmp(&b.0));
    forecasts
}

/// FJ-918: Detect lag between dependent resource convergence.
pub(crate) fn cmd_status_machine_resource_dependency_lag(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let lags = collect_dependency_lag(sd, &targets);
    if json {
        let items: Vec<String> = lags.iter()
            .map(|(m, c, f)| format!("{{\"machine\":\"{}\",\"converged\":{},\"failed\":{},\"lag_detected\":{}}}", m, c, f, *f > 0))
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

fn collect_dependency_lag(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut lags = Vec::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        let converged = lock.resources.values().filter(|r| matches!(r.status, types::ResourceStatus::Converged)).count();
        let failed = lock.resources.values().filter(|r| matches!(r.status, types::ResourceStatus::Failed)).count();
        lags.push(((*m).clone(), converged, failed));
    }
    lags.sort_by(|a, b| a.0.cmp(&b.0));
    lags
}

/// FJ-922: Fleet-wide dependency convergence lag analysis.
pub(crate) fn cmd_status_fleet_resource_dependency_lag(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
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
        println!("Fleet dependency lag: {}/{} resources converged ({:.1}% lagging)", total_converged, total, pct(total_failed, total));
    }
    Ok(())
}

/// FJ-924: Rate of configuration drift per machine over time.
pub(crate) fn cmd_status_machine_resource_config_drift_rate(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let rates = collect_config_drift_rates(sd, &targets);
    if json {
        let items: Vec<String> = rates.iter()
            .map(|(m, d, t)| format!("{{\"machine\":\"{}\",\"drifted\":{},\"total\":{},\"drift_rate\":{:.1}}}", m, d, t, pct(*d, *t)))
            .collect();
        println!("{{\"config_drift_rates\":[{}]}}", items.join(","));
    } else if rates.is_empty() {
        println!("No configuration drift rate data available.");
    } else {
        println!("Machine configuration drift rates:");
        for (m, d, t) in &rates { println!("  {} — {}/{} drifted ({:.1}%)", m, d, t, pct(*d, *t)); }
    }
    Ok(())
}

fn collect_config_drift_rates(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut rates = Vec::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        let total = lock.resources.len();
        let drifted = lock.resources.values().filter(|r| matches!(r.status, types::ResourceStatus::Drifted)).count();
        rates.push(((*m).clone(), drifted, total));
    }
    rates.sort_by(|a, b| a.0.cmp(&b.0));
    rates
}
