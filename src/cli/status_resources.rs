//! Resource status.

use crate::core::state;
use std::path::Path;
use super::helpers::*;


// ── FJ-497: status --resource-age ──

fn format_age_string(age_secs: u64) -> String {
    if age_secs < 3600 {
        format!("{}m", age_secs / 60)
    } else if age_secs < 86400 {
        format!("{}h", age_secs / 3600)
    } else {
        format!("{}d", age_secs / 86400)
    }
}

fn lock_file_age_secs(lock_path: &Path, now: u64) -> Option<u64> {
    let meta = std::fs::metadata(lock_path).ok()?;
    let modified = meta
        .modified()
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Some(now.saturating_sub(modified))
}

fn collect_resource_ages(
    state_dir: &Path,
    machines: &[String],
    now: u64,
) -> Result<Vec<(String, String, u64, String)>, String> {
    let mut ages = Vec::new();
    for m in machines {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        let age_secs = match lock_file_age_secs(&lock_path, now) {
            Some(a) => a,
            None => continue,
        };
        let age_str = format_age_string(age_secs);
        if let Some(lock) = state::load_lock(state_dir, m).map_err(|e| e.to_string())? {
            for (rname, _rl) in &lock.resources {
                ages.push((m.clone(), rname.clone(), age_secs, age_str.clone()));
            }
        }
    }
    Ok(ages)
}

pub(crate) fn cmd_status_resource_age(
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
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let ages = collect_resource_ages(state_dir, &machines, now)?;
    if json {
        let values: Vec<serde_json::Value> = ages.iter().map(|(m, rname, age_secs, age_str)| {
            serde_json::json!({"machine": m, "resource": rname, "age_secs": age_secs, "age": age_str})
        }).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"resource_ages": values}))
                .unwrap_or_default()
        );
    } else {
        for (m, rname, _age_secs, age_str) in &ages {
            println!("  {} {} — age: {}", m, rname, age_str);
        }
    }
    Ok(())
}


/// FJ-612: Estimate resource cost based on type and count.
pub(crate) fn cmd_status_resource_cost(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut type_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

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
            for (_, rlock) in &lock.resources {
                let rtype = format!("{:?}", rlock.resource_type);
                *type_counts.entry(rtype).or_insert(0) += 1;
            }
        }
    }

    // Simple cost model: weight by resource type complexity
    let total_resources: u64 = type_counts.values().sum();
    let weighted_cost: f64 = type_counts
        .iter()
        .map(|(t, &count)| {
            let weight = match t.as_str() {
                "Package" => 2.0,
                "File" => 1.0,
                "Service" => 3.0,
                "Mount" => 2.5,
                "User" => 2.0,
                "Docker" => 5.0,
                "Network" => 3.0,
                _ => 1.0,
            };
            count as f64 * weight
        })
        .sum();

    if json {
        let items: Vec<String> = type_counts
            .iter()
            .map(|(t, c)| format!(r#"{{"type":"{}","count":{}}}"#, t, c))
            .collect();
        println!(
            r#"{{"resource_types":[{}],"total":{},"complexity_score":{:.1}}}"#,
            items.join(","),
            total_resources,
            weighted_cost
        );
    } else if total_resources == 0 {
        println!("No resources found for cost estimate");
    } else {
        println!("Resource cost estimate (complexity: {:.1}):", weighted_cost);
        for (t, c) in &type_counts {
            println!("  {} — {} resources", t, c);
        }
    }
    Ok(())
}


/// FJ-682: Show estimated resource sizes
pub(crate) fn cmd_status_resource_size(
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
        print!("{{\"resources\":[");
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

        for (rname, rlock) in &lock.resources {
            let hash_len = rlock.hash.len();
            let details_size = rlock.details.len();
            if json {
                if !first {
                    print!(",");
                }
                first = false;
                print!(
                    r#"{{"machine":"{}","resource":"{}","type":"{:?}","details_count":{}}}"#,
                    m, rname, rlock.resource_type, details_size
                );
            } else {
                println!(
                    "{}/{}: type={:?}, hash_len={}, details={}",
                    m, rname, rlock.resource_type, hash_len, details_size
                );
            }
        }
    }
    if json {
        println!("]}}");
    }
    Ok(())
}


/// FJ-562: Show resource dependency graph from live state.
pub(crate) fn cmd_status_resource_graph(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let edges: Vec<(String, String)> = Vec::new();
    let mut nodes: std::collections::HashSet<String> = std::collections::HashSet::new();

    for m in &machines {
        if let Some(filter) = machine {
            if m != filter {
                continue;
            }
        }
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if lock_path.exists() {
            let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
            if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
                for (rname, _rlock) in &lock.resources {
                    let node_id = format!("{}:{}", m, rname);
                    nodes.insert(node_id);
                }
            }
        }
    }

    if json {
        let node_items: Vec<String> = nodes.iter().map(|n| format!(r#""{}""#, n)).collect();
        let edge_items: Vec<String> = edges
            .iter()
            .map(|(from, to)| format!(r#"{{"from":"{}","to":"{}"}}"#, from, to))
            .collect();
        println!(
            r#"{{"nodes":[{}],"edges":[{}],"node_count":{},"edge_count":{}}}"#,
            node_items.join(","),
            edge_items.join(","),
            nodes.len(),
            edges.len()
        );
    } else {
        println!(
            "Resource graph ({} nodes, {} edges):",
            nodes.len(),
            edges.len()
        );
        for (from, to) in &edges {
            println!("  {} → {}", from, to);
        }
    }
    Ok(())
}

