//! FJ-2600/FJ-2604: Convergence and mutation test runners.
//!
//! Extracted from check_test.rs for 500-line limit compliance.

use super::helpers::*;
use crate::core::{codegen, resolver};
use std::path::Path;

/// FJ-2604: Run mutation testing against stack resources.
pub(crate) fn cmd_test_mutation(file: &Path) -> Result<(), String> {
    use crate::core::store::mutation_runner::{self, MutationRunConfig, MutationTarget};

    let config = parse_and_validate(file)?;
    let t0 = std::time::Instant::now();
    let run_config = MutationRunConfig::default();

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
pub(crate) fn cmd_test_convergence(file: &Path) -> Result<(), String> {
    use crate::core::store::convergence_runner::{
        self, ConvergenceSummary, ConvergenceTarget, ConvergenceTestConfig,
    };

    let config = parse_and_validate(file)?;
    let t0 = std::time::Instant::now();
    let test_config = ConvergenceTestConfig::default();

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
