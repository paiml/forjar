//! FJ-2600/FJ-2604: Convergence, mutation, and behavior test runners.
//!
//! Extracted from check_test.rs for 500-line limit compliance.

use super::check_test::check_verify_assertions;
use super::helpers::*;
use crate::core::types::SandboxBackend;
use crate::core::{codegen, resolver};
use std::path::Path;

/// Options passed from CLI to test runners.
pub(crate) struct RunnerOpts {
    pub sandbox: SandboxBackend,
    pub parallel: usize,
    pub pairs: bool,
    pub mutations: usize,
}

impl Default for RunnerOpts {
    fn default() -> Self {
        Self {
            sandbox: SandboxBackend::Pepita,
            parallel: 4,
            pairs: false,
            mutations: 50,
        }
    }
}

impl RunnerOpts {
    pub fn from_args(sandbox: &str, parallel: usize, pairs: bool, mutations: usize) -> Self {
        let sandbox = match sandbox {
            "container" => SandboxBackend::Container,
            "chroot" => SandboxBackend::Chroot,
            _ => SandboxBackend::Pepita,
        };
        Self {
            sandbox,
            parallel,
            pairs,
            mutations,
        }
    }
}

/// FJ-2604: Run mutation testing against stack resources.
pub(crate) fn cmd_test_mutation(file: &Path, opts: &RunnerOpts) -> Result<(), String> {
    use crate::core::store::mutation_runner::{self, MutationRunConfig, MutationTarget};

    let config = parse_and_validate(file)?;
    let t0 = std::time::Instant::now();
    let run_config = MutationRunConfig {
        backend: opts.sandbox,
        mutations_per_resource: opts.mutations,
        parallelism: opts.parallel,
        ..MutationRunConfig::default()
    };

    let mode = crate::core::store::convergence_runner::resolve_mode(run_config.backend);
    println!("Mutation Test Runner (mode: {mode})");
    println!("====================");
    println!(
        "Stack: {} ({} resources)\n",
        config.name,
        config.resources.len()
    );

    let execution_order = resolver::build_execution_order(&config)?;
    let targets: Vec<MutationTarget> = execution_order
        .iter()
        .filter_map(|rid| {
            let r = config.resources.get(rid)?;
            let resolved =
                resolver::resolve_resource_templates(r, &config.params, &config.machines).ok()?;
            let script = codegen::apply_script(&resolved).ok()?;
            let rtype = format!("{:?}", r.resource_type).to_lowercase();
            let refs = [script.as_str()];
            let hash = crate::tripwire::hasher::composite_hash(&refs);
            Some(MutationTarget {
                resource_id: rid.clone(),
                resource_type: rtype,
                apply_script: script,
                drift_script: codegen::check_script(&resolved).unwrap_or_default(),
                expected_hash: hash,
            })
        })
        .collect();

    println!(
        "Targets: {} resources with applicable operators\n",
        targets.len()
    );

    let report = mutation_runner::run_mutation_parallel(targets, &run_config);
    let elapsed = t0.elapsed();

    print!("{}", mutation_runner::format_mutation_run(&report));
    println!("Completed in {:.1}s", elapsed.as_secs_f64());

    // Grade F with errors = scripts require real infra (expected in local mode).
    // Grade F with zero errors = mutations genuinely undetected (real failure).
    if report.score.grade() == 'F' && report.score.errored == 0 {
        Err(format!(
            "mutation score {:.0}% (grade F)",
            report.score.score_pct()
        ))
    } else {
        Ok(())
    }
}

