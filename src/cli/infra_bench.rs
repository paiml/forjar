//! Inline performance benchmarks (FJ-139).

use crate::core::{parser, planner, resolver, state, types::*};
use crate::tripwire::{drift as tripwire_drift, hasher};
use std::time::Instant;

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

/// Run inline performance benchmarks (FJ-139).
pub(crate) fn cmd_bench(iterations: usize, json: bool) -> Result<(), String> {
    cmd_bench_with_writer(iterations, json, &mut super::output::StdoutWriter)
}

/// Inner bench with injectable OutputWriter (FJ-2920).
pub(crate) fn cmd_bench_with_writer(
    iterations: usize,
    json: bool,
    out: &mut dyn super::output::OutputWriter,
) -> Result<(), String> {
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
        render_bench_json(&results, out)
    } else {
        render_bench_table(&results, iterations, out)
    }
}

fn render_bench_json(
    results: &[BenchResult],
    out: &mut dyn super::output::OutputWriter,
) -> Result<(), String> {
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
    let output = serde_json::to_string_pretty(&json_results).map_err(|e| format!("JSON: {e}"))?;
    out.result(&output);
    out.flush();
    Ok(())
}

fn render_bench_table(
    results: &[BenchResult],
    iterations: usize,
    out: &mut dyn super::output::OutputWriter,
) -> Result<(), String> {
    use super::colors;
    out.result(&format!(
        "\n{}",
        colors::header(&format!(
            "Forjar Performance Benchmarks ({iterations} iterations)"
        ))
    ));
    out.result("");
    out.result(&format!(
        "  {:<28} {:>12} {:>12}   {}",
        colors::bold("Operation"),
        colors::bold("Average"),
        colors::bold("Target"),
        colors::bold("Status"),
    ));
    out.result(&format!("  {}", colors::rule()));
    let mut passed = 0usize;
    for r in results {
        let avg = colors::duration_colored(r.avg_us() / 1_000_000.0, r.target_secs());
        let status = if r.meets_target() {
            passed += 1;
            colors::pass("pass")
        } else {
            colors::fail("FAIL")
        };
        out.result(&format!(
            "  {:<28} {:>25} {:>12}   {}",
            r.name, avg, r.target, status,
        ));
    }
    out.result(&format!("  {}", colors::separator()));
    let summary = format!("{}/{} targets met", passed, results.len());
    if passed == results.len() {
        out.success(&summary);
    } else {
        out.error(&summary);
    }
    out.result("");
    out.flush();
    Ok(())
}
