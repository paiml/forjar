//! Status count metrics — convergence, failed, drift counts.

use super::helpers::*;
use super::status_resource_detail::tally_machine_health;
use std::path::Path;

/// FJ-750: Show convergence percentage per machine.
pub(crate) fn cmd_status_convergence_percentage(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let data: Vec<_> = targets
        .iter()
        .map(|m| {
            let (t, c, _, _) = tally_machine_health(state_dir, m);
            let pct = if t > 0 { c * 100 / t } else { 100 };
            (m.to_string(), pct, c, t)
        })
        .collect();
    if json {
        let items: Vec<String> = data
            .iter()
            .map(|(m, p, c, t)| {
                format!(
                    "{{\"machine\":\"{m}\",\"converged_pct\":{p},\"converged\":{c},\"total\":{t}}}"
                )
            })
            .collect();
        println!("{{\"convergence\":[{}]}}", items.join(","));
    } else {
        for (m, pct, c, t) in &data {
            println!("  {m} — {pct}% ({c}/{t})");
        }
    }
    Ok(())
}

/// Shared helper for count-per-machine metrics.
fn print_count_metric(
    sd: &Path,
    targets: &[&String],
    json: bool,
    label: &str,
    extract: fn((usize, usize, usize, usize)) -> usize,
) -> Result<(), String> {
    let data: Vec<_> = targets
        .iter()
        .map(|m| (m.to_string(), extract(tally_machine_health(sd, m))))
        .collect();
    if json {
        let items: Vec<String> = data
            .iter()
            .map(|(m, c)| format!("{{\"machine\":\"{m}\",\"{label}\":{c}}}"))
            .collect();
        println!("{{\"{}_counts\":[{}]}}", label, items.join(","));
    } else {
        for (m, c) in &data {
            println!("  {m} — {c} {label}");
        }
    }
    Ok(())
}

/// FJ-754: Show failed count per machine.
pub(crate) fn cmd_status_failed_count(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    print_count_metric(state_dir, &targets, json, "failed", |t| t.2)
}

/// FJ-756: Show drifted count per machine.
pub(crate) fn cmd_status_drift_count(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    print_count_metric(state_dir, &targets, json, "drifted", |t| t.3)
}
