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
    println!(
        "Pepita available: {}",
        convergence_runner::backend_available(SandboxBackend::Pepita)
    );
    println!(
        "Docker available: {}",
        convergence_runner::backend_available(SandboxBackend::Container)
    );
    println!();

    // Create test targets with sandbox-safe scripts.
    // Apply creates files in $FORJAR_SANDBOX, query reads them back.
    // expected_hash is empty → convergence verified by idempotency only.
    let targets: Vec<ConvergenceTarget> = vec![
        make_target("nginx-config", "file", "port: 80"),
        make_target("app-config", "file", "host: localhost"),
        make_target("db-config", "file", "pool: 10"),
        make_target("cache-config", "file", "ttl: 3600"),
    ];

    println!(
        "Running {} convergence tests (parallelism=2)...\n",
        targets.len()
    );
    let results = convergence_runner::run_convergence_parallel(targets, 2);

    print!(
        "{}",
        convergence_runner::format_convergence_report(&results)
    );

    let summary = ConvergenceSummary::from_results(&results);
    println!("\n{summary}");
    println!("Pass rate: {:.0}%", summary.pass_rate());
}

fn make_target(id: &str, rtype: &str, content: &str) -> ConvergenceTarget {
    // Apply: create a config file in the sandbox
    let apply_script = format!(
        r#"mkdir -p "$FORJAR_SANDBOX/etc/forjar" && echo '{content}' > "$FORJAR_SANDBOX/etc/forjar/{id}""#
    );
    // Query: read the file back
    let query_script = format!(
        r#"cat "$FORJAR_SANDBOX/etc/forjar/{id}" 2>/dev/null || echo 'MISSING'"#
    );
    ConvergenceTarget {
        resource_id: id.into(),
        resource_type: rtype.into(),
        apply_script,
        state_query_script: query_script,
        expected_hash: String::new(), // verify idempotency, not exact hash
    }
}
