//! Lock management.

use super::apply_helpers::*;
use super::helpers::*;
use super::workspace::*;
use crate::core::{resolver, state, types};
use std::path::Path;

/// Compare a newly generated lock against an existing lock, collecting mismatches.
fn collect_verify_mismatches(
    machine_name: &str,
    lock: &types::StateLock,
    existing_lock: &types::StateLock,
    mismatches: &mut Vec<String>,
) {
    for (res_id, new_res_lock) in &lock.resources {
        match existing_lock.resources.get(res_id) {
            None => {
                mismatches.push(format!("{}:{}: not in lock file", machine_name, res_id));
            }
            Some(existing_res) => {
                if existing_res.hash != new_res_lock.hash {
                    mismatches.push(format!(
                        "{}:{}: hash mismatch (lock={}, config={})",
                        machine_name,
                        res_id,
                        &existing_res.hash[..15.min(existing_res.hash.len())],
                        &new_res_lock.hash[..15.min(new_res_lock.hash.len())],
                    ));
                }
            }
        }
    }
    // Check for resources in lock that are no longer in config
    for res_id in existing_lock.resources.keys() {
        if !lock.resources.contains_key(res_id) {
            mismatches.push(format!(
                "{}:{}: in lock but not in config",
                machine_name, res_id
            ));
        }
    }
}