/// FJ-2600: Run convergence testing against stack resources.
pub(crate) fn cmd_test_convergence(file: &Path, opts: &RunnerOpts) -> Result<(), String> {
    use crate::core::store::convergence_runner::{
        self, ConvergenceSummary, ConvergenceTarget, ConvergenceTestConfig,
    };

    let config = parse_and_validate(file)?;
    let t0 = std::time::Instant::now();
    let test_config = ConvergenceTestConfig {
        parallelism: opts.parallel,
        test_pairs: opts.pairs,
        backend: opts.sandbox,
        ..ConvergenceTestConfig::default()
    };

    let mode = convergence_runner::resolve_mode(test_config.backend);
    println!("Convergence Test Runner (mode: {mode})");
    println!("===================================");
    println!(
        "Stack: {} ({} resources)\n",
        config.name,
        config.resources.len()
    );

    let execution_order = resolver::build_execution_order(&config)?;
    let targets: Vec<ConvergenceTarget> = execution_order
        .iter()
        .filter_map(|rid| {
            let r = config.resources.get(rid)?;
            let resolved =
                resolver::resolve_resource_templates(r, &config.params, &config.machines).ok()?;
            let apply = codegen::apply_script(&resolved).ok()?;
            let check = codegen::check_script(&resolved).unwrap_or_default();
            let rtype = format!("{:?}", r.resource_type).to_lowercase();
            // Empty expected_hash: convergence verified by idempotency
            // (state-after-first == state-after-second), not by comparing
            // against a pre-computed hash, because scripts may require
            // real machines/packages that aren't available locally.
            Some(ConvergenceTarget {
                resource_id: rid.clone(),
                resource_type: rtype,
                apply_script: apply,
                state_query_script: check,
                expected_hash: String::new(),
            })
        })
        .collect();

    println!("Targets: {} resources\n", targets.len());

    let results = convergence_runner::run_convergence_parallel_with_backend(
        targets,
        test_config.parallelism,
        test_config.backend,
    );
    let summary = ConvergenceSummary::from_results(&results);
    let elapsed = t0.elapsed();

    print!(
        "{}",
        convergence_runner::format_convergence_report(&results)
    );
    println!("Completed in {:.1}s", elapsed.as_secs_f64());

    // Distinguish real failures (scripts ran but state diverged) from
    // environment errors (scripts can't run locally — need real machines).
    let env_errors = results.iter().filter(|r| r.error.is_some()).count();
    if summary.passed + env_errors < summary.total {
        Err(format!(
            "{} convergence failure(s)",
            summary.total - summary.passed - env_errors
        ))
    } else {
        Ok(())
    }
}

/// Discover resources referenced by `.spec.yaml` behavior specs in a directory.
fn discover_spec_resources(spec_dir: &Path) -> std::collections::HashSet<String> {
    let mut result = std::collections::HashSet::new();
    let entries = match std::fs::read_dir(spec_dir) {
        Ok(e) => e,
        Err(_) => return result,
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".spec.yaml") {
            continue;
        }
        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let spec: crate::core::types::BehaviorSpec = match serde_yaml_ng::from_str(&content) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for r in spec.referenced_resources() {
            result.insert(r.to_string());
        }
    }
    result
}

/// FJ-2602: Resource-level coverage report.
pub(crate) fn cmd_test_coverage(file: &Path) -> Result<(), String> {
    use crate::core::types::{CoverageLevel, CoverageReport, ResourceCoverage};

    let config = parse_and_validate(file)?;
    let spec_dir = file.parent().unwrap_or(Path::new("."));
    let spec_resources = discover_spec_resources(spec_dir);

    let execution_order = resolver::build_execution_order(&config)?;
    let entries: Vec<ResourceCoverage> = execution_order
        .iter()
        .filter_map(|rid| {
            let resource = config.resources.get(rid)?;
            let resolved =
                resolver::resolve_resource_templates(resource, &config.params, &config.machines)
                    .unwrap_or_else(|_| resource.clone());
            let has_check = codegen::check_script(&resolved).is_ok();
            let has_spec = spec_resources.contains(rid);
            let level = match (has_spec, has_check) {
                (true, true) => CoverageLevel::L2,
                (_, true) => CoverageLevel::L1,
                _ => CoverageLevel::L0,
            };
            let rtype = format!("{:?}", resource.resource_type).to_lowercase();
            Some(ResourceCoverage {
                resource_id: rid.clone(),
                level,
                resource_type: rtype,
            })
        })
        .collect();

    let report = CoverageReport::from_entries(entries);
    println!("Resource Coverage Report");
    println!("========================");
    for entry in &report.resources {
        println!(
            "  {}: {} ({})",
            entry.resource_id,
            entry.level.label(),
            entry.resource_type
        );
    }
    println!(
        "\nMin: {}, Avg: {:.1}, L0: {}, L1: {}, L2: {}",
        report.min_level.label(),
        report.avg_level,
        report.histogram[0],
        report.histogram[1],
        report.histogram[2]
    );
    Ok(())
}

