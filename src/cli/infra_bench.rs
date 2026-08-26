//! Inline performance benchmarks (FJ-139).

use super::infra_bench_baseline as baseline;
use crate::core::{parser, planner, resolver, state, types::*};
use crate::tripwire::{drift as tripwire_drift, hasher};
use std::time::Instant;

struct BenchResult {
    name: &'static str,
    target: &'static str,
    iterations: usize,
    total_us: u128,
    samples: Vec<u128>,
}

impl BenchResult {
    fn avg_us(&self) -> f64 {
        self.total_us as f64 / self.iterations as f64
    }

    fn p50_us(&self) -> f64 {
        percentile(&self.samples, 50)
    }

    fn p95_us(&self) -> f64 {
        percentile(&self.samples, 95)
    }

    /// Parse the target string to seconds for comparison.
    fn target_secs(&self) -> f64 {
        parse_target_secs(self.target)
    }

    /// Check if the average meets the target.
    fn meets_target(&self) -> bool {
        self.avg_us() / 1_000_000.0 <= self.target_secs()
    }
}

fn parse_target_secs(target: &str) -> f64 {
    let t = target.trim_start_matches("< ");
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

fn percentile(samples: &[u128], pct: u32) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let idx = ((pct as f64 / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)] as f64
}

/// Run inline performance benchmarks (FJ-139).
///
/// Dogfood #208: this is the STRICT entry point — unmet targets and baseline
/// regressions become a non-zero exit. `cmd_bench_with_writer` stays
/// informational so the crate's own tests can run it on a debug build without
/// asserting release-tuned latency budgets.
pub(crate) fn cmd_bench(iterations: usize, json: bool, compare: bool) -> Result<(), String> {
    let outcome =
        cmd_bench_with_writer(iterations, json, compare, &mut super::output::StdoutWriter)?;
    baseline::bench_verdict(&outcome)
}

/// Inner bench with injectable OutputWriter (FJ-2920).
pub(crate) fn cmd_bench_with_writer(
    iterations: usize,
    json: bool,
    compare: bool,
    out: &mut dyn super::output::OutputWriter,
) -> Result<baseline::BenchOutcome, String> {
    cmd_bench_at(
        iterations,
        json,
        compare,
        &baseline::default_baseline_path(),
        out,
    )
}

/// Bench with an explicit baseline path (testable without changing cwd).
pub(crate) fn cmd_bench_at(
    iterations: usize,
    json: bool,
    compare: bool,
    baseline_path: &std::path::Path,
    out: &mut dyn super::output::OutputWriter,
) -> Result<baseline::BenchOutcome, String> {
    // Dogfood #208: `--iterations 0` produced `NaNµs` rows, `0/6 targets met`
    // and rc=0. clap now rejects 0 at parse time; this is the library-level
    // backstop for callers that bypass clap.
    if iterations == 0 {
        return Err("bench: --iterations must be >= 1".to_string());
    }

    let (dir, _guard) = create_bench_dir()?;
    let config_path = setup_bench_config(&dir)?;
    let state_dir = setup_bench_state(&dir)?;

    let results = run_benchmarks(&config_path, &state_dir, iterations)?;

    // Dogfood #208: refuse rather than silently skipping the comparison.
    let baseline = if compare {
        Some(baseline::load_baseline_from(baseline_path)?)
    } else {
        None
    };

    if json {
        render_bench_json(&results, &baseline, out)?;
    } else {
        render_bench_table(&results, iterations, &baseline, out)?;
    }
    Ok(compute_outcome(&results, &baseline))
}

/// Derive the pass tally and baseline regressions from a completed run.
fn compute_outcome(
    results: &[BenchResult],
    baseline: &Option<Vec<baseline::BaselineEntry>>,
) -> baseline::BenchOutcome {
    let passed = results.iter().filter(|r| r.meets_target()).count();
    let mut regressions = Vec::new();
    if let Some(entries) = baseline {
        for r in results {
            if let Some(b) = entries.iter().find(|b| b.name == r.name) {
                if b.avg_us <= 0.0 {
                    continue;
                }
                let delta = (r.avg_us() - b.avg_us) / b.avg_us * 100.0;
                if delta > baseline::REGRESSION_PCT {
                    regressions.push(format!("{} +{:.0}%", r.name, delta));
                }
            }
        }
    }
    baseline::BenchOutcome {
        passed,
        total: results.len(),
        regressions,
    }
}

struct CleanupGuard(std::path::PathBuf);
impl Drop for CleanupGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn create_bench_dir() -> Result<(std::path::PathBuf, CleanupGuard), String> {
    let bench_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("forjar-bench-{}-{}", std::process::id(), bench_id));
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create tempdir: {e}"))?;
    let guard = CleanupGuard(dir.clone());
    Ok((dir, guard))
}

fn setup_bench_config(dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
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
    Ok(config_path)
}

fn setup_bench_state(dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
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
                observed: None,
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
    Ok(state_dir)
}

