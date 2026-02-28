//! Fleet status.

use crate::core::types;
use std::path::Path;
use super::helpers::*;


/// FJ-572: Aggregated fleet summary across all machines.
pub(crate) fn cmd_status_fleet_overview(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut total_resources = 0usize;
    let mut total_converged = 0usize;
    let mut total_failed = 0usize;
    let mut total_drifted = 0usize;
    let mut machine_count = 0usize;

    for m in &machines {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
            machine_count += 1;
            for (_rname, rlock) in &lock.resources {
                total_resources += 1;
                match rlock.status {
                    crate::core::types::ResourceStatus::Converged => total_converged += 1,
                    crate::core::types::ResourceStatus::Failed => total_failed += 1,
                    crate::core::types::ResourceStatus::Drifted => total_drifted += 1,
                    _ => {}
                }
            }
        }
    }

    if json {
        println!(
            r#"{{"fleet":{{"machines":{},"resources":{},"converged":{},"failed":{},"drifted":{}}}}}"#,
            machine_count, total_resources, total_converged, total_failed, total_drifted
        );
    } else {
        println!("Fleet overview:");
        println!("  Machines: {}", machine_count);
        println!(
            "  Resources: {} (converged: {}, failed: {}, drifted: {})",
            total_resources, total_converged, total_failed, total_drifted
        );
        if total_resources > 0 {
            let health =
                (total_converged as f64 / total_resources as f64 * 100.0).clamp(0.0, 100.0);
            println!("  Fleet health: {:.0}%", health);
        }
    }
    Ok(())
}


/// FJ-577: Per-machine health details with resource breakdown.
fn tally_machine_lock(lock: &crate::core::types::StateLock) -> (usize, usize, usize, usize) {
    let mut converged = 0usize;
    let mut failed = 0usize;
    let mut drifted = 0usize;
    for rlock in lock.resources.values() {
        match rlock.status {
            crate::core::types::ResourceStatus::Converged => converged += 1,
            crate::core::types::ResourceStatus::Failed => failed += 1,
            crate::core::types::ResourceStatus::Drifted => drifted += 1,
            _ => {}
        }
    }
    (lock.resources.len(), converged, failed, drifted)
}

fn collect_machine_health_reports(
    state_dir: &Path,
    machines: &[String],
    machine: Option<&str>,
) -> Vec<(String, usize, usize, usize, usize)> {
    let mut reports = Vec::new();
    for m in machines {
        if let Some(filter) = machine {
            if m != filter { continue; }
        }
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            reports.push((m.clone(), 0, 0, 0, 0));
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
            let (total, converged, failed, drifted) = tally_machine_lock(&lock);
            reports.push((m.clone(), total, converged, failed, drifted));
        }
    }
    reports
}

fn machine_health_pct(total: usize, converged: usize) -> f64 {
    if total > 0 { (converged as f64 / total as f64 * 100.0).clamp(0.0, 100.0) } else { 100.0 }
}