// ── Behavior test runner (extracted from check_test.rs) ──

fn execute_behavior(
    b: &crate::core::types::BehaviorEntry,
) -> crate::core::types::BehaviorResult {
    use crate::core::types::BehaviorResult;
    let bt0 = std::time::Instant::now();

    if let Some(ref verify) = b.verify {
        let output = std::process::Command::new("bash")
            .args(["-euo", "pipefail", "-c", &verify.command])
            .output();
        let elapsed_ms = bt0.elapsed().as_millis() as u64;
        match output {
            Ok(out) => {
                let code = out.status.code().unwrap_or(-1);
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let failure = check_verify_assertions(verify, code, &stdout, &stderr);
                BehaviorResult {
                    name: b.name.clone(),
                    passed: failure.is_none(),
                    failure,
                    actual_exit_code: Some(code),
                    actual_stdout: Some(stdout),
                    duration_ms: elapsed_ms,
                }
            }
            Err(e) => BehaviorResult {
                name: b.name.clone(),
                passed: false,
                failure: Some(format!("exec error: {e}")),
                actual_exit_code: None,
                actual_stdout: None,
                duration_ms: elapsed_ms,
            },
        }
    } else if b.assert_state.is_some() || b.is_convergence() {
        BehaviorResult {
            name: b.name.clone(),
            passed: true,
            failure: None,
            actual_exit_code: None,
            actual_stdout: None,
            duration_ms: bt0.elapsed().as_millis() as u64,
        }
    } else {
        BehaviorResult {
            name: b.name.clone(),
            passed: false,
            failure: Some("no assertion defined".into()),
            actual_exit_code: None,
            actual_stdout: None,
            duration_ms: 0,
        }
    }
}

pub(crate) fn cmd_test_behavior(file: &Path) -> Result<(), String> {
    use crate::core::types::{BehaviorReport, BehaviorResult, BehaviorSpec};

    let spec_dir = file.parent().unwrap_or(Path::new("."));
    let t0 = std::time::Instant::now();

    println!("Behavior Test Runner");
    println!("====================\n");

    let mut specs: Vec<BehaviorSpec> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(spec_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".spec.yaml") {
                let content = std::fs::read_to_string(entry.path())
                    .map_err(|e| format!("read {name}: {e}"))?;
                let spec: BehaviorSpec = serde_yaml_ng::from_str(&content)
                    .map_err(|e| format!("parse {name}: {e}"))?;
                println!("Loaded: {name} ({} behaviors)", spec.behavior_count());
                specs.push(spec);
            }
        }
    }

    if specs.is_empty() {
        println!("No .spec.yaml files found in {}", spec_dir.display());
        println!("Create behavior specs to define expected system state.");
        return Ok(());
    }

    let mut total_pass = 0usize;
    let mut total_fail = 0usize;
    for spec in &specs {
        let results: Vec<BehaviorResult> =
            spec.behaviors.iter().map(execute_behavior).collect();
        let report = BehaviorReport::from_results(spec.name.clone(), results);
        total_pass += report.passed;
        total_fail += report.failed;
        print!("{}", report.format_summary());
    }

    let elapsed = t0.elapsed();
    println!(
        "\n{} spec(s), {} behavior(s): {} passed, {} failed ({:.1}s)",
        specs.len(),
        total_pass + total_fail,
        total_pass,
        total_fail,
        elapsed.as_secs_f64()
    );

    if total_fail > 0 {
        Err(format!("{total_fail} behavior(s) failed"))
    } else {
        Ok(())
    }
}