fn run_benchmarks(
    config_path: &std::path::Path,
    state_dir: &std::path::Path,
    iterations: usize,
) -> Result<Vec<BenchResult>, String> {
    let mut results = Vec::new();

    // 1. Validate
    let mut samples = Vec::with_capacity(iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let s = Instant::now();
        let _ = parser::parse_and_validate(config_path)?;
        samples.push(s.elapsed().as_micros());
    }
    results.push(BenchResult {
        name: "validate (3m, 20r)",
        target: "< 10ms",
        iterations,
        total_us: start.elapsed().as_micros(),
        samples,
    });

    // 2. Plan
    let mut samples = Vec::with_capacity(iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let s = Instant::now();
        let config = parser::parse_and_validate(config_path)?;
        let order = resolver::build_execution_order(&config)?;
        let locks = std::collections::HashMap::new();
        let _ = planner::plan(&config, &order, &locks, None);
        samples.push(s.elapsed().as_micros());
    }
    results.push(BenchResult {
        name: "plan (3m, 20r)",
        target: "< 2s",
        iterations,
        total_us: start.elapsed().as_micros(),
        samples,
    });

    // 3. Drift
    let mut samples = Vec::with_capacity(iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let s = Instant::now();
        let lock_data = state::load_lock(state_dir, "bench-host")?.ok_or("bench lock not found")?;
        let _ = tripwire_drift::detect_drift(&lock_data);
        samples.push(s.elapsed().as_micros());
    }
    results.push(BenchResult {
        name: "drift (100 resources)",
        target: "< 1s",
        iterations,
        total_us: start.elapsed().as_micros(),
        samples,
    });

    // 4. BLAKE3 hash (4KB)
    let data = "x".repeat(4096);
    let mut samples = Vec::with_capacity(iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let s = Instant::now();
        let _ = hasher::hash_string(&data);
        samples.push(s.elapsed().as_micros());
    }
    results.push(BenchResult {
        name: "blake3 hash (4KB)",
        target: "< 2µs",
        iterations,
        total_us: start.elapsed().as_micros(),
        samples,
    });

    // 5. Topo sort
    let topo_config = parser::parse_and_validate(config_path)?;
    let mut samples = Vec::with_capacity(iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let s = Instant::now();
        let _ = resolver::build_execution_order(&topo_config)?;
        samples.push(s.elapsed().as_micros());
    }
    results.push(BenchResult {
        name: "topo sort (20 nodes)",
        target: "< 100µs",
        iterations,
        total_us: start.elapsed().as_micros(),
        samples,
    });

    // 6. BLAKE3 hash (1MB)
    let big_data = "x".repeat(1_048_576);
    let mut samples = Vec::with_capacity(iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let s = Instant::now();
        let _ = hasher::hash_string(&big_data);
        samples.push(s.elapsed().as_micros());
    }
    results.push(BenchResult {
        name: "blake3 hash (1MB)",
        target: "< 500µs",
        iterations,
        total_us: start.elapsed().as_micros(),
        samples,
    });

    Ok(results)
}

fn format_us(us: f64) -> String {
    if us >= 1_000_000.0 {
        format!("{:.2}s", us / 1_000_000.0)
    } else if us >= 1000.0 {
        format!("{:.1}ms", us / 1000.0)
    } else {
        format!("{:.1}µs", us)
    }
}

fn render_bench_json(
    results: &[BenchResult],
    baseline: &Option<Vec<baseline::BaselineEntry>>,
    out: &mut dyn super::output::OutputWriter,
) -> Result<(), String> {
    let json_results: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            let mut v = serde_json::json!({
                "name": r.name,
                "target": r.target,
                "iterations": r.iterations,
                "avg_us": r.avg_us(),
                "p50_us": r.p50_us(),
                "p95_us": r.p95_us(),
                "total_us": r.total_us,
                "status": if r.meets_target() { "pass" } else { "fail" },
            });
            if let Some(bl) = baseline {
                if let Some(b) = bl.iter().find(|b| b.name == r.name) {
                    let delta_pct = (r.avg_us() - b.avg_us) / b.avg_us * 100.0;
                    v["baseline_avg_us"] = serde_json::json!(b.avg_us);
                    v["delta_pct"] = serde_json::json!(delta_pct);
                }
            }
            v
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
    baseline: &Option<Vec<baseline::BaselineEntry>>,
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
    let has_baseline = baseline.is_some();
    if has_baseline {
        out.result(&format!(
            "  {:<28} {:>12} {:>8} {:>8} {:>12} {:>8}   {}",
            colors::bold("Operation"),
            colors::bold("Average"),
            colors::bold("p50"),
            colors::bold("p95"),
            colors::bold("Target"),
            colors::bold("Delta"),
            colors::bold("Status"),
        ));
    } else {
        out.result(&format!(
            "  {:<28} {:>12} {:>8} {:>8} {:>12}   {}",
            colors::bold("Operation"),
            colors::bold("Average"),
            colors::bold("p50"),
            colors::bold("p95"),
            colors::bold("Target"),
            colors::bold("Status"),
        ));
    }
    out.result(&format!("  {}", colors::rule()));
    let mut passed = 0usize;
    for r in results {
        let avg = colors::duration_colored(r.avg_us() / 1_000_000.0, r.target_secs());
        let p50 = format_us(r.p50_us());
        let p95 = format_us(r.p95_us());
        let status = if r.meets_target() {
            passed += 1;
            colors::pass("pass")
        } else {
            colors::fail("FAIL")
        };
        if has_baseline {
            let delta_str = baseline
                .as_ref()
                .and_then(|bl| bl.iter().find(|b| b.name == r.name))
                .map(|b| {
                    let d = (r.avg_us() - b.avg_us) / b.avg_us * 100.0;
                    colors::delta_lower_is_better(d)
                })
                .unwrap_or_else(|| "—".to_string());
            out.result(&format!(
                "  {:<28} {:>25} {:>8} {:>8} {:>12} {:>8}   {}",
                r.name, avg, p50, p95, r.target, delta_str, status,
            ));
        } else {
            out.result(&format!(
                "  {:<28} {:>25} {:>8} {:>8} {:>12}   {}",
                r.name, avg, p50, p95, r.target, status,
            ));
        }
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