pub(crate) fn cmd_status_machine_health(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let reports = collect_machine_health_reports(state_dir, &machines, machine);

    if json {
        let items: Vec<String> = reports.iter()
            .map(|(m, t, c, f, d)| {
                let health = machine_health_pct(*t, *c);
                format!(r#"{{"machine":"{}","total":{},"converged":{},"failed":{},"drifted":{},"health":{:.0}}}"#, m, t, c, f, d, health)
            })
            .collect();
        println!(r#"{{"machine_health":[{}]}}"#, items.join(","));
    } else {
        println!("Machine health:");
        for (m, total, converged, failed, drifted) in &reports {
            let health = machine_health_pct(*total, *converged);
            println!(
                "  {} — {:.0}% ({} resources: {} converged, {} failed, {} drifted)",
                m, health, total, converged, failed, drifted
            );
        }
    }
    Ok(())
}


/// FJ-657: Per-machine resource count and health summary
pub(crate) fn cmd_status_machine_summary(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = if let Some(m) = machine {
        machines.iter().filter(|x| x.as_str() == m).collect()
    } else {
        machines.iter().collect()
    };

    if json {
        print!("{{\"machines\":[");
    }
    let mut first = true;
    for m in &targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        let lock: crate::core::types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };

        let total = lock.resources.len();
        let converged = lock
            .resources
            .values()
            .filter(|r| matches!(r.status, crate::core::types::ResourceStatus::Converged))
            .count();
        let failed = lock
            .resources
            .values()
            .filter(|r| matches!(r.status, crate::core::types::ResourceStatus::Failed))
            .count();
        let drifted = lock
            .resources
            .values()
            .filter(|r| matches!(r.status, crate::core::types::ResourceStatus::Drifted))
            .count();

        if json {
            if !first {
                print!(",");
            }
            first = false;
            print!(
                r#"{{"machine":"{}","total":{},"converged":{},"failed":{},"drifted":{}}}"#,
                m, total, converged, failed, drifted
            );
        } else {
            let health = if failed > 0 {
                "UNHEALTHY"
            } else if drifted > 0 {
                "DRIFTED"
            } else {
                "HEALTHY"
            };
            println!(
                "{}: {} resources ({} converged, {} failed, {} drifted) [{}]",
                m, total, converged, failed, drifted, health
            );
        }
    }
    if json {
        println!("]}}");
    }
    Ok(())
}


/// FJ-547: Executive summary — one-line per machine summary.
pub(crate) fn cmd_status_executive_summary(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut summaries: Vec<(String, usize, usize, usize, usize)> = Vec::new();

    for m in &machines {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };

        let total = lock.resources.len();
        let conv = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Converged)
            .count();
        let fail = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Failed)
            .count();
        let drift = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Drifted)
            .count();

        summaries.push((m.clone(), total, conv, fail, drift));
    }

    if json {
        let entries: Vec<String> = summaries
            .iter()
            .map(|(m, t, c, f, d)| {
                format!(
                    r#"{{"machine":"{}","total":{},"converged":{},"failed":{},"drifted":{}}}"#,
                    m, t, c, f, d
                )
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else if summaries.is_empty() {
        println!("No machine state found.");
    } else {
        for (m, total, conv, fail, drift) in &summaries {
            let status = if *fail > 0 {
                red("FAIL")
            } else if *drift > 0 {
                yellow("DRIFT")
            } else {
                green("OK")
            };
            println!(
                "  [{}] {} — {}/{} converged, {} failed, {} drifted",
                status, m, conv, total, fail, drift
            );
        }
    }
    Ok(())
}


/// FJ-622: Show CI/CD pipeline integration status.
pub(crate) fn cmd_status_pipeline_status(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut statuses: Vec<(String, String, String)> = Vec::new(); // (machine, last_apply, status)

    for m in &machines {
        if let Some(filter) = machine {
            if m != filter {
                continue;
            }
        }
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
            let total = lock.resources.len();
            let converged = lock
                .resources
                .values()
                .filter(|r| r.status == crate::core::types::ResourceStatus::Converged)
                .count();
            let status = if converged == total {
                "green"
            } else {
                "yellow"
            };
            statuses.push((m.clone(), lock.generated_at.clone(), status.to_string()));
        }
    }

    if json {
        let items: Vec<String> = statuses
            .iter()
            .map(|(m, ts, s)| {
                format!(
                    r#"{{"machine":"{}","last_apply":"{}","pipeline":"{}"}}"#,
                    m, ts, s
                )
            })
            .collect();
        println!(
            r#"{{"pipeline_statuses":[{}],"count":{}}}"#,
            items.join(","),
            statuses.len()
        );
    } else if statuses.is_empty() {
        println!("No pipeline status available");
    } else {
        println!("Pipeline status ({} machines):", statuses.len());
        for (m, ts, s) in &statuses {
            println!("  {} — {} (last: {})", m, s, ts);
        }
    }
    Ok(())
}