/// Output verify results (JSON or text).
fn output_verify_results(
    mismatches: &[String],
    total_machines: usize,
    total_resources: usize,
    json: bool,
) -> Result<(), String> {
    if json {
        let result = serde_json::json!({
            "verified": mismatches.is_empty(),
            "machines": total_machines,
            "resources": total_resources,
            "mismatches": mismatches,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| format!("JSON error: {}", e))?
        );
    } else if mismatches.is_empty() {
        println!(
            "Lock verified: {} machines, {} resources — all hashes match",
            total_machines, total_resources
        );
    } else {
        println!("Lock verification FAILED:");
        for m in mismatches {
            println!("  - {}", m);
        }
    }
    if !mismatches.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

/// Output lock generation results (JSON or text).
fn output_lock_results(
    state_dir: &Path,
    config_name: &str,
    machine_resources: &indexmap::IndexMap<String, Vec<(String, &types::Resource)>>,
    total_machines: usize,
    total_resources: usize,
    json: bool,
) -> Result<(), String> {
    use crate::tripwire::eventlog::now_iso8601;
    let machine_results: Vec<(String, usize, usize, usize)> = machine_resources
        .iter()
        .map(|(name, resources)| (name.clone(), resources.len(), 0, 0))
        .collect();
    state::update_global_lock(state_dir, config_name, &machine_results)?;

    if json {
        let result = serde_json::json!({
            "locked": true,
            "machines": total_machines,
            "resources": total_resources,
            "state_dir": state_dir.display().to_string(),
            "generated_at": now_iso8601(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| format!("JSON error: {}", e))?
        );
    } else {
        println!(
            "Locked: {} machines, {} resources → {}",
            total_machines,
            total_resources,
            state_dir.display()
        );
    }
    Ok(())
}

// FJ-256: forjar lock — generate lock file without applying
pub(crate) fn cmd_lock(
    file: &Path,
    state_dir: &Path,
    env_file: Option<&Path>,
    workspace: Option<&str>,
    verify: bool,
    json: bool,
) -> Result<(), String> {
    use crate::core::planner::hash_desired_state;

    let mut config = parse_and_validate(file)?;
    if let Some(path) = env_file {
        load_env_params(&mut config, path)?;
    }
    inject_workspace_param(&mut config, workspace);
    resolver::resolve_data_sources(&mut config)?;

    let execution_order = resolver::build_execution_order(&config)?;

    // Group resources by machine
    let mut machine_resources: indexmap::IndexMap<String, Vec<(String, &types::Resource)>> =
        indexmap::IndexMap::new();
    for res_id in &execution_order {
        if let Some(resource) = config.resources.get(res_id) {
            let machines = match &resource.machine {
                types::MachineTarget::Single(m) => vec![m.clone()],
                types::MachineTarget::Multiple(ms) => ms.clone(),
            };
            for m in machines {
                machine_resources
                    .entry(m)
                    .or_default()
                    .push((res_id.clone(), resource));
            }
        }
    }

    let mut mismatches: Vec<String> = Vec::new();
    let mut total_resources = 0usize;
    let mut total_machines = 0usize;

    for (machine_name, resources) in &machine_resources {
        let hostname = config
            .machines
            .get(machine_name)
            .map(|m| m.hostname.as_str())
            .unwrap_or(machine_name);

        let mut lock = state::new_lock(machine_name, hostname);

        for (res_id, resource) in resources {
            let hash = hash_desired_state(resource);
            lock.resources.insert(
                res_id.clone(),
                types::ResourceLock {
                    resource_type: resource.resource_type.clone(),
                    status: types::ResourceStatus::Unknown,
                    applied_at: None,
                    duration_seconds: None,
                    hash: hash.clone(),
                    details: std::collections::HashMap::new(),
                },
            );
            total_resources += 1;
        }

        if verify {
            let existing = state::load_lock(state_dir, machine_name)?;
            match existing {
                None => {
                    mismatches.push(format!("{}: no existing lock file", machine_name));
                }
                Some(existing_lock) => {
                    collect_verify_mismatches(machine_name, &lock, &existing_lock, &mut mismatches);
                }
            }
        } else {
            state::save_lock(state_dir, &lock)?;
        }

        total_machines += 1;
    }

    if verify {
        output_verify_results(&mismatches, total_machines, total_resources, json)?;
    } else {
        output_lock_results(
            state_dir,
            &config.name,
            &machine_resources,
            total_machines,
            total_resources,
            json,
        )?;
    }

    Ok(())
}

// FJ-384: Lock file metadata
pub(crate) fn cmd_lock_info(state_dir: &Path, json: bool) -> Result<(), String> {
    let entries =
        std::fs::read_dir(state_dir).map_err(|e| format!("cannot read state dir: {}", e))?;

    let mut machines = Vec::new();
    let mut total_resources = 0usize;

    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(lock) = state::load_lock(state_dir, &name)? {
            total_resources += lock.resources.len();
            machines.push(serde_json::json!({
                "machine": lock.machine,
                "hostname": lock.hostname,
                "schema": lock.schema,
                "generator": lock.generator,
                "generated_at": lock.generated_at,
                "resources": lock.resources.len(),
            }));
        }
    }

    if json {
        let result = serde_json::json!({
            "machines": machines,
            "total_resources": total_resources,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("Lock Info:\n");
        println!("  Total machines: {}", machines.len());
        println!("  Total resources: {}", total_resources);
        for m in &machines {
            println!(
                "\n  {} ({}): {} resources, schema {}, generated {}",
                bold(m["machine"].as_str().unwrap_or("?")),
                m["hostname"].as_str().unwrap_or("?"),
                m["resources"],
                m["schema"].as_str().unwrap_or("?"),
                m["generated_at"].as_str().unwrap_or("?"),
            );
        }
    }

    Ok(())
}

// FJ-366: Lock prune — remove stale lock entries
pub(crate) fn cmd_lock_prune(file: &Path, state_dir: &Path, yes: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let config_resources: std::collections::HashSet<&String> = config.resources.keys().collect();

    let entries =
        std::fs::read_dir(state_dir).map_err(|e| format!("cannot read state dir: {}", e))?;

    let mut pruned = 0usize;
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let machine_name = entry.file_name().to_string_lossy().to_string();
        if let Some(lock) = state::load_lock(state_dir, &machine_name)? {
            let stale: Vec<String> = lock
                .resources
                .keys()
                .filter(|k| !config_resources.contains(k))
                .cloned()
                .collect();

            if stale.is_empty() {
                continue;
            }

            for s in &stale {
                if yes {
                    println!("  {} Pruned '{}' from {}", red("-"), s, machine_name);
                } else {
                    println!(
                        "  {} Would prune '{}' from {} (use --yes to apply)",
                        yellow("~"),
                        s,
                        machine_name
                    );
                }
            }
            pruned += stale.len();
        }
    }

    if pruned == 0 {
        println!("{} No stale lock entries found.", green("✓"));
    } else if !yes {
        println!(
            "\n{} {} stale entries. Run with --yes to prune.",
            yellow("Total:"),
            pruned
        );
    } else {
        println!("\n{} Pruned {} stale entries.", green("✓"), pruned);
    }

    Ok(())
}

/// FJ-596: Validate lock file integrity (schema, hash consistency).
/// Validate a single lock file, returning issues found.
fn validate_single_lock(m: &str, lock: &crate::core::types::StateLock) -> Vec<(String, String)> {
    let mut issues = Vec::new();
    if lock.schema != "1" {
        issues.push((
            m.to_string(),
            format!("unexpected schema version: {}", lock.schema),
        ));
    }
    for (rname, rlock) in &lock.resources {
        if rlock.hash.is_empty() {
            issues.push((m.to_string(), format!("empty hash for resource: {}", rname)));
        }
    }
    issues
}

pub(crate) fn cmd_lock_validate(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut valid = 0u64;
    let mut invalid = 0u64;
    let mut issues: Vec<(String, String)> = Vec::new();

    for m in &machines {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        match serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
            Ok(lock) => {
                let machine_issues = validate_single_lock(m, &lock);
                if machine_issues.is_empty() {
                    valid += 1;
                } else {
                    invalid += 1;
                    issues.extend(machine_issues);
                }
            }
            Err(e) => {
                issues.push((m.clone(), format!("parse error: {}", e)));
                invalid += 1;
            }
        }
    }

    if json {
        let items: Vec<String> = issues
            .iter()
            .map(|(m, msg)| {
                format!(
                    r#"{{"machine":"{}","issue":"{}"}}"#,
                    m,
                    msg.replace('"', "\\\"")
                )
            })
            .collect();
        println!(
            r#"{{"valid":{},"invalid":{},"issues":[{}]}}"#,
            valid,
            invalid,
            items.join(",")
        );
    } else if issues.is_empty() {
        println!("All {} lock files are valid", valid);
    } else {
        println!("Lock validation: {} valid, {} invalid", valid, invalid);
        for (m, msg) in &issues {
            println!("  {} — {}", m, msg);
        }
    }
    Ok(())
}

/// FJ-675: Check lock file structural integrity
pub(crate) fn cmd_lock_integrity(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut valid = 0u64;
    let mut invalid = 0u64;
    let mut issues = Vec::new();

    for m in &machines {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        match serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
            Ok(lock) => {
                if lock.schema != "1" {
                    issues.push(format!(
                        "{}: unexpected schema version '{}'",
                        m, lock.schema
                    ));
                    invalid += 1;
                } else {
                    valid += 1;
                }
            }
            Err(e) => {
                issues.push(format!("{}: parse error — {}", m, e));
                invalid += 1;
            }
        }
    }

    if json {
        println!(
            r#"{{"valid":{},"invalid":{},"issues_count":{}}}"#,
            valid,
            invalid,
            issues.len()
        );
    } else if issues.is_empty() {
        println!("All {} lock files pass integrity check", valid);
    } else {
        println!("Integrity check: {} valid, {} invalid", valid, invalid);
        for issue in &issues {
            println!("  - {}", issue);
        }
    }
    Ok(())
}
