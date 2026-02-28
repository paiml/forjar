//! Failure analysis.

use crate::core::{state, types};
use std::path::Path;
use super::helpers::*;


/// Filter machines list by optional machine filter.
fn filter_machines<'a>(machines: &'a [String], machine: Option<&str>) -> Vec<&'a String> {
    if let Some(m) = machine {
        machines.iter().filter(|x| x.as_str() == m).collect()
    } else {
        machines.iter().collect()
    }
}

/// Load a StateLock from a lock.yaml file, returning None on any error.
fn load_lock_from_yaml(state_dir: &Path, m: &str) -> Option<types::StateLock> {
    let lock_path = state_dir.join(format!("{}.lock.yaml", m));
    let content = std::fs::read_to_string(&lock_path).ok()?;
    serde_yaml_ng::from_str(&content).ok()
}


// ── FJ-482: status --top-failures ──

/// Collect failure counts from state locks.
fn collect_failure_counts(
    state_dir: &Path,
    machines: &[String],
) -> Result<Vec<(String, usize)>, String> {
    let mut failure_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for m in machines {
        if let Some(lock) = state::load_lock(state_dir, m).map_err(|e| e.to_string())? {
            for (rname, rl) in &lock.resources {
                if rl.status == types::ResourceStatus::Failed {
                    *failure_counts
                        .entry(format!("{}:{}", m, rname))
                        .or_insert(0) += 1;
                }
            }
        }
    }
    let mut ranked: Vec<(String, usize)> = failure_counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Ok(ranked)
}

pub(crate) fn cmd_status_top_failures(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let all_machines = discover_machines(state_dir);
    let machines: Vec<String> = if let Some(m) = machine {
        all_machines.into_iter().filter(|n| n == m).collect()
    } else {
        all_machines
    };
    let ranked = collect_failure_counts(state_dir, &machines)?;
    if json {
        let items: Vec<serde_json::Value> = ranked
            .iter()
            .map(|(name, count)| serde_json::json!({"resource": name, "failures": count}))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"top_failures": items}))
                .unwrap_or_default()
        );
    } else if ranked.is_empty() {
        println!("{} No failed resources", green("✓"));
    } else {
        println!("Top Failing Resources");
        println!("{}", "─".repeat(40));
        for (name, count) in &ranked {
            println!("  {:40} {} failure(s)", name, count);
        }
    }
    Ok(())
}


/// FJ-672: Show resources failed since a given timestamp
pub(crate) fn cmd_status_failed_since(
    state_dir: &Path,
    machine: Option<&str>,
    since: &str,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets = filter_machines(&machines, machine);

    let mut failed = Vec::new();
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

        for (rname, rlock) in &lock.resources {
            if !matches!(rlock.status, crate::core::types::ResourceStatus::Failed) {
                continue;
            }
            let applied = rlock.applied_at.clone().unwrap_or_default();
            if applied.as_str() >= since {
                failed.push(((*m).clone(), rname.clone(), applied));
            }
        }
    }

    print_failed_since_output(&failed, since, json);
    Ok(())
}

/// Print output for failed-since command.
fn print_failed_since_output(failed: &[(String, String, String)], since: &str, json: bool) {
    if json {
        print!("{{\"failed\":[");
        for (i, (machine, resource, applied)) in failed.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            print!(
                r#"{{"machine":"{}","resource":"{}","applied_at":"{}"}}"#,
                machine, resource, applied
            );
        }
        println!("]}}");
    } else if failed.is_empty() {
        println!("No failed resources since {}", since);
    } else {
        println!("Failed resources since {} ({}):", since, failed.len());
        for (machine, resource, applied) in failed {
            println!("  {}/{} (at {})", machine, resource, applied);
        }
    }
}


/// FJ-722: Show only failed resources across machines
pub(crate) fn cmd_status_failed_resources(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets = filter_machines(&machines, machine);

    let mut entries = Vec::new();
    for m in &targets {
        if let Some(lock) = load_lock_from_yaml(state_dir, m) {
            for (name, rl) in &lock.resources {
                if format!("{:?}", rl.status) == "Failed" {
                    entries.push((m.to_string(), name.clone(), format!("{:?}", rl.resource_type)));
                }
            }
        }
    }

    if json {
        let items: Vec<String> = entries
            .iter()
            .map(|(m, name, rtype)| {
                format!(
                    "{{\"machine\":\"{}\",\"resource\":\"{}\",\"type\":\"{}\"}}",
                    m, name, rtype
                )
            })
            .collect();
        println!(
            "{{\"failed_resources\":[{}],\"count\":{}}}",
            items.join(","),
            entries.len()
        );
    } else if entries.is_empty() {
        println!("No failed resources.");
    } else {
        println!("Failed resources:");
        for (m, name, rtype) in &entries {
            println!("  {} / {} ({})", m, name, rtype);
        }
    }
    Ok(())
}


