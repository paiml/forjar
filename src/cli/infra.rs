//! Infrastructure utilities.

use super::helpers::*;
use super::helpers_state::*;
use crate::core::{migrate, parser, types};
use std::path::Path;

pub(crate) fn cmd_migrate(file: &Path, output: Option<&Path>) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // Count docker resources
    let docker_count = config
        .resources
        .values()
        .filter(|r| r.resource_type == types::ResourceType::Docker)
        .count();

    if docker_count == 0 {
        println!("No Docker resources found in {}", file.display());
        return Ok(());
    }

    let (migrated, warnings) = migrate::migrate_config(&config);

    // Print warnings
    if !warnings.is_empty() {
        eprintln!("Migration warnings:");
        for w in &warnings {
            eprintln!("  ⚠ {w}");
        }
        eprintln!();
    }

    // Serialize migrated config
    let yaml = serde_yaml_ng::to_string(&migrated)
        .map_err(|e| format!("Failed to serialize migrated config: {e}"))?;

    if let Some(out_path) = output {
        std::fs::write(out_path, &yaml)
            .map_err(|e| format!("Failed to write {}: {}", out_path.display(), e))?;
        println!(
            "Migrated {} Docker resource(s) → pepita in {}",
            docker_count,
            out_path.display()
        );
    } else {
        print!("{yaml}");
    }

    println!(
        "Migration complete: {} resource(s) converted, {} warning(s)",
        docker_count,
        warnings.len()
    );
    Ok(())
}

pub(crate) fn cmd_mcp() -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create tokio runtime: {e}"))?;
    rt.block_on(crate::mcp::serve())
}

pub(crate) fn cmd_mcp_schema() -> Result<(), String> {
    let schema = crate::mcp::export_schema();
    let json = serde_json::to_string_pretty(&schema).map_err(|e| format!("JSON error: {e}"))?;
    println!("{json}");
    Ok(())
}

