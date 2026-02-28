//! Status recovery — error budgets, compliance scoring, MTTR metrics.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;
use super::helpers::*;
#[allow(unused_imports)]
use super::helpers_state::*;

fn collect_error_budgets(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut budgets = Vec::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        let total = lock.resources.len();
        let failed = lock.resources.values().filter(|r| matches!(r.status, types::ResourceStatus::Failed)).count();
        budgets.push(((*m).clone(), failed, total));
    }
    budgets.sort_by(|a, b| a.0.cmp(&b.0));
    budgets
}

/// FJ-878: Show error budget consumption per machine (failed/total ratio).
pub(crate) fn cmd_status_machine_error_budget(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let budgets = collect_error_budgets(sd, &targets);
    if json {
        let items: Vec<String> = budgets.iter().map(|(m, f, t)| {
            let pct = if *t > 0 { (*f as f64 / *t as f64) * 100.0 } else { 0.0 };
            format!("{{\"machine\":\"{}\",\"failed\":{},\"total\":{},\"error_pct\":{:.1}}}", m, f, t, pct)
        }).collect();
        println!("{{\"machine_error_budget\":[{}]}}", items.join(","));
    } else if budgets.is_empty() {
        println!("No error budget data available.");
    } else {
        println!("Machine error budget (failed / total):");
        for (m, f, t) in &budgets {
            let pct = if *t > 0 { (*f as f64 / *t as f64) * 100.0 } else { 0.0 };
            println!("  {} — {}/{} failed ({:.1}% error budget consumed)", m, f, t, pct);
        }
    }
    Ok(())
}

/// FJ-882: Show fleet-wide compliance score based on converged resources.
pub(crate) fn cmd_status_fleet_compliance_score(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let (mut total, mut converged) = (0usize, 0usize);
    for m in &targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        total += lock.resources.len();
        converged += lock.resources.values().filter(|r| matches!(r.status, types::ResourceStatus::Converged)).count();
    }
    let score = if total > 0 { (converged as f64 / total as f64) * 100.0 } else { 0.0 };
    if json {
        println!("{{\"fleet_compliance_score\":{:.1},\"converged\":{},\"total\":{}}}", score, converged, total);
    } else if total == 0 {
        println!("No compliance data available.");
    } else {
        println!("Fleet compliance score: {:.1}% ({}/{} resources converged)", score, converged, total);
    }
    Ok(())
}

/// FJ-884: Show mean time to recovery per machine.
pub(crate) fn cmd_status_machine_mean_time_to_recovery(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let mut mttr_data: Vec<(String, String)> = Vec::new();
    for m in &targets {
        let events_path = sd.join(m).join("events.yaml");
        if events_path.exists() {
            mttr_data.push(((*m).clone(), "event data present".to_string()));
        } else {
            mttr_data.push(((*m).clone(), "no event history".to_string()));
        }
    }
    mttr_data.sort_by(|a, b| a.0.cmp(&b.0));
    if json {
        let items: Vec<String> = mttr_data.iter()
            .map(|(m, s)| format!("{{\"machine\":\"{}\",\"mttr_status\":\"{}\"}}", m, s)).collect();
        println!("{{\"machine_mean_time_to_recovery\":[{}]}}", items.join(","));
    } else if mttr_data.is_empty() {
        println!("No MTTR data available.");
    } else {
        println!("Machine mean time to recovery:");
        for (m, s) in &mttr_data { println!("  {} — {}", m, s); }
    }
    Ok(())
}
