//! Example: Convergence verification with sandbox integration.
//!
//! Demonstrates apply → verify → re-apply → verify idempotency cycle.
//! Run with: `cargo run --example convergence_runner`

use forjar::core::store::convergence_runner::{
    self, ConvergenceSummary, ConvergenceTarget, ConvergenceTestConfig,
};
use forjar::core::types::SandboxBackend;

fn main() {
    println!("=== Convergence Verification (FJ-2600/FJ-2603) ===\n");

    // Show backend detection
    let config = ConvergenceTestConfig::default();
    let mode = convergence_runner::resolve_mode(config.backend);
    println!("Backend: {} (mode: {mode})", config.backend);
    println!("Pepita available: {}", convergence_runner::backend_available(SandboxBackend::Pepita));
    println!("Docker available: {}", convergence_runner::backend_available(SandboxBackend::Container));
    println!();

    // 1. Create test targets
    let targets: Vec<ConvergenceTarget> = vec![
        make_target("nginx-config", "file"),
        make_target("curl-pkg", "package"),
        make_target("nginx-service", "service"),
        make_target("data-mount", "mount"),
    ];

    // 2. Run convergence tests (parallel, 2 sandboxes)
    println!("Running {} convergence tests (parallelism=2)...\n", targets.len());
    let results = convergence_runner::run_convergence_parallel(targets, 2);

    // 3. Print report
    print!("{}", convergence_runner::format_convergence_report(&results));

    // 4. Show summary
    let summary = ConvergenceSummary::from_results(&results);
    println!("\n{summary}");
    println!("Pass rate: {:.0}%", summary.pass_rate());
}

fn make_target(id: &str, rtype: &str) -> ConvergenceTarget {
    let query_script = format!("echo 'state of {id}'");
    let expected_hash = {
        let refs = [query_script.as_str()];
        forjar::tripwire::hasher::composite_hash(&refs)
    };
    ConvergenceTarget {
        resource_id: id.into(),
        resource_type: rtype.into(),
        apply_script: format!("echo 'applying {id}'"),
        state_query_script: query_script,
        expected_hash,
    }
}