/// Run inline performance benchmarks (FJ-139).
pub(crate) fn cmd_bench(iterations: usize, json: bool) -> Result<(), String> {
    use crate::core::{planner, resolver, state, types::*};
    use crate::tripwire::{drift as tripwire_drift, hasher};
    use std::time::Instant;

    let bench_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let bench_dir =
        std::env::temp_dir().join(format!("forjar-bench-{}-{}", std::process::id(), bench_id));
    std::fs::create_dir_all(&bench_dir).map_err(|e| format!("cannot create tempdir: {e}"))?;

    // Ensure cleanup on exit
    struct CleanupGuard(std::path::PathBuf);
    impl Drop for CleanupGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _guard = CleanupGuard(bench_dir.clone());
    let dir = bench_dir;

    // Build a realistic 3-machine, 20-resource config
    let mut yaml = String::from(
        "version: \"1.0\"\nname: bench\nmachines:\n  web:\n    hostname: web\n    addr: 10.0.1.1\n  db:\n    hostname: db\n    addr: 10.0.1.2\n  cache:\n    hostname: cache\n    addr: 10.0.1.3\nresources:\n",
    );
    for i in 0..8 {
        yaml.push_str(&format!(
            "  web-pkg-{i}:\n    type: package\n    machine: web\n    provider: apt\n    packages: [pkg-{i}]\n"
        ));
    }
    for i in 0..6 {
        yaml.push_str(&format!(
            "  db-file-{i}:\n    type: file\n    machine: db\n    path: /etc/db/conf-{i}.yml\n    content: \"value-{i}\"\n"
        ));
    }
    for i in 0..4 {
        yaml.push_str(&format!(
            "  cache-svc-{i}:\n    type: service\n    machine: cache\n    name: svc-{i}\n"
        ));
    }
    for i in 0..2 {
        yaml.push_str(&format!(
            "  web-mount-{i}:\n    type: mount\n    machine: web\n    source: /dev/sda{}\n    path: /mnt/data-{i}\n",
            i + 1
        ));
    }

    let config_path = dir.join("forjar.yaml");
    std::fs::write(&config_path, &yaml).map_err(|e| format!("write error: {e}"))?;

    // Build a 100-resource lock file for drift bench
    let state_dir = dir.join("state");
    let mut resources = indexmap::IndexMap::new();
    for i in 0..100 {
        resources.insert(
            format!("file-{i:03}"),
            ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: hasher::hash_string(&format!("content-{i}")),
                details: std::collections::HashMap::new(),
            },
        );
    }
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "bench-host".to_string(),
        hostname: "bench-host".to_string(),
        generated_at: "2026-02-26T00:00:00Z".to_string(),
        generator: "forjar-bench".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };
    state::save_lock(&state_dir, &lock).map_err(|e| format!("lock error: {e}"))?;

    struct BenchResult {
        name: &'static str,
        target: &'static str,
        iterations: usize,
        total_us: u128,
    }

    impl BenchResult {
        fn avg_us(&self) -> f64 {
            self.total_us as f64 / self.iterations as f64
        }

        /// Parse the target string to seconds for comparison.
        fn target_secs(&self) -> f64 {
            let t = self.target.trim_start_matches("< ");
            if let Some(s) = t.strip_suffix("µs") {
                s.trim().parse::<f64>().unwrap_or(1.0) / 1_000_000.0
            } else if let Some(s) = t.strip_suffix("ms") {
                s.trim().parse::<f64>().unwrap_or(1.0) / 1_000.0
            } else if let Some(s) = t.strip_suffix('s') {
                s.trim().parse::<f64>().unwrap_or(1.0)
            } else {
                1.0
            }
        }

        /// Check if the average meets the target.
        fn meets_target(&self) -> bool {
            self.avg_us() / 1_000_000.0 <= self.target_secs()
        }
    }

    let mut results = Vec::new();

    // 1. Validate benchmark
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = parser::parse_and_validate(&config_path)?;
    }
    results.push(BenchResult {
        name: "validate (3m, 20r)",
        target: "< 10ms",
        iterations,
        total_us: start.elapsed().as_micros(),
    });

    // 2. Plan benchmark
    let start = Instant::now();
    for _ in 0..iterations {
        let config = parser::parse_and_validate(&config_path)?;
        let order = resolver::build_execution_order(&config)?;
        let locks = std::collections::HashMap::new();
        let _ = planner::plan(&config, &order, &locks, None);
    }
    results.push(BenchResult {
        name: "plan (3m, 20r)",
        target: "< 2s",
        iterations,
        total_us: start.elapsed().as_micros(),
    });

    // 3. Drift benchmark
    let start = Instant::now();
    for _ in 0..iterations {
        let lock_data =
            state::load_lock(&state_dir, "bench-host")?.ok_or("bench lock not found")?;
        let _ = tripwire_drift::detect_drift(&lock_data);
    }
    results.push(BenchResult {
        name: "drift (100 resources)",
        target: "< 1s",
        iterations,
        total_us: start.elapsed().as_micros(),
    });

    // 4. BLAKE3 hash benchmark
    let data = "x".repeat(4096);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = hasher::hash_string(&data);
    }
    results.push(BenchResult {
        name: "blake3 hash (4KB)",
        target: "< 2µs",
        iterations,
        total_us: start.elapsed().as_micros(),
    });

    // 5. Topo sort benchmark
    let topo_config = parser::parse_and_validate(&config_path)?;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = resolver::build_execution_order(&topo_config)?;
    }
    results.push(BenchResult {
        name: "topo sort (20 nodes)",
        target: "< 100µs",
        iterations,
        total_us: start.elapsed().as_micros(),
    });

    // 6. BLAKE3 hash (1MB) — I/O-heavy workload
    let big_data = "x".repeat(1_048_576);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = hasher::hash_string(&big_data);
    }
    results.push(BenchResult {
        name: "blake3 hash (1MB)",
        target: "< 500µs",
        iterations,
        total_us: start.elapsed().as_micros(),
    });

    if json {
        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.name,
                    "target": r.target,
                    "iterations": r.iterations,
                    "avg_us": r.avg_us(),
                    "total_us": r.total_us,
                    "status": if r.meets_target() { "pass" } else { "fail" },
                })
            })
            .collect();
        let output =
            serde_json::to_string_pretty(&json_results).map_err(|e| format!("JSON: {e}"))?;
        println!("{output}");
    } else {
        use super::colors;
        println!(
            "\n{}",
            colors::header(&format!(
                "Forjar Performance Benchmarks ({iterations} iterations)"
            ))
        );
        println!();
        println!(
            "  {:<28} {:>12} {:>12}   {}",
            colors::bold("Operation"),
            colors::bold("Average"),
            colors::bold("Target"),
            colors::bold("Status"),
        );
        println!("  {}", colors::rule());
        let mut passed = 0usize;
        for r in &results {
            let avg = colors::duration_colored(r.avg_us() / 1_000_000.0, r.target_secs());
            let status = if r.meets_target() {
                passed += 1;
                colors::pass("pass")
            } else {
                colors::fail("FAIL")
            };
            println!("  {:<28} {:>25} {:>12}   {}", r.name, avg, r.target, status,);
        }
        println!("  {}", colors::separator());
        let summary = format!("{}/{} targets met", passed, results.len());
        if passed == results.len() {
            println!("  {}", colors::pass(&summary));
        } else {
            println!("  {}", colors::fail(&summary));
        }
        println!();
    }

    Ok(())
}