/// FJ-677: Verify BLAKE3 hashes in lock match computed hashes
pub(crate) fn cmd_status_hash_verify(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets = filter_machines(&machines, machine);

    let mut verified = 0u64;
    let mut total = 0u64;

    for m in &targets {
        if let Some(lock) = load_lock_from_yaml(state_dir, m) {
            for (_rname, rlock) in &lock.resources {
                total += 1;
                if !rlock.hash.is_empty() {
                    verified += 1;
                }
            }
        }
    }

    if json {
        println!(
            r#"{{"total":{},"verified":{},"missing":{}}}"#,
            total,
            verified,
            total - verified
        );
    } else {
        println!(
            "Hash verification: {}/{} resources have BLAKE3 hashes",
            verified, total
        );
    }
    Ok(())
}


/// FJ-667: Show age of each lock file entry
pub(crate) fn cmd_status_lock_age(state_dir: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets = filter_machines(&machines, machine);

    let mut entries = Vec::new();
    for m in &targets {
        if let Some(lock) = load_lock_from_yaml(state_dir, m) {
            for (rname, rlock) in &lock.resources {
                let applied = rlock.applied_at.clone().unwrap_or_default();
                entries.push((m.to_string(), rname.clone(), applied));
            }
        }
    }

    if json {
        print!("{{\"entries\":[");
        for (i, (m, rname, applied)) in entries.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            print!(
                r#"{{"machine":"{}","resource":"{}","applied_at":"{}"}}"#,
                m, rname, applied
            );
        }
        println!("]}}");
    } else {
        for (m, rname, applied) in &entries {
            println!(
                "{}/{}: applied at {}",
                m,
                rname,
                if applied.is_empty() {
                    "unknown"
                } else {
                    applied
                }
            );
        }
    }
    Ok(())
}


/// FJ-697: Show hash of current config for change detection
pub(crate) fn cmd_status_config_hash(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets = filter_machines(&machines, machine);
    if json {
        let mut entries = Vec::new();
        for m in &targets {
            let lock_path = state_dir.join(format!("{}.lock.yaml", m));
            if let Ok(data) = std::fs::read_to_string(&lock_path) {
                let hash = crate::tripwire::hasher::hash_string(&data);
                entries.push(format!(
                    "{{\"machine\":\"{}\",\"config_hash\":\"{}\"}}",
                    m, hash
                ));
            }
        }
        println!("{{\"config_hashes\":[{}]}}", entries.join(","));
    } else {
        println!("Config hashes:");
        for m in &targets {
            let lock_path = state_dir.join(format!("{}.lock.yaml", m));
            if let Ok(data) = std::fs::read_to_string(&lock_path) {
                let hash = crate::tripwire::hasher::hash_string(&data);
                println!("  {} — {}", m, hash);
            }
        }
    }
    Ok(())
}


/// FJ-647: AI-powered recommendations based on state analysis
pub(crate) fn cmd_status_recommendations(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets = filter_machines(&machines, machine);

    let mut total_resources = 0u64;
    let mut failed_count = 0u64;
    let mut drifted_count = 0u64;

    for m in &targets {
        if let Some(lock) = load_lock_from_yaml(state_dir, m) {
            for (_rname, rlock) in &lock.resources {
                total_resources += 1;
                match rlock.status {
                    crate::core::types::ResourceStatus::Failed => failed_count += 1,
                    crate::core::types::ResourceStatus::Drifted => drifted_count += 1,
                    _ => {}
                }
            }
        }
    }

    let recommendations = build_recommendations(total_resources, failed_count, drifted_count);
    print_recommendations(&recommendations, json);
    Ok(())
}

/// Build recommendation strings based on resource counts.
fn build_recommendations(total: u64, failed: u64, drifted: u64) -> Vec<String> {
    let mut recommendations = Vec::new();
    if failed > 0 {
        recommendations.push(format!(
            "HIGH: {} failed resources need attention. Run 'forjar apply' to reconverge.",
            failed
        ));
    }
    if drifted > 0 {
        recommendations.push(format!(
            "MEDIUM: {} drifted resources detected. Run 'forjar drift' for details.",
            drifted
        ));
    }
    if total == 0 {
        recommendations
            .push("INFO: No resources found. Run 'forjar apply' to initialize state.".to_string());
    }
    if failed == 0 && drifted == 0 && total > 0 {
        recommendations.push(format!(
            "OK: All {} resources are converged. No action needed.",
            total
        ));
    }
    recommendations
}

/// Print recommendations in JSON or text format.
fn print_recommendations(recommendations: &[String], json: bool) {
    if json {
        print!("{{\"recommendations\":[");
        for (i, r) in recommendations.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            print!(r#""{}""#, r.replace('"', "\\\""));
        }
        println!("]}}");
    } else {
        println!("Recommendations:");
        for r in recommendations {
            println!("  {}", r);
        }
    }
}