pub(crate) fn cmd_state_list(
    state_dir: &Path,
    machine_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    use crate::core::state;

    if !state_dir.exists() {
        if json {
            println!("[]");
        } else {
            println!("No state directory found.");
        }
        return Ok(());
    }

    let machines = list_state_machines(state_dir)?;
    let mut all_rows: Vec<serde_json::Value> = Vec::new();

    for machine_name in &machines {
        if let Some(filter) = machine_filter {
            if machine_name != filter {
                continue;
            }
        }

        let lock = match state::load_lock(state_dir, machine_name) {
            Ok(Some(l)) => l,
            _ => continue,
        };

        for (res_id, res_lock) in &lock.resources {
            all_rows.push(serde_json::json!({
                "machine": lock.machine,
                "resource": res_id,
                "type": res_lock.resource_type.to_string(),
                "status": format!("{:?}", res_lock.status).to_lowercase(),
                "hash": &res_lock.hash[..12.min(res_lock.hash.len())],
                "applied_at": res_lock.applied_at.as_deref().unwrap_or("-"),
            }));
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&all_rows).unwrap_or_else(|_| "[]".to_string())
        );
    } else if all_rows.is_empty() {
        println!("No resources in state.");
    } else {
        println!(
            "{:<15} {:<25} {:<10} {:<10} {:<14} APPLIED AT",
            "MACHINE", "RESOURCE", "TYPE", "STATUS", "HASH"
        );
        for row in &all_rows {
            println!(
                "{:<15} {:<25} {:<10} {:<10} {:<14} {}",
                row["machine"].as_str().unwrap_or("-"),
                row["resource"].as_str().unwrap_or("-"),
                row["type"].as_str().unwrap_or("-"),
                row["status"].as_str().unwrap_or("-"),
                row["hash"].as_str().unwrap_or("-"),
                row["applied_at"].as_str().unwrap_or("-"),
            );
        }
        println!(
            "\n{} resources across {} machines.",
            all_rows.len(),
            all_rows
                .iter()
                .map(|r| r["machine"].as_str().unwrap_or(""))
                .collect::<std::collections::HashSet<_>>()
                .len()
        );
    }

    Ok(())
}

pub(crate) fn cmd_state_mv(
    state_dir: &Path,
    old_id: &str,
    new_id: &str,
    machine_filter: Option<&str>,
) -> Result<(), String> {
    use crate::core::state;

    if old_id == new_id {
        return Err("old and new resource IDs are the same".to_string());
    }

    if !state_dir.exists() {
        return Err("state directory does not exist".to_string());
    }

    let machines = list_state_machines(state_dir)?;
    let mut moved = false;

    for machine_name in &machines {
        if let Some(filter) = machine_filter {
            if machine_name != filter {
                continue;
            }
        }

        let mut lock = match state::load_lock(state_dir, machine_name) {
            Ok(Some(l)) => l,
            _ => continue,
        };

        if !lock.resources.contains_key(old_id) {
            continue;
        }

        if lock.resources.contains_key(new_id) {
            return Err(format!(
                "resource '{}' already exists on machine '{}'",
                new_id, lock.machine
            ));
        }

        // Move the resource entry
        if let Some(resource_lock) = lock.resources.swap_remove(old_id) {
            lock.resources.insert(new_id.to_string(), resource_lock);
        }

        state::save_lock(state_dir, &lock).map_err(|e| format!("failed to save lock: {e}"))?;

        println!(
            "Renamed '{}' → '{}' on machine '{}'",
            old_id, new_id, lock.machine
        );
        moved = true;
    }

    if !moved {
        return Err(format!("resource '{old_id}' not found in state"));
    }

    Ok(())
}

// ============================================================================
// FJ-213: state-rm — remove a resource from state
// ============================================================================

pub(crate) fn cmd_state_rm(
    state_dir: &Path,
    resource_id: &str,
    machine_filter: Option<&str>,
    force: bool,
) -> Result<(), String> {
    use crate::core::state;

    if !state_dir.exists() {
        return Err("state directory does not exist".to_string());
    }

    let machines = list_state_machines(state_dir)?;
    let mut removed = false;

    for machine_name in &machines {
        if let Some(filter) = machine_filter {
            if machine_name != filter {
                continue;
            }
        }

        let mut lock = match state::load_lock(state_dir, machine_name) {
            Ok(Some(l)) => l,
            _ => continue,
        };

        if !lock.resources.contains_key(resource_id) {
            continue;
        }

        // Check for dependents (other resources whose details reference this one)
        if !force {
            let dependents: Vec<String> = lock
                .resources
                .keys()
                .filter(|k| *k != resource_id)
                .filter(|k| {
                    lock.resources[*k]
                        .details
                        .values()
                        .any(|v| v.as_str().map(|s| s.contains(resource_id)).unwrap_or(false))
                })
                .cloned()
                .collect();

            if !dependents.is_empty() {
                return Err(format!(
                    "resource '{}' may be referenced by: {}. Use --force to skip this check.",
                    resource_id,
                    dependents.join(", ")
                ));
            }
        }

        lock.resources.swap_remove(resource_id);

        state::save_lock(state_dir, &lock).map_err(|e| format!("failed to save lock: {e}"))?;

        println!(
            "Removed '{}' from state on machine '{}' (resource still exists on machine)",
            resource_id, lock.machine
        );
        removed = true;
    }

    if !removed {
        return Err(format!("resource '{resource_id}' not found in state"));
    }

    Ok(())
}

// ============================================================================
// FJ-215: output — resolve and display output values
// ============================================================================
